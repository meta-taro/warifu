//! **端末で `warifu` と打つと、どれが動くか**（0.1.60）。
//!
//! 更新で入れ替わるのは、割符が置いた CLI（Windows は `%LOCALAPPDATA%\warifu\cli\warifu.exe`）だけ。
//! 人が手で置いた別の `warifu`（`~/bin` など）が PATH の前のほうにあると、打つとそちらが動き、
//! 新しい口が「知らない指定です」と断られる。どれが動いているかは、外から見ても分からない。
//!
//! **人が置いたファイルなので、黙って上書きしない。**場所と版を名指しで言うだけにする。
//! 画面（`cli_state`）とエージェントの口（`about`・`room_status`）が同じ判定を使う。

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

/// その OS での `warifu` のファイル名。
#[must_use]
pub const fn 名(windows: bool) -> &'static str {
    if windows { "warifu.exe" } else { "warifu" }
}

/// **PATH を前から見て、最初に当たる `warifu`**（純粋・在るかを引数で受ける）。
///
/// 空の要素（`a::b`）は飛ばす（いまの場所を指すことになり、打つ場所で答えが変わる）。
#[must_use]
pub fn 最初に当たる(
    path: &OsStr,
    windows: bool,
    在るか: impl Fn(&Path) -> bool,
) -> Option<PathBuf> {
    std::env::split_paths(path)
        .filter(|d| !d.as_os_str().is_empty())
        .map(|d| d.join(名(windows)))
        .find(|p| 在るか(p))
}

/// この機械の PATH で、最初に当たる `warifu`。PATH が無ければ `None`。
#[must_use]
pub fn 打つと動く() -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    最初に当たる(&path, cfg!(windows), Path::is_file)
}

/// 2 つの場所が同じファイルか。**読めなければ字面で比べる**（決めつけて「違う」と言わない）。
#[must_use]
pub fn 同じファイルか(a: &Path, b: &Path) -> bool {
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(x), Ok(y)) => x == y,
        _ => a == b,
    }
}

/// `warifu version` の出力から、版の数だけを拾う（`warifu 0.1.55+8f2c683` → `[0, 1, 55]`）。
#[must_use]
pub fn 版を読む(出たもの: &str) -> Option<[u64; 3]> {
    let 語 = 出たもの
        .split_whitespace()
        .find(|語| 語.chars().next().is_some_and(|c| c.is_ascii_digit()))?;
    let mut 数 = [0_u64; 3];
    let mut 見た = 0;
    for (i, 部分) in 語.split('.').take(3).enumerate() {
        let 頭: String = 部分.chars().take_while(char::is_ascii_digit).collect();
        数[i] = 頭.parse().ok()?;
        見た += 1;
    }
    (見た == 3).then_some(数)
}

/// **打つと動く `warifu` が、この口と別のファイルで版も違うなら、その 1 行**（純粋）。
///
/// 同じファイル・同じ版・版が読めないときは `None`（読めないものを古いと決めつけない）。
#[must_use]
pub fn 食い違いの行(
    打つと動く: &Path,
    その出力: &str,
    自分: &Path,
    自分の版: &str,
    同じファイル: bool,
) -> Option<String> {
    if 同じファイル {
        return None;
    }
    let (Some(そちら), Some(こちら)) = (版を読む(その出力), 版を読む(自分の版))
    else {
        return None;
    };
    if そちら == こちら {
        return None;
    }
    let 版 = format!("{}.{}.{}", そちら[0], そちら[1], そちら[2]);
    let 向き = if そちら < こちら {
        "古い"
    } else {
        "新しい"
    };
    Some(format!(
        "端末で warifu と打つと動くのは {}（{版}）で、この口（{自分の版}・{}）より{向き}版です。\
         更新で入れ替わるのは割符が置いた所だけなので、そちらは人が入れ替えるか消してください\
         （割符は人が置いたファイルを上書きしません）",
        打つと動く.display(),
        自分.display()
    ))
}

/// この機械で、打つと動く `warifu` とこの口の食い違いを調べる（版を聞くために 1 回動かす）。
///
/// 食い違いが無い・調べられないときは `None`。
#[must_use]
pub fn この機械の食い違い(自分の版: &str) -> Option<String> {
    let 自分 = std::env::current_exe().ok()?;
    let 打つと = 打つと動く()?;
    let 同じ = 同じファイルか(&打つと, &自分);
    if 同じ {
        return None;
    }
    let 出た = std::process::Command::new(&打つと)
        .arg("version")
        .output()
        .ok()?;
    let 文 =
        String::from_utf8_lossy(&出た.stdout).to_string() + &String::from_utf8_lossy(&出た.stderr);
    食い違いの行(&打つと, &文, &自分, 自分の版, false)
}

#[cfg(test)]
mod 試験 {
    use super::*;
    use std::ffi::OsString;

    fn 並べる(dirs: &[&str]) -> OsString {
        std::env::join_paths(dirs.iter().map(PathBuf::from)).unwrap()
    }

    #[test]
    fn 前にあるほうが動く() {
        let path = 並べる(&["/home/u/bin", "/home/u/.local/bin"]);
        let 在る = |p: &Path| p.starts_with("/home/u");
        assert_eq!(
            最初に当たる(&path, false, 在る),
            Some(PathBuf::from("/home/u/bin/warifu"))
        );
    }

    #[test]
    fn 無い所は飛ばす() {
        let path = 並べる(&["/usr/bin", "/home/u/.local/bin"]);
        let 在る = |p: &Path| p == Path::new("/home/u/.local/bin/warifu");
        assert_eq!(
            最初に当たる(&path, false, 在る),
            Some(PathBuf::from("/home/u/.local/bin/warifu"))
        );
    }

    #[test]
    fn windows_では_exe_を探す() {
        let path = 並べる(&["/a"]);
        let 在る = |p: &Path| p.extension().is_some_and(|e| e == "exe");
        assert_eq!(
            最初に当たる(&path, true, 在る),
            Some(PathBuf::from("/a/warifu.exe"))
        );
        assert_eq!(最初に当たる(&path, false, 在る), None);
    }

    #[test]
    fn どこにも無ければ_none() {
        assert_eq!(最初に当たる(&並べる(&["/a", "/b"]), false, |_| false), None);
    }

    #[test]
    fn 古い別のファイルを名指しで言う() {
        let 行 = 食い違いの行(
            Path::new("/home/u/bin/warifu"),
            "warifu 0.1.55+8f2c683\n",
            Path::new("/home/u/.local/bin/warifu"),
            "0.1.60",
            false,
        )
        .unwrap();
        assert!(行.contains("/home/u/bin/warifu（0.1.55）"), "{行}");
        assert!(行.contains("古い"), "{行}");
        assert!(行.contains("上書きしません"), "{行}");
    }

    #[test]
    fn 新しい別のファイルは新しいと言う() {
        let 行 = 食い違いの行(
            Path::new("/a/warifu"),
            "warifu 0.2.0",
            Path::new("/b/warifu"),
            "0.1.60",
            false,
        )
        .unwrap();
        assert!(行.contains("新しい"), "{行}");
    }

    #[test]
    fn 同じファイルか同じ版か読めなければ黙る() {
        let a = Path::new("/a/warifu");
        let b = Path::new("/b/warifu");
        assert_eq!(食い違いの行(a, "warifu 0.1.55", a, "0.1.60", true), None);
        assert_eq!(
            食い違いの行(a, "warifu 0.1.60+abc", b, "0.1.60", false),
            None
        );
        assert_eq!(
            食い違いの行(a, "知らない指定です", b, "0.1.60", false),
            None
        );
    }
}
