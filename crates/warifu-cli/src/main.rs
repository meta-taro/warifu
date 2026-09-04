//! warifu を**画面なしで**使う口。
//!
//! 画面（Tauri）は人が押すためのもので、**押せない相手**——別の機械で動いている
//! エージェント、CI、遠隔の端末——からは使えない。
//! ここは同じ層（`warifu-net` / `warifu-meeting` / `warifu-app`）を、
//! **標準入出力だけ**で動かす。
//!
//! ```text
//! # 待つ側（会議キーを出す）
//! warifu host
//!
//! # 入る側
//! warifu join '<会議キー>'
//! ```
//!
//! つないだ後は、**打った行がそのまま相手へ飛び、届いた行がそのまま出る。**
//! だから `echo` でも `tail -f` でも使える。
//!
//! **映像は扱わない。**それは画面（WebView の WebRTC）の担当で、
//! ここが引き受けるのは**文字だけ**である。

#![forbid(unsafe_code)]

use std::process::ExitCode;

use tokio::io::{AsyncBufReadExt, BufReader};

use warifu_app::{Conference, format_invite, is_own_invite, parse_invite};
use warifu_core::{Device, PublicKey, Revocations};
use warifu_intent::Channel;
use warifu_meeting::{MeetingId, Notice, Roster};
use warifu_net::{Address, Node};
use warifu_vault::Vault;

mod identity;

/// 会議キーの既定の有効期間（秒）。画面側と揃えてある。
///
/// **`--ttl <秒>` で伸ばせる。**相手が建てている間に切れると、
/// 渡した鍵が使えなくなって最初からやり直しになる（2026-09-04 に実際に切れた）。
const KEY_TTL_SECS: u64 = 600;
/// 相手が割符へ応じるのを待つ限度。**黙って繋いだだけの相手に待ち受けを塞がせない。**
const HANDSHAKE_SECS: u64 = 10;

/// 何も来ない時間がこれを超えたら終わる（`--idle <秒>` で指定したときだけ）。
///
/// **既定では終わらない。**会話は黙っている時間のほうが長く、
/// 黙ったら切られるチャットは使いものにならない。
/// **一往復だけ確かめたいとき**にだけ使う口である。
const IDLE_DEFAULT: Option<u64> = None;

fn 使い方() -> ExitCode {
    eprintln!(
        "warifu — 画面なしで会議に入る\n\
         \n\
         使い方:\n\
         \x20 warifu host [--ttl <秒>] [--from <時刻>] [--until <時刻>]\n\
         \x20            [--idle <秒>] [--remember <呼び名>]\n\
         \x20            待つ。会議キーを標準出力へ出す\n\
         \x20 warifu join <会議キー> [--idle <秒>] [--remember <呼び名>]\n\
         \x20            入る\n\
         \x20 warifu id   自分の公開鍵と、身元の置き場所を出す\n\
         \x20 warifu help この使い方を出す\n\
         \x20 warifu contacts                       覚えた相手を並べる\n\
         \x20 warifu contacts add <公開鍵> <呼び名>  覚える\n\
         \x20 warifu contacts forget <呼び名|公開鍵> 忘れる\n\
         \n\
         つないだ後は、打った行が相手へ飛び、届いた行がそのまま出ます。\n\
         \n\
         --ttl      会議キーの有効期間（既定 600 秒）。相手が建てている間に切れないように\n\
         --from     会議の開始。この時刻までは誰も入れません（予定に紐づく鍵）\n\
         --until    会議の終わり。--ttl より優先します\n\
         \x20          時刻は Unix 秒か +<秒>（いまから）。例: --from +3600 --until +7200\n\
         --idle     何も来ない時間がその秒数を超えたら終わる。付けなければ終わりません\n\
         \x20          （会話は黙っている時間のほうが長いため）\n\
         --remember つながった相手を、その呼び名で覚える\n\
         \n\
         身元はこの端末に残ります。閉じても同じ人でいられます（warifu id で確認）。\n\
         映像は扱いません（それは画面の担当です）。"
    );
    ExitCode::from(2)
}

