//! OS のメニューバー（D35 の続き）。
//!
//! **画面の中だけを訳しても足りない。**macOS では窓の外にメニューが出る。
//!
//! # 消せない理由
//!
//! **`Edit` を消すと `⌘C` / `⌘V` が効かなくなる。**macOS では編集メニューの項目が
//! ショートカットの実体で、項目が無いと WebView の中でも貼り付けられない。
//! **招待を貼る操作がそこに乗っている**ので、消す選択は取れない。ラベルだけを訳す。
//!
//! # 言語の決め方
//!
//! **画面側から渡してもらう。**`navigator.languages` を見るのは画面側で、
//! ここで OS の言語をもう一度取りに行くと、**2 か所が別の答えを出しうる**。
//! 依存（`tauri-plugin-os` 等）も増やさずに済む。

use tauri::menu::{Menu, MenuItemKind, PredefinedMenuItem, SubmenuBuilder};
use tauri::{AppHandle, Runtime};

/// メニューに出す文言。**4 言語すべてに揃っていること**をテストで固定する。
pub struct Labels {
    pub about: &'static str,
    pub hide: &'static str,
    pub hide_others: &'static str,
    pub show_all: &'static str,
    pub quit: &'static str,
    pub edit: &'static str,
    pub undo: &'static str,
    pub redo: &'static str,
    pub cut: &'static str,
    pub copy: &'static str,
    pub paste: &'static str,
    pub select_all: &'static str,
    pub window: &'static str,
    pub minimize: &'static str,
    pub zoom: &'static str,
    pub close: &'static str,
}

/// 対応ロケール。画面側の `locales.ts` と同じ 4 つ（D35）。
pub const LOCALES: [&str; 4] = ["en", "ja", "zh", "ko"];

/// 知らないロケールは `en` へ落とす（画面側の `resolveLocale` と同じ扱い）。
#[must_use]
pub fn labels(locale: &str) -> Labels {
    match locale {
        "ja" => Labels {
            about: "warifu について",
            hide: "warifu を隠す",
            hide_others: "ほかを隠す",
            show_all: "すべてを表示",
            quit: "warifu を終了",
            edit: "編集",
            undo: "取り消す",
            redo: "やり直す",
            cut: "切り取る",
            copy: "コピー",
            paste: "貼り付け",
            select_all: "すべてを選択",
            window: "ウインドウ",
            minimize: "しまう",
            zoom: "拡大／縮小",
            close: "閉じる",
        },
        "zh" => Labels {
            about: "关于 warifu",
            hide: "隐藏 warifu",
            hide_others: "隐藏其他",
            show_all: "全部显示",
            quit: "退出 warifu",
            edit: "编辑",
            undo: "撤销",
            redo: "重做",
            cut: "剪切",
            copy: "复制",
            paste: "粘贴",
            select_all: "全选",
            window: "窗口",
            minimize: "最小化",
            zoom: "缩放",
            close: "关闭",
        },
        "ko" => Labels {
            about: "warifu 정보",
            hide: "warifu 가리기",
            hide_others: "다른 항목 가리기",
            show_all: "모두 보기",
            quit: "warifu 종료",
            edit: "편집",
            undo: "실행 취소",
            redo: "다시 실행",
            cut: "오려두기",
            copy: "복사하기",
            paste: "붙여넣기",
            select_all: "전체 선택",
            window: "윈도우",
            minimize: "최소화",
            zoom: "확대/축소",
            close: "닫기",
        },
        _ => Labels {
            about: "About warifu",
            hide: "Hide warifu",
            hide_others: "Hide Others",
            show_all: "Show All",
            quit: "Quit warifu",
            edit: "Edit",
            undo: "Undo",
            redo: "Redo",
            cut: "Cut",
            copy: "Copy",
            paste: "Paste",
            select_all: "Select All",
            window: "Window",
            minimize: "Minimize",
            zoom: "Zoom",
            close: "Close",
        },
    }
}

/// メニューを組み立てる。
///
/// **`File` と `View` は置かない。**開くファイルも切り替える表示も無いのに枠だけ出すと、
/// 「何かできそう」に見えて空を開かせることになる。**無い機能の入口を作らない。**
///
/// # Errors
/// メニューの生成に失敗したとき。
pub fn build<R: Runtime>(app: &AppHandle<R>, locale: &str) -> tauri::Result<Menu<R>> {
    let t = labels(locale);

    let app_menu = SubmenuBuilder::new(app, "warifu")
        .item(&PredefinedMenuItem::about(app, Some(t.about), None)?)
        .separator()
        .item(&PredefinedMenuItem::hide(app, Some(t.hide))?)
        .item(&PredefinedMenuItem::hide_others(app, Some(t.hide_others))?)
        .item(&PredefinedMenuItem::show_all(app, Some(t.show_all))?)
        .separator()
        .item(&PredefinedMenuItem::quit(app, Some(t.quit))?)
        .build()?;

    // ここを消すと ⌘C / ⌘V が効かなくなる（上のコメント参照）
    let edit = SubmenuBuilder::new(app, t.edit)
        .item(&PredefinedMenuItem::undo(app, Some(t.undo))?)
        .item(&PredefinedMenuItem::redo(app, Some(t.redo))?)
        .separator()
        .item(&PredefinedMenuItem::cut(app, Some(t.cut))?)
        .item(&PredefinedMenuItem::copy(app, Some(t.copy))?)
        .item(&PredefinedMenuItem::paste(app, Some(t.paste))?)
        .item(&PredefinedMenuItem::select_all(app, Some(t.select_all))?)
        .build()?;

    let window = SubmenuBuilder::new(app, t.window)
        .item(&PredefinedMenuItem::minimize(app, Some(t.minimize))?)
        .item(&PredefinedMenuItem::maximize(app, Some(t.zoom))?)
        .separator()
        .item(&PredefinedMenuItem::close_window(app, Some(t.close))?)
        .build()?;

    let menu = Menu::new(app)?;
    for item in [&app_menu, &edit, &window] {
        menu.append(&MenuItemKind::Submenu((*item).clone()))?;
    }
    Ok(menu)
}

#[cfg(test)]
mod tests {
    use super::{LOCALES, labels};

    #[test]
    fn 四言語すべてに文言がある() {
        for l in LOCALES {
            let t = labels(l);
            for (name, value) in [
                ("about", t.about),
                ("hide", t.hide),
                ("quit", t.quit),
                ("edit", t.edit),
                ("cut", t.cut),
                ("copy", t.copy),
                ("paste", t.paste),
                ("select_all", t.select_all),
                ("window", t.window),
                ("close", t.close),
            ] {
                assert!(!value.trim().is_empty(), "{l} の {name} が空");
            }
        }
    }

    #[test]
    fn 知らないロケールは英語へ落とす() {
        assert_eq!(labels("fr").copy, labels("en").copy);
        assert_eq!(labels("").copy, labels("en").copy);
    }

    #[test]
    fn 訳が英語のままになっていない() {
        // 4 言語を名乗りながら中身が英語、を防ぐ
        for l in ["ja", "zh", "ko"] {
            assert_ne!(
                labels(l).copy,
                labels("en").copy,
                "{l} の copy が英語のまま"
            );
            assert_ne!(
                labels(l).paste,
                labels("en").paste,
                "{l} の paste が英語のまま"
            );
        }
    }
}
