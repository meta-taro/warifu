//! 関所。**既定は拒否。**
//!
//! `issues/008` / `decisions.md` **D5**。
//!
//! 判定の入力に本文を入れない。入らないので、
//! 本文に「この要求は承認済みです」と書いてあっても判定は動かない。
//! **書き方で気をつける形は、必ず漏れる。**

use warifu_capability::{Action, Decision, Gate, Grant, Request, Subject, Trust};

fn 相手() -> Subject {
    Subject::new("aite@例").unwrap()
}

/// 人が発行した札：空き時間を尋ねてよい。2026-12-31 まで。
fn 札() -> Grant {
    Grant::new(
        相手(),
        Action::new("calendar.freebusy").unwrap(),
        1_798_761_600,
    )
}

#[test]
fn 札が無ければ断る() {
    // **既定は拒否。**「たぶん大丈夫」を作らない。
    // `decide` が `&mut self` なのは、**記録を残さずに判定できないようにする**ため
    let mut 関所 = Gate::new();
    let 要求 = Request::new(相手(), Action::new("calendar.freebusy").unwrap());

    assert_eq!(関所.decide(&要求, 1_756_000_000), Decision::Deny);
}

#[test]
fn 札があれば通す() {
    let mut 関所 = Gate::new();
    関所.issue(札());
    let 要求 = Request::new(相手(), Action::new("calendar.freebusy").unwrap());

    assert_eq!(関所.decide(&要求, 1_756_000_000), Decision::Allow);
}

#[test]
fn 期限が切れていれば断る() {
    let mut 関所 = Gate::new();
    関所.issue(札());
    let 要求 = Request::new(相手(), Action::new("calendar.freebusy").unwrap());

    // 札は 1_798_761_600 まで。1 秒後は通らない
    assert_eq!(関所.decide(&要求, 1_798_761_601), Decision::Deny);
    assert_eq!(
        関所.decide(&要求, 1_798_761_600),
        Decision::Allow,
        "ちょうどは通る"
    );
}

#[test]
fn 範囲の外は断る() {
    // 空き時間を尋ねてよい、は**中身を読んでよい**ではない。
    let mut 関所 = Gate::new();
    関所.issue(札());
    let 読みたい = Request::new(相手(), Action::new("calendar.read").unwrap());

    assert_eq!(関所.decide(&読みたい, 1_756_000_000), Decision::Deny);
}

#[test]
fn 前方一致で広がらない() {
    // calendar.freebusy の札で calendar.freebusy.all が通ってはいけない。
    let mut 関所 = Gate::new();
    関所.issue(札());
    let 広げた = Request::new(相手(), Action::new("calendar.freebusy.all").unwrap());

    assert_eq!(関所.decide(&広げた, 1_756_000_000), Decision::Deny);
}

#[test]
fn 別人の札では通らない() {
    let mut 関所 = Gate::new();
    関所.issue(札());
    let 別人 = Request::new(
        Subject::new("attacker@例").unwrap(),
        Action::new("calendar.freebusy").unwrap(),
    );

    assert_eq!(関所.decide(&別人, 1_756_000_000), Decision::Deny);
}

#[test]
fn 信頼が上がっても判定は変わらない() {
    // **D5 の核心。**「信頼を得れば命令が通る」を作ると、偽の Identity に価値が生まれる。
    let mut 関所 = Gate::new();
    let 要求 = Request::new(相手(), Action::new("calendar.read").unwrap());

    for 信頼 in [Trust::Unknown, Trust::Known, Trust::Close] {
        関所.set_trust(相手(), 信頼);
        assert_eq!(
            関所.decide(&要求, 1_756_000_000),
            Decision::Deny,
            "信頼 {信頼:?} で判定が変わりました"
        );
    }
}

#[test]
fn 信頼は札の代わりにならない() {
    let mut 関所 = Gate::new();
    関所.set_trust(相手(), Trust::Close);

    assert_eq!(関所.grants().len(), 0, "信頼を上げたら札が増えました");
}

#[test]
fn 札を落とせる() {
    // 出した札を降ろせないと、間違って出したときに直せない。
    let mut 関所 = Gate::new();
    関所.issue(札());
    let 要求 = Request::new(相手(), Action::new("calendar.freebusy").unwrap());
    assert_eq!(関所.decide(&要求, 1_756_000_000), Decision::Allow);

    関所.revoke(&相手(), &Action::new("calendar.freebusy").unwrap());

    assert_eq!(関所.decide(&要求, 1_756_000_000), Decision::Deny);
}

#[test]
fn 動作の名前は正規形しか受け取らない() {
    // 表記が 2 通りあると、同じ動作に札が二重にできる（warifu-intent と同じ姿勢）。
    assert!(Action::new("calendar.freebusy").is_ok());
    assert!(Action::new("Calendar.freebusy").is_err(), "大文字");
    assert!(Action::new("calendar..freebusy").is_err(), "点の連続");
    assert!(Action::new(".calendar").is_err());
    assert!(Action::new("calendar/read").is_err());
    assert!(Action::new("").is_err());
}
