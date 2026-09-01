//! tool の引数と失敗。

use schemars::JsonSchema;
use serde::Deserialize;

/// [`crate::Warifu::inbox_open`] の引数。
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct OpenArgs {
    /// 何通目か（`inbox_list` が返す番号。0 から数える）。
    pub index: usize,
    /// どこまで開くか。`summary` / `structured` / `raw` / `attachments`。
    ///
    /// **段ごとに別の札が要る。**`metadata` はここでは指定できない
    /// （それは `inbox_list` の役目）。
    pub level: String,
}

/// tool が返す失敗。
///
/// **断った理由を、実行できなかった理由と混ぜない。**
/// 混ざると、札を足せば直るのか、直しようが無いのかが読み手に分からなくなる。
#[derive(Debug, PartialEq, Eq)]
pub enum ToolError {
    /// 関所が断った。**札が無いか、期限切れか、範囲外。**
    Denied(String),
    /// 引数が読めない。
    BadArgs(String),
    /// 読み取り層が出せなかった（解釈器が要る等）。
    Unavailable(String),
}

impl core::fmt::Display for ToolError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Denied(w) => write!(f, "関所が断りました: {w}"),
            Self::BadArgs(w) => write!(f, "引数が読めません: {w}"),
            Self::Unavailable(w) => write!(f, "出せません: {w}"),
        }
    }
}

impl core::error::Error for ToolError {}
