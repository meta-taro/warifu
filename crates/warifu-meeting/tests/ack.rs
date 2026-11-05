//! **メンションされた行を受けた**（D130・0.1.45）。
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
