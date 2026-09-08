//! `warifu mcp` —— **MCP の口を、標準入出力で出す。**
//!
//! ここまで `warifu-mcp` はライブラリだけで、**どこからも起動されていなかった。**
//! 「チャットやメールを MCP を通じて行う」ことがこの製品の中心なのに、
//! その口を走らせる経路が 1 本も無かった（2026-09-07 に気づいた）。
//!
//! ```text
//!   Claude Code など ── stdio ── warifu mcp ── 机 ── 割符の画面（人）
//!                                   ↑ 関所
//! ```
//!
//! **札はここでは作らない。**`--allow` は、この命令を書いた**人**が
//! 何を許すかを書く場所である（`.mcp.json` に人が書く）。
//! 書かなければ何も通らない。**既定は拒否。**

use std::path::PathBuf;

use warifu_capability::{Action, Gate, Grant};
use warifu_mcp::{Warifu, subject};
use warifu_read::RuleStore;

/// 札の効き目（秒）。**24 時間。**
///
/// 短くすると、離席のたびに人が出し直すことになる（D54 と同じ理由）。
const 札の効き目: u64 = 60 * 60 * 24;

/// `--allow` に書ける動作。**知らない名前は、黙って無視せず断る。**
///
/// 綴り違いを黙って通すと、「許したのに動かない」を人が延々と探すことになる。
const 許せる動作: &[&str] = &[
    "chat.send",
    "chat.read",
    "inbox.list",
    "inbox.open.summary",
    "inbox.open.structured",
    "inbox.open.raw",
    "inbox.open.attachments",
    "calendar.freebusy",
    "rules.list",
];

/// `warifu mcp` の設定。
pub struct 設定 {
    /// 人が許した動作。
    pub 許す: Vec<String>,
    /// 机の場所。既定は [`warifu_desk::机の場所`]。
    pub 机: PathBuf,
    /// **どこで動いているか。**既定は起動した場所のフォルダ名。
    ///
    /// 1 台の PC で複数のエージェントが同じ机に着くので、
    /// 名乗らないと**どれが喋ったのか人に分からない**（2026-09-08）。
    pub 名乗り: Option<String>,
}

/// 名乗りとして置ける形か。**画面の 1 行に収まる長さに切る。**
///
/// # Errors
/// 空・長すぎるとき。
pub fn 名乗りを検める(名: &str) -> Result<String, String> {
    let 名 = 名.trim().to_owned();
    if 名.is_empty() || 名.chars().count() > warifu_desk::名乗りの上限 {
        return Err(format!(
            "名乗りは 1〜{} 文字にしてください",
            warifu_desk::名乗りの上限
        ));
    }
    Ok(名)
}

/// 起動した場所のフォルダ名。
///
/// **人が書かなくても、どこで動いているかは分かる。**
/// 取れなければ名乗らない（机が既定の呼び方をする）。
pub fn 居場所から名乗る() -> Option<String> {
    let 名 = std::env::current_dir()
        .ok()?
        .file_name()?
        .to_string_lossy()
        .into_owned();
    let 名 = 名.trim().to_owned();
    if 名.is_empty() || 名.chars().count() > warifu_desk::名乗りの上限 {
        return None;
    }
    Some(名)
}

/// 引数を読む。
pub fn 読む(args: &mut impl Iterator<Item = String>) -> Result<設定, String> {
    let mut 設 = 設定 {
        許す: Vec::new(),
        机: warifu_desk::机の場所(),
        名乗り: 居場所から名乗る(),
    };
    while let Some(一つ) = args.next() {
        match 一つ.as_str() {
            "--allow" => {
                let 動作 = args.next().ok_or("--allow のあとに動作がありません")?;
                if !許せる動作.contains(&動作.as_str()) {
                    return Err(format!(
                        "知らない動作です: {動作}\n許せるのは: {}",
                        許せる動作.join(" ")
                    ));
                }
                設.許す.push(動作);
            }
            "--desk" => {
                設.机 = PathBuf::from(args.next().ok_or("--desk のあとに場所がありません")?);
            }
            "--as" => {
                let 名 = args.next().ok_or("--as のあとに名前がありません")?;
                設.名乗り = Some(名乗りを検める(&名)?);
            }
            他 => return Err(format!("知らない指定です: {他}")),
        }
    }
    Ok(設)
}

/// 口を出す。**繋いだ相手が閉じるまで戻らない。**
pub async fn 出す(設: &設定) -> Result<(), Box<dyn std::error::Error>> {
    use rmcp::ServiceExt;

    let 今 = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());

    let mut 関所 = Gate::new();
    for 動作 in &設.許す {
        関所.issue(Grant::new(subject(), Action::new(動作)?, 今 + 札の効き目));
    }

    // 受信箱と規則はまだ空。**空であることを、繋がっていることと混ぜない**
    // （`issues/011` が決まるまで、inbox_* は「無い」を返す）
    let mut 口 = Warifu::new(Vec::new(), RuleStore::new(), 関所, 今).机を覚える(&設.机);
    if let Some(名) = &設.名乗り {
        口 = 口.名乗る(名);
    }

    // **標準出力は MCP のもの。**言いたいことは標準エラーへ出す
    eprintln!(
        "warifu mcp: 名乗り {}／許した動作 {}／机 {}",
        設.名乗り.as_deref().unwrap_or("（名乗らない）"),
        if 設.許す.is_empty() {
            "（なし。--allow を書かないと何も通りません）".to_owned()
        } else {
            設.許す.join(" ")
        },
        設.机.display()
    );

    let 務め = 口.serve(rmcp::transport::stdio()).await?;
    務め.waiting().await?;
    Ok(())
}
