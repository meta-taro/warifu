//! **本人の名乗り**（プロフィール・**D75**）を、線の向こうへ渡す。
//!
//! オーナー指示（2026-09-08）——
//! 「**名前と、github のアバターアイコンみたいなものも設置したいです。
//!   すると、先方にもそれがみれます。**」
//!
//! **これは本人確認ではない。**名乗った名前は誰でも真似できるので、
//! 受け取った側が付けた呼び名があれば**そちらが勝つ**（**D46**）。

use warifu_core::{Device, PublicKey, Seed};
use warifu_meeting::{MeetingId, Notice};

fn 鍵(seed: [u8; 32]) -> PublicKey {
    let d: Device = Seed::from_bytes(seed).profile("Personal").device("PC");
    d.public_key()
}

fn 会議() -> MeetingId {
    MeetingId::generate()
}

fn 往復(n: &Notice) -> Notice {
    Notice::from_intent(&n.to_intent().expect("載せられる")).expect("読める")
}

#[test]
fn 名乗りが往復する() {
    let n = Notice::Profile {
        meeting: 会議(),
        from: 鍵([1u8; 32]),
        名前: "めたたろ".into(),
        紹介: "割符を作っています。".into(),
    };
    assert_eq!(往復(&n), n);
}

#[test]
fn 名乗りを取り消せる() {
    // **空も通す。**消したことが伝わらないと、相手の画面に前の名前が残る
    let n = Notice::Profile {
        meeting: 会議(),
        from: 鍵([2u8; 32]),
        名前: String::new(),
        紹介: String::new(),
    };
    assert_eq!(往復(&n), n);
}

#[test]
fn 紹介だけでも渡せる() {
    let n = Notice::Profile {
        meeting: 会議(),
        from: 鍵([3u8; 32]),
        名前: String::new(),
        紹介: "名前は名乗らないが、何をしているかは言う".into(),
    };
    assert_eq!(往復(&n), n);
}

#[test]
fn 長すぎる名乗りは載せられない() {
    // **受け取る側でも数える**（D15）。ここは出す側で止める
    let n = Notice::Profile {
        meeting: 会議(),
        from: 鍵([4u8; 32]),
        名前: "あ".repeat(64), // 3 バイト × 64 = 192 > 128
        紹介: String::new(),
    };
    assert!(n.to_intent().is_err());
}

#[test]
fn 改行の入った名乗りは載せられない() {
    // 通すと、受け取った側の画面が崩れる
    let n = Notice::Profile {
        meeting: 会議(),
        from: 鍵([5u8; 32]),
        名前: "め\nたたろ".into(),
        紹介: String::new(),
    };
    assert!(n.to_intent().is_err());
}

#[test]
fn 壊れた塊は名乗りとして読まない() {
    let 良い = Notice::Profile {
        meeting: 会議(),
        from: 鍵([6u8; 32]),
        名前: "め".into(),
        紹介: "い".into(),
    };
    let intent = 良い.to_intent().unwrap();
    // 差出人の 32 バイトすら無い
    let 短い = warifu_intent::Intent::with_correlation(
        intent.kind().clone(),
        intent.correlation(),
        vec![0u8; 10],
    );
    assert!(Notice::from_intent(&短い).is_err());
}

#[test]
fn 名前の長さが荷物と食い違えば読まない() {
    // **書いてある長さを信じない**（信じると、範囲の外を読もうとする）
    let 良い = Notice::Profile {
        meeting: 会議(),
        from: 鍵([7u8; 32]),
        名前: "め".into(),
        紹介: String::new(),
    };
    let intent = 良い.to_intent().unwrap();
    let mut 荷物 = intent.payload().to_vec();
    荷物[32] = 200; // 実際には 3 バイトしか無い
    let 壊れ =
        warifu_intent::Intent::with_correlation(intent.kind().clone(), intent.correlation(), 荷物);
    assert!(Notice::from_intent(&壊れ).is_err());
}
