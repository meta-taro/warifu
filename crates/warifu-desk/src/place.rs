//! この機械をどこに開くか。**同じ機械の中だけ。網には出さない。**

use std::ffi::OsStr;
use std::path::PathBuf;

/// 金庫の置き場所を変える環境変数（`warifu-vault` と同じ名前）。
///
/// **口の場所もこれに従う**（2026-09-11）——
/// 従わないと、**同じ機械で 2 つ目の身元が口を開けない**（同じ場所を取り合う）。
/// オーナーが「A さんと B さんのエージェントが人を介さずやりとりできるか」を
/// **同じ PC で見たい**と言ったとき、そこで詰まった。
const HOME_ENV: &str = "WARIFU_HOME";

/// この機械の口の場所。
///
/// **loopback の TCP を使わない。**127.0.0.1 は同じ機械の
/// *どのユーザーの* どのプロセスからも叩ける。この機械は「同じ人」の口なので、
/// OS に持たせられる保証（Unix は所有者権限、Windows は名前付きパイプ）を使う。
#[must_use]
pub fn この機械の場所() -> PathBuf {
    場所を決める(
        std::env::var_os(HOME_ENV).as_deref(),
        std::env::var_os("XDG_RUNTIME_DIR").as_deref(),
        std::env::var_os("HOME").as_deref(),
    )
}

/// 場所の決め方。**環境変数を読むのは呼ぶ側**（ここは渡されたものだけを見る）。
///
/// 順は 3 つ ——
///
/// 1. `WARIFU_HOME`（**試験や 2 つ目の身元で明示的に分けたいとき**）
/// 2. `XDG_RUNTIME_DIR`（走っている間だけのものなので、あるならそこ）
/// 3. `HOME`（macOS は `Library/Application Support`、ほかは `.local/share`）
///
/// **どれも無くても場所は返す**（呼ぶ側を困らせない）。
#[must_use]
pub fn 場所を決める(
    家: Option<&OsStr>,
    実行時: Option<&OsStr>,
    ホーム: Option<&OsStr>,
) -> PathBuf {
    #[cfg(windows)]
    {
        // 名前付きパイプは「場所」ではなく名前。PathBuf に載せて口を揃える。
        // **家を分けたときだけ名前も変える**（2 つ目の身元が開けるように）
        let _ = (実行時, ホーム);
        if let Some(家) = 家 {
            let 印: String = 家
                .to_string_lossy()
                .chars()
                .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
                .collect();
            if !印.is_empty() {
                return PathBuf::from(format!(r"\\.\pipe\warifu-desk-{印}"));
            }
        }
        PathBuf::from(r"\\.\pipe\warifu-desk")
    }
    #[cfg(not(windows))]
    {
        置き場(家, 実行時, ホーム).join("desk.sock")
    }
}

#[cfg(not(windows))]
fn 置き場(家: Option<&OsStr>, 実行時: Option<&OsStr>, ホーム: Option<&OsStr>) -> PathBuf {
    // **明示的に分けた家がいちばん強い。**試験のために分けたのに
    // 実行時ディレクトリへ引っ張られては意味がない
    if let Some(家) = 家 {
        return PathBuf::from(家);
    }
    if let Some(実行時) = 実行時 {
        return PathBuf::from(実行時).join("warifu");
    }
    let 家 = ホーム.map_or_else(|| PathBuf::from("/tmp"), PathBuf::from);
    if cfg!(target_os = "macos") {
        return 家.join("Library/Application Support/warifu");
    }
    家.join(".local/share/warifu")
}
