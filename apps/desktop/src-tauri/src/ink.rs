//! **会議中にペンで書いた線**（0.1.62・書き込み）。
//!
//! 文書を扱う道具（md-business など）が机の線で言った線を、同じ部屋の相手へ運び、
//! 相手から届いた線を、`居場所を聞く` と言った線へ渡す。
//!
//! # 居場所との違い
//!
//! 居場所は「いま」だけが要るので最新にまとめ、詰まった線には落とす。
//! **線は途中の点が 1 つ落ちても形が崩れる**ので、ここでは
//!
//! - **まとめない・捨てない・送った順に届ける**（網の線へは待って入れる）
//! - **道は 1 本にする。**ゲストは主催へだけ送り、主催が残りの面々へ配る。
//!   ゲストどうしの直接の線と主催ごしの 2 本で届くと、同じ線が 2 度描かれる
//! - **速すぎる線は断る**（1 本の線から 1 秒に [`書き込みの毎秒の上限`] 行まで）。断ったら理由を返す
//!
//! **荷は読まない。**誰が書いたかは割符が刻む（会話の発言と同じ約束）。

use std::collections::{BTreeMap, VecDeque};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Manager};
use warifu_core::PublicKey;
use warifu_desk::{FromDesk, 書き込みの毎秒の上限};
use warifu_meeting::{MeetingId, Notice};

use crate::Bridge;

/// 速さを数える幅。
const 数える幅: Duration = Duration::from_secs(1);

/// 線ごとの、直近 1 秒に通した書き込みの時刻。
static 通した時刻: Mutex<BTreeMap<u64, VecDeque<Instant>>> = Mutex::new(BTreeMap::new());

/// **この書き込みを通すか**（純粋な部分）。直近 [`数える幅`] に `上限` 行を通していれば断る。
/// 通すなら時刻を控える。**断った分は数えない**（断られ続けても、間が空けば通る）。
fn 通すか(控え: &mut VecDeque<Instant>, 今: Instant, 上限: usize) -> bool {
    while 控え
        .front()
        .is_some_and(|前| 今.saturating_duration_since(*前) >= 数える幅)
    {
        控え.pop_front();
    }
    if 控え.len() >= 上限 {
        return false;
    }
    控え.push_back(今);
    true
}

/// 速すぎたときの断りの文。
#[must_use]
pub(crate) fn 速すぎる文() -> String {
    format!("書き込みが速すぎます（1 秒に {書き込みの毎秒の上限} 行まで）。この行は送っていません")
}

/// この線の書き込みを、いま通してよいか。
pub(crate) fn この線から通すか(線: u64) -> bool {
    let mut 棚 = 通した時刻.lock().expect("毒されていない");
    通すか(
        棚.entry(線).or_default(),
        Instant::now(),
        書き込みの毎秒の上限,
    )
}

/// 線が切れた。数えを捨てる。
pub(crate) fn 線が切れた(線: u64) {
    通した時刻.lock().expect("毒されていない").remove(&線);
}

/// **送る先**（純粋な部分）。主催なら面々の全員へ、ゲストなら主催へだけ（主催が配る）。
///
/// ゲストで、主催が名簿に居なければ誰へも送らない（空を返す。呼ぶ側が断る）。
fn 送る先<K: Copy + PartialEq>(主催か: bool, 主催: Option<K>, 面々: &[K]) -> Vec<K> {
    if 主催か {
        return 面々.to_vec();
    }
    主催.filter(|h| 面々.contains(h)).into_iter().collect()
}

/// **主催が配り直す先**（純粋な部分）。届いた線の相手と、書いた本人には返さない。
fn 配り直す先<K: Copy + PartialEq>(面々: &[K], 線の相手: K, 書いた人: K) -> Vec<K> {
    面々
        .iter()
        .copied()
        .filter(|k| *k != 線の相手 && *k != 書いた人)
        .collect()
}

/// **この機械の線が書いた。**この機械のほかの線と、その部屋の相手へ送った順に運ぶ。
///
/// # Errors
/// 部屋に相手が居ない・主催への線が無いときは、断る文を返す（黙って捨てない）。
pub(crate) async fn 書いた(
    app: &AppHandle,
    線: u64,
    名札: String,
    文書: String,
    荷: String,
    部屋: MeetingId,
) -> Result<(), String> {
    let bridge = app.state::<Bridge>();
    let _ = bridge.desk.send((
        線,
        FromDesk::書き込み {
            誰: 名札.clone(),
            文書: 文書.clone(),
            荷: 荷.clone(),
            room: Some(部屋.to_string()),
        },
    ));
    let 自分 = bridge.device.public_key();
    let 面々 = crate::そのルームの面々(&bridge.conferences, &bridge.outbound, 部屋, 自分).await;
    if 面々.is_empty() {
        // この機械の中の線には渡した。部屋の相手が居ないだけなので断らない
        return Ok(());
    }
    let 覚えた主催 = bridge.主催たち.lock().await.get(&部屋).copied();
    let 主催か = crate::roster_sync::主催か(覚えた主催.as_ref(), &自分);
    let 鍵たち: Vec<PublicKey> = 面々.iter().map(|(k, _)| *k).collect();
    let 先 = 送る先(主催か, 覚えた主催, &鍵たち);
    if 先.is_empty() {
        return Err(
            "主催が名簿に居ないので、書き込みを運べません（入り直すまで待ってください）".to_owned(),
        );
    }
    let 知らせ = Notice::書き込み {
        meeting: 部屋,
        from: 自分,
        話し手: Some(名札),
        文書,
        荷,
    };
    let mut 運べなかった = 0;
    for (鍵, 線) in &面々 {
        if !先.contains(鍵) {
            continue;
        }
        // **待って入れる。**落とすと線の形が崩れる（居場所とは違う）
        let 入れた = match 線 {
            Some(tx) => tx.send(知らせ.clone()).await.is_ok(),
            None => false,
        };
        if !入れた {
            運べなかった += 1;
        }
    }
    if 運べなかった == 先.len() {
        return Err("部屋の相手への線が無いので、書き込みを運べませんでした".to_owned());
    }
    Ok(())
}

