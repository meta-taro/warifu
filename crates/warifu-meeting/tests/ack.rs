//! **メンションされた行を受けた**（D130・0.1.44）。
//!
//! 指すのは行の印（[`Notice::文字の印`]）だけ。本文は運ばない。
//! 受け方は 3 つ —— 人が押した／人が返事を書いた／エージェントが読んだ。

use warifu_meeting::{Error, MeetingId, Notice, 受け方};

fn 鍵(seed: u8) -> warifu_core::PublicKey {
    warifu_core::Seed::from_bytes([seed; 32])
        .profile("Personal")
        .device("PC")
        .public_key()
}

fn 受けた(札: Option<&str>, 方: 受け方) -> Notice {
    Notice::受けた {
        meeting: MeetingId::generate(),
        from: 鍵(2),
        話し手: 札.map(str::to_owned),
        行: [7; 32],
        方,
    }
}

#[test]
fn 受け方ごとに_札ありも札なしも往復する() {
    for 方 in [受け方::押した, 受け方::返事, 受け方::読んだ] {
        for 札 in [None, Some("カムスタ")] {
            let n = 受けた(札, 方);
            assert_eq!(Notice::from_intent(&n.to_intent().unwrap()).unwrap(), n);
        }
    }
}

#[test]
fn 指す行は_配り直されても同じ文字の印() {
    let 会議 = MeetingId::generate();
    let 行 = Notice::Text {
        meeting: 会議,
        from: 鍵(1),
        話し手: None,
        body: "@カムスタ 見積を確かめてください".to_owned(),
    };
    let 印 = 行.文字の印().unwrap();
    let n = Notice::受けた {
        meeting: 会議,
        from: 鍵(2),
        話し手: Some("カムスタ".to_owned()),
        行: 印,
        方: 受け方::読んだ,
    };
    let Notice::受けた { 行: 戻った, .. } = Notice::from_intent(&n.to_intent().unwrap()).unwrap()
    else {
        panic!("受けたに戻らない");
    };
    assert_eq!(戻った, 印);
}

#[test]
fn 受ける側でも_壊れた荷物と知らない受け方は通さない() {
    let 正しい = 受けた(Some("a"), 受け方::押した).to_intent().unwrap();
    let mut 荷物 = 正しい.payload().to_vec();
    // 末尾の 1 byte が受け方。知らない数は受けない
    *荷物.last_mut().unwrap() = 9;
    for 壊れ in [vec![], vec![1; 10], 荷物] {
        let i = warifu_intent::Intent::with_correlation(
            正しい.kind().clone(),
            正しい.correlation(),
            壊れ,
        );
        assert!(matches!(Notice::from_intent(&i), Err(Error::Malformed)));
    }
}

#[test]
fn 札に行を壊す字は運ばない() {
    assert!(受けた(Some("a\tb"), 受け方::押した).to_intent().is_err());
}

fn 画像(会議: MeetingId, 中身: &[u8]) -> Notice {
    Notice::Image {
        meeting: 会議,
        from: 鍵(1),
        話し手: Some("カムスタ".to_owned()),
        型: "image/png".to_owned(),
        中身: 中身.to_vec(),
    }
}

/// **画像の行にも印が付き、経路を往復しても同じ印になる**（0.1.60・画像への返信）
#[test]
fn 画像の行の印は_往復しても同じで_中身が違えば別() {
    let 会議 = MeetingId::generate();
    let 中身 = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
    let 行 = 画像(会議, &中身);
    let 印 = 行.画像の印().expect("画像には印がある");
    assert_eq!(行.行の印(), Some(印));
    assert_eq!(行.文字の印(), None);
    let 戻った = Notice::from_intent(&行.to_intent().unwrap()).unwrap();
    assert_eq!(戻った.画像の印(), Some(印));
    let mut 違う = 中身.clone();
    違う.push(0);
    assert_ne!(画像(会議, &違う).画像の印(), Some(印));
}

/// **文字の行の印は、行の印でも変わらない**（文字への返信は前のまま）
#[test]
fn 文字の行の印は_行の印でも同じ() {
    let 行 = Notice::Text {
        meeting: MeetingId::generate(),
        from: 鍵(1),
        話し手: None,
        body: "見積".to_owned(),
    };
    assert_eq!(行.行の印(), 行.文字の印());
    assert_eq!(行.画像の印(), None);
}
