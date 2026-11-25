//! **ゲストの名簿を、主催の名簿に合わせる**（0.1.39・2026-10-23）。
//!
//! 規則の本体は 0.1.40 で `warifu_app::room::roster` へ移した（workspace の CI で走る・
//! 机上の網と 3 結び目の試験が同じものを動かすため）。ここは画面側の呼び口を残すだけ。
//!
//! - `名簿の差` …… 主催の名簿と自分の名簿の差（足す・外す）
//! - `配られた発言を通すか` …… 線の相手と差出人が違う発言は、線の相手が主催のときだけ通す
//! - `主催か` …… 覚えた主催で決める。**名簿の先頭では決めない**（ゲストも自分が先頭なので、常に真になる）
//! - `名簿を配るか` …… 主催が、名簿が動いたときだけ配る
//! - `入った本人へ名簿を渡すか` …… 主催が、入った（入り直した）本人へも先に渡す（0.1.60）
//! - `食い違いをいま言うか` …… 入った直後で主催の名簿がまだなら、名簿に居ない相手の発言を控えて待つ（0.1.60）

pub use warifu_app::room::roster::{
    主催か, 入った本人へ名簿を渡すか, 名簿の差, 名簿を待つ秒, 名簿を配るか, 配られた発言を通すか,
    食い違いをいま言うか,
};

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

use warifu_core::PublicKey;
use warifu_meeting::MeetingId;

/// **入った直後の部屋で、主催の名簿を待っている分**（0.1.60）。
///
/// ゲストとして入った（入り直した）時刻と、そのあと主催の名簿が届いたか。
/// 名簿の届く前に来た「名簿に居ない相手」の発言は、食い違いとして言わずに控える
/// （発言そのものは画面へ通す。控えるのは食い違いの知らせだけ）。
#[derive(Debug, Default)]
pub struct 名簿待ち {
    部屋ごと: HashMap<MeetingId, (Instant, bool)>,
    控え: Vec<(MeetingId, PublicKey)>,
}

impl 名簿待ち {
    /// ゲストとして入った（入り直した）。待ちを始め直す。
    pub fn 入った(&mut self, 部屋: MeetingId, 今: Instant) {
        self.部屋ごと.insert(部屋, (今, false));
    }

    /// 主催の名簿が届いた。返りは、その部屋で控えていた差出人（照らし直す分）。
    pub fn 名簿が届いた(&mut self, 部屋: MeetingId) -> Vec<PublicKey> {
        if let Some(様子) = self.部屋ごと.get_mut(&部屋) {
            様子.1 = true;
        }
        self.引き取る(部屋)
    }

    /// 名簿に居ない差出人の発言が来た。**いま言うなら true**。言わないなら控える。
    pub fn 言うか(&mut self, 部屋: MeetingId, 差出人: PublicKey, 今: Instant) -> bool {
        let (入ってから, 届いた) = match self.部屋ごと.get(&部屋) {
            Some((時, 届いた)) => (Some(今.saturating_duration_since(*時).as_secs()), *届いた),
            None => (None, false),
        };
        if 食い違いをいま言うか(入ってから, 届いた) {
            return true;
        }
        if !self.控え.contains(&(部屋, 差出人)) {
            self.控え.push((部屋, 差出人));
        }
        false
    }

    /// 待ちが明けた（名簿が届かないまま）。控えていた差出人を返して、待ちを終える。
    pub fn 待ちが明けた(&mut self, 部屋: MeetingId) -> Vec<PublicKey> {
        if let Some(様子) = self.部屋ごと.get_mut(&部屋) {
            // 届いた扱いにして、以後はすぐ言う
            様子.1 = true;
        }
        self.引き取る(部屋)
    }

    fn 引き取る(&mut self, 部屋: MeetingId) -> Vec<PublicKey> {
        let mut 出 = Vec::new();
        self.控え.retain(|(m, k)| {
            if *m == 部屋 {
                出.push(*k);
                false
            } else {
                true
            }
        });
        出
    }
}

/// この機械の名簿待ち（1 つだけ）。
pub fn 待ちで<R>(f: impl FnOnce(&mut 名簿待ち) -> R) -> R {
    static 棚: OnceLock<Mutex<名簿待ち>> = OnceLock::new();
    let mut g = 棚
        .get_or_init(Mutex::default)
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    f(&mut g)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use warifu_core::Seed;

    fn 鍵(n: u8) -> PublicKey {
        Seed::from_bytes([n; 32])
            .profile("Personal")
            .device("PC")
            .public_key()
    }

    #[test]
    fn 入った直後は控え_名簿が届いたら控えた差出人を返す() {
        let mut w = 名簿待ち::default();
        let 部屋 = MeetingId::generate();
        let t0 = Instant::now();
        w.入った(部屋, t0);
        assert!(!w.言うか(部屋, 鍵(3), t0 + Duration::from_secs(1)));
        assert!(
            !w.言うか(部屋, 鍵(3), t0 + Duration::from_secs(2)),
            "同じ差出人は 1 つだけ控える"
        );
        assert!(!w.言うか(部屋, 鍵(4), t0 + Duration::from_secs(2)));
        assert_eq!(w.名簿が届いた(部屋), vec![鍵(3), 鍵(4)]);
        assert!(
            w.言うか(部屋, 鍵(5), t0 + Duration::from_secs(3)),
            "届いたあとは、すぐ言う"
        );
    }

    #[test]
    fn 待ちが明けたら言う_控えも返す() {
        let mut w = 名簿待ち::default();
        let 部屋 = MeetingId::generate();
        let t0 = Instant::now();
        w.入った(部屋, t0);
        assert!(!w.言うか(部屋, 鍵(3), t0));
        assert!(w.言うか(部屋, 鍵(3), t0 + Duration::from_secs(名簿を待つ秒)));
        assert_eq!(w.待ちが明けた(部屋), vec![鍵(3)]);
        assert!(w.待ちが明けた(部屋).is_empty());
        assert!(w.言うか(部屋, 鍵(4), t0), "明けたあとは、すぐ言う");
    }

    #[test]
    fn 入った覚えのない部屋は_すぐ言う() {
        let mut w = 名簿待ち::default();
        assert!(w.言うか(MeetingId::generate(), 鍵(3), Instant::now()));
    }

    #[test]
    fn 別の部屋の控えは返さない() {
        let mut w = 名簿待ち::default();
        let (a, b) = (MeetingId::generate(), MeetingId::generate());
        let t0 = Instant::now();
        w.入った(a, t0);
        w.入った(b, t0);
        assert!(!w.言うか(a, 鍵(3), t0));
        assert!(!w.言うか(b, 鍵(4), t0));
        assert_eq!(w.名簿が届いた(a), vec![鍵(3)]);
        assert_eq!(w.名簿が届いた(b), vec![鍵(4)]);
    }
}
