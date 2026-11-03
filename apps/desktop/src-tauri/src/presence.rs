//! **同時編集の居場所**（0.1.43・内部の Issue 035）。誰がどの文書のどこに居るか。
//!
//! 文書を扱う道具（md-business など）が机の線で言った居場所を、ルームの相手へ運び、
//! 相手から届いた居場所を、`居場所を聞く` と言った線へ渡す。
//!
//! # ここで守ること
//!
//! - **文書も場所も読まない。**中身は文書を扱う側が決める。長さと形は網と机の線が検める
//! - **誰の居場所かは割符が刻む。**線の側は名乗れない（会話の発言と同じ約束）
//! - **会話を止めない。**居場所は「いま」だけが要るので、詰まった線には渡さず落とす（待たない）
//! - **最新だけ運ぶ。**1 つの線から速く来たら、[`まとめる間`] に 1 回、文書ごとに最後の 1 つだけ送る
//! - **閉じたことも運ぶ。**線が切れた・相手が抜けたら、その分を空の場所（閉じた）として流す

use std::collections::BTreeMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Manager};
use warifu_core::PublicKey;
use warifu_desk::FromDesk;
use warifu_meeting::{MeetingId, Notice};

use crate::Bridge;

/// 1 つの線から居場所を送る間（最短）。**これより速く来た分は、最後の 1 つにまとめる。**
pub(crate) const まとめる間: Duration = Duration::from_millis(200);

/// 1 つの居場所。
#[derive(Debug, Clone, PartialEq, Eq)]
struct 一つ {
    名札: String,
    場所: String,
    部屋: MeetingId,
}

/// **この機械の線が言った、いまの居場所**（線の番号 → 文書 → 居場所）。
///
/// 新しく入った相手へまとめて送るのと、線が切れたときに閉じるのに使う。
static この機械の居場所: Mutex<BTreeMap<u64, BTreeMap<String, 一つ>>> = Mutex::new(BTreeMap::new());

/// 1 つの線の送り方。
#[derive(Default)]
struct 線の送り {
    /// 最後に送った時。
    前: Option<Instant>,
    /// まだ送っていない居場所（文書ごとに最後の 1 つ）。
    溜め: BTreeMap<String, 一つ>,
    /// 後で送る見張りが立っているか。
    見張り中: bool,
}

/// **まだ送っていない居場所**（線の番号ごと）。
static 送り待ち: Mutex<BTreeMap<u64, 線の送り>> = Mutex::new(BTreeMap::new());

/// 相手の開いている文書（その機械の誰・文書 → いまの場所）。
type 開いている文書 = BTreeMap<(String, String), String>;

/// **ルームの相手から届いた、いまの居場所**（部屋・相手の鍵ごと）。抜けたら閉じる。
static 相手の居場所: Mutex<BTreeMap<(MeetingId, [u8; 32]), 開いている文書>> =
    Mutex::new(BTreeMap::new());

/// いまの居場所の控えを直す。**空の場所は閉じた**ので控えから外す。
fn この機械で控える(線: u64, 文書: &str, 居: &一つ) {
    let mut 棚 = この機械の居場所.lock().expect("毒されていない");
    let 文書たち = 棚.entry(線).or_default();
    if 居.場所.is_empty() {
        文書たち.remove(文書);
    } else {
        文書たち.insert(文書.to_owned(), 居.clone());
    }
    if 文書たち.is_empty() {
        棚.remove(&線);
    }
}

/// **送るか、まとめて後で送るか**を決める（純粋な部分）。
///
/// 前に送ってから [`まとめる間`] が過ぎていれば、いま送る。過ぎていなければ待ち、
/// 待つ間に来た分は文書ごとに最後の 1 つへ上書きする。`Some(待つ長さ)` は「後で送る（見張りを立てる）」。
fn いつ送るか(前: Option<Instant>, 今: Instant) -> Option<Duration> {
    let 前 = 前?;
    let 過ぎた = 今.saturating_duration_since(前);
    (過ぎた < まとめる間).then(|| まとめる間 - 過ぎた)
}

