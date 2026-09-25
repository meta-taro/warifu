//! **口を全部 1 回呼ぶ**（2026-09-24 の PDCA・A-1）。
//!
//! # なぜ要るのか
//!
//! **2026-09-24、「書いてあるが、一度も通していない」が 5 件出た。**
//!
//! ```text
//! pass_ask      引数名が日本語 → **API が口ごと除外**していた
//! 押した札       起動時に 1 回しか読まない → 押しても効かない
//! 出した鍵       `仕分ける` に `招いた` が無い → **机は出したのに、頼んだ側は時間切れ**
//! Asked         同じ穴（`pass_ask` が返らない）
//! #34 の並べ替え `Address` が数値順に並べ直していた
//! ```
//!
//! **5 件とも、試験は緑だった。**線の形（`line.rs`）も関所（`gate.rs`）も試験が在り、
//! **途中の 1 段だけ、誰も通していなかった。**
//!
//! **見つけ方は 1 つだけだった** —— **手で 1 回呼ぶ。**
//! **それを機械にやらせるのが、この試験である。**
//!
//! # 何を見るか
//!
//! **返り値の中身は見ない**（それは各々の試験の仕事）。見るのは 1 つだけ ——
//!
//! > **呼んで、返ってくること。**
//!
//! **「この機械が返事をしません」で時間切れになる口が 1 つも無いこと。**

use rmcp::handler::server::wrapper::Parameters;
use warifu_capability::{Action, Gate, Grant};
use warifu_desk::{FromDesk, ToDesk, 受け口, 口 as 行の口};
use warifu_mcp::{
    AskArgs, ChangesArgs, InviteArgs, ProfileArgs, SayArgs, StatusArgs, WaitArgs, WaitPassArgs,
    Warifu, subject,
};
use warifu_read::{Body, Received, RuleStore, SenderId, Source};

fn 試験の机() -> std::path::PathBuf {
    let 走り = std::process::id();
    #[cfg(windows)]
    {
        std::path::PathBuf::from(format!(r"\\.\pipe\warifu-mcp-smoke-{走り}"))
    }
    #[cfg(not(windows))]
    {
        std::env::temp_dir().join(format!("warifu-mcp-smoke-{走り}.sock"))
    }
}

fn 試験の控え() -> std::path::PathBuf {
    std::env::temp_dir().join(format!("warifu-mcp-smoke-控え-{}", std::process::id()))
}

/// **全部の札を出した関所**（この試験は関所を見ない。通り道を見る）。
fn 全部許す() -> Gate {
    let mut 関所 = Gate::new();
    for a in [
        "inbox.list",
        "inbox.open.summary",
        "chat.send",
        "chat.read",
        "profile.write",
        "calendar.slots",
        "room.invite",
    ] {
        関所.issue(Grant::new(
            subject(),
            Action::new(a).unwrap(),
            1_798_761_600,
        ));
    }
    関所
}

fn 受信箱() -> Vec<Received> {
    vec![Received::new(
        Source::Imap,
        SenderId::new("billing@例").unwrap(),
        1_756_000_000,
        Body::new(b"\xe8\xab\x8b\xe6\xb1\x82".to_vec()),
    )]
}

/// **何を言われても、約束どおりの返事をする机。**
///
/// **返事を返さない枝が 1 つでもあると、呼んだ側は時間切れになる**（**D49**）。
/// ここは**全部の枝に返事を書く** —— そうしないと、この試験が
/// 「机が黙っていた」のか「仕分けが落とした」のかを見分けられない。
fn 机を立てる(mut 待ち: 受け口) -> tokio::task::JoinHandle<Vec<String>> {
    // **口は、呼ぶ側より先に開けておく**（2026-09-25 に踏んだ）——
    // `spawn` の中で開くと、**繋ぎに行くほうが先に走って `NotFound` で落ちる。**
    tokio::spawn(async move {
        let mut 口 = 行の口::新しく(待ち.受ける().await.expect("繋がること"));
        let mut 受けた = Vec::new();
        while let Ok(Some(行)) = 口.受ける().await {
            let 中身 = match ToDesk::読む(&行) {
                Ok(中身) => 中身,
                Err(_) => continue,
            };
            受けた.push(format!("{中身:?}"));
            let 返し = match 中身 {
                ToDesk::Listen { .. } | ToDesk::Read { .. } => None,
                ToDesk::Say { .. } => Some(FromDesk::Sent {
                    to: 1,
                    id: 1,
                    届いた: vec!["画面".to_owned()],
                }),
                ToDesk::Status { id } => Some(FromDesk::Status {
                    id,
                    届いた: vec!["画面".to_owned()],
                    読んだ: vec![],
                }),
                ToDesk::Status様子 => Some(FromDesk::様子 {
                    ルーム: Some("AFUF2T4ECVKMFP4L4GPO2E56LU".to_owned()),
                    名簿: vec![],
                    経路: Some("direct".to_owned()),
                    エージェント: vec![],
                    待っているリンク: 0,
                    待っているルーム: vec![],
                    送っている: None,
                    受けている: None,
                    掴んでいる: None,
                    題字: None,
                }),
                ToDesk::Profile { .. } => Some(FromDesk::Wrote {
                    who: "試験の席".to_owned(),
                }),
                ToDesk::Ask { 動作, .. } | ToDesk::札を待つ { 動作, .. } => {
                    Some(FromDesk::Asked {
                        動作,
                        答え: warifu_desk::頼みの返り::まだ,
                    })
                }
                ToDesk::招く { .. } => Some(FromDesk::招いた {
                    鍵たち: vec!["WARIFU1-AAA#BBB#CCC".to_owned()],
                    いつまで: "09-25 00:00 UTC".to_owned(),
                }),
            };
            if let Some(返し) = 返し {
                口.送る(&返し.書く()).await.expect("返せること");
            }
        }
        受けた
    })
}

