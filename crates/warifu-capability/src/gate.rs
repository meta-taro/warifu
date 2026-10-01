//! 関所。**既定は拒否。**

use std::collections::HashMap;

use crate::{Action, Grant, Log, Request, Subject};

/// どう決めたか。
///
/// **2 つしかない。**「たぶん大丈夫」を作らない。
/// 分からないものは [`Decision::Deny`] に倒す。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    /// 通す。**札があり、期限内で、範囲がぴったり合ったときだけ。**
    Allow,
    /// 断る。**既定。**
    Deny,
}

/// どれくらい知っている相手か。
///
/// **判定には使わない。**表示と、人が札を出すかどうかを考えるときの材料でしかない。
///
/// 使わない理由（`decisions.md` **D5**）: 「信頼した相手の言うことは通す」を作ると、
/// **信頼を得ることに価値が生まれる。**価値が生まれれば、
/// そこを狙って偽の Identity を作る意味が出る。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum Trust {
    /// 知らない。**既定。**
    #[default]
    Unknown,
    /// 一度やり取りした。
    Known,
    /// よくやり取りする。
    Close,
}

/// 関所。
///
/// **判定の入力は 3 つだけ** — 誰が / 何を / 今いつか。
/// 本文も、信頼の度合いも、入らない。
#[derive(Debug, Default)]
pub struct Gate {
    grants: Vec<Grant>,
    trust: HashMap<Subject, Trust>,
    log: Log,
}

impl Gate {
    /// 何も許していない関所。
    pub fn new() -> Self {
        Self::default()
    }

    /// 札を出す。**人が通す口。**
    ///
    /// 要求からこの口が呼ばれる経路は、この層のどこにも無い
    /// （`warifu-read` の `RuleStore::approve` と同じ形・**D19**）。
    pub fn issue(&mut self, grant: Grant) {
        self.grants.push(grant);
    }

    /// 出した札を降ろす。
    ///
    /// 降ろせないと、**間違って出したときに直せない。**
    /// （失効の名簿を取り消せなくしたのとは別の話で、あちらは「失効を取り消せない」。
    /// ここは「許可を取り消せる」。**安全の側へ倒す向きが逆にならないように**）
    pub fn revoke(&mut self, subject: &Subject, action: &Action) {
        self.grants
            .retain(|g| !(g.subject() == subject && g.action() == action));
    }

    /// 出してある札。
    pub fn grants(&self) -> &[Grant] {
        &self.grants
    }

    /// 相手をどれくらい知っているかを記す。**判定は変わらない。**
    pub fn set_trust(&mut self, subject: Subject, trust: Trust) {
        self.trust.insert(subject, trust);
    }

    /// どれくらい知っている相手か。
    pub fn trust(&self, subject: &Subject) -> Trust {
        self.trust.get(subject).copied().unwrap_or_default()
    }

    /// 決めたことの記録。
    pub fn log(&self) -> &Log {
        &self.log
    }

    /// 通してよいかを決める。**記録に残る。**
    ///
    /// `&mut self` を取るのは、**記録を残さずに判定できないようにする**ため。
    /// 記録の要らない判定の口を別に置くと、そちらが使われるようになる。
    ///
    /// 見るのは札だけ。**信頼の度合いも本文も見ない。**
    /// 動作は**完全一致**で照合する。前方一致にすると、
    /// `calendar.freebusy` の札で `calendar.freebusy.all` が通ってしまう。
    pub fn decide(&mut self, request: &Request, now: u64) -> Decision {
        let 通る = self.grants.iter().any(|g| {
            g.subject() == request.subject()
                && g.action() == request.action()
                && now <= g.expires_at()
        });
        let 判定 = if 通る {
            Decision::Allow
        } else {
            Decision::Deny
        };
        self.log.push(request, 判定, now);
        判定
    }
}