/// 呼び出しに付いてきた指定。
#[derive(Debug)]
struct Options {
    /// 何も来ない時間がこれを超えたら終わる。**無ければ終わらない。**
    idle: Option<u64>,
    /// 会議キーの有効期間（秒）。
    ttl: u64,
    /// 会議の開始（Unix 秒）。**無ければ「いま」から。**
    from: Option<u64>,
    /// 会議の終わり（Unix 秒）。指定があれば `ttl` より優先する。
    until: Option<u64>,
    /// つながった相手を、この呼び名で覚える。
    remember: Option<String>,
}

/// 秒数を人が読める形にする。
///
/// **unix 秒をそのまま人に見せない。**`1788506979` と出しても、いつなのか分からない
/// （2026-09-04 に別の機械の担当から指摘された）。
///
/// 日時（`2026-09-11 10:00`）に直さないのは、**時間帯の変換をここでやると
/// 「1 時間ずれた表示」が黙って出る**ため。相対なら時間帯が要らない。
fn 間隔を言う(secs: u64) -> String {
    if secs == 0 {
        return "いま".to_owned();
    }
    let (h, m, s) = (secs / 3600, (secs % 3600) / 60, secs % 60);
    let mut parts = Vec::new();
    if h > 0 {
        parts.push(format!("{h} 時間"));
    }
    if m > 0 {
        parts.push(format!("{m} 分"));
    }
    if s > 0 || parts.is_empty() {
        parts.push(format!("{s} 秒"));
    }
    parts.join(" ")
}

/// 時刻の指定を読む。`+<秒>` は「いまから」、数字だけなら Unix 秒。
///
/// 人が打つ日時（`2026-09-11 10:00`）を受けないのは、時間帯の扱いを間違えると
/// 「1 時間ずれた鍵」が黙って出るためである。予定表の側が秒で渡す形にしてある。
/// 引数の読み取りで起きること。**黙って捨てない。**
#[derive(Debug)]
pub enum OptionError {
    /// 知らない引数。打ち間違いをここで止める。
    Unknown(String),
    /// 値の要る引数に値が付いていない。
    Missing(&'static str),
    /// 値が読めない。**既定に落とさない**（指定したつもりと違う鍵が出る）。
    BadValue {
        /// どの引数か。
        arg: &'static str,
        /// 何が来たか。
        got: String,
    },
    /// 使い方を求められた。
    WantsHelp,
}

impl std::fmt::Display for OptionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unknown(a) => write!(f, "知らない引数です: {a}"),
            Self::Missing(a) => write!(f, "{a} に値が付いていません"),
            Self::BadValue { arg, got } => write!(f, "{arg} の値を読めません: {got}"),
            Self::WantsHelp => f.write_str("使い方"),
        }
    }
}

impl std::error::Error for OptionError {}

fn 読む_時刻(text: &str, now: u64) -> Option<u64> {
    let t = text.trim();
    if let Some(rest) = t.strip_prefix('+') {
        return rest.parse::<u64>().ok().map(|d| now.saturating_add(d));
    }
    t.parse::<u64>().ok()
}

