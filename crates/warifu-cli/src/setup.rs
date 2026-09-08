//! `warifu setup` —— **配る相手に、1 行で入れてもらう。**
//!
//! MCP の口は、使うフォルダごとに `.mcp.json` を書くこともできるが、
//! **リポジトリの数だけ書くことになる**（2026-09-08 オーナー指摘
//! 「これいちいち指定しないと対象にならないなら、ちょっと不便ですね」）。
//!
//! Claude Code には**利用者ごとの設定**（`--scope user`）があり、
//! そこへ入れれば**どのフォルダでも出る。**ここはその 1 行を代わりに打つ。
//!
//! # ここで守ること
//!
//! - **何を許すかを、入れる前に画面へ出す。**札を出すのは人である（**D56**）。
//!   黙って許可を増やさない
//! - **確かめてから入れる。**`--yes` を書いた人だけが飛ばせる（baseline §13）
//! - **`claude` が無ければ、手で書くものを出す。**代わりに何かを入れない

use std::process::Command;

/// 既定で許す動作。**会話と、自分の席の名乗りだけ。**
///
/// 受信箱も予定表も既定では許さない —— **要る人が自分で足す。**
/// 「とりあえず全部許す」を配ると、それが既定として広まる。
///
/// `profile.write` を既定に入れたのは、**着いたエージェントが自分で名乗れると楽**
/// だからである（オーナー・2026-09-08）。**書けるのは自分の席だけ**で、
/// **名乗り（どこで動いているか）は変えられない** —— あれは立ち上げるときに人が決める。
const 既定で許す: &[&str] = &["chat.send", "chat.read", "profile.write"];

/// `warifu setup` の設定。
pub struct 設定 {
    /// 確かめずに入れるか。
    pub 確かめない: bool,
}

/// 引数を読む。
pub fn 読む(args: &mut impl Iterator<Item = String>) -> Result<設定, String> {
    let mut 設 = 設定 {
        確かめない: false
    };
    for 一つ in args {
        match 一つ.as_str() {
            "--yes" | "-y" => 設.確かめない = true,
            他 => return Err(format!("知らない指定です: {他}")),
        }
    }
    Ok(設)
}

/// 入れる。
pub fn 入れる(設: &設定) -> Result<(), Box<dyn std::error::Error>> {
    let 実体 = std::env::current_exe()?;
    let 引数: Vec<String> = std::iter::once("mcp".to_owned())
        .chain(
            既定で許す
                .iter()
                .flat_map(|a| ["--allow".to_owned(), (*a).to_owned()]),
        )
        .collect();

    println!("warifu の口を、Claude Code の利用者ごとの設定へ入れます。");
    println!();
    println!("  実体   {}", 実体.display());
    println!("  許す   {}", 既定で許す.join(" "));
    println!();
    println!("会話と、自分の席の名乗り（プロフィール）だけを許します。");
    println!("受信箱も予定表も許しません（要るなら自分で足してください）。");
    println!("名乗りで書けるのは自分の席だけです。どこで動いているかは変えられません。");
    println!("入れたあと、Claude Code を立て直すと承認を聞かれます。");
    println!();

    if !設.確かめない && !尋ねる()? {
        println!("やめました。何も入れていません。");
        return Ok(());
    }

    if which_claude().is_none() {
        // **代わりに何かを入れない。**手で書くものを出して終わる
        eprintln!("`claude` が見つかりません。手で入れる場合はこれを打ってください:");
        eprintln!();
        eprintln!(
            "  claude mcp add warifu --scope user -- {} {}",
            実体.display(),
            引数.join(" ")
        );
        return Ok(());
    }

    let mut 命令 = Command::new("claude");
    命令.args(["mcp", "add", "warifu", "--scope", "user", "--"]);
    命令.arg(&実体);
    命令.args(&引数);
    let 出た = 命令.status()?;
    if !出た.success() {
        return Err("`claude mcp add` が失敗しました".into());
    }
    println!();
    println!("入りました。Claude Code を立て直すと、どのフォルダでも warifu が出ます。");
    Ok(())
}

/// 人に尋ねる。**押していないものを押したことにしない。**
fn 尋ねる() -> std::io::Result<bool> {
    use std::io::{BufRead as _, Write as _};
    print!("入れてよいですか [y/N] ");
    std::io::stdout().flush()?;
    let mut 答え = String::new();
    std::io::stdin().lock().read_line(&mut 答え)?;
    Ok(matches!(答え.trim(), "y" | "Y" | "yes"))
}

fn which_claude() -> Option<()> {
    Command::new("claude")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .ok()
        .filter(std::process::ExitStatus::success)
        .map(|_| ())
}
