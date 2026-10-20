//! **ルームの名前を、居る人みんなで持つ**（0.1.28）。
//!
//! 名前は人が付けるもので、秘密ではない。ただし**行と欄を壊すものは運ばない。**

use warifu_meeting::{Error, MeetingId, Notice, ROOM_NAME_MAX};

fn 鍵(seed: u8) -> warifu_core::PublicKey {
    warifu_core::Seed::from_bytes([seed; 32])
        .profile("Personal")
        .device("PC")
        .public_key()
}

fn 名付け(名前: &str) -> Notice {
    Notice::RoomName {
        meeting: MeetingId::generate(),
        from: 鍵(1),
        名前: 名前.to_owned(),
    }
}

#[test]
fn 名前がそのまま往復する() {
    let n = 名付け("割符の開発");
    let 戻り = Notice::from_intent(&n.to_intent().unwrap()).unwrap();
    assert_eq!(戻り, n);
}

#[test]
fn 空の名前は_名前を外したことを運ぶ() {
    let n = 名付け("");
    let 戻り = Notice::from_intent(&n.to_intent().unwrap()).unwrap();
    assert_eq!(戻り, n);
}

#[test]
fn 行を壊す名前は運ばない() {
    for 名前 in ["一\n二", "タブ\tあり", "\u{7}"] {
        assert!(
            matches!(名付け(名前).to_intent(), Err(Error::Malformed)),
            "通してしまった: {名前:?}"
        );
    }
}

#[test]
fn 長すぎる名前は運ばない() {
    let 長い = "あ".repeat(ROOM_NAME_MAX);
    assert!(matches!(名付け(&長い).to_intent(), Err(Error::Malformed)));
}

#[test]
fn 届いた塊でも同じ検めをする() {
    let 正しい = 名付け("ふつう").to_intent().unwrap();
    let mut 荷物 = 正しい.payload().to_vec();
    荷物.extend_from_slice("\n壊す".as_bytes());
    let 作った =
        warifu_intent::Intent::with_correlation(正しい.kind().clone(), 正しい.correlation(), 荷物);
    assert!(matches!(
        Notice::from_intent(&作った),
        Err(Error::Malformed)
    ));
    // 差出人が欠けていても読まない
    let 短い = warifu_intent::Intent::with_correlation(
        正しい.kind().clone(),
        正しい.correlation(),
        vec![0; 10],
    );
    assert!(matches!(Notice::from_intent(&短い), Err(Error::Malformed)));
}