/// 引数を読む。
///
/// **知らない引数を黙って捨てない。**捨てると `--form +60` のような打ち間違いが
/// そのまま通り、**指定したつもりの窓が付いていない鍵**が出る（2026-09-04 に指摘）。
fn 読む_options(args: &mut impl Iterator<Item = String>) -> Result<Options, OptionError> {
    let now = now_secs();
    let mut o = Options {
        idle: IDLE_DEFAULT,
        ttl: KEY_TTL_SECS,
        from: None,
        until: None,
        remember: None,
    };

    /// 値を 1 つ取り出す。無ければ断る。
    fn 値(
        args: &mut impl Iterator<Item = String>,
        arg: &'static str,
    ) -> Result<String, OptionError> {
        args.next().ok_or(OptionError::Missing(arg))
    }

    while let Some(a) = args.next() {
        match a.as_str() {
            "--help" | "-h" | "help" => return Err(OptionError::WantsHelp),
            "--idle" => {
                let v = 値(args, "--idle")?;
                o.idle = Some(v.parse().map_err(|_| OptionError::BadValue {
                    arg: "--idle",
                    got: v.clone(),
                })?);
            }
            "--ttl" => {
                let v = 値(args, "--ttl")?;
                o.ttl = v.parse().map_err(|_| OptionError::BadValue {
                    arg: "--ttl",
                    got: v.clone(),
                })?;
            }
            "--from" => {
                let v = 値(args, "--from")?;
                o.from = Some(読む_時刻(&v, now).ok_or(OptionError::BadValue {
                    arg: "--from",
                    got: v.clone(),
                })?);
            }
            "--until" => {
                let v = 値(args, "--until")?;
                o.until = Some(読む_時刻(&v, now).ok_or(OptionError::BadValue {
                    arg: "--until",
                    got: v.clone(),
                })?);
            }
            "--remember" => o.remember = Some(値(args, "--remember")?),
            other => return Err(OptionError::Unknown(other.to_owned())),
        }
    }
    Ok(o)
}

/// **終わるときに、標準入力の読み取りを待たない。**
///
/// `tokio::io::stdin` は専用のブロッキングスレッドで `read(2)` を呼ぶ。
/// `#[tokio::main]` はランタイムを畳むときにブロッキング処理の完了を待つため、
/// 標準入力が端末や**開いたままのパイプ**だと read が返らず、**プロセスが終わらない**。
///
/// 実測 2026-09-04: 相手が落ちて `経路で落ちました` を出した `warifu host` が
/// **55 分そのまま残り**、標準入力へ 1 行流し込むまで終了しなかった。
/// 人から見ると「落ちたのに終わっていない」——次の待ち受けを建てたつもりが二重になる。
fn 走らせる<F: std::future::Future<Output = ExitCode>>(仕事: F) -> ExitCode {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio ランタイムを作れませんでした");
    let code = runtime.block_on(仕事);
    // **止まったままのブロッキング処理を待たない。**待つと標準入力に縛られる
    runtime.shutdown_timeout(std::time::Duration::ZERO);
    code
}

fn main() -> ExitCode {
    走らせる(本体())
}

async fn 本体() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let 命令 = args.next();
    let result = match 命令.as_deref() {
        Some("host") => match 読む_options(&mut args) {
            Ok(o) => 待つ(&o).await,
            Err(OptionError::WantsHelp) => return 使い方(),
            Err(e) => Err(e.into()),
        },
        Some("join") => match args.next() {
            // **鍵の位置に --help が来たら使い方を出す。**鍵として読もうとしない
            Some(k) if matches!(k.as_str(), "--help" | "-h" | "help") => return 使い方(),
            Some(key) => match 読む_options(&mut args) {
                Ok(o) => 入る(&key, &o).await,
                Err(OptionError::WantsHelp) => return 使い方(),
                Err(e) => Err(e.into()),
            },
            None => return 使い方(),
        },
        Some("id") => 名乗る(),
        Some("contacts") => 名簿の口(&mut args),
        _ => return 使い方(),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("warifu: {e}");
            ExitCode::FAILURE
        }
    }
}

/// **閉じても同じ身元でいる。**
///
/// 以前は起動のたびに作り直していた（D2 が未決のため）。
/// だが**平常時の置き場所は、全部失ったときの戻し方とは別の話**である（D42）。
/// 毎回別人になると、相手は「同じ人」だと分からず、連絡先が成立しない。
fn 身元() -> Result<(Vault, Device), Box<dyn std::error::Error>> {
    Ok(identity::開く()?)
}

/// 自分の公開鍵と、身元の置き場所を出す。**相手に渡すのはこの鍵。**
fn 名乗る() -> Result<(), Box<dyn std::error::Error>> {
    let (vault, device) = 身元()?;
    // 鍵は標準出力へ（`warifu id | pbcopy` が使えるように）。説明は標準エラーへ
    eprintln!("warifu: 身元の置き場所 {}", vault.dir().display());
    println!("{}", device.public_key());
    Ok(())
}

