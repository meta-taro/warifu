//! **返信**（11-11）。どの行が、どの行への返信か。
//!
//! エージェントは `#1234` と書いて返していたが、発言の番号は機械ごとに振るので、
//! 別の機械の人には、どの発言のことか分からなかった。
//! **線には行の印（`Notice::文字の印`）だけを載せる**（どの機械で作っても同じ印）。
//!
//! - 送る側は、返信の発言より**先に** `Notice::返信` を送る（同じ線なので順は保たれる）
//! - 受ける側は控えておき、発言が届いたら印で結ぶ（画面へは返信先の印、エージェントへは
//!   **この機械の番号**で「（#N への返信）」と本文の頭に書く）
//! - エージェントは `reply_to` に自分の `chat_read` の番号を書く。机がその番号の行の印に直す

use std::collections::VecDeque;
use std::sync::Mutex;

use tauri::{AppHandle, Manager as _};
use warifu_core::PublicKey;
use warifu_meeting::{MeetingId, Notice};

use crate::Bridge;

/// 覚えておく数。**溜め込む所にしない。**
const 覚える数: usize = 1024;

/// 行の印。
pub(crate) type 印 = [u8; 32];

/// **返信の控え**（部屋・返信の行・返信先）と、**机の番号の控え**（番号・部屋・行の印）。
#[derive(Debug, Default)]
pub(crate) struct 棚 {
    返信: VecDeque<(MeetingId, 印, 印)>,
    番号: VecDeque<(u64, MeetingId, 印)>,
}

impl 棚 {
    /// 返信を控える。**同じ行は 1 つだけ**（直接と主催ごしの 2 本で届く）。
    pub(crate) fn 返信を控える(&mut self, 部屋: MeetingId, 行: 印, 先: 印) {
        if self.返信.iter().any(|(m, l, _)| *m == 部屋 && *l == 行) {
            return;
        }
        self.返信.push_back((部屋, 行, 先));
        while self.返信.len() > 覚える数 {
            self.返信.pop_front();
        }
    }

    /// その行は、どの行への返信か。
    #[must_use]
    pub(crate) fn 返信先(&self, 部屋: MeetingId, 行: &印) -> Option<印> {
        self.返信
            .iter()
            .find(|(m, l, _)| *m == 部屋 && l == 行)
            .map(|(_, _, 先)| *先)
    }

    /// 机の番号と行の印を結んで控える。
    pub(crate) fn 番号を控える(&mut self, 番号: u64, 部屋: MeetingId, 行: 印) {
        if 番号 == 0 {
            return;
        }
        self.番号.push_back((番号, 部屋, 行));
        while self.番号.len() > 覚える数 {
            self.番号.pop_front();
        }
    }

    /// その番号の行（部屋と印）。この機械に残っていなければ `None`。
    #[must_use]
    pub(crate) fn 番号の行(&self, 番号: u64) -> Option<(MeetingId, 印)> {
        self.番号
            .iter()
            .find(|(n, _, _)| *n == 番号)
            .map(|(_, m, l)| (*m, *l))
    }

    /// 残す形の写し。
    #[must_use]
    fn 番号の写し(&self) -> Vec<残す行> {
        self.番号
            .iter()
            .map(|(n, m, l)| (*n, m.to_string(), crate::ack::印を字に(l)))
            .collect()
    }

    /// 残した写しから戻す。読めない行は飛ばす。戻した数を返す。
    fn 写しから戻す(&mut self, 行たち: &[残す行]) -> usize {
        let mut 数 = 0;
        for (n, m, l) in 行たち {
            if let (Ok(部屋), Some(行)) = (m.parse::<MeetingId>(), crate::ack::字を印に(l)) {
                self.番号を控える(*n, 部屋, 行);
                数 += 1;
            }
        }
        数
    }

    /// その行の、この機械の番号。
    #[must_use]
    pub(crate) fn 印の番号(&self, 部屋: MeetingId, 行: &印) -> Option<u64> {
        self.番号
            .iter()
            .rev()
            .find(|(_, m, l)| *m == 部屋 && l == 行)
            .map(|(n, _, _)| *n)
    }
}

static 控え: Mutex<棚> = Mutex::new(棚 {
    返信: VecDeque::new(),
    番号: VecDeque::new(),
});

