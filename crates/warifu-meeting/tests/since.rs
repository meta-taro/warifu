//! **入り直す客が「ここまで持っている」と主催へ言う**（0.1.42）。
//!
//! 印は文字の差出人・札・本文から作るので、主催が配り直しても同じになる。

use warifu_meeting::{Error, MeetingId, Notice};

fn 鍵(seed: u8) -> warifu_core::PublicKey {
    warifu_core::Seed::from_bytes([seed; 32])
        .profile("Personal")
        .device("PC")
        .public_key()
}

fn 文字(部屋: MeetingId, 本文: &str, 札: Option<&str>) -> Notice {
    Notice::Text {
        meeting: 部屋,
        from: 鍵(1),
        話し手: 札.map(str::to_owned),
        body: 本文.to_owned(),
    }
}

#[test]
fn 印ありも印なしもそのまま往復する() {
    let 部屋 = MeetingId::generate();
    for 最後 in [None, Some([7u8; 32])] {
        let n = Notice::Since {
            meeting: 部屋,
            最後,
        };
        let 戻り = Notice::from_intent(&n.to_intent().unwrap()).unwrap();
        assert_eq!(戻り, n);
        assert_eq!(戻り.meeting(), 部屋);
    }
}

#[test]
fn 長さの違う印は通さない() {
    let 正しい = Notice::Since {
        meeting: MeetingId::generate(),
        最後: Some([1; 32]),
    }
    .to_intent()
    .unwrap();
    let 作った = warifu_intent::Intent::with_correlation(
        正しい.kind().clone(),
        正しい.correlation(),
        vec![1; 31],
    );
    assert!(matches!(
        Notice::from_intent(&作った),
        Err(Error::Malformed)
    ));
}

#[test]
fn 文字の印は_配り直しても同じで_本文や札や部屋が違えば変わる() {
    let 部屋 = MeetingId::generate();
    let a = 文字(部屋, "こんにちは", None);
    let 配り直し = Notice::from_intent(&a.to_intent().unwrap()).unwrap();
    assert_eq!(a.文字の印(), 配り直し.文字の印());
    assert!(a.文字の印().is_some());
    assert_ne!(a.文字の印(), 文字(部屋, "こんばんは", None).文字の印());
    assert_ne!(
        a.文字の印(),
        文字(部屋, "こんにちは", Some("dev")).文字の印()
    );
    assert_ne!(
        a.文字の印(),
        文字(MeetingId::generate(), "こんにちは", None).文字の印()
    );
    assert_eq!(Notice::Join { meeting: 部屋 }.文字の印(), None);
}
