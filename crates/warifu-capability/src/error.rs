//! 失敗の種類。

use core::fmt;

/// この層の失敗。
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// 名前の形が壊れている。
    Malformed,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Malformed => f.write_str("名前の形が壊れています"),
        }
    }
}

impl core::error::Error for Error {}
