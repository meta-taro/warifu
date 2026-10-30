//! **画面の写し**（0.1.34）。エージェントの `app_view` に、画面がいま見せているものを文字で返す。
//!
//! 本体の知っていること（`room_status`）と、人の画面に出ていることは食い違うことがある ——
//! 10-22、届いた文字が別のルームの欄に積まれていた。本体からは見えなかった。
//! **写しは画面（Svelte）が書く。**本体は頼んで、返りを待って、渡すだけ。

use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use tauri::{AppHandle, Emitter as _};
use tokio::sync::oneshot;

/// 画面へ「写して」と頼む。中身は頼みの番号。
pub(crate) const EVENT_SNAPSHOT_ASK: &str = "warifu://snapshot-ask";

/// 画面が描いて返すまで待つ秒。**エージェントの口の待ち（5 秒）より短くする。**
const 待つ秒: u64 = 4;

static 次の番号: AtomicU64 = AtomicU64::new(1);
static 待ち: Mutex<Option<HashMap<u64, oneshot::Sender<String>>>> = Mutex::new(None);

/// 画面に写してもらう。返す `Err` は人が読む訳。
pub async fn 写してもらう(app: &AppHandle) -> Result<String, String> {
    let 番号 = 次の番号.fetch_add(1, Ordering::Relaxed);
    let (返す, 受ける) = oneshot::channel();
    待ち
        .lock()
        .expect("毒されていない")
        .get_or_insert_with(HashMap::new)
        .insert(番号, 返す);
    if let Err(e) = app.emit(EVENT_SNAPSHOT_ASK, 番号) {
        取り下げる(番号);
        return Err(format!("画面へ頼めませんでした: {e}"));
    }
    match tokio::time::timeout(Duration::from_secs(待つ秒), 受ける).await {
        Ok(Ok(中身)) => Ok(中身),
        _ => {
            取り下げる(番号);
            Err(
                "画面が写しを返しませんでした（窓が閉じている・古い版の画面かもしれません）"
                    .to_owned(),
            )
        }
    }
}

fn 取り下げる(番号: u64) {
    if let Some(棚) = 待ち.lock().expect("毒されていない").as_mut() {
        棚.remove(&番号);
    }
}

/// **画面が写しを返す口。**頼んでいない番号は捨てる。
#[tauri::command]
pub fn snapshot_reply(id: u64, text: String) {
    let 返す = 待ち
        .lock()
        .expect("毒されていない")
        .as_mut()
        .and_then(|棚| 棚.remove(&id));
    if let Some(返す) = 返す {
        let _ = 返す.send(切る(&text, warifu_desk::写しの上限));
    }
}

/// **上限のバイトに収める。**字の途中では切らない。切ったら印を付ける。
fn 切る(文: &str, 上限: usize) -> String {
    if 文.len() <= 上限 {
        return 文.to_owned();
    }
    let 印 = "\n…（長いので、ここで切りました）";
    let mut 端 = 上限.saturating_sub(印.len());
    while !文.is_char_boundary(端) {
        端 -= 1;
    }
    format!("{}{印}", &文[..端])
}

#[cfg(test)]
mod tests {
    use super::切る;

    #[test]
    fn 上限に収まれば_そのまま() {
        assert_eq!(切る("あいう", 100), "あいう");
    }

    #[test]
    fn 長ければ_字の途中で切らずに_上限に収める() {
        let 長い = "あ".repeat(100);
        let 切った = 切る(&長い, 50);
        assert!(切った.len() <= 50, "{}", 切った.len());
        assert!(切った.ends_with("切りました）"));
    }
}
