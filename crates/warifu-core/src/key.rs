//! シード → Profile → Device の鍵。
//!
//! ```text
//!   シード（32 byte）
//!     ├─ Profile 鍵（Personal / Work …）
//!     │    ├─ Device 鍵（PC）
//!     │    └─ Device 鍵（スマホ）
//!     └─ …
//! ```
//!
//! **すべて決定的に導く。**同じシードからは、いつどの端末で計算しても同じ鍵が出る。
//! これが `decisions.md` **D2**（鍵の復旧方式が未決）を実装のブロッカーから
//! 外している根拠 — 復旧方式が何になっても、シードさえ戻れば形は変わらない。

use core::fmt;
use core::str::FromStr;

use ed25519_dalek::{Signer as _, SigningKey, Verifier as _, VerifyingKey};
use hkdf::Hkdf;
use sha2::Sha512;
use zeroize::Zeroize as _;

use crate::base32;
use crate::error::Error;

/// 導出の入口を分けるための固定文字列。
const SALT: &[u8] = b"warifu/v1";

const DOMAIN_PROFILE_SECRET: &[u8] = b"profile-secret";
const DOMAIN_PROFILE_KEY: &[u8] = b"profile-key";
const DOMAIN_DEVICE_KEY: &[u8] = b"device-key";

/// ラベルを長さ付きで混ぜて 32 byte を導く。
///
/// 長さを付けないと `"a" + "b"` と `"ab" + ""` が同じ入力になり、
/// **ラベルを細工して他人の鍵を作れてしまう。**
fn derive(ikm: &[u8; 32], domain: &[u8], label: &str) -> [u8; 32] {
    let hk = Hkdf::<Sha512>::new(Some(SALT), ikm);

    let mut info = Vec::with_capacity(domain.len() + 8 + label.len());
    info.extend_from_slice(domain);
    info.extend_from_slice(&(label.len() as u64).to_be_bytes());
    info.extend_from_slice(label.as_bytes());

    let mut out = [0u8; 32];
    hk.expand(&info, &mut out)
        .expect("32 byte は HKDF-SHA512 の出力上限に収まる");
    info.zeroize();
    out
}

/// すべての鍵の根。**これを失うと全部を失う。**
///
/// 復旧方式（紙に書く / Social Recovery / 端末バックアップ）は
/// `decisions.md` **D2** で未決。ここでは「32 byte が戻れば全部戻る」形だけを固定する。
pub struct Seed([u8; 32]);

impl Seed {
    /// 乱数からシードを作る。
    ///
    /// # Errors
    /// OS の乱数が取れないとき [`Error::Rng`]。
    pub fn generate() -> Result<Self, Error> {
        let mut bytes = [0u8; 32];
        getrandom::fill(&mut bytes).map_err(|_| Error::Rng)?;
        Ok(Self(bytes))
    }

    /// 既にある 32 byte からシードを組み立てる（復旧・テスト用）。
    #[must_use]
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// シードの**中身そのもの**を 32 byte で取り出す。
    ///
    /// **この 32 byte が身元のすべてである。**戻れば全部戻り、漏れれば全部漏れる。
    ///
    /// 開けてあるのは、**この端末に置いておく**（`warifu-vault`）ためと、
    /// 復旧の口（復旧フレーズ・分割・預け先）が、どれもこの 32 byte を対象にするため。
    /// D2 がどの方式を既定に選んでも、**扱う対象はここで変わらない。**
    ///
    /// 呼ぶ側は、使い終わったら [`zeroize`](https://docs.rs/zeroize) で消すこと。
    /// **ログ・画面・通信に出さない。**
    #[must_use]
    pub fn to_bytes(&self) -> [u8; 32] {
        self.0
    }

    /// Profile を導く。`label` は `"Personal"` `"Work"` のような呼び名。
    #[must_use]
    pub fn profile(&self, label: &str) -> Profile {
        Profile {
            secret: derive(&self.0, DOMAIN_PROFILE_SECRET, label),
            label: label.to_owned(),
        }
    }
}

impl Drop for Seed {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

impl fmt::Debug for Seed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Seed(伏せ字)")
    }
}

/// 使い分けの単位。**Personal と Work は互いに結び付かない。**
///
/// 相手から見て同一人物だと分からないことが要件なので、
/// Profile が違えば配下の Device 鍵まで無関係になる。
pub struct Profile {
    secret: [u8; 32],
    label: String,
}

