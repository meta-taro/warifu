//! **切れたゲストどうしの線を、自分で張り直す**（0.1.49）。
//!
//! ```text
//!   主催 ── ゲスト A
//!     └──── ゲスト B        A ↔ B の線は、入ったときに主催の紹介で張るだけ
//! ```
//!
//! 主催と切れたゲストは、帰り道から自分で戻る（`come_back`）。ところが
//! **ゲストどうしの線**が切れても、張り直す者が居なかった。主催とは繋がったままなので
//! 戻る条件に当たらず、10 分後に「戻っていません」と知らせるだけだった（`absence`）。
//! 11-13、1 台から見て 3 台が同時に切れ、人がその機械の割符を立ち上げ直すまで戻らなかった。
//!
//! だから、**名簿に居た相手との線が切れたら、その相手だけを呼び直す。**
//! 道は紹介で呼ぶときと同じ（部屋の合言葉の証しを見せる・`call::部屋の合言葉で呼ぶ`）。
//!
//! - **主催の線は触らない。**自分が主催の部屋なら、ゲストが帰り道から戻ってくる。
//!   相手が主催なら、`come_back` が戻る。両方から叩くと入り直しが重なる
//! - **両側から同時に叩かない。**鍵の小さい側が先に叩き、大きい側は遅れて叩く
//!   （先の側が叩けない機械でも、遅れた側が張り直せる）
//! - しくじったら間を延ばす（`come_back::次に待つ秒`・上限 10 分）。**諦めない** ——
//!   相手が抜けたら（`Notice::Leave`）やめる

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Manager as _};
use warifu_core::PublicKey;
use warifu_meeting::MeetingId;

use crate::Bridge;

/// 見直す間。
const 見直す秒: u64 = 15;
/// 切れてから、最初に叩くまで（鍵の小さい側）。**揺れてすぐ戻る線を叩かない。**
pub const 先に叩く秒: u64 = 20;
/// 鍵の大きい側が、最初に叩くまで。先の側の入り直しと重ねない。
pub const 後で叩く秒: u64 = 50;

/// 1 人ぶんの控え。
#[derive(Debug, Clone)]
pub struct 控え {
    /// 切れたときに一緒に居た部屋（主催の部屋は除く前の、ありのまま）。
    pub 部屋たち: Vec<MeetingId>,
    /// 切れた時刻（最初の）。
    pub 切れた時刻: Instant,
    /// 次に叩いてよい時刻。
    pub 次: Instant,
    /// 続けてしくじった回数。
    pub しくじった: u32,
}

/// 切れた相手の帳面。**鍵ごと。**
#[derive(Default)]
pub struct 張り直し帳 {
    棚: HashMap<String, 控え>,
}

impl 張り直し帳 {
    /// 線が切れた。**揺れて何度も切れても、最初の時刻を残す**（部屋は足す）。
    pub fn 切れた(
        &mut self, 鍵: &str, 部屋たち: &[MeetingId], いま: Instant, 先に叩く: bool
    ) {
        let 待つ = Duration::from_secs(if 先に叩く {
            先に叩く秒
        } else {
            後で叩く秒
        });
        let 控え = self.棚.entry(鍵.to_owned()).or_insert_with(|| 控え {
            部屋たち: Vec::new(),
            切れた時刻: いま,
            次: いま + 待つ,
            しくじった: 0,
        });
        for 部屋 in 部屋たち {
            if !控え.部屋たち.contains(部屋) {
                控え.部屋たち.push(*部屋);
            }
        }
    }

    /// 戻ってきた（どちらが叩いたかは問わない）。
    pub fn 戻った(&mut self, 鍵: &str) {
        self.棚.remove(鍵);
    }

    /// 本人が抜けた。**一員でなくなったので、呼ばない。**
    pub fn 抜けた(&mut self, 鍵: &str) {
        self.棚.remove(鍵);
    }

    /// **いま叩く相手**（鍵と部屋たち）。鍵の順に並べる。
    pub fn 叩く頃(&self, いま: Instant) -> Vec<(String, Vec<MeetingId>)> {
        let mut 並び: Vec<_> = self
            .棚
            .iter()
            .filter(|(_, c)| いま >= c.次)
            .map(|(鍵, c)| (鍵.clone(), c.部屋たち.clone()))
            .collect();
        並び.sort_by(|a, b| a.0.cmp(&b.0));
        並び
    }

    /// 叩いたが戻らなかった。間を延ばす。
    pub fn しくじった(&mut self, 鍵: &str, いま: Instant) {
        if let Some(c) = self.棚.get_mut(鍵) {
            c.しくじった = c.しくじった.saturating_add(1);
            c.次 = いま + Duration::from_secs(crate::come_back::次に待つ秒(c.しくじった));
        }
    }

    /// **切れたままの相手**（鍵と切れた時刻）。`room_status` が並べる。
    pub fn 切れたまま(&self) -> Vec<(String, Instant)> {
        let mut 並び: Vec<_> = self
            .棚
            .iter()
            .map(|(鍵, c)| (鍵.clone(), c.切れた時刻))
            .collect();
        並び.sort_by(|a, b| a.0.cmp(&b.0));
        並び
    }

