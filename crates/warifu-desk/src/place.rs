//! この機械をどこに開くか。**同じ機械の中だけ。網には出さない。**

use std::path::PathBuf;

/// この機械の口の場所。
///
/// **loopback の TCP を使わない。**127.0.0.1 は同じ機械の
/// *どのユーザーの* どのプロセスからも叩ける。この機械は「同じ人」の口なので、
/// OS に持たせられる保証（Unix は所有者権限、Windows は名前付きパイプ）を使う。
#[must_use]
pub fn この機械の場所() -> PathBuf {
    #[cfg(windows)]
    {
        // 名前付きパイプは「場所」ではなく名前。PathBuf に載せて口を揃える
        PathBuf::from(r"\\.\pipe\warifu-desk")
    }
    #[cfg(not(windows))]
    {
        置き場().join("desk.sock")
    }
}

#[cfg(not(windows))]
fn 置き場() -> PathBuf {
    // 走っている間だけのものなので、あるなら実行時ディレクトリを使う
    if let Some(実行時) = std::env::var_os("XDG_RUNTIME_DIR") {
        return PathBuf::from(実行時).join("warifu");
    }
    let 家 = std::env::var_os("HOME").map_or_else(|| PathBuf::from("/tmp"), PathBuf::from);
    if cfg!(target_os = "macos") {
        return 家.join("Library/Application Support/warifu");
    }
    家.join(".local/share/warifu")
}
