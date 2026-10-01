//! 決めたことの記録。**消す口を置かない。**
//!
//! 何を許して何を断ったかが残らないと、後から誰も確かめられない。
//! `warifu-read` の会計（**D20**）と同じ理屈で、`Log` に削除の口は無い。

use core::fmt;

use crate::{Action, Decision, Error, Request, Subject};

/// 記録の見出し。**列の順序と数はここが正本。**
const 見出し: &str = "時刻\t相手\t動作\t判定";

/// 1 件ぶんの記録。
///
/// **本文が入る場所が無い。**要求が本文を持たないので、入りようがない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    at: u64,
    subject: Subject,
    action: Action,
    decision: Decision,
}

impl Record {
    /// いつの判定か。
    pub fn at(&self) -> u64 {
        self.at
    }

    /// 誰の要求か。
    pub fn subject(&self) -> &Subject {
        &self.subject
    }

    /// 何の要求か。
    pub fn action(&self) -> &Action {
        &self.action
    }

    /// どう決めたか。
    pub fn decision(&self) -> Decision {
        self.decision
    }

    fn to_tsv(&self) -> String {
        format!(
            "{}\t{}\t{}\t{}",
            self.at,
            self.subject,
            self.action,
            match self.decision {
                Decision::Allow => "通した",
                Decision::Deny => "断った",
            }
        )
    }

    fn from_tsv(行: &str) -> Result<Self, Error> {
        let 列: Vec<&str> = 行.split('\t').collect();
        if 列.len() != 4 {
            return Err(Error::Malformed);
        }
        Ok(Self {
            at: 列[0].parse().map_err(|_| Error::Malformed)?,
            subject: Subject::new(列[1])?,
            action: Action::new(列[2])?,
            decision: match 列[3] {
                "通した" => Decision::Allow,
                "断った" => Decision::Deny,
                _ => return Err(Error::Malformed),
            },
        })
    }
}

/// 判定の記録。
///
/// **消す口を置いていない。**消せると、断った事実を後から無かったことにできる。
#[derive(Debug, Clone, Default)]
pub struct Log {
    records: Vec<Record>,
}

impl Log {
    /// 空の記録。
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn push(&mut self, request: &Request, decision: Decision, at: u64) {
        self.records.push(Record {
            at,
            subject: request.subject().clone(),
            action: request.action().clone(),
            decision,
        });
    }

    /// 記録の中身。
    pub fn records(&self) -> &[Record] {
        &self.records
    }

    /// 何件残っているか。
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// 1 件も無いか。
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// 通した数。
    pub fn allowed(&self) -> usize {
        self.records
            .iter()
            .filter(|r| r.decision == Decision::Allow)
            .count()
    }

    /// 断った数。
    pub fn denied(&self) -> usize {
        self.records
            .iter()
            .filter(|r| r.decision == Decision::Deny)
            .count()
    }

    /// TSV にする。**1 行目は見出し。**
    pub fn to_tsv(&self) -> String {
        let mut s = String::from(見出し);
        for r in &self.records {
            s.push('\n');
            s.push_str(&r.to_tsv());
        }
        s.push('\n');
        s
    }

    /// TSV から読む。**読めない行があればそこで失敗する。**
    ///
    /// 黙って捨てると、断った件数が後から減る。
    pub fn from_tsv(text: &str) -> Result<Self, Error> {
        let mut 行 = text.lines();
        if 行.next().map(str::trim_end) != Some(見出し) {
            return Err(Error::Malformed);
        }
        Ok(Self {
            records: 行
                .filter(|l| !l.trim().is_empty())
                .map(Record::from_tsv)
                .collect::<Result<Vec<_>, _>>()?,
        })
    }
}

impl fmt::Display for Log {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_tsv())
    }
}
