//! 失効の名簿。
//!
//! 中央 Directory を作らないと決めた以上、失効は**各自が手元に持つ名簿**でしかありえない。
//! 「世界中で失効した」ことは保証しない。保証するのは「**この端末はもう通さない**」だけ。
//!
//! # 取り消せない
//!
//! 一度載せたものは降ろせない。降ろせると、鍵を盗った側が降ろせてしまう。
//! 戻したいときは新しい鍵を作る（＝新しい割符を配り直す）。

use std::collections::BTreeMap;

use crate::key::PublicKey;
use crate::tally::TallyId;

/// 失効させた端末と割符の一覧。
#[derive(Clone, Default, Debug)]
pub struct Revocations {
    devices: BTreeMap<[u8; 32], u64>,
    tallies: BTreeMap<[u8; 32], u64>,
}

impl Revocations {
    /// 空の名簿。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 端末を失効させる。`at` は失効させた時刻（Unix 秒）。
    ///
    /// **二度目以降の時刻は捨てる。**上書きすると、最初に失くした時刻が消えて、
    /// その前後どちらの通信を疑うかが決められなくなる。
    pub fn revoke_device(&mut self, key: PublicKey, at: u64) {
        self.devices.entry(key.to_bytes()).or_insert(at);
    }

    /// 割符を失効させる。`at` は失効させた時刻（Unix 秒）。
    pub fn revoke_tally(&mut self, id: TallyId, at: u64) {
        self.tallies.entry(id.to_bytes()).or_insert(at);
    }

    /// その端末は失効しているか。
    #[must_use]
    pub fn is_revoked_device(&self, key: &PublicKey) -> bool {
        self.devices.contains_key(&key.to_bytes())
    }

    /// その割符は失効しているか。
    #[must_use]
    pub fn is_revoked_tally(&self, id: &TallyId) -> bool {
        self.tallies.contains_key(&id.to_bytes())
    }

    /// 失効させた端末と、その時刻。
    pub fn devices(&self) -> impl Iterator<Item = (PublicKey, u64)> + '_ {
        self.devices
            .iter()
            .map(|(k, at)| (PublicKey::from_raw(*k), *at))
    }

    /// 失効させた割符と、その時刻。
    pub fn tallies(&self) -> impl Iterator<Item = (TallyId, u64)> + '_ {
        self.tallies
            .iter()
            .map(|(k, at)| (TallyId::from_raw(*k), *at))
    }
}