/// **相手から書き込みが届いた。**`居場所を聞く` と言った線へ渡し、主催ならほかの面々へ順に配る。
///
/// 差出人の確かめ（主催ごしの分だけ通す）は、呼ぶ側が会話の発言と同じ規則で済ませてある。
pub(crate) async fn 届いた(
    app: &AppHandle, 線の相手: PublicKey, 知らせ: &Notice, 主催だ: bool
) {
    let Notice::書き込み {
        meeting,
        from,
        話し手,
        文書,
        荷,
    } = 知らせ
    else {
        return;
    };
    let bridge = app.state::<Bridge>();
    crate::desk::配る(
        &bridge,
        FromDesk::書き込み {
            誰: crate::presence::見せる名(from, 話し手.as_deref()),
            文書: 文書.clone(),
            荷: 荷.clone(),
            room: Some(meeting.to_string()),
        },
    );
    if !主催だ {
        return;
    }
    // **主催は、ほかの面々へそのまま配る**（差出人は書き換えない・D48）。**待って入れる**
    let 自分 = bridge.device.public_key();
    let 面々 =
        crate::そのルームの面々(&bridge.conferences, &bridge.outbound, *meeting, 自分).await;
    let 鍵たち: Vec<PublicKey> = 面々.iter().map(|(k, _)| *k).collect();
    let 先 = 配り直す先(&鍵たち, 線の相手, *from);
    for (鍵, 線) in 面々 {
        if !先.contains(&鍵) {
            continue;
        }
        if let Some(tx) = 線
            && tx.send(知らせ.clone()).await.is_err()
        {
            記録!(
                "書き込み: {} への線が閉じていて、配り直せませんでした",
                crate::短く(&crate::key_to_string(鍵))
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 一秒に上限まで通し_超えた分は断る() {
        let mut 控え = VecDeque::new();
        let 今 = Instant::now();
        for i in 0..20 {
            assert!(
                通すか(&mut 控え, 今 + Duration::from_millis(i * 10), 20),
                "{i} 行目を断った"
            );
        }
        assert!(
            !通すか(&mut 控え, 今 + Duration::from_millis(500), 20),
            "21 行目を通した"
        );
        assert!(!通すか(&mut 控え, 今 + Duration::from_millis(999), 20));
    }

    #[test]
    fn 間が空けば_また通す_断った分は数えない() {
        let mut 控え = VecDeque::new();
        let 今 = Instant::now();
        for _ in 0..20 {
            assert!(通すか(&mut 控え, 今, 20));
        }
        for _ in 0..50 {
            assert!(!通すか(&mut 控え, 今 + Duration::from_millis(300), 20));
        }
        assert!(通すか(&mut 控え, 今 + 数える幅, 20), "1 秒たっても通さない");
        assert_eq!(控え.len(), 1, "古い分を捨てていない・断った分を数えている");
    }

    #[test]
    fn 主催は全員へ_ゲストは主催へだけ送る() {
        let 面々 = [1, 2, 3];
        assert_eq!(送る先(true, None, &面々), [1, 2, 3]);
        assert_eq!(
            送る先(false, Some(2), &面々),
            [2],
            "ゲストが主催以外へも送る（2 度届く）"
        );
        assert!(
            送る先(false, Some(9), &面々).is_empty(),
            "名簿に居ない主催へ送る"
        );
        assert!(送る先(false, None, &面々).is_empty());
    }

    #[test]
    fn 主催は_届いた線と書いた本人には配り直さない() {
        assert_eq!(配り直す先(&[1, 2, 3, 4], 2, 2), [1, 3, 4]);
        // 主催ごしでない線（ゲストが別の道で受けた）でも、本人には返さない
        assert_eq!(配り直す先(&[1, 2, 3, 4], 2, 3), [1, 4]);
    }

    #[test]
    fn 書き込みも_聞くと言った線にだけ渡す() {
        let 行 = FromDesk::書き込み {
            誰: "x".to_owned(),
            文書: "a".to_owned(),
            荷: "{}".to_owned(),
            room: None,
        };
        assert!(crate::presence::この線に渡すか(&行, true));
        assert!(
            !crate::presence::この線に渡すか(&行, false),
            "エージェントの線に書き込みを流している"
        );
    }
}
