//! 宛先。**公開鍵と、そこへ届く経路の候補。**
//!
//! 割符と同じで、宛先も QR や貼り付けで人の手を渡る。
//! だから文字列は ASCII だけ・表記は 1 通りに固定する。

use core::fmt;
use core::str::FromStr;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

use warifu_core::{PublicKey, base32};

use crate::Error;

/// 割符の文字列と揃えた頭。中身の種別は目印で分ける。
const PREFIX: &str = "WARIFU1-";
/// 宛先の目印。割符（`WRF1`）と混ざらないようにする。
const MAGIC: &[u8; 4] = b"WRFA";
const KIND_ADDRESS: u8 = 0x03;
const TAG_V4: u8 = 4;
const TAG_V6: u8 = 6;

/// 相手に届くための宛先。
///
/// **公開鍵が名前そのもの。**IP は「今つながる場所」でしかなく、変わってよい。
#[derive(Clone, PartialEq, Eq)]
pub struct Address {
    key: PublicKey,
    ips: Vec<SocketAddr>,
}

impl Address {
    pub(crate) fn from_parts(key: PublicKey, ips: impl IntoIterator<Item = SocketAddr>) -> Self {
        let mut ips: Vec<SocketAddr> = ips.into_iter().collect();
        // 並びを 1 通りに決める。同じ宛先が別の文字列になると、突き合わせができない
        ips.sort_unstable();
        ips.dedup();
        Self { key, ips }
    }

    /// 相手の公開鍵。
    #[must_use]
    pub fn public_key(&self) -> PublicKey {
        self.key
    }

    /// 経路の候補。
    pub fn ip_addrs(&self) -> impl Iterator<Item = SocketAddr> + '_ {
        self.ips.iter().copied()
    }

    /// 公開鍵だけ差し替える。**経路の候補はそのまま。**
    ///
    /// 差し替えた宛先で繋ぐと必ず落ちる（経路の暗号が相手の鍵に紐付いているため）。
    /// それを確かめるために開けてある。
    #[must_use]
    pub fn with_public_key(mut self, key: PublicKey) -> Self {
        self.key = key;
        self
    }

    fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(5 + 32 + self.ips.len() * 19);
        out.extend_from_slice(MAGIC);
        out.push(KIND_ADDRESS);
        out.extend_from_slice(&self.key.to_bytes());
        for ip in &self.ips {
            match ip.ip() {
                IpAddr::V4(v4) => {
                    out.push(TAG_V4);
                    out.extend_from_slice(&v4.octets());
                }
                IpAddr::V6(v6) => {
                    out.push(TAG_V6);
                    out.extend_from_slice(&v6.octets());
                }
            }
            out.extend_from_slice(&ip.port().to_be_bytes());
        }
        out
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() < 5 + 32 || &bytes[..4] != MAGIC || bytes[4] != KIND_ADDRESS {
            return Err(Error::Malformed);
        }
        let mut key = [0u8; 32];
        key.copy_from_slice(&bytes[5..37]);
        let key = PublicKey::from_bytes(key)?;

        let mut ips = Vec::new();
        let mut at = 37;
        while at < bytes.len() {
            let (ip, size) = match bytes[at] {
                TAG_V4 => (IpAddr::V4(Ipv4Addr::from(take::<4>(bytes, at + 1)?)), 4),
                TAG_V6 => (IpAddr::V6(Ipv6Addr::from(take::<16>(bytes, at + 1)?)), 16),
                _ => return Err(Error::Malformed),
            };
            let port = u16::from_be_bytes(take::<2>(bytes, at + 1 + size)?);
            ips.push(SocketAddr::new(ip, port));
            at += 1 + size + 2;
        }

        Ok(Self::from_parts(key, ips))
    }
}

/// `bytes[from..]` から N byte を取り出す。足りなければ受け取らない。
fn take<const N: usize>(bytes: &[u8], from: usize) -> Result<[u8; N], Error> {
    bytes
        .get(from..from + N)
        .ok_or(Error::Malformed)?
        .try_into()
        .map_err(|_| Error::Malformed)
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{PREFIX}{}", base32::encode(&self.to_bytes()))
    }
}

impl fmt::Debug for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Address")
            .field("public_key", &self.key)
            .field("ip_addrs", &self.ips)
            .finish()
    }
}

impl FromStr for Address {
    type Err = Error;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let body = text.strip_prefix(PREFIX).ok_or(Error::Malformed)?;
        let bytes = base32::decode(body).ok_or(Error::Malformed)?;
        Self::from_bytes(&bytes)
    }
}
