//! **部屋の合言葉と、その証し**（**D118** / **#28** の本線 D）。
//!
//! # なぜ要るか
//!
//! **割符は 2 人の間のものである。**主催とゲスト A、主催とゲスト B ——
//! **ゲスト A とゲスト B の間には、何も無い。**
//!
//! だから A が B を呼んでも、**B の戸口が黙って落とす**（**D31** の正しい拒否）。
//! 2026-09-17、3 台で測って、そのまま出た ——
//!
//! ```text
//! 紹介: 呼びに行かせます（I3ILQUUJQHVS…・住所 90 文字）
//! 紹介: 呼べませんでした: **割符が付いていません（宛先だけでは繋げません）**
//! ```
//!
//! **主催は住所を配れるが、割符は配れない。**
//! **主催が名簿を配る**（**D111**・足場 A）だけでは、メッシュは組めない。
//!
//! # 決めたこと
//!
//! **部屋に 1 つ合言葉を持たせる。**
//! **割符で戸口を通った人だけが合言葉を受け取り、
//! 以後その部屋の中では、合言葉を示し合って互いを通す。**
//!
//! ```text
//! 主催 ── 割符（1 本＝1 人・人が手で渡す）──→ ゲスト   ← 変えない（D12・D31）
//! 主催 ── 合言葉（部屋に 1 つ）───────────→ ゲスト   ← 戸口を通った直後
//! ゲスト A ── 証し ──→ ゲスト B                      ← ここが新しい
//! ```
//!
//! # 飲むこと（**隠さない**）
//!
//! **部屋に居る人は、誰でも、その部屋へ他人を入れられる。**
//! 合言葉を渡してしまえば、その人は通る。**D111 より弱い。**
//!
//! それでも採るのは ——
//!
//! - **A では繋がらないことが実測で出た。**弱いが動くものと、強いが動かないものなら、動くほう
//! - **範囲は部屋の中だけ。**合言葉で通った相手は**連絡帳に書かない**（**D12 は無傷**）
//! - **サーバが要らない。**主催が落ちても、残った人は合言葉を持っているので繋がり続ける

use core::fmt;

use zeroize::Zeroize as _;

use crate::tally::{digest, same};
use crate::{Error, PublicKey, base32};

/// 合言葉の長さ。**割符の secret と同じ 32 バイト。**
const 長さ: usize = 32;

/// **部屋の合言葉。**
///
/// **これを持っている人は、その部屋へ誰でも入れられる。**
/// だから**鍵（`WARIFU1-…`）には載せない** ——
/// 鍵は人が手で渡すもので、**public な所に貼られる**（この試験で実際に何度も貼った）。
/// **合言葉が鍵に入っていたら、貼った瞬間に部屋ごと開く。**
///
/// 置くときは **0600**。**記録に書かない。**
#[derive(Clone, PartialEq, Eq)]
pub struct 合言葉([u8; 長さ]);

impl 合言葉 {
    /// **新しく作る。**部屋を建てるときに主催が 1 度だけ呼ぶ。
    ///
    /// # Errors
    /// 乱数が取れなければ [`Error::Rng`]。
    pub fn 作る() -> Result<Self, Error> {
        let mut 中身 = [0u8; 長さ];
        getrandom::fill(&mut 中身).map_err(|_| Error::Rng)?;
        Ok(Self(中身))
    }

    /// 受け取ったバイト列から戻す。
    #[must_use]
    pub const fn から(中身: [u8; 長さ]) -> Self {
        Self(中身)
    }

    /// **渡すためのバイト列。**
    ///
    /// **これは秘密である。**戸口を通った相手へ、**その経路で**渡すだけに使う。
    /// **記録・Issue・commit へ書かない。**
    #[must_use]
    pub const fn バイト列(&self) -> [u8; 長さ] {
        self.0
    }

    /// **証しを作る。**「この部屋の合言葉を知っている」を、**その 2 人に縛って**示す。
    ///
    /// # なぜ 2 人に縛るのか
    ///
    /// ```text
    /// digest(… ‖ 部屋 ‖ **呼ぶ側の鍵** ‖ **受ける側の鍵**)
    /// ```
    ///
    /// **A が B へ出した証しを、B が C へ使い回せない**（C の鍵では合わない）。
    ///
    /// # なぜ乱数も期限も足さないのか
    ///
    /// **経路はすでに暗号化され、相手の公開鍵は下の層が確かめている**
    /// （`warifu-net` の `HELLO`）。**盗み聞きは前提から外せる。**
    /// だから**使い回しだけ止めれば足りる** ——
    /// 足しても、**合言葉そのものが漏れたら意味が無い。**
    #[must_use]
    pub fn 証しを作る(
        &self,
        部屋: &[u8],
        呼ぶ側: PublicKey,
        受ける側: PublicKey,
    ) -> 部屋の証し {
        部屋の証し(digest(
            b"warifu/v1/room-proof",
            &[&self.0, 部屋, &呼ぶ側.to_bytes(), &受ける側.to_bytes()],
        ))
    }

