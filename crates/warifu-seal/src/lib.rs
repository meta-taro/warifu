//! 封。**中継は運ぶだけで、開けられない。**
//!
//! `PRD` も `issues/004` も「**中継者は中身を読めない**」と書いているが、
//! 2026-09-08 まで**そう作られていなかった。**
//!
//! いまの E2EE は**経路（iroh の QUIC/TLS）だけ**である。
//! 相手と直接つながっている間は端から端まで暗号だが、
//! **中継を挟めば、そこで平文になる。**
//!
//! ```text
//!   直接        A ══════════════════ B     経路が暗号。中身は 2 人だけ
//!   中継あり    A ═════ 中継 ═════ B     **中継が平文を見る**
//!   封をする    A ──封──中継──封── B     中継は運ぶだけ
//! ```
//!
//! # 守ること
//!
//! - **鍵を 2 つにしない**（**D42**）。身元の鍵から鍵合わせの鍵を導く ——
//!   2 つにすると**配る身元が 2 つ**になり、
//!   「この公開鍵の人へ封をする」が成り立たなくなる
//! - **封から差出人が分からない。**使い捨ての鍵で封をする ——
//!   出すと、**中継が「誰が誰に送ったか」を読める。**
//!   誰が言ったかは中身の `from` が持つ（**D48**）
//! - **同じ中身でも毎回ちがう封になる。**同じ封が並ぶと、
//!   同じことを言ったと中継に分かる
//! - **書き換えたら開かない。**運ぶ側が中身をすり替えられない

#![forbid(unsafe_code)]

use chacha20poly1305::aead::{Aead as _, KeyInit as _, Payload};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use ed25519_dalek::VerifyingKey;
use hkdf::Hkdf;
use sha2::Sha256;
use warifu_core::{Device, PublicKey};
use x25519_dalek::{PublicKey as X25519Public, StaticSecret};
use zeroize::Zeroize as _;

/// 封の目印。**種別を混ぜない。**
const MAGIC: &[u8; 4] = b"WRFS";
/// 封の版。**版が違えば開けない**（黙って新しい版として扱わない）。
const VERSION: u8 = 1;
/// 使い捨ての鍵（32）＋ nonce（12）。
const HEAD: usize = 4 + 1 + 32 + 12;

/// 鍵を導くときの言い分。**用途ごとに変える。**
const INFO: &[u8] = b"warifu-seal-v1";

/// 封をしたもの。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sealed(Vec<u8>);

impl Sealed {
    /// 運ぶ形にする。
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        self.0.clone()
    }

    /// 運ばれてきた形から読む。**形が合わなければ `None`。**
    #[must_use]
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < HEAD || &bytes[..4] != MAGIC || bytes[4] != VERSION {
            return None;
        }
        Some(Self(bytes.to_vec()))
    }
}

/// 封をするときに失敗したこと。
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// 相手の鍵が読めない。
    BadKey,
    /// 開けられない。**鍵が違うか、書き換えられている。**
    ///
    /// **どちらかを言わない。**言うと、総当たりの手がかりになる。
    CannotOpen,
    /// 乱数が取れなかった。
    Rng,
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::BadKey => f.write_str("相手の鍵が読めません"),
            Self::CannotOpen => f.write_str("開けられません"),
            Self::Rng => f.write_str("乱数が取れませんでした"),
        }
    }
}

impl core::error::Error for Error {}

/// **その人だけが開けられる封をする。**
///
/// # Errors
/// 相手の鍵が読めないとき、乱数が取れないとき。
pub fn seal(相手: PublicKey, 中身: &[u8]) -> Result<Sealed, Error> {
    let 受け手 = montgomery(相手)?;

    // **使い捨ての鍵で封をする。**封から差出人が分からないようにするため
    let mut 種 = [0u8; 32];
    getrandom::fill(&mut 種).map_err(|_| Error::Rng)?;
    let 使い捨て = StaticSecret::from(種);
    種.zeroize();
    let 使い捨ての公開 = X25519Public::from(&使い捨て);

    let mut nonce = [0u8; 12];
    getrandom::fill(&mut nonce).map_err(|_| Error::Rng)?;

    let 鍵 = 導く(
        &使い捨て.diffie_hellman(&受け手).to_bytes(),
        &使い捨ての公開,
        &受け手,
    );
    let 錠 = ChaCha20Poly1305::new(Key::from_slice(&鍵));

    let mut 封 = Vec::with_capacity(HEAD + 中身.len() + 16);
    封.extend_from_slice(MAGIC);
    封.push(VERSION);
    封.extend_from_slice(使い捨ての公開.as_bytes());
    封.extend_from_slice(&nonce);
    // **頭を一緒に確かめる。**すり替えると開かない
    let 頭 = 封.clone();
    let 中 = 錠
        .encrypt(
            Nonce::from_slice(&nonce),
            Payload {
                msg: 中身,
                aad: &頭,
            },
        )
        .map_err(|_| Error::CannotOpen)?;
    封.extend_from_slice(&中);
    Ok(Sealed(封))
}

/// **自分あての封を開ける。**
///
/// # Errors
/// 自分あてでないとき、書き換えられているとき [`Error::CannotOpen`]。
pub fn open(自分: &Device, 封: &Sealed) -> Result<Vec<u8>, Error> {
    let 塊 = &封.0;
    if 塊.len() < HEAD {
        return Err(Error::CannotOpen);
    }
    let 使い捨ての公開: [u8; 32] = 塊[5..37].try_into().map_err(|_| Error::CannotOpen)?;
    let nonce: [u8; 12] = 塊[37..HEAD].try_into().map_err(|_| Error::CannotOpen)?;
    let 使い捨ての公開 = X25519Public::from(使い捨ての公開);

    let mut 種 = 自分.agreement_secret();
    let 自分の鍵 = StaticSecret::from(種);
    種.zeroize();
    let 自分の公開 = X25519Public::from(&自分の鍵);

    let 鍵 = 導く(
        &自分の鍵.diffie_hellman(&使い捨ての公開).to_bytes(),
        &使い捨ての公開,
        &自分の公開,
    );
    let 錠 = ChaCha20Poly1305::new(Key::from_slice(&鍵));
    錠.decrypt(
        Nonce::from_slice(&nonce),
        Payload {
            msg: &塊[HEAD..],
            aad: &塊[..HEAD],
        },
    )
    .map_err(|_| Error::CannotOpen)
}

/// 身元の公開鍵から、鍵合わせの公開鍵を導く。
fn montgomery(相手: PublicKey) -> Result<X25519Public, Error> {
    let v = VerifyingKey::from_bytes(&相手.to_bytes()).map_err(|_| Error::BadKey)?;
    Ok(X25519Public::from(v.to_montgomery().to_bytes()))
}

/// 鍵合わせの結果から、封の鍵を導く。
///
/// **鍵合わせの結果をそのまま鍵にしない。**両端の公開鍵も混ぜて、
/// **別の相手あての封を作り替えられない**ようにする。
fn 導く(
    共有: &[u8; 32], 使い捨ての公開: &X25519Public, 受け手: &X25519Public
) -> [u8; 32] {
    let mut 塩 = Vec::with_capacity(64);
    塩.extend_from_slice(使い捨ての公開.as_bytes());
    塩.extend_from_slice(受け手.as_bytes());
    let hk = Hkdf::<Sha256>::new(Some(&塩), 共有);
    let mut 鍵 = [0u8; 32];
    hk.expand(INFO, &mut 鍵).expect("32 byte は必ず出せる");
    鍵
}
