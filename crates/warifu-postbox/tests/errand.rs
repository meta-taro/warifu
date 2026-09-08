//! 預かり所とのやりとり —— **預ける・受け取る・応じる。**

use warifu_core::{Device, PublicKey, Seed};
use warifu_post::{Ask, Box as 預かり所, Reply};
use warifu_postbox::{応える, 頼みを聞ける形か};

fn 人(seed: [u8; 32]) -> Device {
    Seed::from_bytes(seed).profile("Personal").device("PC")
}

fn 鍵(d: &Device) -> PublicKey {
    d.public_key()
}

const 今: u64 = 1_000_000;

#[test]
fn 預けられたら_預かったと答える() {
    let mut 箱 = 預かり所::new();
    let 宛 = 鍵(&人([2u8; 32]));
    let 返 = 応える(
        &mut 箱,
        鍵(&人([1u8; 32])),
        Ask::Put {
            to: 宛,
            sealed: vec![9; 32],
        },
        今,
    );
    assert_eq!(返, Reply::Kept);
    assert_eq!(箱.len(), 1);
}

#[test]
fn 受け取りに来たら_その人あてだけを渡す() {
    let mut 箱 = 預かり所::new();
    let 私 = 鍵(&人([2u8; 32]));
    let 他人 = 鍵(&人([3u8; 32]));
    箱.put(私, vec![1; 8], 今).unwrap();
    箱.put(他人, vec![2; 8], 今).unwrap();

    let 返 = 応える(&mut 箱, 私, Ask::Take, 今);
    assert_eq!(返, Reply::Handed(vec![vec![1; 8]]));
    // **渡したら手放す。**他人あては残る
    assert_eq!(箱.len(), 1);
}

#[test]
fn 尋ねる側は宛先を名乗れない() {
    // `Ask::Take` は宛先を持たない。**経路で確定した相手あてだけ**を渡すため
    let mut 箱 = 預かり所::new();
    let 私 = 鍵(&人([2u8; 32]));
    箱.put(鍵(&人([3u8; 32])), vec![7; 8], 今).unwrap();
    assert_eq!(応える(&mut 箱, 私, Ask::Take, 今), Reply::Handed(vec![]));
    assert_eq!(箱.len(), 1);
}

#[test]
fn 古い封は渡す前に捨てる() {
    let mut 箱 = 預かり所::new();
    let 私 = 鍵(&人([2u8; 32]));
    箱.put(私, vec![1; 8], 今).unwrap();
    let ずっと後 = 今 + warifu_post::預かれる日数 * 86_400 + 1;
    assert_eq!(
        応える(&mut 箱, 私, Ask::Take, ずっと後),
        Reply::Handed(vec![])
    );
    assert_eq!(箱.len(), 0);
}

#[test]
fn 入りきらない封は_理由を言わずに断る() {
    let mut 箱 = 預かり所::new();
    let 宛 = 鍵(&人([2u8; 32]));
    let 返 = 応える(
        &mut 箱,
        宛,
        Ask::Put {
            to: 宛,
            sealed: vec![0; warifu_post::一通の上限 + 1],
        },
        今,
    );
    // **なぜ断ったかを言わない。**総当たりの手がかりになる
    assert_eq!(返, Reply::Refused);
}

#[test]
fn 読めない塊は_頼みとして受け取らない() {
    assert!(頼みを聞ける形か(b"WRFP\x01").is_none());
    assert!(頼みを聞ける形か(b"").is_none());
    assert_eq!(頼みを聞ける形か(&Ask::Take.to_bytes()), Some(Ask::Take));
}
