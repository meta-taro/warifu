//! **長く戻らない人を知らせる**（0.1.28「ずっとつながる」）。
//!
//! 「抜けた」と「切れた」は別のことである。
//!
//! | | 何が起きたか | 知らせ |
//! |---|---|---|
//! | **抜けた** | 本人が［このルームを抜ける］を押した（`Notice::Leave`） | すぐ（`FromDesk::Left`） |
//! | **切れた** | 経路が落ちた。本人は一員のまま | **10 分戻らなければ**知らせる |
//!
//! 切れた人は、自分で戻ってくる（`come_back`）。だから**切れた瞬間には鳴らさない** ——
//! 経路はよく揺れるので、揺れるたびにエージェントを起こすと、それだけで注意を使い切る。
//! **戻ってこないときだけ**、人とエージェントに言う。

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Manager as _};

use crate::Bridge;

/// この長さ戻らなければ知らせる。
const 不在とみなす秒: u64 = 10 * 60;
/// 見直す間。
const 見直す秒: u64 = 60;

/// 切れた人の帳面。**鍵ごとに、切れた時刻と、もう知らせたか。**
#[derive(Default)]
pub struct 不在帳 {
    棚: HashMap<String, (Instant, bool)>,
}

impl 不在帳 {
    /// 経路が切れた。**最初に切れた時刻を残す**（揺れて何度も切れても、延ばさない）。
    pub fn 切れた(&mut self, 鍵: &str, いま: Instant) {
        self.棚.entry(鍵.to_owned()).or_insert((いま, false));
    }

    /// 戻ってきた。
    pub fn 戻った(&mut self, 鍵: &str) {
        self.棚.remove(鍵);
    }

    /// 本人が抜けた。**一員でなくなったので、待たない。**
    pub fn 抜けた(&mut self, 鍵: &str) {
        self.棚.remove(鍵);
    }

    /// **いま知らせる人**。知らせた印を付けるので、同じ人を 2 度言わない。
    pub fn 知らせる頃(&mut self, いま: Instant, 間: Duration) -> Vec<String> {
        let mut 並び: Vec<String> = self
            .棚
            .iter_mut()
            .filter(|(_, (切れた時刻, 知らせた))| {
                !*知らせた && いま.saturating_duration_since(*切れた時刻) >= 間
            })
            .map(|(鍵, (_, 知らせた))| {
                *知らせた = true;
                鍵.clone()
            })
            .collect();
        並び.sort();
        並び
    }
}

static 不在の棚: Mutex<Option<不在帳>> = Mutex::new(None);

fn 帳を使う<R>(f: impl FnOnce(&mut 不在帳) -> R) -> R {
    let mut 中 = 不在の棚.lock().expect("毒されていない");
    f(中.get_or_insert_with(不在帳::default))
}

/// 経路が切れた（名簿に居た人だけ呼ぶ）。
pub fn 切れた(鍵: &str) {
    帳を使う(|帳| 帳.切れた(鍵, Instant::now()));
}

/// 戻ってきた。
pub fn 戻った(鍵: &str) {
    帳を使う(|帳| 帳.戻った(鍵));
}

/// 本人が抜けた。
pub fn 抜けた(鍵: &str) {
    帳を使う(|帳| 帳.抜けた(鍵));
}

/// **見張る。**起動したら 1 本だけ立てる。
pub async fn 見張る(app: AppHandle) {
    loop {
        tokio::time::sleep(Duration::from_secs(見直す秒)).await;
        let 鍵たち =
            帳を使う(|帳| 帳.知らせる頃(Instant::now(), Duration::from_secs(不在とみなす秒)));
        for 鍵 in 鍵たち {
            知らせる(&app, &鍵);
        }
    }
}

fn 知らせる(app: &AppHandle, 鍵: &str) {
    let bridge = app.state::<Bridge>();
    let 名 = crate::desk::名を引く(&bridge, 鍵);
    let 分 = 不在とみなす秒 / 60;
    記録!("不在: {} が {分} 分戻っていません", crate::短く(鍵));
    // **エージェントへ。**誰が戻っていないかを 1 行で
    crate::desk::配る(
        &bridge,
        crate::desk::聞いた(
            &名,
            &format!("（経路が切れてから {分} 分、ルームに戻っていません。抜けてはいません）"),
        ),
    );
    // **人へ。**窓が後ろにあるときだけ鳴る
    let 見せる名 = if 名 == 鍵 { crate::短く(鍵) } else { 名 };
    crate::notify::不在を知らせる(app, &見せる名, 分);
}

#[cfg(test)]
mod 試験 {
    use super::*;

    const 十分: Duration = Duration::from_secs(600);

    #[test]
    fn 切れてすぐは知らせない() {
        let mut 帳 = 不在帳::default();
        let t0 = Instant::now();
        帳.切れた("A", t0);
        assert!(
            帳.知らせる頃(t0 + Duration::from_secs(599), 十分)
                .is_empty()
        );
    }

    #[test]
    fn 十分戻らなければ_1度だけ知らせる() {
        let mut 帳 = 不在帳::default();
        let t0 = Instant::now();
        帳.切れた("A", t0);
        assert_eq!(帳.知らせる頃(t0 + 十分, 十分), vec!["A".to_owned()]);
        assert!(帳.知らせる頃(t0 + 十分 * 2, 十分).is_empty());
    }

    #[test]
    fn 戻ったら知らせない() {
        let mut 帳 = 不在帳::default();
        let t0 = Instant::now();
        帳.切れた("A", t0);
        帳.戻った("A");
        assert!(帳.知らせる頃(t0 + 十分, 十分).is_empty());
    }

    #[test]
    fn 抜けた人は待たない() {
        let mut 帳 = 不在帳::default();
        let t0 = Instant::now();
        帳.切れた("A", t0);
        帳.抜けた("A");
        assert!(帳.知らせる頃(t0 + 十分, 十分).is_empty());
    }

    #[test]
    fn 揺れて何度切れても_最初の時刻から数える() {
        let mut 帳 = 不在帳::default();
        let t0 = Instant::now();
        帳.切れた("A", t0);
        帳.切れた("A", t0 + Duration::from_secs(300));
        assert_eq!(帳.知らせる頃(t0 + 十分, 十分), vec!["A".to_owned()]);
    }

    #[test]
    fn 戻ってまた切れたら_数え直す() {
        let mut 帳 = 不在帳::default();
        let t0 = Instant::now();
        帳.切れた("A", t0);
        帳.戻った("A");
        帳.切れた("A", t0 + Duration::from_secs(300));
        assert!(帳.知らせる頃(t0 + 十分, 十分).is_empty());
        assert_eq!(
            帳.知らせる頃(t0 + Duration::from_secs(900), 十分),
            vec!["A".to_owned()]
        );
    }
}
