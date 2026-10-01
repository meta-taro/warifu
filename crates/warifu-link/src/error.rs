//! 失敗の種類。

use core::fmt;

/// この層の失敗。
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// 自分の測定値が古い。**回線は変わるので、古い値で決めない。**
    Stale,
    /// 音声ぶんすら通らない。
    ///
    /// **黙って 0 人ぶんを返さない。**通らないなら通らないと言う。
    TooWeak,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Stale => f.write_str("測定値が古すぎます"),
            Self::TooWeak => f.write_str("音声ぶんも通りません"),
        }
    }
}

impl core::error::Error for Error {}
