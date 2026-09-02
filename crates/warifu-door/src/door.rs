//! 戸口。**知らない相手は、人に取り次がずに断る。**

use std::collections::{HashMap, HashSet};

use crate::{Knock, Subject};

/// 叩きを数える窓の長さ（秒）。**1 時間。**
pub const WINDOW: u64 = 3_600;

/// 知らない相手 1 人が、窓のあいだに叩ける回数。
///
/// **断るのにも計算が要る。**断る相手にも上限を掛ける。
pub const STRANGER_QUOTA: usize = 5;

/// 知っている相手 1 人が、窓のあいだに叩ける回数。
///
/// **知り合いでも無制限にはしない。**端末が乗っ取られる場合がある。
pub const KNOWN_QUOTA: usize = 20;

/// 知らない相手ぜんぶで、窓のあいだに残す記録の数。
///
/// 1 人ずつ絞っても、**相手を変えられたら意味が無い。**
/// 全体にも上限を置いて、洪水で記録が膨らまないようにする。
const 知らない相手の記録の上限: usize = 1_000;

/// 戸口の答え。
///
/// **2 つしかない。**「人に聞く」を作らない。
///
/// 作ると、**知らない相手がこちらの注意を消費できる。**
/// **通知を出せること自体が資源**であり、そこを開けたら spam の入口になる。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Answer {
    /// 開ける。
    Open,
    /// 断る。
    ///
    /// **理由を分けない。**「知らない」と「絞られている」を区別できると、
    /// **絞りの境界を探れる**（時間を空けて叩き直せばよい、と分かってしまう）。
    Refuse,
}

/// 戸口。
///
/// 知っている相手と、知らない相手を**別々に数える。**
/// 分けていないと、**洪水を送るだけで会話を止められる。**
#[derive(Debug, Default)]
pub struct Door {
    知り合い: HashSet<Subject>,
    叩き: HashMap<Subject, Vec<u64>>,
}

impl Door {
    /// 誰も知らない戸口。
    pub fn new() -> Self {
        Self::default()
    }

    /// 叩きに答える。
    ///
    /// - **割符があれば開ける。**割符は人が渡したもので、渡した時点で人はもう判断している
    /// - 一度開けた相手は、次から割符なしで開ける
    /// - **それ以外は断る。人に聞かない**
    ///
    /// 開けた相手にも上限は掛かる（[`KNOWN_QUOTA`]）。
    pub fn answer(&mut self, knock: &Knock) -> Answer {
        let 知り合い = self.知り合い.contains(knock.from());
        self.記す(knock, 知り合い);

        let 回数 = self.knocks_from(knock.from());
        let 上限 = if 知り合い {
            KNOWN_QUOTA
        } else {
            STRANGER_QUOTA
        };
        if 回数 > 上限 {
            return Answer::Refuse;
        }
        if knock.has_tally() {
            self.知り合い.insert(knock.from().clone());
            return Answer::Open;
        }
        if 知り合い {
            return Answer::Open;
        }
        Answer::Refuse
    }

    /// この相手を知っているか。
    pub fn knows(&self, who: &Subject) -> bool {
        self.知り合い.contains(who)
    }

    /// 窓のあいだに、この相手が何回叩いたか。**人が見る材料。**
    pub fn knocks_from(&self, who: &Subject) -> usize {
        self.叩き.get(who).map_or(0, Vec::len)
    }

    /// 古い記録を落とす。
    ///
    /// **知り合いは忘れない。**記録を落としても、開けたという事実は残す。
    /// 忘れると、次に来たときにまた割符が要ることになる。
    pub fn forget_old(&mut self, now: u64) {
        for 並び in self.叩き.values_mut() {
            並び.retain(|t| now.saturating_sub(*t) <= WINDOW);
        }
        self.叩き.retain(|_, 並び| !並び.is_empty());
    }

    /// 1 回ぶんを記す。
    ///
    /// 窓の外の記録はここで落ちる。**時計が戻っても、戻ったぶんを数えない。**
    fn 記す(&mut self, knock: &Knock, 知り合い: bool) {
        let 今 = knock.at();
        let 並び = self.叩き.entry(knock.from().clone()).or_default();
        並び.retain(|t| 今.saturating_sub(*t) <= WINDOW && *t <= 今);
        並び.push(今);

        if !知り合い {
            self.知らない相手の記録を抑える(今);
        }
    }

    /// 知らない相手の記録が膨らみすぎないようにする。
    ///
    /// **相手を変えられても記録は増えるので、全体にも上限が要る。**
    /// 落とすのは知らない相手のぶんだけで、**知り合いのぶんは触らない。**
    fn 知らない相手の記録を抑える(&mut self, 今: u64) {
        let 知らない数 = self
            .叩き
            .keys()
            .filter(|k| !self.知り合い.contains(*k))
            .count();
        if 知らない数 <= 知らない相手の記録の上限 {
            return;
        }
        // 古い順に落とす。**新しい叩きのほうが、人にとって意味がある**
        let mut 知らない: Vec<(Subject, u64)> = self
            .叩き
            .iter()
            .filter(|(k, _)| !self.知り合い.contains(*k))
            .map(|(k, v)| (k.clone(), v.iter().copied().max().unwrap_or(今)))
            .collect();
        知らない.sort_by_key(|(_, 最後)| *最後);
        for (誰, _) in 知らない
            .into_iter()
            .take(知らない数 - 知らない相手の記録の上限)
        {
            self.叩き.remove(&誰);
        }
    }
}
