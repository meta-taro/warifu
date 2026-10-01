//! 「何をしたいか」1 つぶんと、その塊への直し方。

use core::fmt;
use core::str::FromStr;

use crate::kind::MAX_KIND;
use crate::{Error, Kind};

/// どの話の続きかを指す印の長さ（バイト）。
const CORRELATION: usize = 16;

/// 塊の先頭に必ず付く分の最大。`名前の長さ 1 + 名前 + 相関`。
const MAX_HEADER: usize = 1 + MAX_KIND + CORRELATION;

/// 一度に運べる荷物の上限。
///
/// 経路の上限（[`warifu_net::MAX_MESSAGE`]）から、塊の先頭に付く分を引いたもの。
/// **これを超える文書は `file.chunk` で切って運ぶ。**
pub const MAX_PAYLOAD: usize = warifu_net::MAX_MESSAGE - MAX_HEADER;

/// どの話の続きかを指す印。
///
/// 申し出を出した側が決め、返事はそれをそのまま返す。
/// **これが無いと、複数の転送を同時に走らせたときにどの返事か分からない。**
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Correlation([u8; CORRELATION]);

impl Correlation {
    /// 新しく起こす。
    ///
    /// # Panics
    /// 乱数が取れないとき。**推測できる値で代用しない**（取り違えが起きる）。
    #[must_use]
    pub fn generate() -> Self {
        let mut raw = [0u8; CORRELATION];
        getrandom::fill(&mut raw).expect("乱数が取れない");
        Self(raw)
    }

    /// そのままのバイト列。
    #[must_use]
    pub fn as_bytes(&self) -> &[u8; CORRELATION] {
        &self.0
    }
}

impl fmt::Display for Correlation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // warifu が外に出す文字列は base32 の 1 種類に揃える（M1・M2 と同じ）
        f.write_str(&warifu_core::base32::encode(&self.0))
    }
}

impl fmt::Debug for Correlation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self}")
    }
}

impl FromStr for Correlation {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Error> {
        let raw = warifu_core::base32::decode(s).ok_or(Error::Malformed)?;
        let raw: [u8; CORRELATION] = raw.try_into().map_err(|_| Error::Malformed)?;
        Ok(Self(raw))
    }
}

/// 相手に伝える「何をしたいか」1 つぶん。
///
/// # この層は中身を読まない
///
/// [`Intent::payload`] が Markdown でも写真でも、warifu からは同じバイト列に見える
/// （`decisions.md` **D11**）。**`.md` だから特別扱い、はやらない。**
/// 一度でも中身を見に行くと、warifu は文書ツールの付属品になる。
///
/// # 受け取ったものは命令ではない
///
/// この型には「受け取ったら実行する」口が 1 つも無い（**D5**）。
/// 開くか・保存するか・捨てるかは、呼ぶ側が決める。
#[derive(Clone, PartialEq, Eq)]
pub struct Intent {
    kind: Kind,
    correlation: Correlation,
    payload: Vec<u8>,
}

impl Intent {
    /// 新しい話として起こす。相関は自動で付く。
    #[must_use]
    pub fn new(kind: Kind, payload: Vec<u8>) -> Self {
        Self {
            kind,
            correlation: Correlation::generate(),
            payload,
        }
    }

    /// 相関を指定して起こす。**既存の話の続きにするとき以外は [`Intent::new`] を使う。**
    #[must_use]
    pub fn with_correlation(kind: Kind, correlation: Correlation, payload: Vec<u8>) -> Self {
        Self {
            kind,
            correlation,
            payload,
        }
    }

    /// これへの返事を作る。**相関はそのまま引き継ぐ。**
    #[must_use]
    pub fn reply(&self, kind: Kind, payload: Vec<u8>) -> Self {
        Self::with_correlation(kind, self.correlation, payload)
    }

    /// 何をしたいか。
    #[must_use]
    pub fn kind(&self) -> &Kind {
        &self.kind
    }

    /// どの話の続きか。
    #[must_use]
    pub fn correlation(&self) -> Correlation {
        self.correlation
    }

    /// 荷物。**warifu はこれを読まない。**
    #[must_use]
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    /// 荷物を取り出す（写さずに渡す）。
    #[must_use]
    pub fn into_payload(self) -> Vec<u8> {
        self.payload
    }

    /// 経路に流せる 1 つの塊にする。
    ///
    /// ```text
    ///   [0]        名前の長さ（1..=64）
    ///   [1..]      名前（小文字・数字・点）
    ///   [..+16]    相関
    ///   [残り]     荷物
    /// ```
    ///
    /// **同じ [`Intent`] は必ず同じバイト列になる。**表記が 2 通りあると、
    /// 同じものが照合で一致しない形で壊れる。
    ///
    /// # Errors
    /// 荷物が [`MAX_PAYLOAD`] を超えたら [`Error::TooLarge`]。
    pub fn encode(&self) -> Result<Vec<u8>, Error> {
        if self.payload.len() > MAX_PAYLOAD {
            return Err(Error::TooLarge);
        }

        let 名前 = self.kind.as_str().as_bytes();
        let 長さ = u8::try_from(名前.len()).map_err(|_| Error::Malformed)?;

        let mut 塊 = Vec::with_capacity(1 + 名前.len() + CORRELATION + self.payload.len());
        塊.push(長さ);
        塊.extend_from_slice(名前);
        塊.extend_from_slice(self.correlation.as_bytes());
        塊.extend_from_slice(&self.payload);
        Ok(塊)
    }

    /// 塊から読み戻す。
    ///
    /// **足りない塊で確保だけさせない。**長さを信じる前に、実際にそれだけ来ているかを見る。
    ///
    /// # Errors
    /// 形が壊れていれば [`Error::Malformed`]。荷物が大きすぎれば [`Error::TooLarge`]。
    pub fn decode(bytes: &[u8]) -> Result<Self, Error> {
        let (&長さ, 残り) = bytes.split_first().ok_or(Error::Malformed)?;
        let 長さ = usize::from(長さ);

        if 長さ == 0 || 長さ > MAX_KIND || 残り.len() < 長さ + CORRELATION {
            return Err(Error::Malformed);
        }

        let (名前, 残り) = 残り.split_at(長さ);
        let 名前 = core::str::from_utf8(名前).map_err(|_| Error::Malformed)?;
        let kind = Kind::new(名前)?;

        let (相関, 荷物) = 残り.split_at(CORRELATION);
        let 相関: [u8; CORRELATION] = 相関.try_into().map_err(|_| Error::Malformed)?;

        if 荷物.len() > MAX_PAYLOAD {
            return Err(Error::TooLarge);
        }

        Ok(Self {
            kind,
            correlation: Correlation(相関),
            payload: 荷物.to_vec(),
        })
    }
}

impl fmt::Debug for Intent {
    /// **荷物の中身は出さない。**
    ///
    /// 荷物は文書そのもの。ログに落ちると、経路を暗号化した意味が消える。
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Intent")
            .field("kind", &self.kind.as_str())
            .field("correlation", &self.correlation)
            .field("payload_len", &self.payload.len())
            .finish()
    }
}
