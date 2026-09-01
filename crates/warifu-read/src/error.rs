//! 読めなかった理由。**「まだ作っていない」と「解釈器が要る」を混ぜない。**

use core::fmt;

use crate::Level;

/// この層の失敗。
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// 形が壊れている（送信元・種別など）。
    Malformed,
    /// **その段は解釈器（LLM 等）を呼ばないと出せない。**
    ///
    /// この層は既定で呼ばない。呼ぶかどうかは**呼ぶ側が決める**（`decisions.md` **D5**）。
    /// 規則が育てば、同じ形式で二度目からはこれが返らなくなる — それがこの層の目的。
    NeedsInterpreter(Level),
    /// **その段はまだ作っていない。**解釈器を呼べば出る、という意味ではない。
    NotBuiltYet(Level),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Malformed => f.write_str("形が壊れています"),
            Self::NeedsInterpreter(l) => write!(f, "{l} を出すには解釈器が要ります"),
            Self::NotBuiltYet(l) => write!(f, "{l} はまだ作っていません"),
        }
    }
}

impl core::error::Error for Error {}
