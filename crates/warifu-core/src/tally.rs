//! 割符 — 二つに割った札。
//!
//! | | 誰が持つ | 何が入っているか |
//! |---|---|---|
//! | [`Tally`] | 差し出した側の手元 | 秘密・期限・使用済みかどうか |
//! | [`TallyToken`] | 相手に渡す | 秘密・差出人・期限・差出人の署名 |
//! | [`Acceptance`] | 受け取った側が返す | 割符の番号・自分の鍵・**秘密を知っている証** |
//!
//! [`TallyToken`] は **warifu の外**を通って相手に届く（QR を撮る・文字列を貼る）。
//! ここにネットワークは出てこない。**経路が無くても割符は成立する。**
//!
//! # 秘密そのものを送り返させない
//!
//! [`Acceptance`] に入るのは秘密ではなく、秘密から作った証だけ。
//! そのまま返させると、経路を覗いていた者が横から同じものを名乗れる。

use core::fmt;
use core::str::FromStr;

use sha2::{Digest as _, Sha512};
use zeroize::Zeroize as _;

use crate::base32;
use crate::error::Error;
use crate::key::{Device, PublicKey, Signature};
use crate::revocation::Revocations;

const MAGIC: &[u8; 4] = b"WRF1";
const KIND_TOKEN: u8 = 0x01;
const KIND_ACCEPTANCE: u8 = 0x02;

/// 目印 4 + 種別 1 + 差出人 32 + 秘密 32 + 期限 8 + 署名 64
const TOKEN_LEN: usize = 4 + 1 + 32 + 32 + 8 + 64;
/// 目印 4 + 種別 1 + 番号 32 + 応じた鍵 32 + 時刻 8 + 証 32 + 署名 64
const ACCEPTANCE_LEN: usize = 4 + 1 + 32 + 32 + 8 + 32 + 64;

const TEXT_PREFIX: &str = "WARIFU1-";

/// 割符の番号。秘密から一方向に決まるので、**番号から秘密は戻らない。**
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TallyId([u8; 32]);

impl TallyId {
    /// 生の 32 byte。
    #[must_use]
    pub fn to_bytes(self) -> [u8; 32] {
        self.0
    }

    pub(crate) fn from_raw(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

impl fmt::Display for TallyId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&base32::encode(&self.0))
    }
}

impl fmt::Debug for TallyId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TallyId({self})")
    }
}

fn digest(domain: &[u8], parts: &[&[u8]]) -> [u8; 32] {
    let mut hasher = Sha512::new();
    hasher.update(domain);
    for part in parts {
        hasher.update((part.len() as u64).to_be_bytes());
        hasher.update(part);
    }
    let full = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&full[..32]);
    out
}

fn tally_id(secret: &[u8; 32]) -> TallyId {
    TallyId(digest(b"warifu/v1/tally-id", &[secret]))
}

fn proof(secret: &[u8; 32], accepter: PublicKey, at: u64) -> [u8; 32] {
    digest(
        b"warifu/v1/tally-proof",
        &[secret, &accepter.to_bytes(), &at.to_be_bytes()],
    )
}

/// 中身が違っても同じ時間で終わる比較。
///
/// 早く抜けると、1 byte ずつ当てて証を作れてしまう。
fn same(a: &[u8; 32], b: &[u8; 32]) -> bool {
    let mut diff = 0u8;
    for i in 0..32 {
        diff |= a[i] ^ b[i];
    }
    diff == 0
}

/// 差し出した側の手元に残る半分。
#[derive(Clone)]
pub struct Tally {
    id: TallyId,
    secret: [u8; 32],
    issuer: PublicKey,
    not_after: u64,
    used_by: Option<PublicKey>,
}

impl Tally {
    /// 割符の番号。
    #[must_use]
    pub fn id(&self) -> TallyId {
        self.id
    }

    /// 差し出した端末の公開鍵。
    #[must_use]
    pub fn issuer(&self) -> PublicKey {
        self.issuer
    }

    /// 期限（Unix 秒）。この時刻**まで**有効。
    #[must_use]
    pub fn not_after(&self) -> u64 {
        self.not_after
    }

    /// すでに応じた相手がいれば、その鍵。
    #[must_use]
    pub fn used_by(&self) -> Option<PublicKey> {
        self.used_by
    }

