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
         \x20 warifu host [--ttl <秒>] [--idle <秒>] [--remember <呼び名>]\n\
         \x20            待つ。会議キーを標準出力へ出す\n\
         \x20 warifu join <会議キー> [--idle <秒>] [--remember <呼び名>]\n\
         \x20            入る\n\
         \x20 warifu id  自分の公開鍵と、身元の置き場所を出す\n\
         \x20 warifu contacts                       覚えた相手を並べる\n\
         \x20 warifu contacts add <公開鍵> <呼び名>  覚える\n\
         \x20 warifu contacts forget <呼び名|公開鍵> 忘れる\n\
         \n\
         つないだ後は、打った行が相手へ飛び、届いた行がそのまま出ます。\n\
         \n\
         --ttl      会議キーの有効期間（既定 600 秒）。相手が建てている間に切れないように\n\
         --idle     何も来ない時間がその秒数を超えたら終わる。付けなければ終わりません\n\
         \x20          （会話は黙っている時間のほうが長いため）\n\
         --remember つながった相手を、その呼び名で覚える\n\
         \n\
         **身元はこの端末に残ります。**閉じても同じ人でいられます（`warifu id` で確認）。\n\
         **映像は扱いません**（それは画面の担当です）。"
    );
    ExitCode::from(2)
}

/// 呼び出しに付いてきた指定。
struct Options {
    /// 何も来ない時間がこれを超えたら終わる。**無ければ終わらない。**
    idle: Option<u64>,
    /// 会議キーの有効期間（秒）。
    ttl: u64,
    /// つながった相手を、この呼び名で覚える。
    remember: Option<String>,
}

fn 読む_options(args: &mut impl Iterator<Item = String>) -> Options {
    let mut o = Options {
        idle: IDLE_DEFAULT,
        ttl: KEY_TTL_SECS,
        remember: None,
    };
    while let Some(a) = args.next() {
        match a.as_str() {
            "--idle" => o.idle = args.next().and_then(|v| v.parse().ok()),
            "--ttl" => {
                if let Some(v) = args.next().and_then(|v| v.parse().ok()) {
                    o.ttl = v;
                }
            }
            "--remember" => o.remember = args.next(),
            _ => {}
        }
    }
    o
}

#[tokio::main]
async fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let 命令 = args.next();
    let result = match 命令.as_deref() {
        Some("host") => {
            let o = 読む_options(&mut args);
            待つ(&o).await
        }
        Some("join") => match args.next() {
            Some(key) => {
                let o = 読む_options(&mut args);
                入る(&key, &o).await
            }
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
    let ttl = o.ttl;
    let node = Node::bind_without_relay(&device).await?;
    let address = node.address().await?.to_string();

    let mut conference = Conference::host(device.public_key(), warifu_app::DEFAULT_CAPACITY)?;
    let (mut tally, token) = device.issue_tally(now_secs(), ttl)?;

    // **会議キーは標準出力へ。**進行の知らせは標準エラーへ分ける。
    // こうしておくと `warifu host | pbcopy` のように使える
    eprintln!("warifu: 待っています（{ttl} 秒で会議キーが切れます）");
    println!("{}", format_invite(&address, &token, conference.id()));

    // **来るまで待ち続ける。**
    //
    // `accept` は下の層の都合で時間切れになることがある（実測: timed out）。
    // 「待っています」と言った以上、**こちらの都合で勝手に諦めない。**
    // `--idle` は繋がった後の話であって、**繋がる前の待ち時間ではない**。
    let session = loop {
        match node.accept(&Revocations::new()).await {
            Ok(s) => break s,
            Err(e) => {
                // 会議キーが切れていたら、待っていても意味が無い
                if now_secs() > token.not_after() {
                    return Err("会議キーの期限が切れました。作り直してください".into());
                }
                eprintln!("warifu: 待ち直します（{e}）");
            }
        }
    };
    let peer = session.peer();
    let mut session = session;

    // **割符を先に確かめる。**会議の話をする前に、通してよいかを決める（D31 / D39）
    let bytes = tokio::time::timeout(
        std::time::Duration::from_secs(HANDSHAKE_SECS),
        session.recv(),
    )
    .await
    .map_err(|_| "相手が割符に応じませんでした")??;
    let acceptance = warifu_core::Acceptance::from_bytes(&bytes)?;
    tally.match_half(&acceptance, now_secs(), &Revocations::new())?;
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

    let node = Node::bind_without_relay(&device).await?;
    let to: Address = address.parse()?;
    let mut session = node.connect(&to, &Revocations::new()).await?;
    let peer = session.peer();

    // 割符に応じる（画面側と同じ順序）
    let acceptance = device.accept(&token, now_secs())?;
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
