//! **預かり所を通った言葉は、誰が言ったかを署名で確かめる。**
//!
//! 直に繋がっている間は、相手が誰かは経路が確かめている（`warifu-net`）。
//! **預かり所を通ると、それが無い。**封の中で名乗るだけなら、
//! **誰でも「覚えている相手」の名で言葉を置ける。**

use warifu_core::{Device, Seed};
use warifu_postbox::{手紙にする, 手紙を読む};

fn 人(seed: u8) -> Device {
    Seed::from_bytes([seed; 32])
        .profile("Personal")
        .device("PC")
}

#[test]
fn 誰が言ったかが残る() {
    let 出す = 人(1);
    let 受ける = 人(2);
    let 手紙 = 手紙にする(
        &出す,
        受ける.public_key(),
        1_700_000_000,
        "こんばんは".as_bytes(),
    );
    let 読めた = 手紙を読む(受ける.public_key(), &手紙).expect("読める");
    assert_eq!(読めた.差出人, 出す.public_key());
    assert_eq!(読めた.時刻, 1_700_000_000);
    assert_eq!(読めた.本文, "こんばんは".as_bytes());
}

#[test]
fn 名乗りだけでは通らない() {
    // **差出人の欄を書き換えても、署名が合わない**
    let 出す = 人(1);
    let 受ける = 人(2);
    let 偽る = 人(3);
    let mut 手紙 = 手紙にする(&偽る, 受ける.public_key(), 100, "金を払って".as_bytes());
    手紙[..32].copy_from_slice(&出す.public_key().to_bytes());
    assert!(手紙を読む(受ける.public_key(), &手紙).is_none());
}

#[test]
fn 本文を書き換えたら読めない() {
    let 出す = 人(1);
    let 受ける = 人(2);
    let mut 手紙 = 手紙にする(
        &出す,
        受ける.public_key(),
        100,
        "ふりこみ 1 万円".as_bytes(),
    );
    let 尻 = 手紙.len() - 1;
    手紙[尻] ^= 0x01;
    assert!(手紙を読む(受ける.public_key(), &手紙).is_none());
}

#[test]
fn 別の人あてに付け替えられない() {
    // 署名は**宛先も含めて**掛かる。横から取って別人へ置き直せない
    let 出す = 人(1);
    let 受ける = 人(2);
    let 別の人 = 人(3);
    let 手紙 = 手紙にする(&出す, 受ける.public_key(), 100, "こんばんは".as_bytes());
    assert!(手紙を読む(別の人.public_key(), &手紙).is_none());
}

#[test]
fn 短すぎる塊は読まない() {
    let 受ける = 人(2);
    assert!(手紙を読む(受ける.public_key(), &[]).is_none());
    assert!(手紙を読む(受ける.public_key(), &[0; 103]).is_none());
}

#[test]
fn 本文が空でも成り立つ() {
    let 出す = 人(1);
    let 受ける = 人(2);
    let 手紙 = 手紙にする(&出す, 受ける.public_key(), 100, b"");
    let 読めた = 手紙を読む(受ける.public_key(), &手紙).expect("読める");
    assert!(読めた.本文.is_empty());
}