/// 覚えた相手を扱う口。
fn 名簿の口(args: &mut impl Iterator<Item = String>) -> Result<(), Box<dyn std::error::Error>> {
    let (vault, _) = 身元()?;
    let mut contacts = vault.contacts()?;
    if contacts.skipped() > 0 {
        eprintln!("warifu: 読めない行を {} 行とばしました", contacts.skipped());
    }

    match args.next().as_deref() {
        None | Some("list") => {
            if contacts.is_empty() {
                eprintln!(
                    "warifu: まだ誰も覚えていません（`warifu host --remember <呼び名>` で覚えます）"
                );
                return Ok(());
            }
            for c in contacts.iter() {
                println!("{}\t{}", c.label(), c.key());
            }
            Ok(())
        }
        Some("add") => {
            let (Some(鍵), Some(呼び名)) = (args.next(), args.next()) else {
                return Err("使い方: warifu contacts add <公開鍵> <呼び名>".into());
            };
            let key: PublicKey = 鍵.trim().parse().map_err(|_| "公開鍵として読めません")?;
            contacts.add(key, &呼び名, now_secs())?;
            vault.save_contacts(&contacts)?;
            eprintln!("warifu: 覚えました: {呼び名}");
            Ok(())
        }
        Some("forget") => {
            let Some(言葉) = args.next() else {
                return Err("使い方: warifu contacts forget <呼び名|公開鍵>".into());
            };
            let Some(key) = identity::相手を引く(&contacts, &言葉) else {
                return Err(format!("覚えていません: {言葉}").into());
            };
            if !contacts.remove(key) {
                return Err(format!("覚えていません: {言葉}").into());
            }
            vault.save_contacts(&contacts)?;
            eprintln!("warifu: 忘れました: {言葉}");
            Ok(())
        }
        Some(other) => Err(format!("知らない指定です: {other}").into()),
    }
}

/// つながった相手を覚える。**呼び名を指定されたときだけ。**
///
/// 黙って覚えると、一度きりのつもりだった相手が名簿に残る。
fn 覚える(vault: &Vault, peer: PublicKey, 呼び名: Option<&String>) {
    let Some(呼び名) = 呼び名 else { return };
    let 結果 = vault.contacts().and_then(|mut c| {
        c.add(peer, 呼び名, now_secs())?;
        vault.save_contacts(&c)?;
        Ok(())
    });
    match 結果 {
        Ok(()) => eprintln!("warifu: 覚えました: {呼び名}"),
        // 覚えられなくても会話は続く。**黙って落とさない**
        Err(e) => eprintln!("warifu: 覚えられませんでした（会話は続きます）: {e}"),
    }
}

/// 相手が誰かを言う。覚えていれば呼び名で。
fn 誰か(vault: &Vault, peer: PublicKey) -> String {
    match vault.contacts() {
        Ok(c) => identity::呼び名(&c, peer),
        Err(_) => peer.to_string(),
    }
}