    /// **いますぐ叩く**（エージェントの `room_reconnect`）。間の控えを捨てる。
    pub fn 今すぐにする(&mut self, いま: Instant) -> usize {
        for c in self.棚.values_mut() {
            c.次 = いま;
        }
        self.棚.len()
    }
}

/// **先に叩く側か。**鍵の小さい側が先。
pub fn 先に叩くか(自分: &str, 相手: &str) -> bool {
    自分 < 相手
}

/// **その部屋で、この相手を自分が呼び直すか。**
///
/// - 部屋をもう持っていない …… 呼ばない（抜けた・畳んだ）
/// - 自分が主催 …… 呼ばない（ゲストが帰り道から戻ってくる）
/// - 相手が主催 …… 呼ばない（`come_back` が戻る）
pub fn 呼び直すか<K: PartialEq>(
    持っている: bool,
    主催: Option<&K>,
    自分: &K,
    相手: &K,
) -> bool {
    let Some(主催) = 主催 else {
        // 主催を知らない部屋は、自分が建てた部屋（`主催か` と同じ読み）
        return false;
    };
    持っている && 主催 != 自分 && 主催 != 相手
}

static 張り直しの棚: Mutex<Option<張り直し帳>> = Mutex::new(None);

fn 帳を使う<R>(f: impl FnOnce(&mut 張り直し帳) -> R) -> R {
    let mut 中 = 張り直しの棚.lock().expect("毒されていない");
    f(中.get_or_insert_with(張り直し帳::default))
}

/// 線が切れた（名簿に居た部屋たちと一緒に）。
pub fn 切れた(自分: &str, 鍵: &str, 部屋たち: &[MeetingId]) {
    let 先 = 先に叩くか(自分, 鍵);
    帳を使う(|帳| 帳.切れた(鍵, 部屋たち, Instant::now(), 先));
}

/// 戻ってきた。
pub fn 戻った(鍵: &str) {
    帳を使う(|帳| 帳.戻った(鍵));
}

/// 本人が抜けた。
pub fn 抜けた(鍵: &str) {
    帳を使う(|帳| 帳.抜けた(鍵));
}

/// 切れたままの相手と、切れてからの秒。
pub fn 切れたまま() -> Vec<(String, u64)> {
    let いま = Instant::now();
    帳を使う(|帳| {
        帳.切れたまま()
            .into_iter()
            .map(|(鍵, t)| (鍵, いま.saturating_duration_since(t).as_secs()))
            .collect()
    })
}

/// いますぐ張り直す。張り直しを待っている相手の数を返す（叩くのは見張り）。
pub fn 今すぐ張り直す() -> usize {
    帳を使う(|帳| 帳.今すぐにする(Instant::now()))
}

/// **見張る。**起動したら 1 本だけ立てる。
pub async fn 見張る(app: AppHandle) {
    loop {
        tokio::time::sleep(Duration::from_secs(見直す秒)).await;
        let 叩く = 帳を使う(|帳| 帳.叩く頃(Instant::now()));
        for (鍵, 部屋たち) in 叩く {
            一人呼ぶ(&app, &鍵, &部屋たち).await;
        }
    }
}

async fn 一人呼ぶ(app: &AppHandle, 鍵: &str, 部屋たち: &[MeetingId]) {
    let bridge = app.state::<Bridge>();
    let Ok(相手) = 鍵.parse::<PublicKey>() else {
        抜けた(鍵);
        return;
    };
    // **もう繋がっている**（相手が叩いてきた・揺れが戻った）
    if bridge.outbound.lock().await.contains_key(&相手.to_bytes()) {
        記録!("張り直し: {} はもう繋がっています", crate::短く(鍵));
        戻った(鍵);
        return;
    }
    let 自分 = bridge.device.public_key();
    let 呼ぶ部屋 = {
        let 棚 = bridge.conferences.lock().await;
        let 主催たち = bridge.主催たち.lock().await;
        部屋たち
            .iter()
            .find(|部屋| 呼び直すか(棚.contains_key(部屋), 主催たち.get(部屋), &自分, &相手))
            .copied()
    };
    let Some(部屋) = 呼ぶ部屋 else {
        // 主催どうしの線か、部屋がもう無い。**こちらからは呼ばない**
        抜けた(鍵);
        return;
    };
    let 住所 = bridge.addresses.lock().await.get(&相手.to_bytes()).cloned();
    let Some(住所) = 住所 else {
        記録!("張り直し: {} の住所を知りません", crate::短く(鍵));
        帳を使う(|帳| 帳.しくじった(鍵, Instant::now()));
        return;
    };
    記録!(
        "張り直し: {} を部屋 {} の証しで呼びます",
        crate::短く(鍵),
        crate::短く(&部屋.to_string())
    );
    match crate::call::部屋の合言葉で呼ぶ(app, &bridge, 相手, &住所, 部屋).await {
        Ok(()) => {
            記録!("張り直し: {} と繋がり直しました", crate::短く(鍵));
            戻った(鍵);
        }
        Err(e) => {
            記録!(
                "張り直し: {} に届きません（{}）",
                crate::短く(鍵),
                e.message
            );
            帳を使う(|帳| 帳.しくじった(鍵, Instant::now()));
        }
    }
}

