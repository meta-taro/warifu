//! 届いたことを、**窓の外へ押し出す。**
//!
//! 2026-09-07。こちらから押し出せなければ、チャットとしては機能しない。
//! ただし押し出しは**2 つの別の問題**に分かれる。**混ぜない。**
//!
//! | | いま |
//! |---|---|
//! | **A 人が気づかない**（窓が後ろ・別の作業中） | **ここで直す** |
//! | **B 相手が落ちていると、そもそも届かない** | **直らない。**預かり所が要る（D13 / D7） |
//!
//! B は度を超えるとうるさくなるので、今回は見送った（2026-09-07）。
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

    知らせを出す(app, &format!("{誰から} から届きました"));
}

/// **人に決めてもらうことがある**と知らせる（鍵の手渡し・2026-10-20）。
///
/// 承認の帯は窓の中に出る。**窓が後ろにあると、帯が出ていることに気づけない。**
/// 窓が前に居るときは鳴らさない（帯が見えている）。
pub fn 承認を頼む(app: &AppHandle, 本文: &str) {
    if 前に居る(app) || !間が空いた() {
        return;
    }
    if let Some(窓) = app.get_webview_window("main") {
        let _ = 窓.request_user_attention(Some(tauri::UserAttentionType::Critical));
    }
    知らせを出す(app, 本文);
}

/// **誰かがルームに入った**ことを知らせる（2026-10-20）。
///
/// 連絡先から呼ばれた側は、知り合いなら人に聞かずにルームへ入る。
/// **鳴らさないと、呼んだ相手だけが画面の前で待つことになる。**
/// 届いたときと同じく、窓が前に居るときは鳴らさず、短い間に何度も鳴らさない。
pub fn 入ったと知らせる(app: &AppHandle, 誰: &str) {
    if 前に居る(app) || !間が空いた() {
        return;
    }
    if let Some(窓) = app.get_webview_window("main") {
        let _ = 窓.request_user_attention(Some(tauri::UserAttentionType::Informational));
    }
    知らせを出す(app, &format!("{誰} がルームに入りました"));
}

/// **長く戻らない人がいる**と知らせる（0.1.28）。窓が前に居るときは鳴らさない。
pub fn 不在を知らせる(app: &AppHandle, 誰: &str, 分: u64) {
    if 前に居る(app) || !間が空いた() {
        return;
    }
    if let Some(窓) = app.get_webview_window("main") {
        let _ = 窓.request_user_attention(Some(tauri::UserAttentionType::Informational));
    }
    知らせを出す(app, &format!("{誰} が {分} 分、ルームに戻っていません"));
}

/// **エージェントがルームキーで入った**（0.1.30）。**いつも鳴らす**（間を空けない・前に居ても）。
///
/// 人が画面の前に居なくても入れる口なので、**入ったことは必ず人に見せる**
/// （流れてきた鍵で入れられたなら、人が気づいて抜けられるように）。
pub fn エージェントが入ったと知らせる(app: &AppHandle, 誰: &str, 主催: &str) {
    if let Some(窓) = app.get_webview_window("main") {
        let _ = 窓.request_user_attention(Some(tauri::UserAttentionType::Informational));
    }
    知らせを出す(
        app,
        &format!("{誰} がルームキーで別のルームへ入りました（主催 {主催}）。心当たりが無ければ、そのルームを抜けてください"),
    );
}

/// **エージェントが住所から連絡先を足したら、人に必ず知らせる**（0.1.33）。
///
/// 戸口にも載せるので、その相手は鍵なしでこの PC を呼べるようになる。心当たりが無ければ人が降ろす。
pub fn エージェントが連絡先を足したと知らせる(app: &AppHandle, 誰: &str, 名前: &str, 鍵の頭: &str) {
    if let Some(窓) = app.get_webview_window("main") {
        let _ = 窓.request_user_attention(Some(tauri::UserAttentionType::Informational));
    }
    知らせを出す(
        app,
        &format!("{誰} が「{名前}」（{鍵の頭}）を連絡先に足しました。この相手は鍵なしでこの PC を呼べます。心当たりが無ければ、連絡帳から降ろしてください"),
    );
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
fn 知らせを出す(app: &AppHandle, 本文: &str) {
    let 口 = app.notification();
    // 許可を尋ねるのは 1 回だけ。**断られたら、もう尋ねない**（D31 と同じ構え）
    if matches!(口.permission_state(), Ok(PermissionState::Prompt)) {
        let _ = 口.request_permission();
    }
    if !matches!(口.permission_state(), Ok(PermissionState::Granted)) {
        記録!("通知: 許可が無いので出さない（窓の合図だけ）");
        return;
    }
    // **会話の本文を入れない。**誰が・何が起きたかだけ。通知はロック画面にも出る
    let 結果 = 口.builder().title("warifu").body(本文).show();
    if let Err(e) = 結果 {
        記録!("通知: 出せませんでした: {e}");
    }
}