/// この機械の線が言った居場所を受ける（`ToDesk::居場所`）。**まとめてから** [`運ぶ`]。
pub(crate) fn 言った(
    app: &AppHandle,
    線: u64,
    名札: String,
    文書: String,
    場所: String,
    部屋: MeetingId,
) {
    let 居 = 一つ {
        名札, 場所, 部屋
    };
    この機械で控える(線, &文書, &居);
    let 今 = Instant::now();
    let 見張りを立てる = {
        let mut 待ち = 送り待ち.lock().expect("毒されていない");
        let 送り = 待ち.entry(線).or_default();
        match いつ送るか(送り.前, 今) {
            None => {
                送り.前 = Some(今);
                None
            }
            Some(間) => {
                送り.溜め.insert(文書.clone(), 居.clone());
                if 送り.見張り中 {
                    return;
                }
                送り.見張り中 = true;
                Some(間)
            }
        }
    };
    let app = app.clone();
    match 見張りを立てる {
        None => {
            tauri::async_runtime::spawn(async move { 運ぶ(&app, 線, &文書, &居).await });
        }
        Some(間) => {
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(間).await;
                let 溜め = {
                    let mut 待ち = 送り待ち.lock().expect("毒されていない");
                    let Some(送り) = 待ち.get_mut(&線) else {
                        return;
                    };
                    送り.前 = Some(Instant::now());
                    送り.見張り中 = false;
                    std::mem::take(&mut 送り.溜め)
                };
                for (文書, 居) in 溜め {
                    運ぶ(&app, 線, &文書, &居).await;
                }
            });
        }
    }
}

/// **線が切れた**。その線が開いていた文書を、閉じたとして流す。
pub(crate) fn 線が切れた(app: &AppHandle, 線: u64) {
    送り待ち.lock().expect("毒されていない").remove(&線);
    let 開いていた = この機械の居場所
        .lock()
        .expect("毒されていない")
        .remove(&線)
        .unwrap_or_default();
    if 開いていた.is_empty() {
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        for (文書, mut 居) in 開いていた {
            居.場所.clear();
            運ぶ(&app, 線, &文書, &居).await;
        }
    });
}

/// 1 つの居場所を、この機械のほかの線と、その部屋の相手へ運ぶ。**詰まった線は待たない。**
async fn 運ぶ(app: &AppHandle, 線: u64, 文書: &str, 居: &一つ) {
    let bridge = app.state::<Bridge>();
    let _ = bridge.desk.send((
        線,
        FromDesk::居場所 {
            誰: 居.名札.clone(),
            文書: 文書.to_owned(),
            場所: 居.場所.clone(),
            room: Some(居.部屋.to_string()),
        },
    ));
    let 自分 = bridge.device.public_key();
    let 知らせ = Notice::Presence {
        meeting: 居.部屋,
        from: 自分,
        話し手: Some(居.名札.clone()),
        文書: 文書.to_owned(),
        場所: 居.場所.clone(),
    };
    let 面々 =
        crate::そのルームの面々(&bridge.conferences, &bridge.outbound, 居.部屋, 自分).await;
    let mut 渡せた = 0;
    for (_, 線) in &面々 {
        // **待たない。**居場所は次の 1 つで追いつく（会話の線を詰まらせない）
        if let Some(tx) = 線
            && tx.try_send(知らせ.clone()).is_ok()
        {
            渡せた += 1;
        }
    }
    記録!(
        "居場所: {} の{}を {}/{} 人へ（部屋 {}）",
        居.名札,
        if 居.場所.is_empty() {
            "閉じた印"
        } else {
            "居場所"
        },
        渡せた,
        面々.len(),
        crate::短く(&居.部屋.to_string())
    );
}

/// **新しく入った相手へ、この機械のいまの居場所をまとめて送る**（その部屋の分だけ）。
pub(crate) async fn 入った相手へ(app: &AppHandle, 相手: PublicKey, 部屋: MeetingId) {
    let 分: Vec<(String, 一つ)> = この機械の居場所
        .lock()
        .expect("毒されていない")
        .values()
        .flat_map(|文書たち| 文書たち.iter())
        .filter(|(_, 居)| 居.部屋 == 部屋)
        .map(|(文書, 居)| (文書.clone(), 居.clone()))
        .collect();
    if 分.is_empty() {
        return;
    }
    let bridge = app.state::<Bridge>();
    let Some(tx) = bridge.outbound.lock().await.get(&相手.to_bytes()).cloned() else {
        return;
    };
    let 自分 = bridge.device.public_key();
    for (文書, 居) in 分 {
        let _ = tx.try_send(Notice::Presence {
            meeting: 部屋,
            from: 自分,
            話し手: Some(居.名札),
            文書,
            場所: 居.場所,
        });
    }
}

