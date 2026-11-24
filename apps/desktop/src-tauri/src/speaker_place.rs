//! **話し手の場所**（0.1.52）。名乗り直したエージェントが、参加者の欄と `@` の候補で増えないように。
//!
//! 名札は名乗り直すと変わる。起動した場所（`--as` か起動したフォルダ）は変わらないので、
//! 発言と一緒に「名札 ↔ 場所」を送り、受けた画面が「機械 ＋ 場所」で 1 人にまとめる。

use tauri::{AppHandle, Emitter as _, Manager as _};
use warifu_core::PublicKey;
use warifu_meeting::{MeetingId, Notice};

use crate::Bridge;

/// 届いたことを画面へ知らせる（機械の鍵・名札・場所）。
pub const EVENT_SPEAKER_PLACE: &str = "warifu://speaker-place";

/// 机の呼び方（`◯◯ のエージェント`）から、起動した場所（`◯◯`）を取る。名乗っていなければ `None`。
#[must_use]
pub fn 呼び方から場所(呼び方: &str) -> Option<&str> {
    呼び方
        .strip_suffix(" のエージェント")
        .map(str::trim)
        .filter(|s| !s.is_empty())
}

/// **発言と一緒に送る。**届かない相手が居ても止めない（落ちても発言は届く）。
pub(crate) async fn 送る(
    線たち: &[tokio::sync::mpsc::Sender<Notice>],
    meeting: MeetingId,
    自分: PublicKey,
    札: &str,
    場所: &str,
) {
    let 知らせ = Notice::話し手の場所 {
        meeting,
        from: 自分,
        札: 札.to_owned(),
        場所: 場所.to_owned(),
    };
    // **組めないもの（長すぎる・行を壊す）は送らない**
    if 知らせ.to_intent().is_err() {
        return;
    }
    // 届かない相手が居ても止めない。**詰まった線で机を止めない**（同時に・時間を区切って渡す）
    crate::desk::添え物を配る(線たち, &知らせ).await;
}

/// **届いた。**画面へ出し、主催ならほかの面々へ配る。差出人の確かめは呼ぶ側が済ませてある。
pub(crate) async fn 届いた(app: &AppHandle, 線の相手: PublicKey, 知らせ: &Notice, 主催だ: bool) {
    let Notice::話し手の場所 {
        meeting,
        from,
        札,
        場所,
    } = 知らせ
    else {
        return;
    };
    let _ = app.emit(
        EVENT_SPEAKER_PLACE,
        (crate::key_to_string(*from), 札.clone(), 場所.clone()),
    );
    if !主催だ {
        return;
    }
    // **主催は、ほかの面々へそのまま配る**（差出人は書き換えない）。発言を配るのと同じ線の順
    let bridge = app.state::<Bridge>();
    let 自分 = bridge.device.public_key();
    let 面々 =
        crate::そのルームの面々(&bridge.conferences, &bridge.outbound, *meeting, 自分).await;
    for (鍵, 線) in 面々 {
        if 鍵 == 線の相手 || 鍵 == *from {
            continue;
        }
        if let Some(tx) = 線 {
            let _ = tx.send(知らせ.clone()).await;
        }
    }
}

#[cfg(test)]
mod 試験 {
    use super::*;

    #[test]
    fn 呼び方の頭が場所() {
        assert_eq!(呼び方から場所("warifu のエージェント"), Some("warifu"));
        assert_eq!(呼び方から場所("名乗りのないエージェント"), None);
        assert_eq!(呼び方から場所(" のエージェント"), None);
    }
}
