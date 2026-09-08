//! **誰が言ったかを、線の向こうでも残す**（**D48** の穴・2026-09-08）。
//!
//! 机（**D55**）で同じ PC の AI が喋れるようになったが、その発言は
//! **`from` に人の公開鍵を載せて飛んでいた。**
//! こちらの画面では「zumen の AI」と出るのに、**相手の画面では人が打ったのと
//! 区別が付かない。**
//!
//! 机の中では「**繋いできた側は差出人を名乗れない。誰が言ったかは机が刻む**」を
//! 守っているのに、**線の向こうでその区別が消えていた。**

use warifu_core::{Device, PublicKey, Seed};
use warifu_meeting::{MeetingId, Notice};

fn 鍵(seed: [u8; 32]) -> PublicKey {
    let d: Device = Seed::from_bytes(seed).profile("Personal").device("PC");
    d.public_key()
}

fn 会議() -> MeetingId {
    MeetingId::generate()
}

#[test]
fn 人が言ったものには話し手が入らない() {
    // **既定は人。**札が付いていないものは、その席の人が言ったものである
    let 元 = Notice::Text {
        meeting: 会議(),
        from: 鍵([1u8; 32]),
        話し手: None,
        body: "やあ".to_owned(),
    };
    let 塊 = 元.to_intent().unwrap();
    assert_eq!(Notice::from_intent(&塊).unwrap(), 元);
}

#[test]
fn エージェントが言ったものには札が付いて渡る() {
    // **相手の画面でも AI だと分かる必要がある。**
    // 付いていないと、人が打ったのと区別が付かない
    let 元 = Notice::Text {
        meeting: 会議(),
        from: 鍵([1u8; 32]),
        話し手: Some("zumen の AI".to_owned()),
        body: "直しました".to_owned(),
    };
    let 塊 = 元.to_intent().unwrap();
    let 戻り = Notice::from_intent(&塊).unwrap();
    assert_eq!(戻り, 元);
}

#[test]
fn 席の持ち主は札とは別に残る() {
    // **`from` は席の持ち主のまま。**札は「その席の誰が言ったか」である。
    // 混ぜると、AI が別人の席から喋れることになる
    let 元 = Notice::Text {
        meeting: 会議(),
        from: 鍵([1u8; 32]),
        話し手: Some("zumen の AI".to_owned()),
        body: "x".to_owned(),
    };
    let 戻り = Notice::from_intent(&元.to_intent().unwrap()).unwrap();
    let Notice::Text {
        from, 話し手, ..
    } = 戻り
    else {
        panic!("text のはず");
    };
    assert_eq!(from, 鍵([1u8; 32]));
    assert_eq!(話し手.as_deref(), Some("zumen の AI"));
}

#[test]
fn 区切りを壊す札は送れない() {
    // **画面の 1 行に収まらないもの・行を割るものを通さない**
    for 壊す in ["", "a\nb", "a\tb"] {
        let 元 = Notice::Text {
            meeting: 会議(),
            from: 鍵([1u8; 32]),
            話し手: Some(壊す.to_owned()),
            body: "x".to_owned(),
        };
        assert!(元.to_intent().is_err(), "{壊す:?} を通した");
    }
}

#[test]
fn 長すぎる札は送れない() {
    let 元 = Notice::Text {
        meeting: 会議(),
        from: 鍵([1u8; 32]),
        話し手: Some("あ".repeat(200)),
        body: "x".to_owned(),
    };
    assert!(元.to_intent().is_err());
}

#[test]
fn 札があっても空の本文は送れない() {
    // **中身の無い通知で相手の注意を消費できてはいけない**（D31）
    let 元 = Notice::Text {
        meeting: 会議(),
        from: 鍵([1u8; 32]),
        話し手: Some("zumen の AI".to_owned()),
        body: String::new(),
    };
    assert!(元.to_intent().is_err());
}