/// 届いた居場所を、相手ごとの控えに足す（空なら外す）。**抜けたときに閉じるため。**
///
/// **前と同じなら `false`**（線に渡さない）。客どうしは直接の線と主催の配り直しの
/// 2 本の道で同じ居場所を受けるので、2 本目を捨てる。
fn 相手を控える(
    部屋: MeetingId, 相手: PublicKey, 誰: &str, 文書: &str, 場所: &str
) -> bool {
    let mut 棚 = 相手の居場所.lock().expect("毒されていない");
    let 鍵 = (部屋, 相手.to_bytes());
    let 文書たち = 棚.entry(鍵).or_default();
    let 一つ = (誰.to_owned(), 文書.to_owned());
    let 変わった = if 場所.is_empty() {
        文書たち.remove(&一つ).is_some()
    } else {
        文書たち.insert(一つ, 場所.to_owned()).as_deref() != Some(場所)
    };
    if 文書たち.is_empty() {
        棚.remove(&鍵);
    }
    変わった
}

/// 相手から届いた居場所の、この機械の線に見せる名。**会話の発言と同じ決め方**（鍵の頭と札）。
#[must_use]
pub(crate) fn 見せる名(from: &PublicKey, 話し手: Option<&str>) -> String {
    match 話し手 {
        Some(札) => format!("{}（{札}）", crate::短く(&crate::key_to_string(*from))),
        None => crate::key_to_string(*from),
    }
}