impl Profile {
    /// 呼び名。
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }

    /// この Profile 自身の公開鍵。**Device 鍵とは別物。**
    ///
    /// 端末を 1 台失っても Profile ごと失わないために分けてある。
    #[must_use]
    pub fn public_key(&self) -> PublicKey {
        PublicKey::from(&signing_key(&self.secret, DOMAIN_PROFILE_KEY, ""))
    }

    /// 端末の鍵を導く。`label` は `"PC"` `"スマホ"` のような呼び名。
    #[must_use]
    pub fn device(&self, label: &str) -> Device {
        Device {
            signing: signing_key(&self.secret, DOMAIN_DEVICE_KEY, label),
            label: label.to_owned(),
        }
    }
}

impl Drop for Profile {
    fn drop(&mut self) {
        self.secret.zeroize();
    }
}

impl fmt::Debug for Profile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Profile")
            .field("label", &self.label)
            .field("public_key", &self.public_key())
            .finish()
    }
}

fn signing_key(secret: &[u8; 32], domain: &[u8], label: &str) -> SigningKey {
    let mut material = derive(secret, domain, label);
    let key = SigningKey::from_bytes(&material);
    material.zeroize();
    key
}

/// 1 台の端末。**署名できるのはここだけ。**
#[derive(Clone)]
pub struct Device {
    signing: SigningKey,
    label: String,
}

impl Device {
    /// 呼び名。
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }

    /// この端末の公開鍵。相手に見せてよい。
    #[must_use]
    pub fn public_key(&self) -> PublicKey {
        PublicKey::from(&self.signing)
    }

    /// この端末の**秘密鍵そのもの**を 32 byte で取り出す。
    ///
    /// 経路（`warifu-net`）が QUIC の鍵として同じ値を使うために開けてある。
    /// 別の鍵で繋ぐと、**割符で確定した相手と、実際に繋がった相手が別物になる。**
    ///
    /// 呼ぶ側は、使い終わったら [`zeroize`](https://docs.rs/zeroize) で消すこと。
    /// **保存・表示・送信をしない。**
    #[must_use]
    pub fn secret_key_bytes(&self) -> [u8; 32] {
        self.signing.to_bytes()
    }

    /// 署名する。
    #[must_use]
    pub fn sign(&self, message: &[u8]) -> Signature {
        Signature(self.signing.sign(message))
    }
}

impl fmt::Debug for Device {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Device")
            .field("label", &self.label)
            .field("public_key", &self.public_key())
            .finish()
    }
}

/// 公開鍵。**warifu ではこれが相手の名前そのもの。**
///
/// 文字列にすると base32（大文字 52 文字）。
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PublicKey([u8; 32]);

impl PublicKey {
    /// 生の 32 byte。
    #[must_use]
    pub fn to_bytes(self) -> [u8; 32] {
        self.0
    }

    /// 32 byte から組み立てる。曲線上の点として不正なら受け取らない。
    ///
    /// # Errors
    /// 点として不正なら [`Error::Malformed`]。
    pub fn from_bytes(bytes: [u8; 32]) -> Result<Self, Error> {
        VerifyingKey::from_bytes(&bytes).map_err(|_| Error::Malformed)?;
        Ok(Self(bytes))
    }

    /// 検証済みの 32 byte から組み立て直す。
    ///
    /// 名簿に載せた時点で形は見ているので、取り出すたびに見直さない。
    pub(crate) fn from_raw(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// 署名を検証する。
    ///
    /// # Errors
    /// 合わなければ [`Error::BadSignature`]。
    pub fn verify(&self, message: &[u8], signature: &Signature) -> Result<(), Error> {
        VerifyingKey::from_bytes(&self.0)
            .map_err(|_| Error::BadSignature)?
            .verify(message, &signature.0)
            .map_err(|_| Error::BadSignature)
    }
}

impl From<&SigningKey> for PublicKey {
    fn from(signing: &SigningKey) -> Self {
        Self(signing.verifying_key().to_bytes())
    }
}

impl fmt::Display for PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&base32::encode(&self.0))
    }
}

impl fmt::Debug for PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PublicKey({self})")
    }
}

impl FromStr for PublicKey {
    type Err = Error;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let bytes = base32::decode(text).ok_or(Error::Malformed)?;
        let bytes: [u8; 32] = bytes.try_into().map_err(|_| Error::Malformed)?;
        Self::from_bytes(bytes)
    }
}

/// Ed25519 の署名。
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Signature(ed25519_dalek::Signature);

impl Signature {
    /// 生の 64 byte。
    #[must_use]
    pub fn to_bytes(self) -> [u8; 64] {
        self.0.to_bytes()
    }

    /// 64 byte から組み立てる。
    #[must_use]
    pub fn from_bytes(bytes: [u8; 64]) -> Self {
        Self(ed25519_dalek::Signature::from_bytes(&bytes))
    }
}

impl fmt::Debug for Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Signature({})", base32::encode(&self.0.to_bytes()))
    }
}
