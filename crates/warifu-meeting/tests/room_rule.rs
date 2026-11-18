//! **ルームでのエージェントの扱い**（0.1.52）。
//!
//! 主催が決めた扱いを、居る人みんなへ運ぶ。**知らない扱いは運ばない**
//! （新しい版が足した扱いを、古い扱いに読み替えると、止めるはずのエージェントが話す）。

use warifu_meeting::{Error, MeetingId, Notice, エージェントの扱い};

fn 決める(扱い: エージェントの扱い) -> Notice {
    Notice::RoomRule {
        meeting: MeetingId::generate(),
        扱い,
    }
}

#[test]
fn 三つの扱いがそのまま往復する() {
    for 扱い in [
        エージェントの扱い::ふつう,
        エージェントの扱い::名指しだけ,
        エージェントの扱い::読むだけ,
    ] {
        let n = 決める(扱い);
        let 戻り = Notice::from_intent(&n.to_intent().unwrap()).unwrap();
        assert_eq!(戻り, n);
    }
}

#[test]
fn 扱いの数は往復する_知らない数は読まない() {
    for n in 0..=2 {
        assert_eq!(エージェントの扱い::数から(n).unwrap().数(), n);
    }
    assert_eq!(エージェントの扱い::数から(3), None);
}

fn 塊を差し替える(荷物: Vec<u8>) -> warifu_intent::Intent {
    let 正しい = 決める(エージェントの扱い::読むだけ).to_intent().unwrap();
    warifu_intent::Intent::with_correlation(正しい.kind().clone(), 正しい.correlation(), 荷物)
}

#[test]
fn 知らない扱い_空_長すぎる塊は捨てる() {
    for 荷物 in [vec![9], vec![], vec![1, 1]] {
        assert!(
            matches!(
                Notice::from_intent(&塊を差し替える(荷物.clone())),
                Err(Error::Malformed)
            ),
            "通してしまった: {荷物:?}"
        );
    }
}

#[test]
fn 既定はふつう() {
    assert_eq!(エージェントの扱い::default(), エージェントの扱い::ふつう);
}
