//! 失敗の種類。

use core::fmt;

/// この層の失敗。
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// 形が壊れている（区間の向き・長さ 0 など）。
    Malformed,
    /// 尋ねる窓が広すぎる。
    ///
    /// **広く取れるなら、空き枠を尋ねるだけで予定表を丸ごと写し取れる。**
    WindowTooWide,
    /// 候補に無い枠を承認しようとした。
    NotOffered,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Malformed => f.write_str("形が壊れています"),
            Self::WindowTooWide => f.write_str("尋ねる窓が広すぎます"),
            Self::NotOffered => f.write_str("候補に無い枠です"),
        }
    }
}

impl core::error::Error for Error {}