/// 控えに触る（錠は短く持つ）。
pub(crate) fn 棚で<T>(f: impl FnOnce(&mut 棚) -> T) -> T {
    f(&mut 控え.lock().expect("毒されていない"))
}

/// **番号と行の印を結んで控え、手元にも残す**（11-12）。
///
/// 記憶の中にしか無かったので、画面を立ち上げ直すと、前の起動の `chat_read` に出た番号を
/// `reply_to` に書いても「この機械に残っていない」で断っていた（見せた番号が使えない）。
pub(crate) fn 番号を控える(番号: u64, 部屋: MeetingId, 行: 印) {
    let 写し = 棚で(|棚| {
        棚.番号を控える(番号, 部屋, 行);
        棚.番号の写し()
    });
    if let Some((置き場, 自分)) = 残す先.get() {
        let _錠 = 書く錠.lock().expect("毒されていない");
        if let Err(e) = 書く(置き場, *自分, &写し) {
            記録!("返信の番号の控えを残せませんでした（返信はこの起動の間だけ使えます）: {e}");
        }
    }
}

/// 番号の控えを残す先。立ち上げで決まる。**試験では決めない**（本物の置き場を書かない）。
static 残す先: std::sync::OnceLock<(std::path::PathBuf, PublicKey)> = std::sync::OnceLock::new();
/// 残すのは 1 度に 1 つ（古い写しが新しい写しを上書きしない）。
static 書く錠: Mutex<()> = Mutex::new(());

/// 残す形（番号・部屋の id・行の印の字）。
type 残す行 = (u64, String, String);

fn 控えの道(置き場: &std::path::Path) -> std::path::PathBuf {
    置き場.join("desk-reply-marks.seal")
}

/// **封をして置く**（`recent_keep` と同じ置き方。先に別の名で書いて入れ替える）。
fn 書く(置き場: &std::path::Path, 自分: PublicKey, 行たち: &[残す行]) -> Result<(), String> {
    let 中身 = serde_json::to_vec(行たち).map_err(|e| format!("形にできません: {e}"))?;
    let 封 = warifu_seal::seal(自分, &中身).map_err(|e| format!("封ができません: {e}"))?;
    std::fs::create_dir_all(置き場).map_err(|e| format!("置き場所を作れません: {e}"))?;
    let 途中 = 置き場.join("desk-reply-marks.seal.tmp");
    crate::chat_keep::自分だけで書く(&途中, &封.to_bytes()).map_err(|e| format!("書けません: {e}"))?;
    std::fs::rename(&途中, 控えの道(置き場)).map_err(|e| format!("置けません: {e}"))
}

