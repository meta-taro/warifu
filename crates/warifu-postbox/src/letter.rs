//! **預かり所を通った言葉に、誰が言ったかを残す。**
//!
//! 直に繋がっている間は、相手が誰かを**経路が確かめている**（`warifu-net`）。
//! **預かり所を通ると、それが無い。**封の中で名乗るだけなら、
//! **誰でも「覚えている相手」の名で言葉を置ける** ——
//! 預かり所は差出人を見ないので（見ないほうが良いので）、そこでも止まらない。
//!
//! ```text
//!   差出人(32) ‖ 署名(64) ‖ 時刻(8) ‖ 本文
//!   署名の対象 = "WRFL1" ‖ 宛先(32) ‖ 時刻(8) ‖ 本文
//! ```
//!
//! **宛先も署名に入れる。**入れないと、横から取って別の人あてに置き直せる。
//! **頭の目印を入れる。**入れないと、この署名を別の所の署名として使い回せる。

use warifu_core::{Device, PublicKey, Signature};

/// 何の署名かを分ける目印。**他の所の署名と取り違えないため。**
const 目印: &[u8] = b"WRFL1";

/// 差出人(32) ＋ 署名(64) ＋ 時刻(8)。
const 頭の長さ: usize = 32 + 64 + 8;

/// 預かり所を通ってきた 1 通。**署名が合ったものだけがこの形になる。**
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct 手紙 {
    /// 誰が言ったか。**署名で確かめたもの**であって、名乗りではない。
    pub 差出人: PublicKey,
    /// 出した側の時計（Unix 秒）。**受け取った側の時計ではない。**
    pub 時刻: u64,
    /// 中身。**解釈しない。**
    pub 本文: Vec<u8>,
}

/// 署名の対象を組み立てる。
fn 署名の対象(宛先: PublicKey, 時刻: u64, 本文: &[u8]) -> Vec<u8> {
    let mut 対象 = Vec::with_capacity(目印.len() + 32 + 8 + 本文.len());
    対象.extend_from_slice(目印);
    対象.extend_from_slice(&宛先.to_bytes());
    対象.extend_from_slice(&時刻.to_be_bytes());
    対象.extend_from_slice(本文);
    対象
}

/// **1 通に組む。**この中身に封をして預ける。
#[must_use]
pub fn 手紙にする(
    差出人: &Device, 宛先: PublicKey, 時刻: u64, 本文: &[u8]
) -> Vec<u8> {
    let 署名 = 差出人.sign(&署名の対象(宛先, 時刻, 本文));
    let mut 塊 = Vec::with_capacity(頭の長さ + 本文.len());
    塊.extend_from_slice(&差出人.public_key().to_bytes());
    塊.extend_from_slice(&署名.to_bytes());
    塊.extend_from_slice(&時刻.to_be_bytes());
    塊.extend_from_slice(本文);
    塊
}

/// **開けたあとの中身を、1 通として読む。**
///
/// 署名が合わなければ [`None`]。**なぜ合わなかったかは分けない**
/// （分けると、当てずっぽうの手がかりになる）。
#[must_use]
pub fn 手紙を読む(私: PublicKey, 中身: &[u8]) -> Option<手紙> {
    if 中身.len() < 頭の長さ {
        return None;
    }
    let 差出人 = PublicKey::from_bytes(中身[..32].try_into().ok()?).ok()?;
    let 署名 = Signature::from_bytes(中身[32..96].try_into().ok()?);
    let 時刻 = u64::from_be_bytes(中身[96..頭の長さ].try_into().ok()?);
    let 本文 = &中身[頭の長さ..];
    差出人
        .verify(&署名の対象(私, 時刻, 本文), &署名)
        .ok()
        .map(|()| 手紙 {
            差出人,
            時刻,
            本文: 本文.to_vec(),
        })
}
