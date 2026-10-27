//! **同じ発言が 2 本の道で届いたとき、2 本目を落とす**（0.1.40・内部の Issue 026）。
//!
//! ルームは星形で、ゲストの発言は主催が配り直す（D48）。ところが紹介でゲストどうしが
//! 直接つながっていると、同じ発言が**直接の線**と**主催の配り直し**の 2 本の道で届き、
//! 受け手はどちらも正しい道として通していた（直接は差出人＝線の相手、主催経由は線の相手＝主催）。
//! 2026-10-27、6 台の部屋で mac air と ASUS の発言が全部 2 回ずつ出た。
//!
//! 発言に番号は無い（`Notice::Text` は本文だけ）ので、**差出人・部屋・本文が同じものが短い窓の中で
//! もう一度来たら落とす**。同じ人が同じ文を数秒内に 2 度打つことはまず無く、あっても 1 本に見える方が
//! 2 回ずつ出るより害が小さい。窓を過ぎれば同じ文でも通す。

use std::collections::VecDeque;
use std::hash::{DefaultHasher, Hash, Hasher};

/// 覚えておく発言の数の上限。古いものから捨てる（溜め込む所にしない）。
const 覚える数: usize = 256;

/// 最近届いた発言の控え。`K` は「差出人と部屋」の組。
#[derive(Debug, Clone)]
pub struct 重複の控え<K> {
    窓の秒: u64,
    最近: VecDeque<(u64, K, u64)>,
}

impl<K: PartialEq> 重複の控え<K> {
    /// 窓（秒）を決めて作る。
    #[must_use]
    pub fn new(窓の秒: u64) -> Self {
        Self {
            窓の秒,
            最近: VecDeque::new(),
        }
    }

    /// **この発言は、窓の中で既に届いたものか。**初めてなら控えて `false`。
    pub fn 重複か(&mut self, 鍵: K, 本文: &[u8], 今: u64) -> bool {
        let 指紋 = 指紋(本文);
        while let Some((時, _, _)) = self.最近.front()
            && 今.saturating_sub(*時) > self.窓の秒
        {
            self.最近.pop_front();
        }
        if self.最近.iter().any(|(_, k, f)| *k == 鍵 && *f == 指紋) {
            return true;
        }
        self.最近.push_back((今, 鍵, 指紋));
        if self.最近.len() > 覚える数 {
            self.最近.pop_front();
        }
        false
    }
}

fn 指紋(本文: &[u8]) -> u64 {
    let mut h = DefaultHasher::new();
    本文.hash(&mut h);
    h.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 同じ発言が窓の中でもう一度来たら_2_本目は重複() {
        let mut 控え = 重複の控え::new(5);
        assert!(
            !控え.重複か(("A", "部屋1"), "点呼".as_bytes(), 100),
            "初めては通す"
        );
        assert!(
            控え.重複か(("A", "部屋1"), "点呼".as_bytes(), 101),
            "主催経由の 2 本目は落とす"
        );
        assert!(
            控え.重複か(("A", "部屋1"), "点呼".as_bytes(), 105),
            "窓の内はまだ落とす"
        );
    }

    #[test]
    fn 差出人や部屋や本文が違えば_重複ではない() {
        let mut 控え = 重複の控え::new(5);
        assert!(!控え.重複か(("A", "部屋1"), "点呼".as_bytes(), 100));
        assert!(
            !控え.重複か(("B", "部屋1"), "点呼".as_bytes(), 100),
            "別の人の同じ文は通す"
        );
        assert!(
            !控え.重複か(("A", "部屋2"), "点呼".as_bytes(), 100),
            "別の部屋は通す"
        );
        assert!(
            !控え.重複か(("A", "部屋1"), "点呼 2 回目".as_bytes(), 100),
            "別の文は通す"
        );
    }

    #[test]
    fn 窓を過ぎれば_同じ文でも通す() {
        let mut 控え = 重複の控え::new(5);
        assert!(!控え.重複か(("A", "部屋1"), "はい".as_bytes(), 100));
        assert!(
            !控え.重複か(("A", "部屋1"), "はい".as_bytes(), 106),
            "6 秒後の同じ文は別の発言"
        );
    }

    #[test]
    fn 溜め込まない() {
        let mut 控え = 重複の控え::new(600);
        for i in 0..1000u64 {
            assert!(!控え.重複か(("A", "部屋1"), i.to_string().as_bytes(), 100));
        }
        assert!(控え.最近.len() <= 覚える数);
    }
}