    /// 返ってきた片割れが、この割符の相方かどうかを見る。
    ///
    /// 合えば相手が確定し、**その割符は使用済みになる。**
    ///
    /// # Errors
    /// - [`Error::Expired`] 期限が切れている
    /// - [`Error::AlreadyUsed`] すでに誰かが応じている
    /// - [`Error::WrongTally`] 別の割符に対する片割れ、または証が合わない
    /// - [`Error::Revoked`] 割符か相手の端末が失効している
    pub fn match_half(
        &mut self,
        acceptance: &Acceptance,
        now: u64,
        revocations: &Revocations,
    ) -> Result<Peer, Error> {
        if now > self.not_after {
            return Err(Error::Expired);
        }
        if self.used_by.is_some() {
            return Err(Error::AlreadyUsed);
        }
        if acceptance.tally != self.id {
            return Err(Error::WrongTally);
        }
        if revocations.is_revoked_tally(&self.id)
            || revocations.is_revoked_device(&acceptance.accepter)
        {
            return Err(Error::Revoked);
        }
        if !same(
            &acceptance.proof,
            &proof(&self.secret, acceptance.accepter, acceptance.at),
        ) {
            return Err(Error::WrongTally);
        }

        self.used_by = Some(acceptance.accepter);
        Ok(Peer {
            public_key: acceptance.accepter,
            tally: self.id,
            at: acceptance.at,
        })
    }
}

impl Drop for Tally {
    fn drop(&mut self) {
        self.secret.zeroize();
    }
}

impl fmt::Debug for Tally {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Tally")
            .field("id", &self.id)
            .field("secret", &"伏せ字")
            .field("issuer", &self.issuer)
            .field("not_after", &self.not_after)
            .field("used_by", &self.used_by)
            .finish()
    }
}

/// 相手に渡す半分。**これを見られた時点で、その割符は他人が使える。**
#[derive(Clone)]
pub struct TallyToken {
    issuer: PublicKey,
    secret: [u8; 32],
    not_after: u64,
    signature: Signature,
}

impl TallyToken {
    /// 差し出した端末の公開鍵。**署名済みなので、途中で差し替えられない。**
    #[must_use]
    pub fn issuer(&self) -> PublicKey {
        self.issuer
    }

    /// この割符の番号。
    #[must_use]
    pub fn id(&self) -> TallyId {
        tally_id(&self.secret)
    }

    /// 期限（Unix 秒）。
    #[must_use]
    pub fn not_after(&self) -> u64 {
        self.not_after
    }

    /// 渡すためのバイト列。
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = self.signed_part();
        out.extend_from_slice(&self.signature.to_bytes());
        out
    }

    fn signed_part(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(TOKEN_LEN);
        out.extend_from_slice(MAGIC);
        out.push(KIND_TOKEN);
        out.extend_from_slice(&self.issuer.to_bytes());
        out.extend_from_slice(&self.secret);
        out.extend_from_slice(&self.not_after.to_be_bytes());
        out
    }

    /// バイト列から読む。**署名が合わなければ受け取らない。**
    ///
    /// # Errors
    /// - [`Error::Malformed`] 長さ・目印・種別・鍵の形が合わない
    /// - [`Error::BadSignature`] 中身が書き換わっている、または差出人が違う
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() != TOKEN_LEN || &bytes[..4] != MAGIC || bytes[4] != KIND_TOKEN {
            return Err(Error::Malformed);
        }

        let issuer = PublicKey::from_bytes(take32(bytes, 5))?;
        let secret = take32(bytes, 37);
        let not_after = u64::from_be_bytes(bytes[69..77].try_into().map_err(|_| Error::Malformed)?);
        let signature =
            Signature::from_bytes(bytes[77..].try_into().map_err(|_| Error::Malformed)?);

        issuer.verify(&bytes[..77], &signature)?;

        Ok(Self {
            issuer,
            secret,
            not_after,
            signature,
        })
    }
}

impl fmt::Display for TallyToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{TEXT_PREFIX}{}", base32::encode(&self.to_bytes()))
    }
}

impl FromStr for TallyToken {
    type Err = Error;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let body = text.strip_prefix(TEXT_PREFIX).ok_or(Error::Malformed)?;
        Self::from_bytes(&base32::decode(body).ok_or(Error::Malformed)?)
    }
}

impl Drop for TallyToken {
    fn drop(&mut self) {
        self.secret.zeroize();
    }
}

impl fmt::Debug for TallyToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TallyToken")
            .field("issuer", &self.issuer)
            .field("secret", &"伏せ字")
            .field("not_after", &self.not_after)
            .finish()
    }
}

/// 受け取った側が返す片割れ。
#[derive(Clone)]
pub struct Acceptance {
    tally: TallyId,
    accepter: PublicKey,
    at: u64,
    proof: [u8; 32],
    signature: Signature,
}

impl Acceptance {
    /// どの割符に応じたか。
    #[must_use]
    pub fn tally(&self) -> TallyId {
        self.tally
    }

    /// 応じた端末の公開鍵。
    #[must_use]
    pub fn accepter(&self) -> PublicKey {
        self.accepter
    }

    /// 応じた時刻（Unix 秒）。**相手が申告した時刻であって、信用しない。**
    #[must_use]
    pub fn at(&self) -> u64 {
        self.at
    }

