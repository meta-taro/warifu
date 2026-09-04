//! 招待の文字列（M5-c3）。**宛先と割符を 1 本にする。**
//!
//! 宛先だけを渡す形にすると、**受け取った側は誰でも繋げてしまう。**
//! それは D31（知らない相手は断る）と D12（割符で相手を確定する）に反する。
//!
//! 割符を一緒に渡すことで、戸口が「**割符があるから開ける**」と言えるようになる。
//! 割符は人が渡したものであり、渡した時点で人はもう判断している。

use core::str::FromStr;

use warifu_core::{PublicKey, TallyToken, base32};
use warifu_meeting::MeetingId;

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
    /// 会議 id が無い、または読めない。
    ///
    /// **古い形の会議キーをここで止める。**黙って受け取ると、入る側が別の会議を名乗り、
    /// 相手が「別の会議あて」として捨てる —— **原因の分からない不通になる**
    /// （2026-09-04 に実機で踏んだ）。
    BadMeeting,
}

impl core::fmt::Display for InviteError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NoSeparator => write!(f, "割符が付いていません（宛先だけでは繋げません）"),
            Self::EmptyAddress => write!(f, "宛先がありません"),
            Self::BadTally => write!(f, "割符として読めません"),
            Self::BadMeeting => write!(f, "会議が分かりません（古い形の会議キーかもしれません）"),
        }
    }
}

impl core::error::Error for InviteError {}

/// 宛先と割符と**会議 id** を 1 本の文字列にする。
///
/// **空白も改行も入れない。**紙・口頭・QR のどれでも運べる必要がある（M1）。
///
/// **会議 id を入れる理由。**入る側が自分で会議 id を作ると、
/// 相手の会議とは別の id になり、送った知らせが「別の会議あて」として捨てられる。
/// **主催者が決めた id を渡す**（2026-09-04 に実機で踏んだ）。
#[must_use]
pub fn format_invite(address: &str, token: &TallyToken, meeting: MeetingId) -> String {
    format!(
        "{address}{SEPARATOR}{}{SEPARATOR}{meeting}",
        base32::encode(&token.to_bytes())
    )
}

/// 招待を読む。
///
/// **署名は `TallyToken::from_bytes` が見る。**1 文字書き換われば通らない。
///
/// # Errors
/// 区切りが無い・宛先が空・割符が読めない・**会議 id が無いか読めない**。
pub fn parse_invite(text: &str) -> Result<(String, TallyToken, MeetingId), InviteError> {
    let trimmed = text.trim();
    let (address, rest) = trimmed
        .split_once(SEPARATOR)
        .ok_or(InviteError::NoSeparator)?;
    if address.is_empty() {
        return Err(InviteError::EmptyAddress);
    }
    // **会議 id が無い形は受け取らない**（古い会議キー）
    let (tally, meeting) = rest.split_once(SEPARATOR).ok_or(InviteError::BadMeeting)?;
    let bytes = base32::decode(tally).ok_or(InviteError::BadTally)?;
    let token = TallyToken::from_bytes(&bytes).map_err(|_| InviteError::BadTally)?;
    let meeting = MeetingId::from_str(meeting).map_err(|_| InviteError::BadMeeting)?;
    Ok((address.to_string(), token, meeting))
}

/// **自分が出した招待か。**
///
/// 1 台で 2 窓を開いて試すと必ず踏む。下の層（iroh）は
/// `Connecting to ourself is not supported` としか言わないので、
/// **繋ぎに行く前にここで気づいて、人の言葉で返す。**
#[must_use]
pub fn is_own_invite(me: PublicKey, token: &TallyToken) -> bool {
    token.issuer() == me
}
