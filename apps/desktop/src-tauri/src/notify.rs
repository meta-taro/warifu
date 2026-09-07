//! 届いたことを、**窓の外へ押し出す。**
//!
//! オーナーの言（2026-09-07）——
//! 「**プッシュできないと、LINE の連絡みたいに。そうじゃないとチャットとしては
//! あまり機能しないっすね。**」
//!
//! そのとおりで、こちらから押し出せなければチャットにならない。
//! ただし押し出しは**2 つの別の問題**に分かれる。**混ぜない。**
//!
//! | | いま |
//! |---|---|
//! | **A 人が気づかない**（窓が後ろ・別の作業中） | **ここで直す** |
//! | **B 相手が落ちていると、そもそも届かない** | **直らない。**預かり所が要る（D13 / D7） |
//!
//! B は「度を超えるとうるさい」ためオーナーが今回は見送った（2026-09-07）。
//! **相手が起動していない間のものは、いまも消える。**
//!
//! # うるさくしないための 3 つ
//!
//! - **窓が前に居るときは鳴らさない。**見えているものを知らせない
//! - **本文を出さない。**通知はロック画面にも出る ——
//!   会議の中身をそこへ置かない（`warifu.log` に中身を書かないのと同じ筋）
//! - **短い間に何度も鳴らさない**

use std::sync::atomic::{AtomicU64, Ordering};

use tauri::{AppHandle, Manager as _};
use tauri_plugin_notification::{NotificationExt as _, PermissionState};

/// 次に鳴らしてよくなるまでの間（秒）。
///
/// **1 通ごとに鳴らさない。**会話は続けて飛んでくるものなので、
/// 1 通ずつ鳴らすと、それだけで人の注意を使い切る（**D31** と同じ考え）。
const 鳴らす間隔の秒: u64 = 20;

/// 最後に鳴らした時刻（Unix 秒）。
static 最後に鳴らした: AtomicU64 = AtomicU64::new(0);

/// 届いたことを知らせる。
///
/// **中身は渡さない。**渡す口を作らないので、うっかり出すこともない。
pub fn 届いたと知らせる(app: &AppHandle, 誰から: &str) {
    if 前に居る(app) {
        // 見えている。**知らせる必要が無い**
        return;
    }
    if !間が空いた() {
        return;
    }

    // **窓そのものにも合図する。**通知が切られていても、これは効く
    // （macOS は Dock が跳ね、Windows はタスクバーが光る）。核の口なので依存が増えない
    if let Some(窓) = app.get_webview_window("main") {
        let _ = 窓.request_user_attention(Some(tauri::UserAttentionType::Informational));
    }

    知らせを出す(app, 誰から);
}

fn 前に居る(app: &AppHandle) -> bool {
    app.get_webview_window("main")
        .and_then(|w| w.is_focused().ok())
        .unwrap_or(false)
}

fn 間が空いた() -> bool {
    let 今 = crate::now_secs();
    let 前 = 最後に鳴らした.load(Ordering::Relaxed);
    if 今.saturating_sub(前) < 鳴らす間隔の秒 {
        return false;
    }
    最後に鳴らした.store(今, Ordering::Relaxed);
    true
}

/// OS の通知を出す。**出せなくても止めない。**
///
/// 人が通知を切っていることも、まだ許していないこともある。
/// **そのときは Dock が跳ねるだけで済ませる** —— 会議は続く。
fn 知らせを出す(app: &AppHandle, 誰から: &str) {
    let 口 = app.notification();
    // 許可を尋ねるのは 1 回だけ。**断られたら、もう尋ねない**（D31 と同じ構え）
    if matches!(口.permission_state(), Ok(PermissionState::Prompt)) {
        let _ = 口.request_permission();
    }
    if !matches!(口.permission_state(), Ok(PermissionState::Granted)) {
        記録!("通知: 許可が無いので出さない（窓の合図だけ）");
        return;
    }
    // **本文を入れない。**誰からかだけ。通知はロック画面にも出る
    let 結果 = 口
        .builder()
        .title("warifu")
        .body(format!("{誰から} から届きました"))
        .show();
    if let Err(e) = 結果 {
        記録!("通知: 出せませんでした: {e}");
    }
}
