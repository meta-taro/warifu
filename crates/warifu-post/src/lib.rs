//! 預かり所。**相手が起動していない間、封を預かる。**
//!
//! これが LINE との本当の差だった —— LINE は中央のサーバが預かるから、
//! 相手が寝ていても届く。割符は**サーバーを 1 台も立てない**ので、
//! 2026-09-08 まで**相手が起動していなければ消えていた。**
//!
//! # ここで守ること
//!
//! - **中身を読まない。**封のまま持つ（**D69**）。開ける鍵を持っていない
//! - **溜め込む場所にしない。**渡したら手放す。古いものは捨てる
//! - **無限に置ける所を作らない。**置くだけで機械を埋められる
//! - **並び替えない。**話の順が変わると読めなくなる
//!
//! **預かり所は「割符が用意する中央」ではない。**
//! **導入した人が自分で立てるもの**である（**D68**）——
//! 割符が中央を持った瞬間、配る側が説明責任を負い、
//! 「どのクラウドにもアカウントを作らない」（PRD）とも逆を向く。

#![forbid(unsafe_code)]

mod wire;

use std::collections::HashMap;

use warifu_core::PublicKey;

pub use wire::{Ask, Reply};

/// 何日まで預かるか。
///
/// **相手が次に起動するまで**が預かる理由である。
/// 長く持つほど、**預かり所が「そこに全部ある場所」になっていく。**
pub const 預かれる日数: u64 = 7;

/// 1 人あたり、いくつまで預かるか。
///
/// **無限に置ける所を作らない。**置くだけで機械を埋められる。
pub const 預かる上限: usize = 200;

/// 1 通の大きさの上限（バイト）。
pub const 一通の上限: usize = 64 * 1024;

/// 預けられなかった理由。
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// その人の分がいっぱい。
    Full,
    /// 1 通が大きすぎる。
    TooLarge,
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Full => write!(f, "預かれる数を超えています（1 人 {預かる上限} 通まで）"),
            Self::TooLarge => write!(f, "1 通が大きすぎます（{一通の上限} バイトまで）"),
        }
    }
}

impl core::error::Error for Error {}

/// 預かっている 1 通。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    sealed: Vec<u8>,
    at: u64,
}

impl Item {
    /// 封そのもの。**預かり所は開けられない。**
    #[must_use]
    pub fn sealed(&self) -> &[u8] {
        &self.sealed
    }

    /// 預かった時刻（Unix 秒）。
    #[must_use]
    pub fn at(&self) -> u64 {
        self.at
    }
}

/// 預かり所。
#[derive(Debug, Default)]
pub struct Box {
    /// 宛先ごとの預かり物。**鍵はそのまま宛先の公開鍵。**
    棚: HashMap<[u8; 32], Vec<Item>>,
}

impl Box {
    /// 何も預かっていない所。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// いま預かっている総数。
    #[must_use]
    pub fn len(&self) -> usize {
        self.棚.values().map(Vec::len).sum()
    }

    /// 何も預かっていないか。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// 預かる。
    ///
    /// # Errors
    /// その人の分がいっぱいなら [`Error::Full`]、大きすぎるなら [`Error::TooLarge`]。
    pub fn put(&mut self, to: PublicKey, sealed: Vec<u8>, now: u64) -> Result<(), Error> {
        if sealed.len() > 一通の上限 {
            return Err(Error::TooLarge);
        }
        let 棚 = self.棚.entry(to.to_bytes()).or_default();
        // **1 人が埋めても、他の人が使えなくならない。**上限は 1 人ごとに掛ける
        if 棚.len() >= 預かる上限 {
            return Err(Error::Full);
        }
        棚.push(Item { sealed, at: now });
        Ok(())
    }

    /// その人あてを、**まとめて渡して手放す。**
    ///
    /// **渡したら消える。**溜め込む場所にしない。
    /// 古いものは渡さない（[`預かれる日数`]）。
    pub fn take(&mut self, to: PublicKey, now: u64) -> Vec<Item> {
        let Some(棚) = self.棚.remove(&to.to_bytes()) else {
            return Vec::new();
        };
        // **並び替えない。**話の順が変わると読めなくなる
        棚.into_iter().filter(|i| !古い(i, now)).collect()
    }

    /// 古いものを片付ける。**受け取りに来なくても溜まらない。**
    pub fn forget_old(&mut self, now: u64) {
        for 棚 in self.棚.values_mut() {
            棚.retain(|i| !古い(i, now));
        }
        self.棚.retain(|_, 棚| !棚.is_empty());
    }
}

fn 古い(i: &Item, now: u64) -> bool {
    now.saturating_sub(i.at) > 預かれる日数 * 24 * 60 * 60
}