async fn 待つ(o: &Options) -> Result<(), Box<dyn std::error::Error>> {
    let (vault, device) = 身元()?;

    // **開始と終わりを先に決める。**`--from` を付けたときだけ、いまより後ろから始まる
    let 開始 = o.from.unwrap_or_else(now_secs);
    let 終わり = o.until.unwrap_or_else(|| 開始.saturating_add(o.ttl));
    let ttl = 終わり.saturating_sub(now_secs());
    let node = Node::bind_without_relay(&device).await?;
    let address = node.address().await?.to_string();

    let mut conference = Conference::host(device.public_key(), warifu_app::DEFAULT_CAPACITY)?;
    let (mut tally, token) = device.issue_tally_between(開始, 終わり)?;

    // **会議キーは標準出力へ。**進行の知らせは標準エラーへ分ける。
    // こうしておくと `warifu host | pbcopy` のように使える
    let いま = now_secs();
    if o.from.is_some() {
        eprintln!(
            "warifu: 待っています。開始まで {}（{開始}）／終わりまで {}（{終わり}）",
            間隔を言う(開始.saturating_sub(いま)),
            間隔を言う(終わり.saturating_sub(いま))
        );
        eprintln!("warifu: 始まるまでは、鍵を渡した相手でも入れません");
    } else {
        eprintln!(
            "warifu: 待っています。{}で会議キーが切れます（{終わり}）",
            間隔を言う(ttl)
        );
    }
    println!("{}", format_invite(&address, &token, conference.id()));

    // **来るまで待ち続ける。**
    //
    // `accept` は下の層の都合で時間切れになることがある（実測: timed out）。
    // 「待っています」と言った以上、**こちらの都合で勝手に諦めない。**
    // `--idle` は繋がった後の話であって、**繋がる前の待ち時間ではない**。
    //
    // **割符が合わない相手が来ても、そこで終わらない**（実測 2026-09-04）。
    // 予定に紐づく鍵では、**始まる前に一度叩かれただけで待ち受けが落ちていた。**
    // 主催は会議が始まるまで待っていなければならない。落ちてよいのは鍵が切れたときだけ。
    let (session, peer) = loop {
        // 会議キーが切れていたら、待っていても意味が無い
        if now_secs() > token.not_after() {
            return Err("会議キーの期限が切れました。作り直してください".into());
        }

        let mut session = match node.accept(&Revocations::new()).await {
            Ok(s) => s,
            Err(e) => {
                eprintln!("warifu: 待ち直します（{e}）");
                continue;
            }
        };
        let peer = session.peer();

        // **割符を先に確かめる。**会議の話をする前に、通してよいかを決める（D31 / D39）
        let 応答 = tokio::time::timeout(
            std::time::Duration::from_secs(HANDSHAKE_SECS),
            session.recv(),
        )
        .await;

        let 結果 = match 応答 {
            Err(_) => Err("相手が割符に応じませんでした".to_owned()),
            Ok(Err(e)) => Err(e.to_string()),
            Ok(Ok(bytes)) => warifu_core::Acceptance::from_bytes(&bytes)
                .and_then(|a| tally.match_half(&a, now_secs(), &Revocations::new()))
                .map_err(|e| e.to_string()),
        };

        match 結果 {
            Ok(_) => break (session, peer),
            // **理由は主催の手元にだけ出す。**相手には返さない（戸口の構え・D31）
            Err(why) => eprintln!("warifu: 通しませんでした（{why}）。待ち直します"),
        }
    };

    eprintln!(
        "warifu: 割符が合いました。つながっています（{}）",
        誰か(&vault, peer)
    );
    覚える(&vault, peer, o.remember.as_ref());

    let channel = Channel::new(session);
    やり取り(channel, &mut conference, peer, o.idle).await
}

async fn 入る(key: &str, o: &Options) -> Result<(), Box<dyn std::error::Error>> {
    let (vault, device) = 身元()?;
    let (address, token, meeting) = parse_invite(key)?;
    if is_own_invite(device.public_key(), &token) {
        return Err("自分の会議キーです。相手に渡してください".into());
    }

    // **呼ぶ前に窓を見る。**始まっていない・切れている鍵で相手を叩かない
    // （叩かれた側は「割符に応じない相手」として待ち直すことになる）
    let acceptance = device.accept(&token, now_secs())?;

    let node = Node::bind_without_relay(&device).await?;
    let to: Address = address.parse()?;
    let mut session = node.connect(&to, &Revocations::new()).await?;
    let peer = session.peer();

    session.send(&acceptance.to_bytes()).await?;
    eprintln!("warifu: つながりました（{}）", 誰か(&vault, peer));
    覚える(&vault, peer, o.remember.as_ref());

    let mut channel = Channel::new(session);
    channel.send(&Notice::Join { meeting }.to_intent()?).await?;

    // **会議キーに書かれた会議へ入る。**自分で id を作らない
    let mut roster = Roster::with_capacity(device.public_key(), warifu_app::DEFAULT_CAPACITY)?;
    roster.add(peer)?;
    let mut conference = Conference::joined(device.public_key(), meeting, roster);

    やり取り(channel, &mut conference, peer, o.idle).await
}