/// **相手から居場所が届いた。**`居場所を聞く` と言った線へ渡し、主催ならほかの面々へ配る。
///
/// 差出人の確かめ（主催ごしの分だけ通す）は、呼ぶ側が会話の発言と同じ規則で済ませてある。
pub(crate) async fn 届いた(
    app: &AppHandle, 線の相手: PublicKey, 知らせ: &Notice, 主催だ: bool
) {
    let Notice::Presence {
        meeting,
        from,
        話し手,
        文書,
        場所,
    } = 知らせ
    else {
        return;
    };
    let 誰 = 見せる名(from, 話し手.as_deref());
    if !相手を控える(*meeting, *from, &誰, 文書, 場所) {
        // 2 本目の道で届いた同じ居場所（直接と主催ごし）。**線にも配り直しにも回さない**
        return;
    }
    記録!(
        "受信: 居場所（{誰}・{}）",
        if 場所.is_empty() {
            "閉じた"
        } else {
            "開いている"
        }
    );
    crate::desk::配る(
        &app.state::<Bridge>(),
        FromDesk::居場所 {
            誰,
            文書: 文書.clone(),
            場所: 場所.clone(),
            room: Some(meeting.to_string()),
        },
    );
    if !主催だ {
        return;
    }
    // **主催は、ほかの面々へそのまま配る**（差出人は書き換えない・D48）。待たない
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

/// **相手が部屋を抜けた。**その相手の居場所を、閉じたとして線へ流す。
pub(crate) fn 相手が抜けた(app: &AppHandle, 部屋: MeetingId, 相手: PublicKey) {
    let 開いていた = 相手の居場所
        .lock()
        .expect("毒されていない")
        .remove(&(部屋, 相手.to_bytes()))
        .unwrap_or_default();
    let bridge = app.state::<Bridge>();
    for (誰, 文書) in 開いていた.into_keys() {
        crate::desk::配る(
            &bridge,
            FromDesk::居場所 {
                誰,
                文書,
                場所: String::new(),
                room: Some(部屋.to_string()),
            },
        );
    }
}

/// 居場所を聞く線にだけ渡すか（それ以外の行は、どの線にも渡す）。
#[must_use]
pub(crate) const fn この線に渡すか(行: &FromDesk, 居場所を聞く: bool) -> bool {
    居場所を聞く || !matches!(行, FromDesk::居場所 { .. })
}

/// 試験のための、控えの中身（線ごとの文書の数）。
#[cfg(test)]
fn 控えの文書(線: u64) -> Vec<String> {
    この機械の居場所
        .lock()
        .expect("毒されていない")
        .get(&線)
        .map(|m| m.keys().cloned().collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 初めてか_間が過ぎていれば_いま送る() {
        let 今 = Instant::now();
        assert_eq!(いつ送るか(None, 今), None);
        assert_eq!(いつ送るか(Some(今 - まとめる間), 今), None);
        assert_eq!(いつ送るか(Some(今 - まとめる間 * 3), 今), None);
    }

    #[test]
    fn 間の内に来たら_残りの間だけ待つ() {
        let 今 = Instant::now();
        let 待つ = いつ送るか(Some(今 - Duration::from_millis(50)), 今).unwrap();
        assert_eq!(待つ, まとめる間 - Duration::from_millis(50));
    }

    #[test]
    fn 空の場所は閉じた印で_控えから外す() {
        let 部屋 = MeetingId::generate();
        let 線 = 9_000_001;
        let 居 = |場所: &str| 一つ {
            名札: "md".to_owned(),
            場所: 場所.to_owned(),
            部屋,
        };
        この機械で控える(線, "a", &居("{}"));
        この機械で控える(線, "b", &居("{}"));
        assert_eq!(控えの文書(線), ["a", "b"]);
        この機械で控える(線, "a", &居(""));
        assert_eq!(控えの文書(線), ["b"]);
        この機械で控える(線, "b", &居(""));
        assert!(控えの文書(線).is_empty());
        assert!(
            !この機械の居場所.lock().unwrap().contains_key(&線),
            "空の線を残している"
        );
    }

    #[test]
    fn 居場所は_聞くと言った線にだけ渡す() {
        let 居場所 = FromDesk::居場所 {
            誰: "x".to_owned(),
            文書: "a".to_owned(),
            場所: "{}".to_owned(),
            room: None,
        };
        assert!(この線に渡すか(&居場所, true));
        assert!(
            !この線に渡すか(&居場所, false),
            "エージェントの線に居場所を流している"
        );
        let 入室 = FromDesk::Joined {
            who: "x".to_owned(),
        };
        assert!(この線に渡すか(&入室, false));
        assert!(この線に渡すか(&入室, true));
    }

    #[test]
    fn 抜けた相手の控えは_開いていた文書を閉じる分だけ持つ() {
        let 部屋 = MeetingId::generate();
        let 相手 = warifu_core::Seed::from_bytes([3; 32])
            .profile("Personal")
            .device("PC")
            .public_key();
        assert!(相手を控える(部屋, 相手, "A（md）", "a", "{}"));
        assert!(相手を控える(部屋, 相手, "A（md）", "b", "{}"));
        assert!(相手を控える(部屋, 相手, "A（md）", "b", ""));
        let 開いていた: Vec<(String, String)> = 相手の居場所
            .lock()
            .unwrap()
            .get(&(部屋, 相手.to_bytes()))
            .unwrap()
            .keys()
            .cloned()
            .collect();
        assert_eq!(開いていた, [("A（md）".to_owned(), "a".to_owned())]);
    }

    #[test]
    fn 二本の道で届いた同じ居場所は_二度目を渡さない() {
        let 部屋 = MeetingId::generate();
        let 相手 = warifu_core::Seed::from_bytes([4; 32])
            .profile("Personal")
            .device("PC")
            .public_key();
        assert!(相手を控える(部屋, 相手, "B", "a", r#"{"line":1}"#));
        assert!(
            !相手を控える(部屋, 相手, "B", "a", r#"{"line":1}"#),
            "同じ居場所を 2 度渡す"
        );
        assert!(相手を控える(部屋, 相手, "B", "a", r#"{"line":2}"#));
        assert!(相手を控える(部屋, 相手, "B", "a", ""));
        assert!(
            !相手を控える(部屋, 相手, "B", "a", ""),
            "閉じた印を 2 度渡す"
        );
    }
}
