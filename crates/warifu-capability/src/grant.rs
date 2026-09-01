//! 札と要求。**要求は本文を持たない。**

use crate::{Action, Subject};

/// 何をしてよいかの札。**人が発行する。**
///
/// 要求の側から札を作れる口を置いていない。
/// 置いた時点で、**要求が自分に許可を出せる**（`issues/008`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Grant {
    subject: Subject,
    action: Action,
    expires_at: u64,
}

impl Grant {
    /// 札を出す。**期限は必ず要る。**期限なしの札を作れるようにしない。
    ///
    /// 期限が無い札は、出したことを忘れられる。忘れられた許可は、
    /// 誰も見ていないところで効き続ける。
    pub fn new(subject: Subject, action: Action, expires_at: u64) -> Self {
        Self {
            subject,
            action,
            expires_at,
        }
    }

    /// 誰に出した札か。
    pub fn subject(&self) -> &Subject {
        &self.subject
    }

    /// 何をしてよい札か。
    pub fn action(&self) -> &Action {
        &self.action
    }

    /// いつまで有効か（この時刻**まで**は有効）。
    pub fn expires_at(&self) -> u64 {
        self.expires_at
    }
}

/// 要求。
///
/// **本文が入る場所が無い。**
///
/// 入らないので、相手が何を書いてきても判定は動かない。
/// 「本文を見ないように気をつける」形は必ず漏れるので、**型で入れなくしてある**
/// （`warifu-read` の Level 0 と同じ手）。
///
/// 本文を読むのは `warifu-read` の仕事。
/// **読むことと、してよいかを決めることを分ける。**
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    subject: Subject,
    action: Action,
}

impl Request {
    /// 要求を作る。
    pub fn new(subject: Subject, action: Action) -> Self {
        Self { subject, action }
    }

    /// 誰の要求か。
    pub fn subject(&self) -> &Subject {
        &self.subject
    }

    /// 何をしたいのか。
    pub fn action(&self) -> &Action {
        &self.action
    }
}