/// 打った行を相手へ、届いた行を標準出力へ。
///
/// **どちらかが閉じたら終わる。**片方だけ生かしておくと、
/// 「入力を待っているのか、相手を待っているのか」が分からなくなる。
async fn やり取り(
    mut channel: Channel,
    conference: &mut Conference,
    peer: warifu_core::PublicKey,
    idle: Option<u64>,
) -> Result<(), Box<dyn std::error::Error>> {
    let meeting = conference.id();
    let mut 入力 = BufReader::new(tokio::io::stdin()).lines();
    // **入力が尽きても会議は終わらない。**
    //
    // `echo … | warifu join` のように使うと、送り終えた時点で標準入力は閉じる。
    // それを「終わり」にすると、**相手の返事を受け取る前に切れる**
    // （背景で動かした側は最初から EOF なので、繋がった瞬間に閉じてしまう。
    // 2026-09-04 に実測）。**送るのを止めるだけで、受け取りは続ける。**
    let mut 送信終わり = false;

    // **既定では待ち続ける。**`--idle` を付けたときだけ、静かな時間で切り上げる
    let 限度 = idle.map(std::time::Duration::from_secs);

    loop {
        let 待つ限度 = 限度.unwrap_or(std::time::Duration::from_secs(60 * 60 * 24));
        tokio::select! {
            _ = tokio::time::sleep(待つ限度), if 限度.is_some() => {
                eprintln!("warifu: {} 秒なにも来なかったので終わります", 待つ限度.as_secs());
                break;
            }
            行 = 入力.next_line(), if !送信終わり => {
                match 行? {
                    None => {
                        送信終わり = true;
                        continue;
                    }
                    Some(text) if text.is_empty() => continue,
                    Some(text) => {
                        channel
                            .send(&Notice::Text { meeting, body: text }.to_intent()?)
                            .await?;
                    }
                }
            }
            届いた = channel.recv() => {
                // **相手が閉じたのは失敗ではない。**終わりとして扱う
                let Ok(intent) = 届いた else { break };
                let Ok(notice) = Notice::from_intent(&intent) else {
                    // 会議のものでない口は、経路としては通る。**会議は受け取らない**
                    continue;
                };
                match notice {
                    Notice::Text { body, .. } => println!("{body}"),
                    other => {
                        // 名簿は動かす。**中身は出さない**（文字だけを標準出力へ）
                        if let Ok(events) = conference.on_notice(peer, &other) {
                            for e in events {
                                eprintln!("warifu: {e:?}");
                            }
                        }
                    }
                }
            }
        }
    }

    // **締めてから終わる。**送ったものが相手へ流れきるのを待つ
    channel.finish().await?;
    Ok(())
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// `MeetingId` を使う所が上にしか無いので、型を持っておく足場。
#[allow(dead_code)]
fn _keep(_: MeetingId) {}

#[cfg(test)]
mod tests {
    use super::*;

    /// **相手が落ちたら終わる。**標準入力の read が返らなくても、である。
    ///
    /// `warifu host` は標準入力を開いたまま背景で動かす使い方をする
    /// （`warifu host < パイプ`）。ここが縛られると、経路が落ちて
    /// 誤りを出したあとも**プロセスが残り続ける**。
    #[test]
    fn 標準入力の読み取りが止まっていても終われる() {
        let (合図, 受け) = std::sync::mpsc::channel::<()>();
        let 始まり = std::time::Instant::now();

        走らせる(async move {
            // **返らない read(2) の代わり。**合図が来るまで戻らない
            tokio::task::spawn_blocking(move || {
                let _ = 受け.recv_timeout(std::time::Duration::from_secs(10));
            });
            tokio::task::yield_now().await;
            ExitCode::SUCCESS
        });

        let 掛かった = 始まり.elapsed();
        drop(合図);
        assert!(
            掛かった < std::time::Duration::from_secs(3),
            "標準入力を待って {掛かった:?} 掛かった。落ちても終わらない"
        );
    }
}
