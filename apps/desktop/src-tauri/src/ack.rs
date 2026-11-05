//! **メンションされた行を受けた**（D130・0.1.45）。
//!
//! 既読ではない（D94 はそのまま）。人の画面に出ただけでは返さない。返すのは 3 つの時だけ ——
//!
//! - 人が［受けました］を押した（`chat_ack`）
//! - 人がその部屋で返事を書いた（`send_text` が、その部屋でまだ受けていない分を返す）
//! - この機械のエージェントが、自分を呼んだ行を読んだ（`desk` の `Read`）
//!
//! 指すのは行の印（`Notice::文字の印`）だけ。**本文は運ばない。**

use std::collections::{BTreeMap, VecDeque};
use std::sync::Mutex;

use tauri::{AppHandle, Emitter as _, Manager as _};
use warifu_core::PublicKey;
use warifu_meeting::{MeetingId, Notice, 受け方};

use crate::Bridge;

/// 画面へ「受けた」を渡す出来事。`(部屋, 行の印, 誰, 受け方)`。
pub(crate) const EVENT_ACK: &str = "warifu://ack";

/// 覚えておく「呼ばれた行」の数。**溜め込む所にしない。**
const 覚える行: usize = 512;

/// **この機械で呼ばれた行**（受けたを返す前の控え）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct 呼ばれた行 {
    /// 机の通し番号（エージェントの読みと突き合わせる）。
    pub 番号: u64,
    /// どの部屋か。
    pub 部屋: MeetingId,
    /// 行の印。
    pub 印: [u8; 32],
    /// 本文（誰が呼ばれたかを、読んだエージェントの名で見直すため）。**線には載せない。**
    pub 本文: String,
    /// もう人が受けたか（押した・返事を書いた）。
    pub 人が受けた: bool,
}

static 呼ばれた: Mutex<VecDeque<呼ばれた行>> = Mutex::new(VecDeque::new());

/// 届いた文字を控える。**`@` で誰かを呼んでいる行だけ**（呼んでいない行に受けたは要らない）。
pub(crate) fn 控える(番号: u64, 部屋: MeetingId, 印: [u8; 32], 本文: &str) {
    if !本文.contains('@') {
        return;
    }
    let mut 棚 = 呼ばれた.lock().expect("毒されていない");
    棚.push_back(呼ばれた行 {
        番号,
        部屋,
        印,
        本文: 本文.to_owned(),
        人が受けた: false,
    });
    while 棚.len() > 覚える行 {
        棚.pop_front();
    }
}

/// **エージェントが読んだ範囲のうち、そのエージェントを呼んだ行**（`前 < 番号 <= まで`）。
#[must_use]
pub(crate) fn 読んで受けた行(
    棚: &VecDeque<呼ばれた行>,
    前: u64,
    まで: u64,
    呼ばれる名: &[String],
) -> Vec<(MeetingId, [u8; 32])> {
    棚.iter()
        .filter(|行| 行.番号 > 前 && 行.番号 <= まで)
        .filter(|行| warifu_desk::宛てられたか(&行.本文, 呼ばれる名))
        .map(|行| (行.部屋, 行.印))
        .collect()
}

/// **人がまだ受けていない、人を呼んだ行**を取り出し、受けた印を付ける（その部屋の分だけ）。
#[must_use]
pub(crate) fn 人が返事で受けた行(
    棚: &mut VecDeque<呼ばれた行>,
    部屋: MeetingId,
    呼ばれる名: &[String],
) -> Vec<[u8; 32]> {
    棚.iter_mut()
        .filter(|行| 行.部屋 == 部屋 && !行.人が受けた)
        .filter(|行| warifu_desk::宛てられたか(&行.本文, 呼ばれる名))
        .map(|行| {
            行.人が受けた = true;
            行.印
        })
        .collect()
}

