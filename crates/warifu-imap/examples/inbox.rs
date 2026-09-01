//! 受信箱を 1 回ぶん取り込んで、**今日、人間が判断すること**だけを出す。
//!
//! `issues/007` の最後の完了条件
//! 「実際に自分の受信箱で 1 週間使い、token の実測値を人が記録した」を、
//! 人が実際に回せるようにするための口。**AI は代筆しない**（baseline §19 / §29）。
//!
//! ```text
//!   cp .env.example .env      # 値は自分で入れる
//!   set -a; . ./.env; set +a
//!   cargo run -p warifu-imap --example inbox
//! ```
//!
//! **本文は 1 文字も表示しない。**出るのは metadata だけで、
//! 中身を見たいときは段を上げる（この例では上げない）。

use std::path::PathBuf;

use warifu_imap::{Account, DEFAULT_LIMIT, Mailbox, connect};
use warifu_read::{Entry, Ledger, Reader, RuleStore};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let 繋ぎ先 = Account::new(
        &環境("WARIFU_IMAP_HOST")?,
        環境("WARIFU_IMAP_PORT")?.parse().unwrap_or(993),
        &環境("WARIFU_IMAP_USER")?,
        &環境("WARIFU_IMAP_PASSWORD")?,
    )?;
    let 上限 = std::env::var("WARIFU_IMAP_LIMIT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_LIMIT);
    let 帳簿の置き場 = PathBuf::from(
        std::env::var("WARIFU_LEDGER").unwrap_or_else(|_| "warifu-ledger.tsv".to_owned()),
    );
    let 規則の置き場 = PathBuf::from(
        std::env::var("WARIFU_RULES").unwrap_or_else(|_| "warifu-rules.tsv".to_owned()),
    );

    // 承認済みの規則があれば読む。無ければ規則なしで進む（種別は unknown のままになる）
    let 棚 = match std::fs::read_to_string(&規則の置き場) {
        Ok(text) => RuleStore::from_tsv(&text)?,
        Err(_) => RuleStore::new(),
    };
    println!(
        "承認済みの規則 {} 件（{}）",
        棚.len(),
        規則の置き場.display()
    );

    let 今 = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();

    let mut 受信箱 = Mailbox::open(connect(&繋ぎ先).await?, &繋ぎ先).await?;
    let 届いた = 受信箱.fetch_unseen(上限, 今).await?;
    受信箱.logout().await?;

    let 読む人 = Reader::with_rules(棚);
    let mut 帳簿 = Ledger::new();
    let mut 判断が要る = Vec::new();
    let mut 開かない = 0usize;

    for 一通 in &届いた {
        // **段は上げない。**Level 0 のまま数える
        let 見え方 = 読む人.read(一通);
        帳簿.record(Entry::without_interpreter(&見え方));

        if 見え方.metadata().action_required() {
            判断が要る.push((
                見え方.metadata().kind().to_string(),
                見え方.metadata().sender().as_str().to_owned(),
            ));
        } else {
            開かない += 1;
        }
    }

    帳簿.append_to(&帳簿の置き場)?;

    println!("\n── 今日、人間が判断すること ──");
    if 判断が要る.is_empty() {
        println!("  なし");
    } else {
        for (種別, 差出人) in &判断が要る {
            println!("  {種別:12} {差出人}");
        }
    }
    println!("  開かない {開かない}");

    let (入り, 出) = 帳簿.tokens();
    println!(
        "\n取り込み {} 通 / 解釈器を呼んだ {} 回 / token {入り} + {出}",
        届いた.len(),
        帳簿.interpreter_calls()
    );
    println!("会計を書き足しました: {}", 帳簿の置き場.display());
    Ok(())
}

/// 環境変数を読む。**無ければ止める。**既定値で勝手に繋ぎに行かない。
fn 環境(名: &str) -> Result<String, String> {
    std::env::var(名).map_err(|_| format!("{名} が設定されていません（.env.example を見ること）"))
}
