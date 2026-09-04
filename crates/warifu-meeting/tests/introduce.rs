//! 紹介（**D41**）。3 人目が、既に居る人の住所を知るための知らせ。
//!
//! **住所は解釈しない。**`warifu-meeting` は `warifu-net` に依存しておらず、
//! ここで運ぶのは「そのまま渡す文字列」である（SDP を読まないのと同じ構え）。

use warifu_core::{PublicKey, Seed};
use warifu_meeting::{Error, MeetingId, Notice};

fn 鍵(seed: u8) -> PublicKey {
    Seed::from_bytes([seed; 32])
        .profile("Personal")
        .device("端末")
        .public_key()
}

fn 往復(n: &Notice) -> Notice {
    Notice::from_intent(&n.to_intent().unwrap()).unwrap()
}

#[test]
fn 誰の住所かと住所そのものが往復する() {
    let 会議 = MeetingId::generate();
    let n = Notice::Introduce {
        meeting: 会議,
        who: 鍵(7),
        address: "WARIFU1-AAAABBBBCCCC".into(),
    };

    match 往復(&n) {
        Notice::Introduce {
            meeting,
            who,
            address,
        } => {
            assert_eq!(meeting, 会議);
            assert_eq!(who, 鍵(7));
            assert_eq!(address, "WARIFU1-AAAABBBBCCCC");
        }
        other => panic!("紹介として読めない: {other:?}"),
    }
}

#[test]
fn 住所の中身は解釈しない() {
    // warifu-meeting は経路の層を知らない。**読めない文字列でもそのまま運ぶ**
    let n = Notice::Introduce {
        meeting: MeetingId::generate(),
        who: 鍵(1),
        address: "これは宛先ではない".into(),
    };
    match 往復(&n) {
        Notice::Introduce { address, .. } => assert_eq!(address, "これは宛先ではない"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn 会議_id_がそのまま相関になる() {
    let 会議 = MeetingId::generate();
    let n = Notice::Introduce {
        meeting: 会議,
        who: 鍵(2),
        address: "WARIFU1-AAAA".into(),
    };
    assert_eq!(n.meeting(), 会議);
    assert_eq!(往復(&n).meeting(), 会議);
}

#[test]
fn 長すぎる住所は組み立てない() {
    // 相手が上限を守る保証は無い。**受け取る側でも数える**（D15）
    let n = Notice::Introduce {
        meeting: MeetingId::generate(),
        who: 鍵(1),
        address: "A".repeat(4096),
    };
    assert!(matches!(n.to_intent(), Err(Error::Malformed)));
}

#[test]
fn 頭が欠けた塊は受け取らない() {
    let n = Notice::Introduce {
        meeting: MeetingId::generate(),
        who: 鍵(3),
        address: "WARIFU1-AAAA".into(),
    };
    let intent = n.to_intent().unwrap();
    // 公開鍵の 32 バイトに足りない塊
    let 壊れた = warifu_intent::Intent::with_correlation(
        intent.kind().clone(),
        intent.correlation(),
        vec![0u8; 8],
    );
    assert!(Notice::from_intent(&壊れた).is_err());
}

#[test]
fn 住所が空でも受け取れる() {
    // **空を弾かない。**住所がまだ分からない相手を紹介する場面がありうる
    let n = Notice::Introduce {
        meeting: MeetingId::generate(),
        who: 鍵(4),
        address: String::new(),
    };
    match 往復(&n) {
        Notice::Introduce { address, .. } => assert!(address.is_empty()),
        other => panic!("{other:?}"),
    }
}
