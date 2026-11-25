//! **会議中にペンで書いた線**（書き込み）。居場所と違い、まとめず・捨てず・順に運ぶ。
//!
//! 荷は割符が読まない（中身は文書を扱う側が決める）。長さと、行を壊す字だけを見る。

use warifu_meeting::{INK_MAX, MeetingId, Notice, PRESENCE_MAX};

fn 鍵(seed: u8) -> warifu_core::PublicKey {
    warifu_core::Seed::from_bytes([seed; 32])
        .profile("Personal")
        .device("PC")
        .public_key()
}

fn 書き込み(文書: &str, 荷: &str, 札: Option<&str>) -> Notice {
    Notice::書き込み {
        meeting: MeetingId::generate(),
        from: 鍵(1),
        話し手: 札.map(str::to_owned),
        文書: 文書.to_owned(),
        荷: 荷.to_owned(),
    }
}

#[test]
fn 荷は書き換えずに往復する() {
    for n in [
        書き込み(
            "github.com/o/r#docs/a.md",
            r#"{"pts":[[1,2],[3,4]],"色":"赤"}"#,
            Some("md-business"),
        ),
        書き込み("a", r#"{"end":true}"#, None),
    ] {
        let 戻り = Notice::from_intent(&n.to_intent().unwrap()).unwrap();
        assert_eq!(戻り, n);
    }
}

#[test]
fn 荷が空や長すぎるもの_文書が空のものは運ばない() {
    assert!(書き込み("a", "", None).to_intent().is_err());
    assert!(書き込み("", "{}", None).to_intent().is_err());
    assert!(
        書き込み("a", &"x".repeat(INK_MAX + 1), None)
            .to_intent()
            .is_err()
    );
    assert!(
        書き込み(&"a".repeat(PRESENCE_MAX + 1), "{}", None)
            .to_intent()
            .is_err()
    );
    assert!(
        書き込み("a", &"x".repeat(INK_MAX), None)
            .to_intent()
            .is_ok()
    );
}

#[test]
fn 行を壊す字は運ばない() {
    assert!(書き込み("a", "{\n}", None).to_intent().is_err());
    assert!(書き込み("a\tb", "{}", None).to_intent().is_err());
}

#[test]
fn 居場所とは別の知らせとして読む() {
    let n = 書き込み("a", "{}", None);
    let 戻り = Notice::from_intent(&n.to_intent().unwrap()).unwrap();
    assert!(matches!(戻り, Notice::書き込み { .. }));
    assert_eq!(戻り.meeting(), n.meeting());
}
