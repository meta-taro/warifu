//! 既定で返すもの。**本文が入る場所が無い。**

use core::fmt;

use crate::Error;

/// 種別の名前の上限（バイト）。`warifu-intent` の口と同じ長さに揃えてある。
const MAX_KIND: usize = 64;

/// 何の通知か。
///
/// **決めるのは規則であって、送信者ではない。**規則がまだ無ければ [`Kind::unknown`]。
/// 知らないものを知っているふりはしない（`warifu-intent::Kind::is_known` と同じ姿勢）。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Kind(String);

impl Kind {
    /// まだ何か分かっていない、を表す名前。
    pub const UNKNOWN: &'static str = "unknown";

    /// 分かっていない種別。
    pub fn unknown() -> Self {
        Self(Self::UNKNOWN.to_owned())
    }

    /// 種別を作る。**正規形しか受け取らない**（小文字・数字・点。点は区切りにしか置けない）。
    ///
    /// 表記が 2 通りあると、同じ種別が別物として通り、規則が二重にできる。
    pub fn new(s: &str) -> Result<Self, Error> {
        let 形が正しい = !s.is_empty()
            && s.len() <= MAX_KIND
            && !s.starts_with('.')
            && !s.ends_with('.')
            && !s.contains("..")
            && s.bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'.');
        if !形が正しい {
            return Err(Error::Malformed);
        }
        Ok(Self(s.to_owned()))
    }

    /// 文字列として見る。
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// 何か分かっているか。
    pub fn is_known(&self) -> bool {
        self.0 != Self::UNKNOWN
    }
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// 優先度。
///
/// **こちらが決める。**送信者は申告できない。
/// 申告できるなら全員が [`Priority::High`] を付けるので、並べ替えの役に立たなくなる。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
#[non_exhaustive]
pub enum Priority {
    /// 後で見ればよい。
    Low,
    /// 既定。**規則が無ければ常にこれ。**
    #[default]
    Normal,
    /// 先に見る。
    High,
}

/// 既定で返すもの。
///
/// **本文が入る場所が無い。**「Level 0 では本文を返さない」を実行時の判定ではなく
/// 型で保証する。判定にすると、いつか判定を通さない経路ができる。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Metadata {
    sender: crate::SenderId,
    source: crate::Source,
    received_at: u64,
    kind: Kind,
    priority: Priority,
    action_required: bool,
}

impl Metadata {
    pub(crate) fn new(
        sender: crate::SenderId,
        source: crate::Source,
        received_at: u64,
        kind: Kind,
        priority: Priority,
        action_required: bool,
    ) -> Self {
        Self {
            sender,
            source,
            received_at,
            kind,
            priority,
            action_required,
        }
    }

    /// 送信元。
    pub fn sender(&self) -> &crate::SenderId {
        &self.sender
    }

    /// どの経路から来たか。
    pub fn source(&self) -> crate::Source {
        self.source
    }

    /// こちらの時計で受け取った時刻。
    pub fn received_at(&self) -> u64 {
        self.received_at
    }

    /// 何の通知か。
    pub fn kind(&self) -> &Kind {
        &self.kind
    }

    /// 優先度。**こちらが決めたもの。**
    pub fn priority(&self) -> Priority {
        self.priority
    }

    /// 人が判断する必要があるか。**こちらが決めたもの。**
    pub fn action_required(&self) -> bool {
        self.action_required
    }
}
