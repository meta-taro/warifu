//! 口の名前。`名前空間 . 動作`。

use core::fmt;
use core::str::FromStr;

use crate::Error;

/// 口の名前の上限（バイト）。
///
/// 長さを 1 バイトで書くので 255 までは入るが、**そこまで要る名前は設計を間違えている。**
pub(crate) const MAX_KIND: usize = 64;

/// warifu 自身が意味を知っている口。
///
/// D11 が warifu の担当と定めたのは **経路 / `file.*` / `meeting.*`** の 3 つだけ。
/// `invoice.*` `quotation.*` は md-business の領域なので、ここには入れない。
const KNOWN: [&str; 9] = [
    // 「これを渡したい / 受け取った」。中身はバイト列で、**種類を解釈しない**
    "file.offer",
    "file.accept",
    "file.reject",
    "file.chunk",
    "file.complete",
    // 会議の招集・参加・退出と、その下ごしらえ（SDP / ICE）
    "meeting.invite",
    "meeting.join",
    "meeting.leave",
    "meeting.signal",
];

/// 口の名前。
///
/// **正規形しか受け取らない。**小文字・数字・点だけで、点は区切りとしてしか置けない。
/// 表記が 2 通りあると、同じ口が別物として通る。
///
/// ```
/// use warifu_intent::Kind;
///
/// let 口 = Kind::new("file.offer")?;
/// assert_eq!(口.namespace(), "file");
/// assert!(口.is_known());
///
/// // 大文字は同じ意味に見えて別のバイト列になる
/// assert!(Kind::new("File.offer").is_err());
/// # Ok::<(), warifu_intent::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Kind(String);

impl Kind {
    /// 名前から作る。
    ///
    /// # Errors
    /// 正規形でなければ [`Error::Malformed`]。
    pub fn new(name: &str) -> Result<Self, Error> {
        if name.is_empty() || name.len() > MAX_KIND {
            return Err(Error::Malformed);
        }

        let mut 区切りの数 = 0usize;
        for 節 in name.split('.') {
            if 節.is_empty() {
                // 先頭・末尾の点も、点が 2 つ続くのもここで落ちる。
                // **`..` を弾くのはここ**（パスに見える名前を作らせない）
                return Err(Error::Malformed);
            }
            if !節
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
            {
                return Err(Error::Malformed);
            }
            区切りの数 += 1;
        }

        // 名前空間だけでは何をしたいのか決まらない
        if 区切りの数 < 2 {
            return Err(Error::Malformed);
        }

        Ok(Self(name.to_owned()))
    }

    /// 名前そのもの。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// 最初の節。`file.offer` なら `file`。
    #[must_use]
    pub fn namespace(&self) -> &str {
        self.0.split('.').next().unwrap_or_default()
    }

    /// warifu 自身が意味を知っている口かどうか。
    ///
    /// **知らない口も経路は通る。**版が 1 つずれただけで繋がらなくなるのを避けるため。
    /// 知らないものをどう扱うかは、呼ぶ側が決める（warifu は実行しない）。
    #[must_use]
    pub fn is_known(&self) -> bool {
        KNOWN.contains(&self.0.as_str())
    }
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for Kind {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Error> {
        Self::new(s)
    }
}
