//! 取り込みで起きる失敗。

use core::fmt;

/// この層の失敗。
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// 塊がメールとして読めない。
    Unparsable,
    /// 差出人が無い。
    ///
    /// **送信元が無いと規則の照合が成り立たない**（誰の規則で読むのかが決まらない）。
    NoSender,
    /// 読み取り層が受け取らなかった。**理由を捨てない。**
    Read(warifu_read::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unparsable => f.write_str("メールとして読めません"),
            Self::NoSender => f.write_str("差出人がありません"),
            Self::Read(e) => write!(f, "読み取り層が受け取りませんでした: {e}"),
        }
    }
}

impl core::error::Error for Error {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Read(e) => Some(e),
            _ => None,
        }
    }
}

impl From<warifu_read::Error> for Error {
    fn from(e: warifu_read::Error) -> Self {
        Self::Read(e)
    }
}
