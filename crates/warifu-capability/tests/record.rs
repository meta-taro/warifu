//! 決めたことの記録。**消す口を置かない。**
//!
//! 何を許して何を断ったかが残らないと、後から誰も確かめられない。
//! `warifu-read` の会計（**D20**）と同じ理屈。

use warifu_capability::{Action, Gate, Grant, Request, Subject};

fn 相手() -> Subject {
    Subject::new("aite@例").unwrap()
}

#[test]
fn 通したことも断ったことも残る() {
    let mut 関所 = Gate::new();
    関所.issue(Grant::new(
        相手(),
        Action::new("calendar.freebusy").unwrap(),
        1_798_761_600,
    ));

    関所.decide(
        &Request::new(相手(), Action::new("calendar.freebusy").unwrap()),
        100,
    );
    関所.decide(
        &Request::new(相手(), Action::new("calendar.read").unwrap()),
        100,
    );

    let 記録 = 関所.log();
    assert_eq!(記録.len(), 2);
    assert_eq!(記録.allowed(), 1);
    assert_eq!(記録.denied(), 1);
}

#[test]
fn 記録に本文が入らない() {
    // 要求が本文を持たないので、記録にも入りようがない。**型でそうなっている。**
    let mut 関所 = Gate::new();
    関所.decide(
        &Request::new(相手(), Action::new("calendar.read").unwrap()),
        100,
    );

    let 出力 = 関所.log().to_tsv();
    assert!(出力.contains("aite@例"));
    assert!(出力.contains("calendar.read"));
    assert!(出力.contains("断った"));
}

#[test]
fn 記録が往復する() {
    let mut 関所 = Gate::new();
    関所.decide(
        &Request::new(相手(), Action::new("calendar.read").unwrap()),
        100,
    );

    let 戻り = warifu_capability::Log::from_tsv(&関所.log().to_tsv()).unwrap();

    assert_eq!(戻り.len(), 1);
    assert_eq!(戻り.denied(), 1);
}

#[test]
fn 壊れた記録は受け取らない() {
    assert!(warifu_capability::Log::from_tsv("でたらめ").is_err());
}
