//! **送った時刻**（0.1.53）。その発言を、送った側の時計でいつ出したか。
//!
//! 発言（`Notice::Text`）は時刻を持たないので、受け手の画面は**受け取った時刻**を出していた。
//! 遅れて届いた発言や、入り直して追いついた分が、受け取った時刻で並んでいた。
//!
//! - 送る側は、発言より**先に** `Notice::送った時刻` を送る（同じ線なので順は保たれる）
//! - 受ける側は控えておき、発言が届いたら行の印で結んで、画面へ送った時刻を渡す
//! - 主催は、聞いた時刻をほかの面々へ配る。留守の客へ預かった発言を渡すときも、先に時刻を渡す
//! - 古い版の相手は時刻を送らない。そのときは今までどおり受け取った時刻を出す

use std::collections::VecDeque;
use std::sync::Mutex;

use tauri::{AppHandle, Manager as _};
use warifu_core::PublicKey;
use warifu_meeting::{MeetingId, Notice};

use crate::Bridge;
use crate::reply::印;

/// 覚えておく数。**溜め込む所にしない。**預かりの上限（1 人 200 件）より多く持つ。
const 覚える数: usize = 2048;

/// 控え（部屋・行の印・送った時刻のミリ秒）。
#[derive(Debug, Default)]
pub(crate) struct 棚 {
    時刻: VecDeque<(MeetingId, 印, u64)>,
}

impl 棚 {
    /// 控える。**同じ行は 1 つだけ**（直接と主催ごしの 2 本で届く）。初めてなら `true`。
    pub(crate) fn 控える(&mut self, 部屋: MeetingId, 行: 印, ミリ秒: u64) -> bool {
        if self.引く(部屋, &行).is_some() {
            return false;
        }
        self.時刻.push_back((部屋, 行, ミリ秒));
        while self.時刻.len() > 覚える数 {
            self.時刻.pop_front();
        }
        true
    }

    /// その行を送った時刻。
    #[must_use]
    pub(crate) fn 引く(&self, 部屋: MeetingId, 行: &印) -> Option<u64> {
        self.時刻
            .iter()
            .rev()
            .find(|(m, l, _)| *m == 部屋 && l == 行)
            .map(|(_, _, t)| *t)
    }
}

static 控え: Mutex<棚> = Mutex::new(棚 {
    時刻: VecDeque::new(),
});

/// 控えに触る（錠は短く持つ）。
pub(crate) fn 棚で<T>(f: impl FnOnce(&mut 棚) -> T) -> T {
    f(&mut 控え.lock().expect("毒されていない"))
}

/// いまの時刻（エポックからのミリ秒）。
#[must_use]
pub(crate) fn いま() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

/// **画面へ出す時刻を決める。**送った時刻が分からなければ `None`（画面は受け取った時刻を出す）。
///
/// **先の時刻は信じない。**相手の時計が進んでいると、まだ来ていない時刻の発言が並ぶ。
/// 受け取った時刻より後なら、受け取った時刻に倒す。
#[must_use]
pub(crate) fn 出す時刻(送った: Option<u64>, 受けた: u64) -> Option<u64> {
    送った.map(|t| t.min(受けた))
}

/// **自分の発言の時刻を刻んで控える。**影へ言う（預ける）より先に呼ぶ ——
/// 誰にも線が無いときの発言も、あとで入り直した客へ時刻つきで渡せるように。
pub(crate) fn 刻む(文: &Notice) -> Option<(印, u64)> {
    let Notice::Text { meeting, .. } = 文 else {
        return None;
    };
    let 行 = 文.文字の印()?;
    let ミリ秒 = いま();
    棚で(|棚| 棚.控える(*meeting, 行, ミリ秒));
    Some((行, ミリ秒))
}

/// 刻んだ時刻を、その部屋の面々へ渡す。**発言より先に呼ぶ**（同じ線なので順が保たれる）。
pub(crate) async fn 送る(
    面々: &[tokio::sync::mpsc::Sender<Notice>],
    部屋: MeetingId,
    自分: PublicKey,
    (行, ミリ秒): (印, u64),
) {
    let 知らせ = Notice::送った時刻 {
        meeting: 部屋,
        from: 自分,
        行,
        ミリ秒,
    };
    for tx in 面々 {
        // 届かない相手が居ても止めない（時刻が落ちても、発言そのものは届く）
        let _ = tx.send(知らせ.clone()).await;
    }
}

