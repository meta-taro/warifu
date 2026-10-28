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
    AskArgs, CallArgs, ChangesArgs, ContactAddArgs, InviteArgs, JoinArgs, KeyGiveArgs, ProfileArgs,
    SayArgs, StatusArgs, VoiceArgs, WaitArgs, WaitPassArgs, Warifu, subject,
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
        "voice.say",
        "contacts.read",
        "contacts.call",
        "contacts.add",
        "screen.read",
        "room.join",
        "chat.export",
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
                    room: None,
                }),
                ToDesk::Status { id } => Some(FromDesk::Status {
                    id,
                    届いた: vec!["画面".to_owned()],
                    読んだ: vec![],
                }),
                ToDesk::Status様子 => Some(FromDesk::様子 {
                    ルーム: Some("ROOMAAAAAAAAAAAAAAAAAAAAAA".to_owned()),
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
                // 0.1.40：部屋の片付けの 3 つの口
                ToDesk::部屋たち => Some(FromDesk::部屋たち {
                    部屋: vec![warifu_desk::部屋の行 {
                        id: "ROOMAAAAAAAAAAAAAAAAAAAAAA".to_owned(),
                        名前: Some("試験の部屋".to_owned()),
                        人数: 1,
                        主催か: true,
                        見ているか: true,
                    }],
                }),
                ToDesk::抜ける { 部屋 } => Some(FromDesk::抜けた { 部屋 }),
                // 0.1.41：更新の口
                ToDesk::更新を確かめる => Some(FromDesk::更新 {
                    いまの版: "0.1.41".to_owned(),
                    新しい版: Some("0.1.42".to_owned()),
                    何が変わったか: Some("試験の直し".to_owned()),
                    確かめられない: None,
                }),
                ToDesk::相手に名前を付ける { 名前, .. } => {
                    Some(FromDesk::名付けた {
                        鍵: "PEERAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".to_owned(),
                        名前,
                    })
                }
                ToDesk::名前を付ける { 部屋, 名前 } => {
                    Some(FromDesk::付けた { 部屋, 名前 })
                }
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
                ToDesk::声 { .. } => Some(FromDesk::声の返り {
                    結果: warifu_desk::声の結果::流した,
                }),
                ToDesk::連絡先 => Some(FromDesk::連絡先たち {
                    行たち: vec![warifu_desk::連絡先の行 {
                        名前: "試験の相手".to_owned(),
                        鍵: "AAAAAAAAAAAAAAAA".to_owned(),
                        住所あり: true,
                    }],
                }),
                ToDesk::呼ぶ { 相手 } => Some(FromDesk::呼んだ { 相手 }),
                ToDesk::鍵を渡す { 相手, .. } => Some(FromDesk::手渡し {
                    結果: warifu_desk::手渡しの返り::渡した {
                        相手,
                        いつまで: "10-21 03:42 UTC".to_owned(),
                    },
                }),
                ToDesk::連絡先を足す { 名前, .. } => Some(FromDesk::足した { 名前 }),
                ToDesk::画面の写し => Some(FromDesk::写し {
                    中身: "版 0.1.34".to_owned(),
                }),
                ToDesk::入る { .. } => Some(FromDesk::入った {
                    ルーム: "ROOMAAAAAAAAAAAAAAAAAAAAAA".to_owned(),
                }),
                ToDesk::書き出す => Some(FromDesk::書き出し {
                    行たち: vec![warifu_desk::書き出しの行 {
                        番号: 1,
                        誰: "試験の相手".to_owned(),
                        時刻: "09:00".to_owned(),
                        本文: "煙の確かめ".to_owned(),
                    }],
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
                body: "煙の確かめ".to_owned(),
                room: None,
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

    結果.push((
        "voice_say",
        format!(
            "{:?}",
            口.voice_say(Parameters(VoiceArgs {
                text: "煙の確かめ".to_owned()
            }))
            .await
        ),
    ));

    結果.push(("contacts_list", format!("{:?}", 口.contacts_list().await)));
    結果.push(("app_view", format!("{:?}", 口.app_view().await)));
    結果.push(("chat_export", format!("{:?}", 口.chat_export().await)));
    結果.push((
        "contact_call",
        format!(
            "{:?}",
            口.contact_call(Parameters(CallArgs {
                name: "試験の相手".to_owned()
            }))
            .await
        ),
    ));

    結果.push((
        "room_join",
        format!(
            "{:?}",
            口.room_join(Parameters(JoinArgs {
                key: "WARIFU1-AAA#BBB#CCC".to_owned()
            }))
            .await
        ),
    ));

    // **room.join は使ったら消える**（0.1.30・セキュリティ審査）。2 回目は関所が断る
    let 二度目 = 口
        .room_join(Parameters(JoinArgs {
            key: "WARIFU1-AAA#BBB#CCC".to_owned(),
        }))
        .await;
    assert!(
        format!("{二度目:?}").contains("room.join"),
        "2 回目の room_join が通った: {二度目:?}"
    );

    // 0.1.40：相手に名前を付ける口
    結果.push((
        "contact_name",
        format!(
            "{:?}",
            口.contact_name(Parameters(warifu_mcp::ContactNameArgs {
                who: "PEERAAAA".to_owned(),
                name: "試験の相手".to_owned(),
            }))
            .await
        ),
    ));
    // 0.1.40：部屋の片付けの 3 つの口
    結果.push(("room_list", format!("{:?}", 口.room_list().await)));
    // 0.1.41：更新の口（返りの中身まで見る）
    let 更新 = 口.update_check().await;
    結果.push(("update_check", format!("{更新:?}")));
    let 更新の文 = 更新.as_deref().unwrap_or("");
    assert!(
        更新の文.contains("新しい版: 0.1.42") && 更新の文.contains("入れ替えは人が押します"),
        "update_check が偽の机の返りをそのまま言い直していない: {更新の文}"
    );
    結果.push((
        "room_rename",
        format!(
            "{:?}",
            口.room_rename(Parameters(warifu_mcp::RoomRenameArgs {
                room: "ROOMAAAAAAAA".to_owned(),
                name: "試験の部屋".to_owned(),
            }))
            .await
        ),
    ));
    結果.push((
        "room_leave",
        format!(
            "{:?}",
            口.room_leave(Parameters(warifu_mcp::RoomLeaveArgs {
                room: "ROOMAAAAAAAA".to_owned(),
            }))
            .await
        ),
    ));

    結果.push((
        "contact_add",
        format!(
            "{:?}",
            口.contact_add(Parameters(ContactAddArgs {
                address: "WARIFU1-AAAA".to_owned(),
                name: "試験の相手".to_owned(),
            }))
            .await
        ),
    ));

    // **contacts.add も使ったら消える**（0.1.33）。戸口も開けるので、2 回目は関所が断る
    let 二度目 = 口
        .contact_add(Parameters(ContactAddArgs {
            address: "WARIFU1-AAAA".to_owned(),
            name: "試験の相手".to_owned(),
        }))
        .await;
    assert!(
        format!("{二度目:?}").contains("contacts.add"),
        "2 回目の contact_add が通った: {二度目:?}"
    );

    結果.push((
        "room_key_give",
        format!(
            "{:?}",
            口.room_key_give(Parameters(KeyGiveArgs {
                to: "試験の相手".to_owned(),
                why: "煙の確かめ".to_owned(),
                seconds: Some(1),
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
    assert!(
        机が受けた.iter().any(|x| x.contains("声")),
        "声の頼みが机まで届いていない: {机が受けた:?}"
    );
    assert!(
        机が受けた.iter().any(|x| x.contains("書き出す")),
        "書き出す頼みが机まで届いていない: {机が受けた:?}"
    );
    assert!(
        机が受けた.iter().any(|x| x.contains("呼ぶ")),
        "呼ぶ頼みが机まで届いていない: {机が受けた:?}"
    );
    assert!(
        机が受けた.iter().any(|x| x.contains("入る")),
        "入る頼みが机まで届いていない: {机が受けた:?}"
    );
    let _ = std::fs::remove_dir_all(&控え);
}
