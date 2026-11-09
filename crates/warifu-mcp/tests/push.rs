//! **届いたことを、口から押し出す**（`push.rs`・Claude Code の channels）。
//!
//! 口を本当に立てて、MCP の線に何が出るかを見る。
//! **名乗り（`claude/channel`）と、通知の名前を取り違えると、受け手は黙って捨てる** ——
//! 捨てられても誰も気づかないので、線の上の形を試験で押さえる。

use std::time::Duration;

use tokio::io::{AsyncBufReadExt as _, AsyncWriteExt as _, BufReader};
use warifu_capability::{Action, Gate, Grant};
use warifu_desk::{FromDesk, ToDesk, 受け口, 口 as 行の口};
use warifu_mcp::{Warifu, subject};
use warifu_read::RuleStore;

fn 試験の机(名: &str) -> std::path::PathBuf {
    let 走り = std::process::id();
    #[cfg(windows)]
    {
        std::path::PathBuf::from(format!(r"\\.\pipe\warifu-mcp-push-{名}-{走り}"))
    }
    #[cfg(not(windows))]
    {
        std::env::temp_dir().join(format!("warifu-mcp-push-{名}-{走り}.sock"))
    }
}

fn 試験の控え(名: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("warifu-mcp-push-控え-{名}-{}", std::process::id()))
}

/// 聞くと言われたら、**誰かが入ったことと、`本文たち` を発言として渡す机**。
fn 言う机を立てる(
    mut 待ち: 受け口,
    本文たち: [&'static str; 2],
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut 口 = 行の口::新しく(待ち.受ける().await.expect("繋がること"));
        while let Ok(Some(行)) = 口.受ける().await {
            if let Ok(ToDesk::Listen { .. }) = ToDesk::読む(&行) {
                // **連絡先から呼ばれて、相手が入ってきた**形
                let 入室 = FromDesk::Joined {
                    who: "相手A（ABCDEFGHIJKL…）".to_owned(),
                };
                if 口.送る(&入室.書く()).await.is_err() {
                    return;
                }
                for (id, body) in [(5, 本文たち[0]), (6, 本文たち[1])] {
                    let 発言 = FromDesk::Heard {
                        id,
                        from: "ABCDEFGH…".to_owned(),
                        body: body.to_owned(),
                        at: "13:07".to_owned(),
                        room: None,
                        任された: false,
                    };
                    if 口.送る(&発言.書く()).await.is_err() {
                        return;
                    }
                }
            }
        }
    })
}

/// 口を立てて支度を済ませ、**`秒` の間に線に出た行を全部返す。**
async fn 立てて聞く(
    名: &str,
    札: &[&str],
    秒: u64,
    起こし方: warifu_mcp::push::起こし方,
) -> Vec<String> {
    立てて聞く_本文と名は(名, 札, 秒, 起こし方, ["一つめの中身", "二つめの中身"], None).await
}

/// [`立てて聞く`] に、**机が流す本文**と、**画面に控えた名乗り**（0.1.46）を足したもの。
async fn 立てて聞く_本文と名は(
    名: &str,
    札: &[&str],
    秒: u64,
    起こし方: warifu_mcp::push::起こし方,
    本文たち: [&'static str; 2],
    控えた名: Option<Vec<String>>,
) -> Vec<String> {
    use rmcp::ServiceExt as _;

    let 控え = 試験の控え(名);
    std::fs::create_dir_all(&控え).expect("フォルダが作れること");
    let 場所 = 試験の机(名);
    let 待ち = 受け口::開く(&場所).await.expect("この机が開くこと");
    let _机 = 言う机を立てる(待ち, 本文たち);

    let mut 関所 = Gate::new();
    for a in 札 {
        関所.issue(Grant::new(
            subject(),
            Action::new(a).unwrap(),
            1_798_761_600,
        ));
    }
    let mut 口 = Warifu::new(Vec::new(), RuleStore::new(), 関所, 1_756_000_000)
        .起こし方を決める(起こし方)
        .名乗る("vegas-gift のエージェント");
    if let Some(名たち) = 控えた名 {
        口 = 口.控えた名を読むには(std::sync::Arc::new(move || 名たち.clone()));
    }
    let 口 = 口
        .この機械につながる_控えは(&場所, &控え)
        .await
        .expect("つながれること");

    let (mut 頼む側, 口の入力) = tokio::io::duplex(64 * 1024);
    let (口の出力, 聞く側) = tokio::io::duplex(64 * 1024);
    let 務め = tokio::spawn(async move {
        let 務め = 口.serve((口の入力, 口の出力)).await.unwrap();
        let _ = 務め.waiting().await;
    });

    let 頼み = concat!(
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"t","version":"0"}}}"#,
        "\n",
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        "\n",
    );
    頼む側.write_all(頼み.as_bytes()).await.unwrap();

    let mut 行たち = BufReader::new(聞く側).lines();
    let mut 出た = Vec::new();
    let 期限 = tokio::time::Instant::now() + Duration::from_secs(秒);
    while let Ok(Ok(Some(行))) = tokio::time::timeout_at(期限, 行たち.next_line()).await {
        出た.push(行);
    }
    drop(頼む側);
    務め.abort();
    let _ = std::fs::remove_dir_all(&控え);
    出た
}