#[cfg(test)]
mod 試験 {
    use super::*;

    /// 試験のあいだ同じ値を返す部屋 id（n ごとに 1 つ）。
    fn 部屋(n: usize) -> MeetingId {
        static 棚: std::sync::OnceLock<Vec<MeetingId>> = std::sync::OnceLock::new();
        棚.get_or_init(|| (0..4).map(|_| MeetingId::generate()).collect())[n]
    }

    #[test]
    fn 切れてすぐは叩かず_先の側は20秒で叩く() {
        let mut 帳 = 張り直し帳::default();
        let t0 = Instant::now();
        帳.切れた("B", &[部屋(1)], t0, true);
        assert!(帳.叩く頃(t0 + Duration::from_secs(19)).is_empty());
        assert_eq!(
            帳.叩く頃(t0 + Duration::from_secs(先に叩く秒)),
            vec![("B".to_owned(), vec![部屋(1)])]
        );
    }

    #[test]
    fn 後の側は先の側より遅れて叩く() {
        let mut 帳 = 張り直し帳::default();
        let t0 = Instant::now();
        帳.切れた("B", &[部屋(1)], t0, false);
        assert!(帳.叩く頃(t0 + Duration::from_secs(先に叩く秒)).is_empty());
        assert_eq!(帳.叩く頃(t0 + Duration::from_secs(後で叩く秒)).len(), 1);
    }

    #[test]
    fn 揺れて何度切れても最初の時刻のままで_部屋は足す() {
        let mut 帳 = 張り直し帳::default();
        let t0 = Instant::now();
        帳.切れた("B", &[部屋(1)], t0, true);
        帳.切れた("B", &[部屋(2), 部屋(1)], t0 + Duration::from_secs(15), true);
        let 並び = 帳.叩く頃(t0 + Duration::from_secs(先に叩く秒));
        assert_eq!(並び, vec![("B".to_owned(), vec![部屋(1), 部屋(2)])]);
        assert_eq!(帳.切れたまま()[0].1, t0);
    }

    #[test]
    fn しくじったら間を延ばし_10分で止める() {
        let mut 帳 = 張り直し帳::default();
        let t0 = Instant::now();
        帳.切れた("B", &[部屋(1)], t0, true);
        帳.しくじった("B", t0);
        assert!(帳.叩く頃(t0 + Duration::from_secs(59)).is_empty());
        assert_eq!(帳.叩く頃(t0 + Duration::from_secs(60)).len(), 1);
        for _ in 0..20 {
            帳.しくじった("B", t0);
        }
        assert_eq!(帳.叩く頃(t0 + Duration::from_secs(600)).len(), 1);
    }

    #[test]
    fn 戻ったか抜けたら叩かない() {
        let mut 帳 = 張り直し帳::default();
        let t0 = Instant::now();
        帳.切れた("B", &[部屋(1)], t0, true);
        帳.切れた("C", &[部屋(1)], t0, true);
        帳.戻った("B");
        帳.抜けた("C");
        assert!(帳.叩く頃(t0 + Duration::from_secs(600)).is_empty());
        assert!(帳.切れたまま().is_empty());
    }

    #[test]
    fn 今すぐにすると_待ちを捨てて叩く() {
        let mut 帳 = 張り直し帳::default();
        let t0 = Instant::now();
        帳.切れた("B", &[部屋(1)], t0, false);
        帳.しくじった("B", t0);
        assert_eq!(帳.今すぐにする(t0 + Duration::from_secs(1)), 1);
        assert_eq!(帳.叩く頃(t0 + Duration::from_secs(1)).len(), 1);
    }

    #[test]
    fn 鍵の小さい側が先に叩く() {
        assert!(先に叩くか("A", "B"));
        assert!(!先に叩くか("B", "A"));
    }

    #[test]
    fn ゲストどうしの線だけを呼び直す() {
        let (主, 甲, 乙) = (1, 2, 3);
        // 自分も相手もゲスト
        assert!(呼び直すか(true, Some(&主), &甲, &乙));
        // 自分が主催：ゲストが帰り道から戻る
        assert!(!呼び直すか(true, Some(&主), &主, &乙));
        // 相手が主催：come_back が戻る
        assert!(!呼び直すか(true, Some(&主), &甲, &主));
        // 主催を知らない部屋は自分が建てた部屋
        assert!(!呼び直すか(true, None, &甲, &乙));
        // 部屋をもう持っていない
        assert!(!呼び直すか(false, Some(&主), &甲, &乙));
    }
}
