//! **ルームキーを、その相手にだけ手渡す**（2026-10-20）。
//!
//! 人が別の道で渡すと鍵が外の網に出るので、割符の線で渡す。
//! ここは**運ぶだけ**で、鍵の中身（宛先・割符）は読まない ——
//! ただし**鍵でないものを鍵の口で運ばせない。**

use warifu_meeting::{Error, KEY_MAX, MeetingId, Notice};

fn 鍵(seed: u8) -> warifu_core::PublicKey {
    warifu_core::Seed::from_bytes([seed; 32])
        .profile("Personal")
        .device("PC")
        .public_key()
}

fn 手渡し(中身: &str) -> Notice {
    Notice::Key {
        meeting: MeetingId::generate(),
        from: 鍵(1),
        鍵: 中身.to_owned(),
    }
}

#[test]
fn 鍵がそのまま往復する() {
    let n = 手渡し("WARIFU1-ABCDEF234567#GHIJKL#MNOPQR");
    let 戻り = Notice::from_intent(&n.to_intent().unwrap()).unwrap();
    assert_eq!(戻り, n);
}

#[test]
fn 鍵でないものは運ばない() {
    for 中身 in [
        "",
        "WARIFU1-",
        "こんにちは",
        "rm -rf ~",
        "WARIFU1-ABC DEF",
        "WARIFU1-ABC\nrm",
        "warifu1-abc",
    ] {
        assert!(
            matches!(手渡し(中身).to_intent(), Err(Error::Malformed)),
            "通してしまった: {中身:?}"
        );
    }
}

#[test]
fn 長すぎる鍵は運ばない() {
    let 長い = format!("WARIFU1-{}", "A".repeat(KEY_MAX));
    assert!(matches!(手渡し(&長い).to_intent(), Err(Error::Malformed)));
}

#[test]
fn 届いた塊でも同じ検めをする() {
    // **組み立てる側を通らない塊**（相手が手で作ったもの）も、読む側で止める
    let 正しい = 手渡し("WARIFU1-ABCDEF").to_intent().unwrap();
    let mut 荷物 = 正しい.payload().to_vec();
    荷物.truncate(32);
    荷物.extend_from_slice("ただの文字".as_bytes());
    let 壊した =
        warifu_intent::Intent::with_correlation(正しい.kind().clone(), 正しい.correlation(), 荷物);
    assert!(matches!(
        Notice::from_intent(&壊した),
        Err(Error::Malformed)
    ));
}

#[test]
fn 文字の口とは混ざらない() {
    let n = 手渡し("WARIFU1-ABCDEF");
    assert_eq!(n.to_intent().unwrap().kind().as_str(), "meeting.key");
}
