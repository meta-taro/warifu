//! **入れ替えてよい置き方か。**
//!
//! 更新（`plugin-updater`）は、macOS では**実行ファイルの 2 つ上を .app とみなして、
//! そこを丸ごと入れ替える。**`.app` の外で素の実行ファイルを動かしていると、
//! **その置き場所のフォルダが、中身ごと新しい .app の中身に置き換わる**
//! （2026-10-21、試験で `target/debug` と試験の道具が消えた）。
//!
//! だから **`.app` の中で動いているときだけ、更新を確かめる。**
//! Windows と Linux は入れ替え方が違う（入れる道具を走らせる）ので、止めない。

use std::path::Path;

/// その実行ファイルの置き方で、更新を入れ替えてよいか。
pub fn 入れ替えてよいか(実行ファイル: &Path, mac: bool) -> bool {
    if !mac {
        return true;
    }
    let 親 = 実行ファイル.parent();
    let 祖父 = 親.and_then(Path::parent);
    let 曾祖父 = 祖父.and_then(Path::parent);
    let 名 = |p: Option<&Path>| {
        p.and_then(Path::file_name)
            .and_then(|n| n.to_str())
            .map(str::to_owned)
    };
    名(親).as_deref() == Some("MacOS")
        && 名(祖父).as_deref() == Some("Contents")
        && 名(曾祖父).is_some_and(|n| n.ends_with(".app"))
}

/// 画面が更新を確かめる前に聞く。
#[tauri::command]
pub fn can_update() -> bool {
    match std::env::current_exe() {
        Ok(p) => 入れ替えてよいか(&p, cfg!(target_os = "macos")),
        Err(e) => {
            記録!("実行ファイルの場所が分かりません。更新は確かめません: {e}");
            false
        }
    }
}

#[cfg(test)]
mod 試験 {
    use super::*;

    #[test]
    fn app_の中なら入れ替えてよい() {
        let p = Path::new("/Applications/warifu.app/Contents/MacOS/warifu-desktop");
        assert!(入れ替えてよいか(p, true));
    }

    #[test]
    fn 素の実行ファイルなら入れ替えない() {
        let p = Path::new("/Users/me/warifu/target/debug/warifu-desktop");
        assert!(!入れ替えてよいか(p, true));
    }

    #[test]
    fn app_でないフォルダの_contents_macos_なら入れ替えない() {
        let p = Path::new("/tmp/x/Contents/MacOS/warifu-desktop");
        assert!(!入れ替えてよいか(p, true));
    }

    #[test]
    fn macos_でなければ止めない() {
        let p = Path::new("C:/Program Files/warifu/warifu-desktop.exe");
        assert!(入れ替えてよいか(p, false));
    }
}