#[tokio::test(flavor = "multi_thread")]
async fn 届いたら_本文を運ばずに知らせる() {
    // **全部で起こす席**（`--wake all`）。既定（呼ばれたときだけ）は下の試験
    let 出た = 立てて聞く("on", &["chat.read"], 6, warifu_mcp::push::起こし方::全部).await;

    let 名乗り = 出た
        .iter()
        .find(|l| l.contains(r#""id":1"#))
        .expect("initialize の返事が無い");
    let 名乗り: serde_json::Value = serde_json::from_str(名乗り).unwrap();
    assert!(
        名乗り["result"]["capabilities"]["experimental"]["claude/channel"].is_object(),
        "claude/channel を名乗っていない: {名乗り}"
    );

    let 知らせ: Vec<&String> = 出た
        .iter()
        .filter(|l| l.contains("notifications/claude/channel"))
        .collect();
    assert_eq!(知らせ.len(), 1, "2 通は 1 回に束ねる: {出た:#?}");
    let 知らせ: serde_json::Value = serde_json::from_str(知らせ[0]).unwrap();
    let 文 = 知らせ["params"]["content"].as_str().unwrap();
    assert!(
        文.contains("2 件") && 文.contains("#6"),
        "件数と番号が無い: {文}"
    );
    assert!(!文.contains("中身"), "本文を運んでいる: {文}");
    assert!(
        文.contains("相手A（ABCDEFGHIJKL…） が入りました"),
        "入ったことを言っていない: {文}"
    );
    assert_eq!(知らせ["params"]["meta"]["joined"], "相手A（ABCDEFGHIJKL…）");
    assert_eq!(知らせ["params"]["meta"]["latest"], "6");
}

#[tokio::test(flavor = "multi_thread")]
async fn 読む札が無ければ_知らせない() {
    let 出た = 立てて聞く("off", &["chat.send"], 4, warifu_mcp::push::起こし方::全部).await;
    assert!(
        出た.iter().any(|l| l.contains(r#""id":1"#)),
        "initialize の返事が無い（試験の対照）: {出た:#?}"
    );
    assert!(
        !出た
            .iter()
            .any(|l| l.contains("notifications/claude/channel")),
        "札が無いのに知らせた: {出た:#?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn 既定の起こし方では_呼ばれていない発言と入室で起こさない() {
    // 試験の机が流すのは、呼んでいない発言 2 つと入室だけ（`喋る机を立てる`）
    let 出た = 立てて聞く(
        "quiet",
        &["chat.read"],
        4,
        warifu_mcp::push::起こし方::default(),
    )
    .await;
    assert!(
        出た.iter().any(|l| l.contains(r#""id":1"#)),
        "initialize の返事が無い（試験の対照）: {出た:#?}"
    );
    assert!(
        !出た
            .iter()
            .any(|l| l.contains("notifications/claude/channel")),
        "呼ばれていないのに起こした: {出た:#?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn 画面に控えた名乗りで呼ばれたら_繋いだばかりでも起こす() {
    // 繋ぎ直した口は、`profile_set` で書いた名前を覚えていない。画面の控えから足す（0.1.46）
    let 本文 = ["@ベガスギフト 見てください", "ただの話"];
    let 出た = 立てて聞く_本文と名は(
        "named",
        &["chat.read"],
        4,
        warifu_mcp::push::起こし方::default(),
        本文,
        Some(vec!["ベガスギフト".to_owned()]),
    )
    .await;
    assert!(
        出た
            .iter()
            .any(|l| l.contains("notifications/claude/channel")),
        "控えた名前で呼ばれたのに起こさない: {出た:#?}"
    );
    // 対照：控えを読まなければ、フォルダ名しか知らないので起こさない（0.1.45 までの形）
    let 出た = 立てて聞く_本文と名は(
        "unnamed",
        &["chat.read"],
        4,
        warifu_mcp::push::起こし方::default(),
        本文,
        None,
    )
    .await;
    assert!(
        出た.iter().any(|l| l.contains(r#""id":1"#)),
        "initialize の返事が無い（試験の対照）: {出た:#?}"
    );
    assert!(
        !出た
            .iter()
            .any(|l| l.contains("notifications/claude/channel")),
        "控えを読まないのに起こした（対照が効いていない）: {出た:#?}"
    );
}