/// **時間切れは、この試験の落ちである。**
fn 時間切れでないこと(口の名: &str, 返り: &str) {
    for 拙い in ["返事をしません", "想定しない返事", "この機械が閉じ"] {
        assert!(
            !返り.contains(拙い),
            "**{口の名} が返ってこない** —— {返り}"
        );
    }
}

#[tokio::test]
async fn 口を全部_1回ずつ呼ぶ() {
    let 控え = 試験の控え();
    std::fs::create_dir_all(&控え).expect("フォルダが作れること");
    let 場所 = 試験の机();
    let 待ち = 受け口::開く(&場所).await.expect("この機械が開くこと");
    let 机 = 机を立てる(待ち);

    let 口 = Warifu::new(受信箱(), RuleStore::new(), 全部許す(), 1_756_000_000)
        .この機械につながる_控えは(&場所, &控え)
        .await
        .expect("つながれること");

    // **机を使わない口**（受信箱・予定・説明）
    let mut 結果: Vec<(&str, String)> = Vec::new();
    結果.push(("about", format!("{:?}", 口.about())));
    結果.push((
        "changes",
        format!("{:?}", 口.changes(Parameters(ChangesArgs::default()))),
    ));
    結果.push(("rules_list", format!("{:?}", 口.rules_list().await)));
    結果.push(("inbox_list", format!("{:?}", 口.inbox_list().await)));

    // **机を通る口**（ここが 2026-09-24 に落ちていた所）
    結果.push((
        "chat_send",
        format!(
            "{:?}",
            口.chat_send(Parameters(SayArgs {
                body: "煙の確かめ".to_owned()
            }))
            .await
        ),
    ));
    結果.push(("chat_read", format!("{:?}", 口.chat_read().await)));
    結果.push((
        "chat_status",
        format!(
            "{:?}",
            口.chat_status(Parameters(StatusArgs { id: 1 })).await
        ),
    ));
    結果.push((
        "chat_wait",
        format!(
            "{:?}",
            口.chat_wait(Parameters(WaitArgs { seconds: Some(1) }))
                .await
        ),
    ));
    結果.push(("room_status", format!("{:?}", 口.room_status().await)));
    結果.push((
        "room_invite",
        format!(
            "{:?}",
            口.room_invite(Parameters(InviteArgs {
                count: Some(1),
                ttl_secs: Some(3600)
            }))
            .await
        ),
    ));
    結果.push((
        "profile_set",
        format!(
            "{:?}",
            口.profile_set(Parameters(ProfileArgs {
                name: "試験の席".to_owned(),
                bio: "煙の確かめ".to_owned()
            }))
            .await
        ),
    ));
    結果.push((
        "pass_ask",
        format!(
            "{:?}",
            口.pass_ask(Parameters(AskArgs {
                action: "inbox.open.raw".to_owned(),
                why: "煙の確かめ".to_owned()
            }))
            .await
        ),
    ));
    結果.push((
        "pass_wait",
        format!(
            "{:?}",
            口.pass_wait(Parameters(WaitPassArgs {
                action: "inbox.open.raw".to_owned(),
                why: "煙の確かめ".to_owned(),
                seconds: Some(1)
            }))
            .await
        ),
    ));

    for (名, 返り) in &結果 {
        時間切れでないこと(名, 返り);
    }

    // **呼び残しを、機械が数える** —— 口を足したら、ここに 1 行足すまで落ちる
    let 呼んだ: std::collections::BTreeSet<&str> = 結果.iter().map(|(名, _)| *名).collect();
    let 全部: std::collections::BTreeSet<String> = Warifu::tool_names().into_iter().collect();
    // **この試験で呼べないもの**（引数に実物が要る）——**理由を書いて外す**
    let 外す: std::collections::BTreeSet<&str> = [
        // 受信箱の中身に依る（**段ごとの試験が `gate.rs` に在る**）
        "inbox_open",
        // 予定表の窓に依る（**`gate.rs` に在る**）
        "calendar_slots",
    ]
    .into_iter()
    .collect();
    let 呼び残し: Vec<&String> = 全部
        .iter()
        .filter(|名| !呼んだ.contains(名.as_str()) && !外す.contains(名.as_str()))
        .collect();
    assert!(
        呼び残し.is_empty(),
        "**呼んでいない口が在る**（足したら、この試験にも 1 行足す）: {呼び残し:?}"
    );

    drop(口);
    let 机が受けた = tokio::time::timeout(std::time::Duration::from_secs(5), 机)
        .await
        .expect("机が閉じること")
        .expect("机が落ちていないこと");
    // **机まで届いていることも見る**（返事だけ作って、送っていない形を防ぐ）
    assert!(
        机が受けた.iter().any(|x| x.contains("招く")),
        "机まで届いていない: {机が受けた:?}"
    );
    let _ = std::fs::remove_dir_all(&控え);
}
