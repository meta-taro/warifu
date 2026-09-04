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
    /// **まだ始まっていない。**予定に紐づく会議キーを、始まる前に使おうとした。
    TooEarly,
    /// 有効な時間の窓になっていない（終わりが始まりより前）。
    BadWindow,
    /// 署名が合わない。中身が書き換わっているか、差出人が違う。
    BadSignature,
    /// 別の割符に対する片割れだった。
    WrongTally,
    /// その割符はもう使われている。
    AlreadyUsed,
    /// **その割符で入っていた相手ではない。**
    ///
    /// 一度切れた相手が戻るときにだけ使う（[`crate::Tally::rematch_half`]）。
    /// **`AlreadyUsed` と混ぜない** —— あちらは「もう誰かが使った」で、
    /// こちらは「使ったのはあなたではない」である。混ぜると、
    /// **戻ってきた本人まで断られたのか、別人を断ったのかが分からなくなる。**
    NotTheHolder,
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
            Self::TooEarly => "まだ始まっていません",
            Self::BadWindow => "終わりが始まりより前になっています",
            Self::BadSignature => "署名が合いません",
            Self::WrongTally => "別の割符に対する片割れです",
            Self::AlreadyUsed => "その割符はすでに使われています",
            Self::NotTheHolder => "その会議キーで入っていた相手ではありません",
            Self::Revoked => "失効しています",
            Self::Malformed => "形が壊れています",
            Self::Rng => "乱数が取れませんでした",
        };
        f.write_str(s)
    }
}

impl core::error::Error for Error {}
