//! **中継は中身を読めない。**
//!
//! `PRD` も `issues/004` も「中継者は中身を読めない」と書いているが、
//! 2026-09-08 まで**そう作られていなかった。**
//! いまの E2EE は経路（iroh の QUIC/TLS）だけで、
//! **相手と直接繋がっている間しか端から端まで暗号ではない。**
//! 中継を挟めば、中継がそこで平文を見る。
//!
//! だから封をする。**中継は運ぶだけで、開けられない。**

use warifu_core::{Device, Seed};
use warifu_seal::{Sealed, open, seal};

fn 端末(seed: [u8; 32]) -> Device {
    Seed::from_bytes(seed).profile("Personal").device("PC")
}

#[test]
fn 相手だけが開けられる() {
    let 送り手 = 端末([1u8; 32]);
    let 受け手 = 端末([2u8; 32]);

    let 封 = seal(受け手.public_key(), "会議の中身".as_bytes()).unwrap();
    let 中身 = open(&受け手, &封).unwrap();

    assert_eq!(中身, "会議の中身".as_bytes());
    let _ = 送り手;
}

#[test]
fn 別人は開けられない() {
    // **中継が持っていても開けられない。**これがこの層の目的
    let 受け手 = 端末([2u8; 32]);
    let 中継 = 端末([9u8; 32]);

    let 封 = seal(受け手.public_key(), "会議の中身".as_bytes()).unwrap();

    assert!(open(&中継, &封).is_err());
}

#[test]
fn 封から中身が透けない() {
    // **平文がそのまま入っていない**
    let 受け手 = 端末([2u8; 32]);
    let 平文 = b"SHIRUSHI-honbun-9f3a";

    let 封 = seal(受け手.public_key(), 平文).unwrap();
    let 塊 = 封.to_bytes();

    assert!(
        !塊.windows(平文.len()).any(|w| w == 平文),
        "平文が封に残っている"
    );
}

#[test]
fn 封から差出人が分からない() {
    // **使い捨ての鍵で封をする。**誰が言ったかは中身の `from` が持つ（D48）——
    // 封の外に出すと、**中継が「誰が誰に送ったか」を読める**
    let 送り手 = 端末([1u8; 32]);
    let 受け手 = 端末([2u8; 32]);

    let 封 = seal(受け手.public_key(), b"x").unwrap();
    let 塊 = 封.to_bytes();

    let 差出人 = 送り手.public_key().to_bytes();
    assert!(
        !塊.windows(32).any(|w| w == 差出人),
        "差出人の鍵が封に入っている"
    );
}

#[test]
fn 同じ中身でも_毎回ちがう封になる() {
    // **同じ封が並ぶと、同じことを言ったと中継に分かる**
    let 受け手 = 端末([2u8; 32]);

    let 一 = seal(受け手.public_key(), "はい".as_bytes())
        .unwrap()
        .to_bytes();
    let 二 = seal(受け手.public_key(), "はい".as_bytes())
        .unwrap()
        .to_bytes();

    assert_ne!(一, 二);
}

#[test]
fn 中身を書き換えたら開かない() {
    // **運ぶ側が中身をすり替えられない**
    let 受け手 = 端末([2u8; 32]);
    let mut 塊 = seal(受け手.public_key(), "承認します".as_bytes())
        .unwrap()
        .to_bytes();
    let 尻 = 塊.len() - 1;
    塊[尻] ^= 0x01;

    let 封 = Sealed::from_bytes(&塊).unwrap();
    assert!(open(&受け手, &封).is_err());
}

#[test]
fn 壊れた封は読めない() {
    assert!(Sealed::from_bytes(b"").is_none());
    assert!(Sealed::from_bytes("みじかい".as_bytes()).is_none());
}

#[test]
fn 空の中身も封をできる() {
    // **中身が無いことと、開けられないことを混ぜない**
    let 受け手 = 端末([2u8; 32]);
    let 封 = seal(受け手.public_key(), b"").unwrap();
    assert_eq!(open(&受け手, &封).unwrap(), b"");
}
