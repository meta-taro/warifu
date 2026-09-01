//! MCP の口。**すべての呼び出しが関所を通る。**
//!
//! `issues/008` の続き。roadmap Phase 3 の「MCP Adapter」と
//! PRD §12-2 の「**モデルから直接 tool を呼ばせない / policy engine 経由**」。
//!
//! AI が受信箱を人のように扱えることが、この口の目的である。
//! **だが、何をしてよいかを AI が決めてよいわけではない。**

use warifu_capability::{Action, Gate, Grant};
use warifu_mcp::{OpenArgs, Warifu, subject};
use warifu_read::{
    Body, Extract, Kind, Priority, Received, RuleDraft, RuleStore, SenderId, Source,
};

const 目印: &str = "SHIRUSHI-本文-mcp-3e7a";

fn 受信箱() -> Vec<Received> {
    vec![Received::new(
        Source::Imap,
        SenderId::new("billing@例").unwrap(),
        1_756_000_000,
        Body::new(format!("請求書 {目印}\n合計 12,000 円\n").into_bytes()),
    )]
}

fn 規則() -> RuleStore {
    let mut 棚 = RuleStore::new();
    棚.approve(
        RuleDraft::new(
            SenderId::new("billing@例").unwrap(),
            Kind::new("invoice").unwrap(),
        )
        .marker("請求書")
        .priority(Priority::High)
        .action_required(true)
        .extract(Extract::new("金額", "合計 ")),
    )
    .unwrap();
    棚
}

/// 指定した動作だけを許した関所。
fn 札を出す(動作: &[&str]) -> Gate {
    let mut 関所 = Gate::new();
    for a in 動作 {
        関所.issue(Grant::new(
            subject(),
            Action::new(a).unwrap(),
            1_798_761_600,
        ));
    }
    関所
}

fn 用意(動作: &[&str]) -> Warifu {
    Warifu::new(受信箱(), 規則(), 札を出す(動作), 1_756_000_000)
}

#[tokio::test]
async fn 札が無ければ一覧すら出せない() {
    // **既定は拒否。**MCP の口だからといって素通りしない
    let 口 = 用意(&[]);

    let 結果 = 口.inbox_list().await;

    assert!(結果.is_err(), "札が無いのに一覧が出ました");
    assert!(format!("{結果:?}").contains("関所が断りました"));
}

#[tokio::test]
async fn 札があれば一覧が出る() {
    let 口 = 用意(&["inbox.list"]);

    let 一覧 = 口.inbox_list().await.unwrap();

    assert!(一覧.contains("billing@例"));
    assert!(一覧.contains("invoice"), "規則が当たっていません: {一覧}");
    assert!(一覧.contains("要"), "要判断が出ていません: {一覧}");
}

#[tokio::test]
async fn 一覧に本文が入らない() {
    // Level 0 のまま数える。**段を上げるのは別の tool・別の札。**
    let 口 = 用意(&["inbox.list"]);

    let 一覧 = 口.inbox_list().await.unwrap();

    assert!(!一覧.contains(目印), "一覧に本文が漏れています: {一覧}");
}

#[tokio::test]
async fn 段ごとに札が要る() {
    // 一覧の札で本文は読めない。**関所の照合は完全一致**（D24）
    let 口 = 用意(&["inbox.list"]);

    let 結果 = 口
        .inbox_open(rmcp::handler::server::wrapper::Parameters(OpenArgs {
            index: 0,
            level: "raw".to_owned(),
        }))
        .await;

    assert!(結果.is_err(), "一覧の札で本文が読めました");
}

#[tokio::test]
async fn 本文の札があれば読める() {
    let 口 = 用意(&["inbox.open.raw"]);

    let 本文 = 口
        .inbox_open(rmcp::handler::server::wrapper::Parameters(OpenArgs {
            index: 0,
            level: "raw".to_owned(),
        }))
        .await
        .unwrap();

    assert!(本文.contains(目印), "段を上げたのに本文が出ません");
}

#[tokio::test]
async fn 本文の札で構造化は読めない() {
    // raw の札は structured の札ではない
    let 口 = 用意(&["inbox.open.raw"]);

    let 結果 = 口
        .inbox_open(rmcp::handler::server::wrapper::Parameters(OpenArgs {
            index: 0,
            level: "structured".to_owned(),
        }))
        .await;

    assert!(結果.is_err());
}

#[tokio::test]
async fn 断ったことが記録に残る() {
    let 口 = 用意(&[]);
    let _ = 口.inbox_list().await;
    let _ = 口.rules_list().await;

    let 記録 = 口.log_tsv();

    assert!(記録.contains("inbox.list\t断った"), "記録: {記録}");
    assert!(記録.contains("rules.list\t断った"), "記録: {記録}");
}

#[tokio::test]
async fn 知らない段は受け取らない() {
    let 口 = 用意(&["inbox.open.raw"]);

    let 結果 = 口
        .inbox_open(rmcp::handler::server::wrapper::Parameters(OpenArgs {
            index: 0,
            level: "everything".to_owned(),
        }))
        .await;

    assert!(結果.is_err());
    // **知らない段の札を関所に尋ねに行かない**（尋ねると記録が汚れる）
    assert!(!口.log_tsv().contains("everything"));
}

#[test]
fn 承認の口を出していない() {
    // **ここが一番大事。**承認を tool にすると、AI が自分に許可を出せる。
    // 生成と適用を分けた意味（D19 / D24）が、そこで消える。
    let 名前 = Warifu::tool_names();

    for 禁止 in ["approve", "issue", "grant", "revoke", "trust"] {
        assert!(
            !名前.iter().any(|n| n.contains(禁止)),
            "{禁止} を含む tool が出ています: {名前:?}"
        );
    }
    assert!(名前.contains(&"inbox_list".to_owned()), "{名前:?}");
}

#[test]
fn 出している口は_3_つだけ() {
    // 増やすときは、**その口に札の種類が要るか**を先に決める
    let 名前 = Warifu::tool_names();

    assert_eq!(名前.len(), 3, "{名前:?}");
}
