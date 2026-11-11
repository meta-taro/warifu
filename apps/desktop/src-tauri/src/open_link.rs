//! **発言の中の繋ぎ（URL）を、外のブラウザで開く**（11-11）。
//!
//! 画面の中で外の頁へ移ると、割符の窓ごと置き換わる。**開くのは OS の既定のブラウザ。**
//! シェルは通さない（引数として渡す）。受けるのは `http://` と `https://` だけで、
//! 空白・制御文字・引用符など、URL に要らない字が入っていれば開かない。
//! 届いた発言は相手の言い分なので、`file:` や `javascript:` などは渡さない。

use crate::{Answer, Failure};

/// 開いてよい長さの上限（字）。
const 長さの上限: usize = 2048;

/// **開いてよい繋ぎか。**よければ、そのまま返す。
///
/// # Errors
/// `http(s)://` で始まらない・長すぎる・URL に要らない字が入っている。
pub fn 開いてよい先(先: &str) -> Result<&str, String> {
    let 小さく = 先.to_ascii_lowercase();
    if !(小さく.starts_with("https://") || 小さく.starts_with("http://")) {
        return Err("http(s) の繋ぎだけ開けます".to_owned());
    }
    if 先.len() > 長さの上限 {
        return Err("繋ぎが長すぎます".to_owned());
    }
    if 先.len() <= "https://".len() {
        return Err("繋ぎの先がありません".to_owned());
    }
    let 要らない字 = |c: char| {
        !c.is_ascii_graphic() || matches!(c, '"' | '<' | '>' | '`' | '\\' | '^' | '|' | '{' | '}')
    };
    if 先.chars().any(要らない字) {
        return Err("繋ぎに使えない字が入っています".to_owned());
    }
    Ok(先)
}

/// 画面から：その繋ぎを、外のブラウザで開く。
#[tauri::command]
pub(crate) fn open_link(url: String) -> Answer<()> {
    let 先 = 開いてよい先(&url).map_err(|message| Failure {
        message,
        code: None,
    })?;
    開く(先).map_err(|e| Failure {
        message: format!("ブラウザで開けませんでした: {e}"),
        code: None,
    })?;
    記録!("繋ぎを外のブラウザで開きました");
    Ok(())
}

#[cfg(target_os = "macos")]
fn 開く(先: &str) -> std::io::Result<()> {
    std::process::Command::new("open")
        .arg(先)
        .spawn()
        .map(|_| ())
}

#[cfg(windows)]
fn 開く(先: &str) -> std::io::Result<()> {
    // explorer は URL を既定のブラウザへ渡す。cmd の start と違い、シェルの読み替えが無い
    std::process::Command::new("explorer")
        .arg(先)
        .spawn()
        .map(|_| ())
}

#[cfg(not(any(target_os = "macos", windows)))]
fn 開く(先: &str) -> std::io::Result<()> {
    std::process::Command::new("xdg-open")
        .arg(先)
        .spawn()
        .map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::開いてよい先;

    #[test]
    fn http_と_https_だけ開く() {
        assert!(開いてよい先("https://example.com/a?b=1#c").is_ok());
        assert!(開いてよい先("HTTP://example.com").is_ok());
        for 断る in [
            "file:///etc/passwd",
            "javascript:alert(1)",
            "warifu://join/x",
            "ftp://example.com",
            "https://",
            "",
        ] {
            assert!(開いてよい先(断る).is_err(), "{断る}");
        }
    }

    #[test]
    fn url_に要らない字があれば開かない() {
        for 断る in [
            "https://example.com/a b",
            "https://example.com/\"&calc",
            "https://example.com/\n",
            "https://例え.jp",
            "https://example.com/^x",
        ] {
            assert!(開いてよい先(断る).is_err(), "{断る:?}");
        }
        assert!(開いてよい先(&format!("https://example.com/{}", "a".repeat(3000))).is_err());
    }
}
