//! **同じ機械に入っている CLI が、画面と同じ版か。**
//!
//! 2026-09-12（内部の Issue 017）。
//!
//! CLI（`~/.local/bin/warifu`）は画面とは別のバイナリなので、アプリを更新しても
//! 古い版のまま残る。画面と CLI が別の鍵を名乗る不具合では、**アプリだけ上げても
//! 直らない。**症状は「繋がらない」だけなので、CLI が古いせいだと気づけない。
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
    読めない {
        場所: String, 出たもの: String
    },
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
    let 名 = if cfg!(windows) {
        "warifu.exe"
    } else {
        "warifu"
    };
    vec![
        home.join(".local/bin").join(名),
        PathBuf::from("/usr/local/bin").join(名),
        PathBuf::from("/opt/homebrew/bin").join(名),
    ]
}

/// **入れ替えてよい場所。**画面より古いときだけ（内部の Issue 021）。
///
/// 同じ・新しい（画面を上げ忘れている）・読めない CLI は触らない。**決めつけて差し替えない。**
#[must_use]
pub fn 置き換える場所(様子: &CLIの様子) -> Option<&Path> {
    match 様子 {
        CLIの様子::古い { 場所, .. } => Some(Path::new(場所)),
        _ => None,
    }
}

/// **束の中の CLI を、見つかった場所へ置く**（内部の Issue 021）。
///
/// **上書きしない。消してから置く** —— macOS は実行ファイルの署名を覚えていて、
/// 上書きしたものは `Killed: 9` で黙って死ぬ。
/// 束の中に無いときは、**元の CLI を消す前に**止める（消したまま置けない形を作らない）。
///
/// # Errors
/// 束の中に無いとき、消せない・置けないとき（権限の無い場所など）。
pub fn 置く(束の中: &Path, 置き場所: &Path) -> std::io::Result<()> {
    if !束の中.is_file() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("{} がありません", 束の中.display()),
        ));
    }
    match std::fs::remove_file(置き場所) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e),
    }
    std::fs::copy(束の中, 置き場所)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(置き場所, std::fs::Permissions::from_mode(0o755))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 試験ごとに別の置き場所（並んで走っても混ざらない）。
    fn 試しの場所(名: &str) -> PathBuf {
        let 場所 = std::env::temp_dir().join(format!("warifu-cli-{}-{名}", std::process::id()));
        let _ = std::fs::remove_dir_all(&場所);
        std::fs::create_dir_all(&場所).expect("作れる");
        場所
    }

    #[test]
    fn 置くときは_消してから置き_実行できる形にする() {
        let 場所 = 試しの場所("置く");
        let 束の中 = 場所.join("束の warifu");
        let 置き場所 = 場所.join("warifu");
        std::fs::write(&束の中, "新しい").expect("書ける");
        std::fs::write(&置き場所, "古い").expect("書ける");
        #[cfg(unix)]
        let 前の番号 = {
            use std::os::unix::fs::MetadataExt;
            std::fs::metadata(&置き場所).expect("在る").ino()
        };

        置く(&束の中, &置き場所).expect("置ける");

        assert_eq!(
            std::fs::read_to_string(&置き場所).expect("読める"),
            "新しい"
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::{MetadataExt, PermissionsExt};
            let 後 = std::fs::metadata(&置き場所).expect("在る");
            // **上書きではなく、別のファイルとして置き直した**（macOS は上書きした実行ファイルを殺す）
            assert_ne!(後.ino(), 前の番号);
            assert_eq!(後.permissions().mode() & 0o777, 0o755);
        }
        let _ = std::fs::remove_dir_all(&場所);
    }

    #[test]
    fn 束の中に無ければ_元の_cli_を消さない() {
        let 場所 = 試しの場所("無い");
        let 置き場所 = 場所.join("warifu");
        std::fs::write(&置き場所, "古い").expect("書ける");
        assert!(置く(&場所.join("無い warifu"), &置き場所).is_err());
        assert_eq!(std::fs::read_to_string(&置き場所).expect("読める"), "古い");
        let _ = std::fs::remove_dir_all(&場所);
    }

    #[test]
    fn 置き換えてよいのは_古いときだけ() {
        let 古い = CLIの様子::古い {
            場所: "/x/warifu".to_owned(),
            版: "0.1.40".to_owned(),
        };
        assert_eq!(置き換える場所(&古い), Some(Path::new("/x/warifu")));
        for 他 in [
            CLIの様子::無い,
            CLIの様子::同じ {
                場所: "/x/warifu".to_owned(),
                版: "0.1.42".to_owned(),
            },
            CLIの様子::新しい {
                場所: "/x/warifu".to_owned(),
                版: "0.1.43".to_owned(),
            },
            CLIの様子::読めない {
                場所: "/x/warifu".to_owned(),
                出たもの: "?".to_owned(),
            },
        ] {
            assert_eq!(置き換える場所(&他), None, "{他:?}");
        }
    }

    #[test]
    fn 版の行から数を拾う() {
        assert_eq!(版を読む("warifu 0.1.3"), Some([0, 1, 3]));
        assert_eq!(版を読む("  0.1.2\n"), Some([0, 1, 2]));
    }

    #[test]
    fn 後ろ置きが付いていても数だけ読む() {
        assert_eq!(版を読む("warifu 0.1.3-alpha.2"), Some([0, 1, 3]));
        // CLI は建てた commit まで名乗る（`warifu version`）
        assert_eq!(版を読む("warifu 0.1.21+c1dcaef\n"), Some([0, 1, 21]));
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
        assert!(
            並び[0].ends_with(".local/bin/warifu") || 並び[0].ends_with(".local/bin/warifu.exe")
        );
        assert_eq!(並び.len(), 3);
    }
}
