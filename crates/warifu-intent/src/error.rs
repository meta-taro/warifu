use core::fmt;

/// 口のやり取りで起きる失敗。
///
/// 経路の失敗は [`Error::Route`] に包んで**捨てない**。
/// 「読めなかった」のか「切れた」のかが混ざると、直しようがなくなる。
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// 口の名前か塊の形が壊れている。**相手が warifu とは限らない。**
    Malformed,
    /// 一度に運ぶには大きすぎる。
    TooLarge,
    /// 下の経路で落ちた。
    Route(warifu_net::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Malformed => f.write_str("口の形が壊れています"),
            Self::TooLarge => f.write_str("一度に運ぶには大きすぎます"),
            Self::Route(e) => write!(f, "経路で落ちました: {e}"),
        }
    }
}

impl core::error::Error for Error {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Route(e) => Some(e),
            _ => None,
        }
    }
}

impl From<warifu_net::Error> for Error {
    fn from(e: warifu_net::Error) -> Self {
        Self::Route(e)
    }
}
