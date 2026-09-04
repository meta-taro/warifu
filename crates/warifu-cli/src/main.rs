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
use warifu_core::{Device, Revocations, Seed};
use warifu_intent::Channel;
use warifu_meeting::{MeetingId, Notice, Roster};
use warifu_net::{Address, Node};

/// 会議キーの有効期間（秒）。画面側と揃える。
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
         \x20 warifu host [--idle <秒>]            待つ。会議キーを標準出力へ出す\n\
         \x20 warifu join <会議キー> [--idle <秒>]  入る\n\
         \n\
         つないだ後は、打った行が相手へ飛び、届いた行がそのまま出ます。\n\
         \n\
         --idle を付けると、何も来ない時間がその秒数を超えた時点で終わります。\n\
         付けなければ終わりません（会話は黙っている時間のほうが長いため）。\n\
         **映像は扱いません**（それは画面の担当です）。"
    );
    ExitCode::from(2)
}

/// `--idle <秒>` を読む。**無ければ終わらない。**
fn 読む_idle(args: &mut impl Iterator<Item = String>) -> Option<u64> {
    let mut idle = IDLE_DEFAULT;
    while let Some(a) = args.next() {
        if a == "--idle" {
            idle = args.next().and_then(|v| v.parse().ok());
        }
    }
    idle
}

#[tokio::main]
async fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let 命令 = args.next();
    let result = match 命令.as_deref() {
        Some("host") => {
            let idle = 読む_idle(&mut args);
            待つ(idle).await
        }
        Some("join") => match args.next() {
            Some(key) => {
                let idle = 読む_idle(&mut args);
                入る(&key, idle).await
            }
            None => return 使い方(),
        },
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

/// **起動のたびに新しい身元を作る。**
///
/// 鍵を保存する形は `decisions.md` の **D2** が未決で、
/// 決める前に「とりあえずファイルへ置く」をやると、それが既成事実になる。
/// 画面側（`src-tauri`）と同じ扱いにしてある。
fn 身元() -> Result<Device, Box<dyn std::error::Error>> {
    Ok(Seed::generate()?.profile("Personal").device("cli"))
}

async fn 待つ(idle: Option<u64>) -> Result<(), Box<dyn std::error::Error>> {
    let device = 身元()?;
    let node = Node::bind_without_relay(&device).await?;
    let address = node.address().await?.to_string();

    let mut conference = Conference::host(device.public_key(), warifu_app::DEFAULT_CAPACITY)?;
    let (mut tally, token) = device.issue_tally(now_secs(), KEY_TTL_SECS)?;

    // **会議キーは標準出力へ。**進行の知らせは標準エラーへ分ける。
    // こうしておくと `warifu host | pbcopy` のように使える
    eprintln!("warifu: 待っています（{KEY_TTL_SECS} 秒で会議キーが切れます）");
    println!("{}", format_invite(&address, &token, conference.id()));

    let session = node.accept(&Revocations::new()).await?;
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
    eprintln!("warifu: 割符が合いました。つながっています");

    let channel = Channel::new(session);
    やり取り(channel, &mut conference, peer, idle).await
}

async fn 入る(key: &str, idle: Option<u64>) -> Result<(), Box<dyn std::error::Error>> {
    let device = 身元()?;
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
    eprintln!("warifu: つながりました");

    let mut channel = Channel::new(session);
    channel.send(&Notice::Join { meeting }.to_intent()?).await?;

    // **会議キーに書かれた会議へ入る。**自分で id を作らない
    let mut roster = Roster::with_capacity(device.public_key(), warifu_app::DEFAULT_CAPACITY)?;
    roster.add(peer)?;
    let mut conference = Conference::joined(device.public_key(), meeting, roster);

    やり取り(channel, &mut conference, peer, idle).await
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
