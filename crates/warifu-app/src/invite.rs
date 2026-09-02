//! 招待の文字列（M5-c3）。**宛先と割符を 1 本にする。**
//!
//! 宛先だけを渡す形にすると、**受け取った側は誰でも繋げてしまう。**
//! それは D31（知らない相手は断る）と D12（割符で相手を確定する）に反する。
//!
//! 割符を一緒に渡すことで、戸口が「**割符があるから開ける**」と言えるようになる。
//! 割符は人が渡したものであり、渡した時点で人はもう判断している。

use warifu_core::{TallyToken, base32};

/// 宛先と割符の区切り。**base32 の字（A–Z / 2–7）に無いものを選ぶ。**
/// 区切りが本文に現れうると、どこで切るかが曖昧になる。
const SEPARATOR: char = '#';

/// 招待を組み立てられなかった／読めなかった理由。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InviteError {
    /// 区切りが無い。宛先だけを渡そうとしている。
    NoSeparator,
    /// 宛先が空。
    EmptyAddress,
    /// 割符として読めない（表記が壊れている・署名が合わない・長さが違う）。
    BadTally,
}

impl core::fmt::Display for InviteError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NoSeparator => write!(f, "割符が付いていません（宛先だけでは繋げません）"),
            Self::EmptyAddress => write!(f, "宛先がありません"),
            Self::BadTally => write!(f, "割符として読めません"),
        }
    }
}

impl core::error::Error for InviteError {}

/// 宛先と割符を 1 本の文字列にする。
///
/// **空白も改行も入れない。**紙・口頭・QR のどれでも運べる必要がある（M1）。
#[must_use]
pub fn format_invite(address: &str, token: &TallyToken) -> String {
    format!("{address}{SEPARATOR}{}", base32::encode(&token.to_bytes()))
}

/// 招待を読む。
///
/// **署名は `TallyToken::from_bytes` が見る。**1 文字書き換われば通らない。
///
/// # Errors
/// 区切りが無い・宛先が空・割符が読めない。
pub fn parse_invite(text: &str) -> Result<(String, TallyToken), InviteError> {
    let trimmed = text.trim();
    let (address, tally) = trimmed
        .split_once(SEPARATOR)
        .ok_or(InviteError::NoSeparator)?;
    if address.is_empty() {
        return Err(InviteError::EmptyAddress);
    }
    let bytes = base32::decode(tally).ok_or(InviteError::BadTally)?;
    let token = TallyToken::from_bytes(&bytes).map_err(|_| InviteError::BadTally)?;
    Ok((address.to_string(), token))
}
