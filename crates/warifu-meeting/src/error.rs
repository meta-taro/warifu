use core::fmt;

/// 会議まわりで起きる失敗。
///
/// **「上限で断った」と「形が壊れている」を混ぜない。**
/// 混ぜると、4 人の上限に当たったのか相手が warifu でないのかが分からなくなる。
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// もう 4 人いる（`decisions.md` **D7** に触れない境界）。
    Full,
    /// もう名簿に載っている。
    AlreadyIn,
    /// 会議の口ではない（`file.*` など）。
    NotMeeting,
    /// 塊の形が壊れている。**相手が warifu とは限らない。**
    Malformed,
    /// 一度に運ぶには大きすぎる。
    TooLarge,
    /// 下の口の層で落ちた。
    Intent(warifu_intent::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Full => f.write_str("会議はもう 4 人です"),
            Self::AlreadyIn => f.write_str("もう名簿に載っています"),
            Self::NotMeeting => f.write_str("会議の口ではありません"),
            Self::Malformed => f.write_str("会議の知らせの形が壊れています"),
            Self::TooLarge => f.write_str("一度に運ぶには大きすぎます"),
            Self::Intent(e) => write!(f, "口の層で落ちました: {e}"),
        }
    }
}

impl core::error::Error for Error {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Intent(e) => Some(e),
            _ => None,
        }
    }
}

impl From<warifu_intent::Error> for Error {
    fn from(e: warifu_intent::Error) -> Self {
        Self::Intent(e)
    }
}
