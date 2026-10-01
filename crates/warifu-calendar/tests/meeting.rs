//! 会議の調整。**双方の承認が揃うまで確定しない。**
//!
//! 企画書 v2 §17。片方の Agent が勝手に予定を入れられるなら、
//! **予定表は相手の Agent に開放されているのと同じ**になる。

use warifu_calendar::{Coordination, Error, Side, Span};

const 朝: u64 = 1_756_803_600;

fn 候補() -> Vec<Span> {
    vec![
        Span::new(朝, 朝 + 3_600).unwrap(),
        Span::new(朝 + 7_200, 朝 + 10_800).unwrap(),
    ]
}

#[test]
fn 片方の承認だけでは確定しない() {
    let mut 調整 = Coordination::new(候補());
    調整.accept(Side::Organizer, &候補()[0]).unwrap();

    assert_eq!(調整.confirmed(), None, "招く側だけで確定しました");
}

#[test]
fn 双方が同じ枠を承認して確定する() {
    let mut 調整 = Coordination::new(候補());
    調整.accept(Side::Organizer, &候補()[0]).unwrap();
    調整.accept(Side::Invitee, &候補()[0]).unwrap();

    assert_eq!(調整.confirmed(), Some(候補()[0]));
}

#[test]
fn 違う枠を承認しても確定しない() {
    let mut 調整 = Coordination::new(候補());
    調整.accept(Side::Organizer, &候補()[0]).unwrap();
    調整.accept(Side::Invitee, &候補()[1]).unwrap();

    assert_eq!(調整.confirmed(), None, "別々の枠で確定しました");
}

#[test]
fn 候補に無い枠は承認できない() {
    // 候補の外を通せるなら、承認は「候補を出した意味」を失う。
    let mut 調整 = Coordination::new(候補());
    let 勝手な枠 = Span::new(朝 + 100_000, 朝 + 103_600).unwrap();

    assert_eq!(
        調整.accept(Side::Organizer, &勝手な枠).unwrap_err(),
        Error::NotOffered
    );
}

#[test]
fn 承認を変えられる() {
    // 間違えて押したときに直せないと、人は押すのを怖がる。
    let mut 調整 = Coordination::new(候補());
    調整.accept(Side::Organizer, &候補()[0]).unwrap();
    調整.accept(Side::Organizer, &候補()[1]).unwrap();
    調整.accept(Side::Invitee, &候補()[1]).unwrap();

    assert_eq!(調整.confirmed(), Some(候補()[1]));
}

#[test]
fn 断れば確定しない() {
    let mut 調整 = Coordination::new(候補());
    調整.accept(Side::Organizer, &候補()[0]).unwrap();
    調整.accept(Side::Invitee, &候補()[0]).unwrap();
    assert!(調整.confirmed().is_some());

    調整.decline(Side::Invitee);

    assert_eq!(調整.confirmed(), None, "断ったのに確定が残っています");
}

#[test]
fn 候補が空の調整は作れない() {
    assert!(Coordination::try_new(Vec::new()).is_err());
}
