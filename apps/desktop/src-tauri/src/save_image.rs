//! **届いた画像を、人がダウンロードのフォルダへ保存する**（0.1.43）。
//!
//! 画面は画像を大きく見せるだけで、手元に残す口が無かった。共有された画像を、
//! ほかの道具で開いたり送り直したりできない。

use std::path::{Path, PathBuf};

use tauri::{AppHandle, Manager};

use crate::{Answer, Failure};

/// **保存する名を決める。**`warifu-<日時>.<拡張子>`。同じ名があれば `-2`・`-3` … を付ける（上書きしない）。
fn 保存する名(種類: &str, 日時: &str, 在るか: impl Fn(&str) -> bool) -> String {
    let 拡張子 = match 種類 {
        "image/png" => "png",
        "image/webp" => "webp",
        "image/gif" => "gif",
        _ => "jpg",
    };
    let 頭 = format!("warifu-{日時}");
    let mut 名 = format!("{頭}.{拡張子}");
    let mut 次 = 2;
    while 在るか(&名) {
        名 = format!("{頭}-{次}.{拡張子}");
        次 += 1;
    }
    名
}

fn 書く(置き場: &Path, 種類: &str, 中身: &[u8]) -> std::io::Result<PathBuf> {
    let 日時 = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
    let 名 = 保存する名(種類, &日時, |名| 置き場.join(名).exists());
    let 道 = 置き場.join(名);
    std::fs::write(&道, 中身)?;
    Ok(道)
}

/// 画像をダウンロードのフォルダへ書き、書いた場所を返す。
#[tauri::command]
pub(crate) fn save_image(app: AppHandle, bytes: Vec<u8>, kind: String) -> Answer<String> {
    if bytes.is_empty() || bytes.len() > warifu_meeting::IMAGE_MAX {
        return Err(Failure {
            message: "保存できる画像ではありません".into(),
            code: Some("image.too_large".into()),
        });
    }
    let 置き場 = app.path().download_dir().map_err(|e| Failure {
        message: format!("ダウンロードのフォルダが分かりません: {e}"),
        code: None,
    })?;
    let 道 = 書く(&置き場, &kind, &bytes)?;
    記録!("画像を保存しました（{} バイト）", bytes.len());
    Ok(道.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::保存する名;

    #[test]
    fn 種類から拡張子を決め_同じ名があれば番号を足す() {
        assert_eq!(
            保存する名("image/png", "20261104-1200", |_| false),
            "warifu-20261104-1200.png"
        );
        assert_eq!(
            保存する名("image/jpeg", "20261104-1200", |_| false),
            "warifu-20261104-1200.jpg"
        );
        let 在る = ["warifu-20261104-1200.webp", "warifu-20261104-1200-2.webp"];
        assert_eq!(
            保存する名("image/webp", "20261104-1200", |名| 在る.contains(&名)),
            "warifu-20261104-1200-3.webp"
        );
    }
}
