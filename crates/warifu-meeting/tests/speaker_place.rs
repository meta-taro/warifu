//! **話し手の場所**（0.1.52）。名乗り直しても同じエージェントだと分かるように、名札と起動した場所を結ぶ。

use warifu_meeting::{Error, MeetingId, Notice};

fn 鍵(seed: u8) -> warifu_core::PublicKey {
    warifu_core::Seed::from_bytes([seed; 32])
        .profile("Personal")
        .device("PC")
        .public_key()
}

fn 場所(札: &str, 場所: &str) -> Notice {
    Notice::話し手の場所 {
        meeting: MeetingId::generate(),
        from: 鍵(1),
        札: 札.to_owned(),
        場所: 場所.to_owned(),
    }
}

#[test]
fn 札と場所がそのまま往復する() {
    let n = 場所("割符（mac mini）（warifu）", "warifu");
    assert_eq!(Notice::from_intent(&n.to_intent().unwrap()).unwrap(), n);
}

#[test]
fn 空や行を壊す札と場所は運ばない() {
    for (札, 所) in [
        ("", "warifu"),
        ("割符", ""),
        ("一\n二", "warifu"),
        ("割符", "a\tb"),
    ] {
        assert!(
            matches!(場所(札, 所).to_intent(), Err(Error::Malformed)),
            "通してしまった: {札:?} {所:?}"
        );
    }
}

#[test]
fn 届いた塊が短すぎる_札の長さが合わないものは捨てる() {
    let 正しい = 場所("割符", "warifu").to_intent().unwrap();
    let 作る = |荷物: Vec<u8>| {
        warifu_intent::Intent::with_correlation(正しい.kind().clone(), 正しい.correlation(), 荷物)
    };
    let mut 長すぎる札 = 正しい.payload()[..32].to_vec();
    長すぎる札.push(200);
    長すぎる札.extend_from_slice(b"ab");
    for 荷物 in [正しい.payload()[..10].to_vec(), 長すぎる札] {
        assert!(matches!(
            Notice::from_intent(&作る(荷物)),
            Err(Error::Malformed)
        ));
    }
}
