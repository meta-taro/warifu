//! **エージェントの専用カーソル**（11-12）。席の `screen` の指示を画面へ渡し、動かした結果を返す。
//!
//! 動かすのは画面（Svelte の `lib/agent/cursor.ts`）。本体は頼んで、返りを待って、渡すだけ。
//! 人のマウスとキーボードは使わない。人が押すもの（札の［通す］など）は画面が断る。
//! 返りが無ければ「画面を開いてから」と返す（画面が閉じている・古い版の画面）。

use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use tauri::{AppHandle, Emitter as _};
use tokio::sync::oneshot;

/// 画面へ「動かして」と頼む。中身は `(頼みの番号, 指示の JSON)`。
pub(crate) const EVENT_AGENT_CURSOR: &str = "warifu://agent-cursor";

/// 画面が動かして返すまで待つ秒。**エージェントの口の待ち（5 秒）より短くする。**
/// 動きは 0.65 秒。前の指示が並んでいると、その分だけ遅れる
const 待つ秒: u64 = 4;

static 次の番号: AtomicU64 = AtomicU64::new(1);
static 待ち: Mutex<Option<HashMap<u64, oneshot::Sender<String>>>> = Mutex::new(None);

/// 画面に動かしてもらう。返す `Err` は人が読む訳。
pub async fn 動かしてもらう(
    app: &AppHandle,
    指示: serde_json::Value,
) -> Result<String, String> {
    let 番号 = 次の番号.fetch_add(1, Ordering::Relaxed);
    let (返す, 受ける) = oneshot::channel();
    待ち
        .lock()
        .expect("毒されていない")
        .get_or_insert_with(HashMap::new)
        .insert(番号, 返す);
    if let Err(e) = app.emit(EVENT_AGENT_CURSOR, (番号, 指示)) {
        取り下げる(番号);
        return Err(format!("画面へ頼めませんでした: {e}"));
    }
    match tokio::time::timeout(Duration::from_secs(待つ秒), 受ける).await {
        Ok(Ok(結果)) => Ok(結果),
        _ => {
            取り下げる(番号);
            Err("画面が応えませんでした。割符の画面を開いてから、もう一度呼んでください（古い版の画面かもしれません）".to_owned())
        }
    }
}

fn 取り下げる(番号: u64) {
    if let Some(棚) = 待ち.lock().expect("毒されていない").as_mut() {
        棚.remove(&番号);
    }
}

/// **画面が結果を返す口。**頼んでいない番号は捨てる。結果は 1 文なので短く切る。
#[tauri::command]
pub fn agent_cursor_reply(id: u64, text: String) {
    let 返す = 待ち
        .lock()
        .expect("毒されていない")
        .as_mut()
        .and_then(|棚| 棚.remove(&id));
    if let Some(返す) = 返す {
        let _ = 返す.send(text.chars().take(600).collect());
    }
}