    /// 返すためのバイト列。
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = self.signed_part();
        out.extend_from_slice(&self.signature.to_bytes());
        out
    }

    fn signed_part(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(ACCEPTANCE_LEN);
        out.extend_from_slice(MAGIC);
        out.push(KIND_ACCEPTANCE);
        out.extend_from_slice(&self.tally.to_bytes());
        out.extend_from_slice(&self.accepter.to_bytes());
        out.extend_from_slice(&self.at.to_be_bytes());
        out.extend_from_slice(&self.proof);
        out
    }

    /// バイト列から読む。**署名が合わなければ受け取らない。**
    ///
    /// # Errors
    /// - [`Error::Malformed`] 長さ・目印・種別・鍵の形が合わない
    /// - [`Error::BadSignature`] 中身が書き換わっている
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() != ACCEPTANCE_LEN || &bytes[..4] != MAGIC || bytes[4] != KIND_ACCEPTANCE {
            return Err(Error::Malformed);
        }

        let tally = TallyId(take32(bytes, 5));
        let accepter = PublicKey::from_bytes(take32(bytes, 37))?;
        let at = u64::from_be_bytes(bytes[69..77].try_into().map_err(|_| Error::Malformed)?);
        let proof = take32(bytes, 77);
        let signature =
            Signature::from_bytes(bytes[109..].try_into().map_err(|_| Error::Malformed)?);

        accepter.verify(&bytes[..109], &signature)?;

        Ok(Self {
            tally,
            accepter,
            at,
            proof,
            signature,
        })
    }
}

impl fmt::Display for Acceptance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{TEXT_PREFIX}{}", base32::encode(&self.to_bytes()))
    }
}

impl FromStr for Acceptance {
    type Err = Error;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let body = text.strip_prefix(TEXT_PREFIX).ok_or(Error::Malformed)?;
        Self::from_bytes(&base32::decode(body).ok_or(Error::Malformed)?)
    }
}

impl fmt::Debug for Acceptance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Acceptance")
            .field("tally", &self.tally)
            .field("accepter", &self.accepter)
            .field("at", &self.at)
            .finish()
    }
}

/// 片割れが合った相手。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Peer {
    public_key: PublicKey,
    tally: TallyId,
    at: u64,
}

impl Peer {
    /// 相手の端末の公開鍵。
    #[must_use]
    pub fn public_key(&self) -> PublicKey {
        self.public_key
    }

    /// どの割符で結び付いたか。**あとで「誰に配った札か」を辿るために残す。**
    #[must_use]
    pub fn tally(&self) -> TallyId {
        self.tally
    }

    /// 相手が応じたと申告した時刻（Unix 秒）。
    #[must_use]
    pub fn accepted_at(&self) -> u64 {
        self.at
    }
}

fn take32(bytes: &[u8], from: usize) -> [u8; 32] {
    let mut out = [0u8; 32];
    out.copy_from_slice(&bytes[from..from + 32]);
    out
}

impl Device {
    /// 割符を作る。手元に残す半分と、相手に渡す半分が出る。
    ///
    /// `ttl` は有効期間（秒）。期限は `now + ttl` で、**その時刻まで**有効。
    ///
    /// # Errors
    /// OS の乱数が取れないとき [`Error::Rng`]。
    pub fn issue_tally(&self, now: u64, ttl: u64) -> Result<(Tally, TallyToken), Error> {
        let mut secret = [0u8; 32];
        getrandom::fill(&mut secret).map_err(|_| Error::Rng)?;

        let not_after = now.saturating_add(ttl);
        let issuer = self.public_key();

        let token = {
            let unsigned = TallyToken {
                issuer,
                secret,
                not_after,
                signature: Signature::from_bytes([0u8; 64]),
            };
            let signature = self.sign(&unsigned.signed_part());
            TallyToken {
                issuer,
                secret,
                not_after,
                signature,
            }
        };

        let tally = Tally {
            id: tally_id(&secret),
            secret,
            issuer,
            not_after,
            used_by: None,
        };

        secret.zeroize();
        Ok((tally, token))
    }

    /// 受け取った割符に応じる。**期限と差出人の署名を見てから作る。**
    ///
    /// 署名の検証は [`TallyToken::from_bytes`] で済んでいるので、ここでは期限だけを見る。
    ///
    /// # Errors
    /// [`Error::Expired`] 期限が切れている。
    pub fn accept(&self, token: &TallyToken, now: u64) -> Result<Acceptance, Error> {
        if now > token.not_after {
            return Err(Error::Expired);
        }

        let accepter = self.public_key();
        let unsigned = Acceptance {
            tally: token.id(),
            accepter,
            at: now,
            proof: proof(&token.secret, accepter, now),
            signature: Signature::from_bytes([0u8; 64]),
        };
        let signature = self.sign(&unsigned.signed_part());

        Ok(Acceptance {
            signature,
            ..unsigned
        })
    }
}