/// エージェントが読んだ。**そのエージェントを呼んだ行に、受けたを返す。**
pub(crate) fn エージェントが読んだ(app: &AppHandle, 前: u64, まで: u64, 名札: &str) {
    let mut 呼ばれる名 = vec![名札.to_owned()];
    if let Some(頭) = 名札.strip_suffix(" のエージェント") {
        呼ばれる名.push(頭.to_owned());
    }
    let 行たち = {
        let 棚 = 呼ばれた.lock().expect("毒されていない");
        読んで受けた行(&棚, 前, まで, &呼ばれる名)
    };
    if 行たち.is_empty() {
        return;
    }
    let app = app.clone();
    let 名札 = 名札.to_owned();
    tauri::async_runtime::spawn(async move {
        let bridge = app.state::<Bridge>();
        for (部屋, 印) in 行たち {
            送る(&bridge, 部屋, 印, Some(名札.clone()), 受け方::読んだ).await;
        }
    });
}

/// 人がその部屋で返事を書いた。**まだ受けていない、人を呼んだ行に受けたを返す。**
pub(crate) async fn 人が返事を書いた(bridge: &Bridge, 部屋: MeetingId) {
    let 呼ばれる名 = crate::profile::人の呼ばれる名();
    let 印たち = {
        let mut 棚 = 呼ばれた.lock().expect("毒されていない");
        人が返事で受けた行(&mut 棚, 部屋, &呼ばれる名)
    };
    for 印 in 印たち {
        送る(bridge, 部屋, 印, None, 受け方::返事).await;
    }
}

/// 人が［受けました］を押した。
pub(crate) async fn 人が押した(bridge: &Bridge, 部屋: MeetingId, 印: [u8; 32]) {
    {
        let mut 棚 = 呼ばれた.lock().expect("毒されていない");
        for 行 in 棚.iter_mut().filter(|行| 行.印 == 印) {
            行.人が受けた = true;
        }
    }
    送る(bridge, 部屋, 印, None, 受け方::押した).await;
}

/// 受けたを、その部屋の面々へ渡す。**待たない**（会話の線を詰まらせない）。
async fn 送る(
    bridge: &Bridge,
    部屋: MeetingId,
    印: [u8; 32],
    話し手: Option<String>,
    方: 受け方,
) {
    let 自分 = bridge.device.public_key();
    let 知らせ = Notice::受けた {
        meeting: 部屋,
        from: 自分,
        話し手,
        行: 印,
        方,
    };
    let 面々 = crate::そのルームの面々(&bridge.conferences, &bridge.outbound, 部屋, 自分).await;
    let mut 渡せた = 0;
    for (_, 線) in &面々 {
        if let Some(tx) = 線
            && tx.try_send(知らせ.clone()).is_ok()
        {
            渡せた += 1;
        }
    }
    記録!(
        "受けた: {}（{:?}）を {}/{} 人へ（部屋 {}）",
        印の頭(&印),
        方,
        渡せた,
        面々.len(),
        crate::短く(&部屋.to_string())
    );
}

/// **相手から受けたが届いた。**画面へ渡し、主催ならほかの面々へ配る。
///
/// 差出人の確かめ（主催ごしの分だけ通す）は、呼ぶ側が会話の発言と同じ規則で済ませてある。
pub(crate) async fn 届いた(
    app: &AppHandle, 線の相手: PublicKey, 知らせ: &Notice, 主催だ: bool
) {
    let Notice::受けた {
        meeting,
        from,
        話し手,
        行,
        方,
    } = 知らせ
    else {
        return;
    };
    if 二本目か(*meeting, *from, 話し手.as_deref(), 行) {
        return;
    }
    let 誰 = crate::presence::見せる名(from, 話し手.as_deref());
    記録!("受信: 受けた（{}・{:?}）", 印の頭(行), 方);
    let _ = app.emit(
        EVENT_ACK,
        (
            meeting.to_string(),
            印を字に(行),
            誰,
            match 方 {
                受け方::押した => "押した",
                受け方::返事 => "返事",
                受け方::読んだ => "読んだ",
            },
        ),
    );
    if !主催だ {
        return;
    }
    // **主催は、ほかの面々へそのまま配る**（差出人は書き換えない・D48）
    let bridge = app.state::<Bridge>();
    let 自分 = bridge.device.public_key();
    let 面々 =
        crate::そのルームの面々(&bridge.conferences, &bridge.outbound, *meeting, 自分).await;
    for (鍵, 線) in 面々 {
        if 鍵 == 線の相手 || 鍵 == *from {
            continue;
        }
        if let Some(tx) = 線 {
            let _ = tx.try_send(知らせ.clone());
        }
    }
}

