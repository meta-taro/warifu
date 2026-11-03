//! **同時編集の居場所**（0.1.43・内部の Issue 035）。誰がどの文書のどこに居るか。
//!
//! 文書も場所も割符は読まない（中身は文書を扱う側が決める）。長さと、行を壊す字だけを見る。

use warifu_meeting::{Error, MeetingId, Notice, PRESENCE_MAX};

fn 鍵(seed: u8) -> warifu_core::PublicKey {
    warifu_core::Seed::from_bytes([seed; 32])
        .profile("Personal")
        .device("PC")
        .public_key()
}

fn 居場所(文書: &str, 場所: &str, 札: Option<&str>) -> Notice {
    Notice::Presence {
        meeting: MeetingId::generate(),
        from: 鍵(1),
        話し手: 札.map(str::to_owned),
        文書: 文書.to_owned(),
        場所: 場所.to_owned(),
    }
}

#[test]
fn 札ありも札なしも_閉じた印の空の場所も_そのまま往復する() {
    for n in [
        居場所(
            "github.com/o/r#docs/a.md",
            r#"{"branch":"main","line":12}"#,
            Some("md-business"),
        ),
        居場所("github.com/o/r#docs/a.md", r#"{"line":1}"#, None),
        居場所("github.com/o/r#docs/a.md", "", Some("md-business")),
    ] {
        let 戻り = Notice::from_intent(&n.to_intent().unwrap()).unwrap();
        assert_eq!(戻り, n);
    }
}

#[test]
fn 文書の名前が空や長すぎるものは運ばない() {
    assert!(居場所("", "{}", None).to_intent().is_err());
    let 長い = "a".repeat(PRESENCE_MAX + 1);
    assert!(居場所(&長い, "{}", None).to_intent().is_err());
    assert!(居場所("a", &長い, None).to_intent().is_err());
    let ちょうど = "a".repeat(PRESENCE_MAX);
    assert!(居場所(&ちょうど, &ちょうど, None).to_intent().is_ok());
}

#[test]
fn 行を壊す字は運ばない() {
    assert!(居場所("a\nb", "{}", None).to_intent().is_err());
    assert!(居場所("a", "{\n}", None).to_intent().is_err());
    assert!(居場所("a", "{}", Some("札\t")).to_intent().is_err());
}

#[test]
fn 受ける側でも_壊れた荷物は通さない() {
    let 正しい = 居場所("a", "{}", None).to_intent().unwrap();
    for 荷物 in [vec![1; 10], vec![], {
        let mut v = 正しい.payload().to_vec();
        v.truncate(34);
        v
    }] {
        let 作った = warifu_intent::Intent::with_correlation(
            正しい.kind().clone(),
            正しい.correlation(),
            荷物,
        );
        assert!(matches!(
            Notice::from_intent(&作った),
            Err(Error::Malformed)
        ));
    }
}
