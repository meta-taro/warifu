//! 時刻の区間。

use crate::Error;

/// 時刻の区間（epoch 秒）。**始まりより終わりが後でなければ作れない。**
///
/// 長さ 0 の区間を許さないのは、**空き枠として返ると意味が無い**からである。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Span {
    start: u64,
    end: u64,
}

impl Span {
    /// 区間を作る。`start < end` でなければ受け取らない。
    pub fn new(start: u64, end: u64) -> Result<Self, Error> {
        if start >= end {
            return Err(Error::Malformed);
        }
        Ok(Self { start, end })
    }

    /// 始まり。
    pub fn start(&self) -> u64 {
        self.start
    }

    /// 終わり。
    pub fn end(&self) -> u64 {
        self.end
    }

    /// 長さ（秒）。
    ///
    /// `len` ではなく `duration` にしてある。**長さ 0 を作れない型**なので、
    /// `len` に付いてくる `is_empty` が嘘になる。
    pub fn duration(&self) -> u64 {
        self.end - self.start
    }

    /// 重なっているか。**接しているだけは重なりとしない。**
    pub fn overlaps(&self, other: &Self) -> bool {
        self.start < other.end && other.start < self.end
    }
}
