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

use tokio::io::{AsyncBufReadExt as _, AsyncWriteExt as _, BufReader};
use warifu_capability::Gate;
use warifu_mcp::Warifu;
use warifu_read::RuleStore;

async fn 一往復(頼み: &str) -> serde_json::Value {
    use rmcp::ServiceExt as _;
    let 口 = Warifu::new(Vec::new(), RuleStore::new(), Gate::new(), 1_756_000_000);
    let (mut 頼む側, 口の入力) = tokio::io::duplex(64 * 1024);
    let (口の出力, 聞く側) = tokio::io::duplex(64 * 1024);
    tokio::spawn(async move {
        if let Ok(務め) = 口.serve((口の入力, 口の出力)).await {
            let _ = 務め.waiting().await;
        }
    });
    頼む側.write_all(頼み.as_bytes()).await.unwrap();
    頼む側.write_all(b"\n").await.unwrap();
    let mut 行 = String::new();
    let mut 読み = BufReader::new(聞く側);
    tokio::time::timeout(std::time::Duration::from_secs(5), 読み.read_line(&mut 行))
        .await
        .expect("返事が来ること")
        .unwrap();
    serde_json::from_str(&行).expect("JSON であること")
}

#[tokio::test]
async fn 握手の無い新しい版で来た相手は_断って握手へ戻す() {
    let 返事 = 一往復(
        r#"{"jsonrpc":"2.0","id":1,"method":"server/discover","params":{"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientCapabilities":{}}}}"#,
    )
    .await;
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
