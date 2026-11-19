//! **送った時刻**（0.1.53）。その発言を、送った側の時計でいつ出したか。
//!
//! 発言（`Text`）は時刻を持たないので、受け手の画面は**受け取った時刻**を出していた。
//! 遅れて届いた発言や、入り直して追いついた分が、受け取った時刻で並んでいた。
//! 指すのは行の印（[`Notice::文字の印`]）だけ。古い版は知らない知らせとして読み飛ばす。

use warifu_meeting::{Error, MeetingId, Notice};

fn 鍵(seed: u8) -> warifu_core::PublicKey {
    warifu_core::Seed::from_bytes([seed; 32])
        .profile("Personal")
        .device("PC")
        .public_key()
}

#[test]
fn 送った時刻は往復し_会議を取り違えない() {
    let 会議 = MeetingId::generate();
    let 発言 = Notice::Text {
        meeting: 会議,
        from: 鍵(1),
        話し手: None,
        body: "見積を確かめてください".to_owned(),
    };
    let n = Notice::送った時刻 {
        meeting: 会議,
        from: 鍵(1),
        行: 発言.文字の印().unwrap(),
        ミリ秒: 1_795_000_000_123,
    };
    let 戻った = Notice::from_intent(&n.to_intent().unwrap()).unwrap();
    assert_eq!(戻った, n);
    assert_eq!(戻った.meeting(), 会議);
}

#[test]
fn 受ける側でも_長さの違う荷物は通さない() {
    let 正しい = Notice::送った時刻 {
        meeting: MeetingId::generate(),
        from: 鍵(2),
        行: [1; 32],
        ミリ秒: 1,
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