/// **預かった発言を渡す前に、その発言の時刻を添える**（控えに無ければ `None`）。
#[must_use]
pub(crate) fn 預かりに添える(文: &Notice) -> Option<Notice> {
    let Notice::Text { meeting, from, .. } = 文 else {
        return None;
    };
    let 行 = 文.文字の印()?;
    let ミリ秒 = 棚で(|棚| 棚.引く(*meeting, &行))?;
    Some(Notice::送った時刻 {
        meeting: *meeting,
        from: *from,
        行,
        ミリ秒,
    })
}

/// **相手から時刻が届いた。**控え、主催ならほかの面々へ配る。
///
/// 差出人の確かめ（主催ごしの分だけ通す）は、呼ぶ側が会話の発言と同じ規則で済ませてある。
pub(crate) async fn 届いた(
    app: &AppHandle, 線の相手: PublicKey, 知らせ: &Notice, 主催だ: bool
) {
    let Notice::送った時刻 {
        meeting,
        from,
        行,
        ミリ秒,
    } = 知らせ
    else {
        return;
    };
    let 初めて = 棚で(|棚| 棚.控える(*meeting, *行, *ミリ秒));
    if !初めて || !主催だ {
        return;
    }
    // **主催は、ほかの面々へそのまま配る**（差出人は書き換えない・D48）。発言より先に着くように、
    // 発言を配るのと同じ線の順で渡す
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
mod tests {
    use super::*;

    #[test]
    fn 時刻は控えた行の印で引け_部屋を取り違えず_2本目で書き換えない() {
        let (a, b) = (MeetingId::generate(), MeetingId::generate());
        let mut 棚 = 棚::default();
        assert!(棚.控える(a, [1; 32], 100));
        assert!(!棚.控える(a, [1; 32], 999), "主催ごしの 2 本目");
        assert_eq!(棚.引く(a, &[1; 32]), Some(100));
        assert_eq!(棚.引く(b, &[1; 32]), None, "別の部屋の同じ印は別の行");
    }

    #[test]
    fn 溜め込まない() {
        let a = MeetingId::generate();
        let mut 棚 = 棚::default();
        for n in 0..(覚える数 + 10) {
            let mut 印 = [0u8; 32];
            印[..8].copy_from_slice(&(n as u64).to_be_bytes());
            棚.控える(a, 印, n as u64);
        }
        assert_eq!(棚.時刻.len(), 覚える数);
        assert_eq!(棚.引く(a, &[0; 32]), None, "古い分から落とす");
    }

    #[test]
    fn 送った時刻を出し_分からなければ受けた時刻_先の時刻は受けた時刻に倒す() {
        assert_eq!(
            出す時刻(Some(1_000), 5_000),
            Some(1_000),
            "遅れて届いた発言は送った時刻"
        );
        assert_eq!(出す時刻(None, 5_000), None, "古い版の相手");
        assert_eq!(
            出す時刻(Some(9_000), 5_000),
            Some(5_000),
            "相手の時計が進んでいる"
        );
    }

    #[test]
    fn 預かった発言には_控えた時刻を添える() {
        let 部屋 = MeetingId::generate();
        let 差出人 = warifu_core::Seed::from_bytes([7; 32])
            .profile("Personal")
            .device("PC")
            .public_key();
        let 文 = Notice::Text {
            meeting: 部屋,
            from: 差出人,
            話し手: None,
            body: "預かり試験の発言".to_owned(),
        };
        assert_eq!(預かりに添える(&文), None, "控えに無ければ添えない");
        let 行 = 文.文字の印().unwrap();
        棚で(|棚| 棚.控える(部屋, 行, 42));
        assert_eq!(
            預かりに添える(&文),
            Some(Notice::送った時刻 {
                meeting: 部屋,
                from: 差出人,
                行,
                ミリ秒: 42,
            })
        );
    }
}
