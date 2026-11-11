//! **返信の印**（11-11）。どの行が、どの行への返信か。
//!
//! 発言の番号は機械ごとに振るので、`#1234` と書いても別の機械では別の行を指す。
//! 指すのは行の印（[`Notice::文字の印`]）だけ。どの機械で作っても同じ印になる。
//! 本文は運ばない。古い版は知らない知らせとして読み飛ばす（発言そのものは変えない）。

use warifu_meeting::{Error, MeetingId, Notice};

fn 鍵(seed: u8) -> warifu_core::PublicKey {
    warifu_core::Seed::from_bytes([seed; 32])
        .profile("Personal")
        .device("PC")
        .public_key()
}

#[test]
fn 返信の印は往復し_会議を取り違えない() {
    let 会議 = MeetingId::generate();
    let 元 = Notice::Text {
        meeting: 会議,
        from: 鍵(1),
        話し手: None,
        body: "見積を確かめてください".to_owned(),
    };
    let 返事 = Notice::Text {
        meeting: 会議,
        from: 鍵(2),
        話し手: Some("カムスタ".to_owned()),
        body: "確かめました".to_owned(),
    };
    let n = Notice::返信 {
        meeting: 会議,
        from: 鍵(2),
        行: 返事.文字の印().unwrap(),
        先: 元.文字の印().unwrap(),
    };
    let 戻った = Notice::from_intent(&n.to_intent().unwrap()).unwrap();
    assert_eq!(戻った, n);
    assert_eq!(戻った.meeting(), 会議);
}

#[test]
fn 受ける側でも_長さの違う荷物は通さない() {
    let 正しい = Notice::返信 {
        meeting: MeetingId::generate(),
        from: 鍵(2),
        行: [1; 32],
        先: [2; 32],
    }
    .to_intent()
    .unwrap();
    let mut 長い = 正しい.payload().to_vec();
    長い.push(0);
    for 壊れ in [vec![], vec![1; 64], 長い] {
        let i = warifu_intent::Intent::with_correlation(
            正しい.kind().clone(),
            正しい.correlation(),
            壊れ,
        );
        assert!(matches!(Notice::from_intent(&i), Err(Error::Malformed)));
    }
}

#[test]
fn 自分自身への返信は作らない() {
    let n = Notice::返信 {
        meeting: MeetingId::generate(),
        from: 鍵(2),
        行: [3; 32],
        先: [3; 32],
    };
    assert!(matches!(n.to_intent(), Err(Error::Malformed)));
}