/// 直接と主催ごしの 2 本の道で届いた同じ受けたを、1 本にする。
fn 二本目か(
    部屋: MeetingId, from: PublicKey, 話し手: Option<&str>, 行: &[u8; 32]
) -> bool {
    static 見た: Mutex<BTreeMap<(String, [u8; 32], String, [u8; 32]), ()>> =
        Mutex::new(BTreeMap::new());
    let 鍵 = (
        部屋.to_string(),
        from.to_bytes(),
        話し手.unwrap_or_default().to_owned(),
        *行,
    );
    let mut 棚 = 見た.lock().expect("毒されていない");
    if 棚.contains_key(&鍵) {
        return true;
    }
    棚.insert(鍵, ());
    while 棚.len() > 覚える行 * 4 {
        let 古い = 棚.keys().next().cloned().expect("空でない");
        棚.remove(&古い);
    }
    false
}

/// 行の印を、画面と行き来する字にする（16 進）。
#[must_use]
pub(crate) fn 印を字に(印: &[u8; 32]) -> String {
    印.iter().map(|b| format!("{b:02x}")).collect()
}

/// 画面から来た字を、行の印に戻す。**読めなければ `None`。**
#[must_use]
pub(crate) fn 字を印に(字: &str) -> Option<[u8; 32]> {
    if 字.len() != 64 || !字.is_ascii() {
        return None;
    }
    let mut 印 = [0u8; 32];
    for (i, b) in 印.iter_mut().enumerate() {
        *b = u8::from_str_radix(&字[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(印)
}

fn 印の頭(印: &[u8; 32]) -> String {
    印を字に(印)[..8].to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn 行(部屋: MeetingId, 番号: u64, 本文: &str) -> 呼ばれた行 {
        呼ばれた行 {
            番号,
            部屋,
            印: [番号 as u8; 32],
            本文: 本文.to_owned(),
            人が受けた: false,
        }
    }

    #[test]
    fn エージェントは_読んだ範囲で自分を呼んだ行にだけ受けたを返す() {
        let a = MeetingId::generate();
        let 棚: VecDeque<_> = [
            行(a, 1, "@カムスタ 前の行"),
            行(a, 2, "@カムスタ 見積を見て"),
            行(a, 3, "@営業 こちらは別の人"),
            行(a, 4, "@カムスタ まだ読んでいない"),
        ]
        .into();
        let 名 = vec!["カムスタ".to_owned()];
        let 受けた = 読んで受けた行(&棚, 1, 3, &名);
        assert_eq!(受けた, vec![(a, [2; 32])]);
    }

    #[test]
    fn 人は_返事を書くとその部屋で呼ばれた行を一度だけ受ける() {
        let (a, b) = (MeetingId::generate(), MeetingId::generate());
        let mut 棚: VecDeque<_> = [行(a, 1, "@営業 受注です"), 行(a, 2, "@カムスタ 別の人")].into();
        let 名 = vec!["営業".to_owned()];
        assert_eq!(人が返事で受けた行(&mut 棚, a, &名), vec![[1; 32]]);
        // 2 度目の返事で、同じ行に受けたを返し直さない
        assert!(人が返事で受けた行(&mut 棚, a, &名).is_empty());
        // 別の部屋の返事では受けない
        let mut 棚: VecDeque<_> = [行(a, 1, "@営業 受注です")].into();
        assert!(人が返事で受けた行(&mut 棚, b, &名).is_empty());
    }

    #[test]
    fn 印は字と行き来でき_壊れた字は読まない() {
        let 印 = [0xab; 32];
        assert_eq!(字を印に(&印を字に(&印)), Some(印));
        assert_eq!(字を印に("zz"), None);
        assert_eq!(字を印に(&"g".repeat(64)), None);
    }
}
