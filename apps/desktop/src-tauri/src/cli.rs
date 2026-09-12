//! **同じ機械に入っている CLI が、画面と同じ版か。**
//!
//! 2026-09-12、Mac Air のエージェントの指摘（`.claude/issues/017`）——
//!
//! > 案内は「アプリを終了して開き直すと更新の案内が出ます」でしたが、
//! > CLI（`~/.local/bin/warifu`）は別バイナリで、そのままだと 0.1.2 のまま残ります。
//! >
//! > 今回の不具合は「画面と CLI が別の鍵を名乗る」ものなので、**アプリだけ上げた人は
//! > 直ったつもりで直っていません。**しかも症状は「繋がらない」だけなので、
//! > CLI が古いせいだと気づけません。
//!
//! **画面のほうから見つけて、名指しで言う。**

use std::path::{Path, PathBuf};

/// 同じ機械の CLI の様子。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub enum CLIの様子 {
    /// CLI が見つからない（入れていない）。**これは不具合ではない。**
    無い,
    /// 画面と同じ版。
    同じ { 場所: String, 版: String },
    /// **画面より古い。**言わなければ気づけない所である。
    古い { 場所: String, 版: String },
    /// 画面より新しい（画面を上げ忘れている）。
    新しい { 場所: String, 版: String },
    /// 版が読めない（形が違う）。**「古い」と決めつけない。**
    読めない { 場所: String, 出たもの: String },
}

/// `warifu version` が出す行から、版だけを取り出す。
///
/// 版の行は `warifu 0.1.3` の形で出る。**数だけを拾う** ——
/// 前置きや後ろの空白で判定が揺れないようにする。
#[must_use]
pub fn 版を読む(出たもの: &str) -> Option<[u64; 3]> {
    let 語 = 出たもの
        .split_whitespace()
        .find(|語| 語.chars().next().is_some_and(|c| c.is_ascii_digit()))?;
    let mut 数 = [0_u64; 3];
    let mut 見た = 0;
    for (i, 部分) in 語.split('.').take(3).enumerate() {
        // `0.1.3-alpha` のような後ろ置きは落とす（数だけ読む）
        let 頭: String = 部分.chars().take_while(char::is_ascii_digit).collect();
        数[i] = 頭.parse().ok()?;
        見た += 1;
    }
    (見た == 3).then_some(数)
}

/// 画面と CLI の版を見比べる。
#[must_use]
pub fn 見比べる(場所: &Path, 画面の版: &str, cliの出力: &str) -> CLIの様子 {
    let 場所 = 場所.display().to_string();
    let (Some(画面), Some(cli)) = (版を読む(画面の版), 版を読む(cliの出力)) else {
        return CLIの様子::読めない {
            場所,
            出たもの: cliの出力.trim().to_owned(),
        };
    };
    let 版 = format!("{}.{}.{}", cli[0], cli[1], cli[2]);
    match cli.cmp(&画面) {
        std::cmp::Ordering::Less => CLIの様子::古い { 場所, 版 },
        std::cmp::Ordering::Equal => CLIの様子::同じ { 場所, 版 },
        std::cmp::Ordering::Greater => CLIの様子::新しい { 場所, 版 },
    }
}

/// CLI を探す場所。**入れ方の案内と同じ順で見る。**
#[must_use]
pub fn 探す場所(home: &Path) -> Vec<PathBuf> {
    let 名 = if cfg!(windows) { "warifu.exe" } else { "warifu" };
    vec![
        home.join(".local/bin").join(名),
        PathBuf::from("/usr/local/bin").join(名),
        PathBuf::from("/opt/homebrew/bin").join(名),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 版の行から数を拾う() {
        assert_eq!(版を読む("warifu 0.1.3"), Some([0, 1, 3]));
        assert_eq!(版を読む("  0.1.2\n"), Some([0, 1, 2]));
    }

    #[test]
    fn 後ろ置きが付いていても数だけ読む() {
        assert_eq!(版を読む("warifu 0.1.3-alpha.2"), Some([0, 1, 3]));
    }

    #[test]
    fn 版でないものは読まない() {
        assert_eq!(版を読む("command not found"), None);
        assert_eq!(版を読む(""), None);
        // **3 つ揃っていなければ読まない**（`0.1` を `0.1.0` と決めつけない）
        assert_eq!(版を読む("warifu 0.1"), None);
    }

    #[test]
    fn 古い_cli_を古いと言う() {
        assert_eq!(
            見比べる(Path::new("/x/warifu"), "0.1.4", "warifu 0.1.2"),
            CLIの様子::古い {
                場所: "/x/warifu".into(),
                版: "0.1.2".into()
            }
        );
    }

    #[test]
    fn 同じ版なら同じと言う() {
        assert_eq!(
            見比べる(Path::new("/x/warifu"), "0.1.4", "warifu 0.1.4"),
            CLIの様子::同じ {
                場所: "/x/warifu".into(),
                版: "0.1.4".into()
            }
        );
    }

    #[test]
    fn 画面のほうが古いなら_新しいと言う() {
        // **画面を上げ忘れている。**黙って「同じ」にしない
        assert_eq!(
            見比べる(Path::new("/x/warifu"), "0.1.2", "warifu 0.1.4"),
            CLIの様子::新しい {
                場所: "/x/warifu".into(),
                版: "0.1.4".into()
            }
        );
    }

    #[test]
    fn 読めない出力を_古いと決めつけない() {
        // **決めつけると、入れ直しても消えない警告になる**
        assert_eq!(
            見比べる(Path::new("/x/warifu"), "0.1.4", "zsh: command not found"),
            CLIの様子::読めない {
                場所: "/x/warifu".into(),
                出たもの: "zsh: command not found".into()
            }
        );
    }

    #[test]
    fn 探す場所は_案内と同じ順で並ぶ() {
        let 並び = 探す場所(Path::new("/home/x"));
        assert!(並び[0].ends_with(".local/bin/warifu") || 並び[0].ends_with(".local/bin/warifu.exe"));
        assert_eq!(並び.len(), 3);
    }
}
