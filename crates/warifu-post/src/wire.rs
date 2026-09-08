//! 預かり所とのやり取り。**1 通で 1 つ。**
//!
//! # 差出人を名乗らない
//!
//! 預ける側は**宛先と封だけ**を渡す。差出人を書く場所が無い。
//! 書けると、**預かり所が「誰が誰に送ったか」を読める** ——
//! 封の中身を読めないようにした意味（**D69**）が薄れる。
//!
//! # 受け取る側は宛先を名乗らない
//!
//! 「私あては？」と尋ねるだけで、**誰あてかは経路が決める。**
//! 尋ねる側が宛先を名乗れると、**他人あてを引き取れる**ことになる。

use warifu_core::PublicKey;

/// 目印。**別の口と混ぜない。**
const MAGIC: &[u8; 4] = b"WRFP";
/// 版。**版が違えば受けない。**
const VERSION: u8 = 1;

const PUT: u8 = 1;
const TAKE: u8 = 2;
const KEPT: u8 = 3;
const REFUSED: u8 = 4;
const HANDED: u8 = 5;

/// 預かり所へ頼むこと。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ask {
    /// 預ける。**差出人を名乗る場所は無い。**
    Put {
        /// 宛先。
        to: PublicKey,
        /// 封（**預かり所は開けられない**）。
        sealed: Vec<u8>,
    },
    /// **自分あてを受け取る。**宛先を名乗らない（経路が決める）。
    Take,
}

/// 預かり所からの返事。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reply {
    /// 預かった。
    Kept,
    /// 断った。**理由は言わない**（総当たりの手がかりになる）。
    Refused,
    /// 渡す。**空のこともある** —— それは断りではない。
    Handed(Vec<Vec<u8>>),
}

impl Ask {
    /// 運ぶ形にする。
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut 塊 = 頭();
        match self {
            Self::Put { to, sealed } => {
                塊.push(PUT);
                塊.extend_from_slice(&to.to_bytes());
                塊.extend_from_slice(sealed);
            }
            Self::Take => 塊.push(TAKE),
        }
        塊
    }

    /// 運ばれてきた形から読む。
    #[must_use]
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let 中 = 中身(bytes)?;
        match *中.first()? {
            PUT => {
                if 中.len() < 33 {
                    return None;
                }
                let to = PublicKey::from_bytes(中[1..33].try_into().ok()?).ok()?;
                Some(Self::Put {
                    to,
                    sealed: 中[33..].to_vec(),
                })
            }
            TAKE if 中.len() == 1 => Some(Self::Take),
            _ => None,
        }
    }
}

impl Reply {
    /// 運ぶ形にする。
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut 塊 = 頭();
        match self {
            Self::Kept => 塊.push(KEPT),
            Self::Refused => 塊.push(REFUSED),
            Self::Handed(封たち) => {
                塊.push(HANDED);
                // **1 通ごとに長さを前置きする。**境目が要る
                for 封 in 封たち {
                    塊.extend_from_slice(
                        &u32::try_from(封.len()).unwrap_or(u32::MAX).to_be_bytes(),
                    );
                    塊.extend_from_slice(封);
                }
            }
        }
        塊
    }

    /// 運ばれてきた形から読む。
    #[must_use]
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let 中 = 中身(bytes)?;
        match *中.first()? {
            KEPT if 中.len() == 1 => Some(Self::Kept),
            REFUSED if 中.len() == 1 => Some(Self::Refused),
            HANDED => {
                let mut 封たち = Vec::new();
                let mut i = 1;
                while i < 中.len() {
                    if i + 4 > 中.len() {
                        return None;
                    }
                    let n = u32::from_be_bytes(中[i..i + 4].try_into().ok()?) as usize;
                    i += 4;
                    if i + n > 中.len() {
                        return None;
                    }
                    封たち.push(中[i..i + n].to_vec());
                    i += n;
                }
                Some(Self::Handed(封たち))
            }
            _ => None,
        }
    }
}

fn 頭() -> Vec<u8> {
    let mut 塊 = Vec::with_capacity(8);
    塊.extend_from_slice(MAGIC);
    塊.push(VERSION);
    塊
}

/// 目印と版を確かめて、中身だけを返す。
fn 中身(bytes: &[u8]) -> Option<&[u8]> {
    if bytes.len() < 6 || &bytes[..4] != MAGIC || bytes[4] != VERSION {
        return None;
    }
    Some(&bytes[5..])
}