/// **読む。**無ければ空。開けなければ空で始める（返信の番号が使えないだけで、会話は止めない）。
fn 読む(置き場: &std::path::Path, 自分: &warifu_core::Device) -> Vec<残す行> {
    let Ok(中身) = std::fs::read(控えの道(置き場)) else {
        return Vec::new();
    };
    warifu_seal::Sealed::from_bytes(&中身)
        .and_then(|封| warifu_seal::open(自分, &封).ok())
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

/// 立ち上げたとき、残した番号の控えを戻し、以後の控えを手元へ残すようにする。
pub(crate) fn 控えを開く(置き場: &std::path::Path, 自分: &warifu_core::Device) {
    let 行たち = 読む(置き場, 自分);
    let 数 = 棚で(|棚| 棚.写しから戻す(&行たち));
    記録!("返信の番号の控えを戻しました（{数} 行）");
    let _ = 残す先.set((置き場.to_path_buf(), 自分.public_key()));
}

/// **エージェントへ渡す本文の頭に、返信先をこの機械の番号で書く。**
///
/// 机の行の形（`Heard`）は変えない —— 古い口は知らない欄の付いた行を 1 行も読めない。
#[must_use]
pub(crate) fn 頭に書く(本文: &str, 先の番号: Option<u64>) -> String {
    match 先の番号 {
        Some(n) => format!("（#{n} への返信）\n{本文}"),
        None => format!("（返信・元の発言はこの機械に残っていません）\n{本文}"),
    }
}

/// 返信の印を、その部屋の面々へ渡す。**返信の発言より先に呼ぶ**（同じ線なので順が保たれる）。
pub(crate) async fn 送る(
    面々: &[tokio::sync::mpsc::Sender<Notice>],
    部屋: MeetingId,
    自分: PublicKey,
    行: 印,
    先: 印,
) {
    let 知らせ = Notice::返信 {
        meeting: 部屋,
        from: 自分,
        行,
        先,
    };
    for tx in 面々 {
        // 届かない相手が居ても止めない（返信の印が落ちても、発言そのものは届く）
        let _ = tx.send(知らせ.clone()).await;
    }
}

/// **相手から返信の印が届いた。**控え、主催ならほかの面々へ配る。
///
/// 差出人の確かめ（主催ごしの分だけ通す）は、呼ぶ側が会話の発言と同じ規則で済ませてある。
pub(crate) async fn 届いた(app: &AppHandle, 線の相手: PublicKey, 知らせ: &Notice, 主催だ: bool) {
    let Notice::返信 {
        meeting,
        from,
        行,
        先,
    } = 知らせ
    else {
        return;
    };
    let 初めて = 棚で(|棚| {
        let 前から = 棚.返信先(*meeting, 行).is_some();
        棚.返信を控える(*meeting, *行, *先);
        !前から
    });
    if !初めて || !主催だ {
        return;
    }
    // **主催は、ほかの面々へそのまま配る**（差出人は書き換えない・D48）。発言より先に着くように、
    // 発言を配るのと同じ線の順で渡す
    let bridge = app.state::<Bridge>();
    let 自分 = bridge.device.public_key();
    let 面々 = crate::そのルームの面々(&bridge.conferences, &bridge.outbound, *meeting, 自分).await;
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
    fn 返信は控えた行の印で引け_部屋を取り違えない() {
        let (a, b) = (MeetingId::generate(), MeetingId::generate());
        let mut 棚 = 棚::default();
        棚.返信を控える(a, [1; 32], [2; 32]);
        棚.返信を控える(a, [1; 32], [9; 32]);
        assert_eq!(棚.返信先(a, &[1; 32]), Some([2; 32]), "2 本目で書き換えない");
        assert_eq!(棚.返信先(b, &[1; 32]), None, "別の部屋の同じ印は別の行");
    }

    #[test]
    fn 机の番号と行の印を_両方向に引ける() {
        let a = MeetingId::generate();
        let mut 棚 = 棚::default();
        棚.番号を控える(2127, a, [5; 32]);
        棚.番号を控える(0, a, [6; 32]);
        assert_eq!(棚.番号の行(2127), Some((a, [5; 32])));
        assert_eq!(棚.印の番号(a, &[5; 32]), Some(2127));
        assert_eq!(棚.印の番号(a, &[6; 32]), None, "番号の無い行は控えない");
    }

    #[test]
    fn 番号の控えは_残して戻せる() {
        // 11-12、立ち上げ直すと、前の起動の chat_read に出た番号を reply_to に使えなかった
        let a = MeetingId::generate();
        let mut 前 = 棚::default();
        前.番号を控える(1772, a, [5; 32]);
        let 写し = 前.番号の写し();
        let 置き場 = std::env::temp_dir().join(format!("warifu-reply-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&置き場);
        let 身元 = warifu_core::Seed::from_bytes([3; 32]).profile("Personal").device("PC");
        書く(&置き場, 身元.public_key(), &写し).unwrap();
        let mut 後 = 棚::default();
        assert_eq!(後.写しから戻す(&読む(&置き場, &身元)), 1);
        assert_eq!(後.番号の行(1772), Some((a, [5; 32])));
        let 別の身元 = warifu_core::Seed::from_bytes([4; 32]).profile("Personal").device("PC");
        assert!(読む(&置き場, &別の身元).is_empty(), "ほかの身元では開けない");
        let _ = std::fs::remove_dir_all(&置き場);
    }

    #[test]
    fn 溜め込まない() {
        let a = MeetingId::generate();
        let mut 棚 = 棚::default();
        for n in 1..=(覚える数 as u64 + 10) {
            棚.番号を控える(n, a, [0; 32]);
        }
        assert_eq!(棚.番号の行(1), None);
        assert!(棚.番号の行(覚える数 as u64 + 10).is_some());
    }

    #[test]
    fn エージェントへは_この機械の番号で返信先を書く() {
        assert_eq!(頭に書く("確かめました", Some(2127)), "（#2127 への返信）\n確かめました");
        assert!(頭に書く("確かめました", None).starts_with("（返信・元の発言はこの機械に残っていません）"));
    }
}
