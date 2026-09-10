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

/// [`crate::Warifu::calendar_slots`] の引数。
///
/// **件数の上限がここに無いのは意図。**何件返すかを相手に決めさせない
/// （`warifu-calendar` の考え方と同じで、一度に見える量はこちらが決める）。
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct SlotsArgs {
    /// 窓の始まり（epoch 秒）。
    pub start: u64,
    /// 窓の終わり（epoch 秒）。**広すぎれば断られる。**
    pub end: u64,
    /// ほしい長さ（秒）。
    pub duration: u64,
}

/// [`crate::Warifu::chat_send`] の引数。
///
/// **差出人を書く場所が無いのは意図。**誰が言ったかは机が刻む
/// （[`warifu_desk::ToDesk`] と同じ約束）。
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct SayArgs {
    /// 流す本文。空は流せない。長すぎるものは受けない。
    pub body: String,
}

/// [`crate::Warifu::profile_set`] の引数。
///
/// **「誰の」を書く場所が無いのは意図。**どの席かは繋いできた口で決まる ——
/// 書けると、**同じ机の別のエージェントに化けられる。**
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct ProfileArgs {
    /// 表に出す名前。**空にすると消える。**
    pub name: String,
    /// 短い紹介。**空にすると消える。**
    pub bio: String,
}

/// [`crate::Warifu::chat_status`] の引数。
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct StatusArgs {
    /// どの発言か（`chat_send` の返りに出る番号）。
    pub id: u64,
}

/// [`crate::Warifu::changes`] の引数（**D82**）。
#[derive(Debug, Clone, Default, Deserialize, JsonSchema)]
pub struct ChangesArgs {
    /// どの版か（`v0.1.0-alpha.16`）。**省くと載っている版が全部出る。**
    #[serde(default)]
    pub version: Option<String>,
}

/// [`crate::Warifu::chat_wait`] の引数。
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct WaitArgs {
    /// 何秒まで待つか。省くと 30 秒。**上限は 60 秒。**
    ///
    /// **永遠には待たない。**待っている間、そのエージェントは何もできない。
    pub seconds: Option<u64>,
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
            // **何が要るかまで言う**（`issues/4`・2026-09-08）。
            // 「断られたことは分かるが、次に何をすればよいか分からない」を無くす。
            //
            // **D31（断る理由を相手に返さない）はここには当たらない。**
            // あれは**網の向こうの知らない相手**の話で、この口は
            // **同じ PC の中**の、札を出すのが目の前の人である相手である。
            Self::Denied(w) => write!(
                f,
                "関所が断りました: {w}。この口には `--allow {w}` の札が要ります。                 札を出せるのはこの PC の人だけです（`warifu setup` か、                 エージェントの設定に書きます）。この口からは出せません。"
            ),
            Self::BadArgs(w) => write!(f, "引数が読めません: {w}"),
            Self::Unavailable(w) => write!(f, "出せません: {w}"),
        }
    }
}

impl core::error::Error for ToolError {}
