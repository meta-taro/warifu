//! **どの版の MCP で話すか**（0.1.38）。
//!
//! 2026-07-28 版は `initialize` の握手を無くし、頼みごとに版を載せる形（`server/discover`）になった。
//! Claude Code は**その版で決まった口を channels として登録しない**（docs: channels-reference /
//! mcp「Push messages with channels」）。しかも握手が無いので `on_initialized` が呼ばれず、
//! **押し出し（`push.rs`）が始まらない。**10-23、channels を付けて立ち上げた 2 つの席で、
//! 部屋の発言の知らせが 1 件も来なかった。
//!
//! この口は**握手のある版だけ**を名乗る。新しい形で来た相手には「その版は無い」と返し、
//! `initialize` へ戻ってもらう。
//!
//! **断るのは rmcp の手前で**（[`warifu_mcp::握手へ戻す`]）。rmcp は最初の頼みが握手でないと、
//! その後の頼みすべてに新しい版の `_meta` を求めるようになり、握手し直しても戻らない。
//! 10-23、channels の知らせは届くようになったが、`tools/list` が断られて道具が 1 つも出なかった。

use tokio::io::{AsyncBufReadExt as _, AsyncWriteExt as _, BufReader};
use warifu_capability::Gate;
use warifu_mcp::Warifu;
use warifu_read::RuleStore;

async fn 一往復(頼み: &str) -> serde_json::Value {
    何往復(&[頼み], 1).await.remove(0)
}

/// 頼みを順に流し、返事を `数` 行読む（知らせの行は数えない）。
async fn 何往復(頼み: &[&str], 数: usize) -> Vec<serde_json::Value> {
    use rmcp::ServiceExt as _;
    let 口 = Warifu::new(Vec::new(), RuleStore::new(), Gate::new(), 1_756_000_000);
    let (mut 頼む側, 口の入力) = tokio::io::duplex(64 * 1024);
    let (口の出力, 聞く側) = tokio::io::duplex(64 * 1024);
    tokio::spawn(async move {
        if let Ok(務め) = 口.serve(warifu_mcp::握手へ戻す(口の入力, 口の出力)).await
        {
            let _ = 務め.waiting().await;
        }
    });
    for 一つ in 頼み {
        頼む側.write_all(一つ.as_bytes()).await.unwrap();
        頼む側.write_all(b"\n").await.unwrap();
    }
    let mut 読み = BufReader::new(聞く側);
    let mut 返事たち = Vec::new();
    while 返事たち.len() < 数 {
        let mut 行 = String::new();
        tokio::time::timeout(std::time::Duration::from_secs(5), 読み.read_line(&mut 行))
            .await
            .expect("返事が来ること")
            .unwrap();
        let 値: serde_json::Value = serde_json::from_str(&行).expect("JSON であること");
        if 値.get("id").is_some() {
            返事たち.push(値);
        }
    }
    返事たち
}

const 新しい版で尋ねる: &str = r#"{"jsonrpc":"2.0","id":0,"method":"server/discover","params":{"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientCapabilities":{}}}}"#;

#[tokio::test]
async fn 握手の無い新しい版で来た相手は_断って握手へ戻す() {
    let 返事 = 一往復(新しい版で尋ねる).await;
    assert!(
        返事.get("error").is_some(),
        "2026-07-28 で受けてはいけない: {返事}"
    );
}

#[tokio::test]
async fn 新しい版を頼まれても_握手では握手のある版で答え_channelsを名乗る() {
    let 返事 = 一往復(
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2026-07-28","capabilities":{},"clientInfo":{"name":"t","version":"0"}}}"#,
    )
    .await;
    let 結果 = &返事["result"];
    assert_eq!(結果["protocolVersion"], "2025-11-25");
    assert!(結果["capabilities"]["experimental"]["claude/channel"].is_object());
}

#[tokio::test]
async fn 断られて握手し直した相手に_道具の一覧を返す() {
    let 返事 = 何往復(
        &[
            新しい版で尋ねる,
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2026-07-28","capabilities":{},"clientInfo":{"name":"t","version":"0"}}}"#,
            r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#,
        ],
        3,
    )
    .await;
    assert!(返事[0].get("error").is_some(), "尋ねは断る: {}", 返事[0]);
    assert_eq!(返事[1]["result"]["protocolVersion"], "2025-11-25");
    assert!(
        返事[2]["result"]["tools"]
            .as_array()
            .is_some_and(|t| !t.is_empty()),
        "握手し直したら道具が出ること: {}",
        返事[2]
    );
}