    /// **証しを検める。**合っていれば通してよい。
    ///
    /// **中身が違っても同じ時間で終わる**（[`same`] を使う）——
    /// 早く抜けると、**1 バイトずつ当てて証しを作れてしまう。**
    #[must_use]
    pub fn 証しが合うか(
        &self,
        証し: &部屋の証し,
        部屋: &[u8],
        呼ぶ側: PublicKey,
        受ける側: PublicKey,
    ) -> bool {
        same(&self.証しを作る(部屋, 呼ぶ側, 受ける側).0, &証し.0)
    }
}

impl Drop for 合言葉 {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

/// **合言葉そのものは出さない。**記録に落ちる道を作らない。
impl fmt::Debug for 合言葉 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("合言葉(出しません)")
    }
}

/// **「この部屋の合言葉を知っている」の証し。**
///
/// **合言葉そのものではない。**これを見た人は、**合言葉を復元できない。**
/// だから**証しは経路に流してよい。**
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct 部屋の証し([u8; 長さ]);

impl 部屋の証し {
    /// 渡すためのバイト列。
    #[must_use]
    pub const fn バイト列(&self) -> [u8; 長さ] {
        self.0
    }

    /// 受け取ったバイト列から戻す。
    #[must_use]
    pub const fn から(中身: [u8; 長さ]) -> Self {
        Self(中身)
    }
}

impl fmt::Display for 部屋の証し {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&base32::encode(&self.0))
    }
}

impl fmt::Debug for 部屋の証し {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "部屋の証し({self})")
    }
}

/// **部屋の証しを差し出す 1 通**（**D118**）。
///
/// # なぜ割符の片割れと別の形なのか
///
/// **受ける側は、最初の 1 通を読んで見分けなければならない** ——
/// 「割符の片割れ（[`crate::Acceptance`]）」か「部屋の証し」か。
/// **種別を分けていないと、片方を他方として読もうとして、
/// 理由の分からない不通になる**（2026-09-04 に、会議 id で同じ形を踏んだ）。
///
/// # 中に何が入っていないか
///
/// **呼ぶ側・受ける側の公開鍵は入れない。**
/// **経路が確定させた鍵を使う**（`warifu-net` の `Session::peer`）——
/// 入れてしまうと、**名乗った鍵と経路の鍵が食い違ったときに、
/// どちらを信じるかという要らない判断が生まれる。**
///
/// **合言葉そのものも入っていない。**入っているのは証しだけなので、
/// **これを見た人は合言葉を復元できない。**
#[derive(Clone, PartialEq, Eq)]
pub struct 部屋の叩き {
    部屋: Vec<u8>,
    証し: 部屋の証し,
}

impl 部屋の叩き {
    /// 組み立てる。
    #[must_use]
    pub fn new(部屋: &[u8], 証し: 部屋の証し) -> Self {
        Self {
            部屋: 部屋.to_vec(),
            証し,
        }
    }

    /// どの部屋についての証しか。
    #[must_use]
    pub fn 部屋(&self) -> &[u8] {
        &self.部屋
    }

    /// 証し。
    #[must_use]
    pub const fn 証し(&self) -> 部屋の証し {
        self.証し
    }

    /// 渡すためのバイト列。
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(4 + 1 + 2 + self.部屋.len() + 長さ);
        out.extend_from_slice(crate::tally::MAGIC);
        out.push(crate::tally::KIND_ROOM_PROOF);
        // **長さを前に置く。**置かないと、部屋 id と証しの境目が決まらない
        let 長 = u16::try_from(self.部屋.len()).unwrap_or(u16::MAX);
        out.extend_from_slice(&長.to_be_bytes());
        out.extend_from_slice(&self.部屋);
        out.extend_from_slice(&self.証し.0);
        out
    }

    /// 読む。
    ///
    /// # Errors
    /// 目印・種別・長さが合わなければ [`Error::Malformed`]。
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        const 頭: usize = 4 + 1 + 2;
        if bytes.len() < 頭 + 長さ
            || &bytes[..4] != crate::tally::MAGIC
            || bytes[4] != crate::tally::KIND_ROOM_PROOF
        {
            return Err(Error::Malformed);
        }
        let 長 = usize::from(u16::from_be_bytes([bytes[5], bytes[6]]));
        // **長さが合っているかを、必ず見る。**
        // 見ないと、短い部屋 id を渡して証しの一部を部屋 id として読ませられる
        if bytes.len() != 頭 + 長 + 長さ {
            return Err(Error::Malformed);
        }
        let 部屋 = bytes[頭..頭 + 長].to_vec();
        let mut 証し = [0u8; 長さ];
        証し.copy_from_slice(&bytes[頭 + 長..]);
        Ok(Self {
            部屋,
            証し: 部屋の証し(証し),
        })
    }
}

impl fmt::Debug for 部屋の叩き {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("部屋の叩き")
            .field("部屋", &String::from_utf8_lossy(&self.部屋))
            .field("証し", &self.証し)
            .finish()
    }
}
