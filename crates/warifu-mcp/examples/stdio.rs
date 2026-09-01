//! 受信箱を MCP の口として stdio で出す。
//!
//! Claude Desktop / Claude Code などの MCP クライアントから繋ぐための口。
//!
//! ```text
//!   WARIFU_GRANTS=warifu-grants.tsv \
//!   WARIFU_RULES=warifu-rules.tsv \
//!   cargo run -p warifu-mcp --example stdio
//! ```
//!
//! # 札が無ければ、何も返らない
//!
//! `WARIFU_GRANTS` が無い／空なら、**すべての tool が断られる。**
//! それが既定であって、事故ではない（`decisions.md` **D24**）。
//!
//! 札は**人が書く**。1 行 1 枚で、`動作<TAB>期限（epoch 秒）`。
//!
//! ```text
//! inbox.list<TAB>1798761600
//! inbox.open.structured<TAB>1798761600
//! ```
//!
//! **`inbox.open.raw` を既定で書かない。**本文を読ませるかどうかは、
//! そのつど人が決めるところである。
//!
//! # この例は受信箱を空で起動する
//!
//! IMAP から入れる配線（`warifu-imap`）は、**資格情報が要るので別の口**にしてある。
//! ここでは口の形と関所の効き方だけを確かめられる。

use std::path::PathBuf;

use warifu_capability::{Action, Gate, Grant};
use warifu_mcp::{Warifu, subject};
use warifu_read::RuleStore;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let 規則 = 読む(&置き場("WARIFU_RULES", "warifu-rules.tsv"))
        .map(|t| RuleStore::from_tsv(&t))
        .transpose()?
        .unwrap_or_default();

    let 関所 = 札を読む(&置き場("WARIFU_GRANTS", "warifu-grants.tsv"))?;

    // **標準出力は MCP のもの。**人向けの文言は標準エラーへ出す
    eprintln!(
        "warifu-mcp: 規則 {} 件 / 札 {} 枚 / 口 {:?}",
        規則.len(),
        関所.grants().len(),
        Warifu::tool_names()
    );
    if 関所.grants().is_empty() {
        eprintln!("警告: 札が 1 枚もありません。**すべての tool が断られます**（既定）。");
    }

    let 今 = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();

    let 口 = Warifu::new(Vec::new(), 規則, 関所, 今);
    let 繋がり = rmcp::ServiceExt::serve(口, rmcp::transport::stdio()).await?;
    繋がり.waiting().await?;
    Ok(())
}

fn 置き場(名: &str, 既定: &str) -> PathBuf {
    PathBuf::from(std::env::var(名).unwrap_or_else(|_| 既定.to_owned()))
}

fn 読む(p: &PathBuf) -> Option<String> {
    std::fs::read_to_string(p).ok()
}

/// 札の一覧を読む。**書くのは人。**
///
/// 読めない行があればそこで止める。黙って飛ばすと、
/// **出したつもりの札が効いていない**ことに気づけない。
fn 札を読む(p: &PathBuf) -> Result<Gate, Box<dyn std::error::Error>> {
    let mut 関所 = Gate::new();
    let Some(text) = 読む(p) else {
        return Ok(関所);
    };
    for (i, 行) in text.lines().enumerate() {
        let 行 = 行.trim();
        if 行.is_empty() || 行.starts_with('#') {
            continue;
        }
        let mut 列 = 行.split('\t');
        let (Some(動作), Some(期限), None) = (列.next(), 列.next(), 列.next()) else {
            return Err(format!("{}: {} 行目の形が違います", p.display(), i + 1).into());
        };
        関所.issue(Grant::new(
            subject(),
            Action::new(動作)?,
            期限.trim().parse()?,
        ));
    }
    Ok(関所)
}
