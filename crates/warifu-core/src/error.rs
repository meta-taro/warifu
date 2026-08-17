use core::fmt;

/// 割符と鍵の取り扱いで起きる失敗。
///
/// **どれも「なぜ落ちたか」を相手に返してよい種類にしてある。**
/// 秘密そのものは入らない。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// 期限が切れている。
    Expired,
    /// 署名が合わない。中身が書き換わっているか、差出人が違う。
    BadSignature,
    /// 別の割符に対する片割れだった。
    WrongTally,
    /// その割符はもう使われている。
    AlreadyUsed,
    /// 失効している端末・割符だった。
    Revoked,
    /// 形が壊れている。長さ・目印・文字が合わない。
    Malformed,
    /// 乱数が取れなかった。
    Rng,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Expired => "期限が切れています",
            Self::BadSignature => "署名が合いません",
            Self::WrongTally => "別の割符に対する片割れです",
            Self::AlreadyUsed => "その割符はすでに使われています",
            Self::Revoked => "失効しています",
            Self::Malformed => "形が壊れています",
            Self::Rng => "乱数が取れませんでした",
        };
        f.write_str(s)
    }
}

impl core::error::Error for Error {}
