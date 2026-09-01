//! 空き時間。**中身は出さない。**
//!
//! 企画書 v2 §17 の会議調整（roadmap **Phase 3** の代表 Demo）。
//! `meeting.request` に対して**空いている枠だけ**を返し、
//! **予定の題名も場所も相手には渡さない。**
//!
//! これが Capability の実地テストになる（`decisions.md` **D5** / **D24**）。
//! 「相手に見せてよいもの」と「見せてはいけないもの」が、
//! **同じ予定表の中に混ざっている**からである。

use warifu_calendar::{Calendar, Error, Event, MAX_WINDOW, Span};

/// 予定の題名。**どこにも漏れてはいけない文字列。**
const 題名: &str = "SHIRUSHI-予定-b4c1";

/// 2026-09-02 09:00〜18:00（epoch 秒。UTC で計算した固定値）
const 朝: u64 = 1_756_803_600;
const 夜: u64 = 1_756_836_000;

fn 一日() -> Span {
    Span::new(朝, 夜).unwrap()
}

fn 予定あり() -> Calendar {
    let mut 予定表 = Calendar::new();
    // 12:00〜13:00 が埋まっている
    予定表.add(Event::new(
        Span::new(朝 + 10_800, 朝 + 14_400).unwrap(),
        題名,
    ));
    予定表
}

#[test]
fn 予定が無ければ窓いっぱいが空く() {
    let 空き = Calendar::new().slots(&一日(), 3_600, 10).unwrap();

    assert_eq!(空き.len(), 1);
    assert_eq!(空き[0], 一日());
}

#[test]
fn 予定と重なる時間は空かない() {
    let 空き = 予定あり().slots(&一日(), 3_600, 10).unwrap();

    assert_eq!(空き.len(), 2, "前後 2 つに割れるはず: {空き:?}");
    assert_eq!(空き[0].end(), 朝 + 10_800);
    assert_eq!(空き[1].start(), 朝 + 14_400);
}

#[test]
fn 空き時間に予定の中身が入らない() {
    // **型に入る場所が無い**うえで、表示にも出ないことを確かめる。
    let 空き = 予定あり().slots(&一日(), 3_600, 10).unwrap();

    let 出力 = format!("{空き:?}");
    assert!(!出力.contains(題名), "空き時間に題名が漏れています: {出力}");
}

#[test]
fn 予定表の_debug_に題名が出ない() {
    let 出力 = format!("{:?}", 予定あり());
    assert!(
        !出力.contains(題名),
        "予定表の Debug に題名が出ています: {出力}"
    );
}

#[test]
fn 求めた長さに満たない隙間は返さない() {
    let mut 予定表 = Calendar::new();
    // 10:00〜10:30 と 11:00〜18:00。隙間は 30 分しかない
    予定表.add(Event::new(Span::new(朝 + 3_600, 朝 + 5_400).unwrap(), 題名));
    予定表.add(Event::new(Span::new(朝 + 7_200, 夜).unwrap(), 題名));

    let 一時間 = 予定表.slots(&一日(), 3_600, 10).unwrap();

    // 09:00〜10:00 の 1 時間だけが残る（10:30〜11:00 は 30 分なので落ちる）
    assert_eq!(一時間.len(), 1, "{一時間:?}");
    assert_eq!(一時間[0], Span::new(朝, 朝 + 3_600).unwrap());
}

#[test]
fn 接している予定は空きを余分に作らない() {
    let mut 予定表 = Calendar::new();
    予定表.add(Event::new(Span::new(朝, 朝 + 3_600).unwrap(), 題名));
    予定表.add(Event::new(Span::new(朝 + 3_600, 朝 + 7_200).unwrap(), 題名));

    let 空き = 予定表.slots(&一日(), 3_600, 10).unwrap();

    assert_eq!(
        空き.len(),
        1,
        "接した予定の間に 0 秒の空きを作っています: {空き:?}"
    );
    assert_eq!(空き[0].start(), 朝 + 7_200);
}

#[test]
fn 重なった予定をまとめる() {
    let mut 予定表 = Calendar::new();
    予定表.add(Event::new(
        Span::new(朝 + 3_600, 朝 + 10_800).unwrap(),
        題名,
    ));
    予定表.add(Event::new(
        Span::new(朝 + 7_200, 朝 + 14_400).unwrap(),
        題名,
    ));

    let 空き = 予定表.slots(&一日(), 1_800, 10).unwrap();

    assert_eq!(空き.len(), 2, "{空き:?}");
    assert_eq!(空き[0].end(), 朝 + 3_600);
    assert_eq!(空き[1].start(), 朝 + 14_400);
}

#[test]
fn 窓が広すぎれば断る() {
    // **ここが要。**窓を広く取れるなら、空き枠を尋ねるだけで
    // 相手の予定表を丸ごと写し取れる。
    let 一年 = Span::new(朝, 朝 + MAX_WINDOW + 1).unwrap();

    assert_eq!(
        Calendar::new().slots(&一年, 3_600, 10).unwrap_err(),
        Error::WindowTooWide
    );
    // 上限ちょうどは通る
    assert!(
        Calendar::new()
            .slots(&Span::new(朝, 朝 + MAX_WINDOW).unwrap(), 3_600, 10)
            .is_ok()
    );
}

#[test]
fn 返す件数に上限がある() {
    // 件数を絞らないと、細かく刻んで尋ねることで結局ぜんぶ分かる。
    let mut 予定表 = Calendar::new();
    for i in 0..8 {
        let s = 朝 + i * 3_600;
        予定表.add(Event::new(Span::new(s + 1_800, s + 2_400).unwrap(), 題名));
    }

    let 空き = 予定表.slots(&一日(), 600, 3).unwrap();

    assert_eq!(空き.len(), 3, "上限を超えて返しています: {空き:?}");
}

#[test]
fn 長さ_0_の枠は求められない() {
    assert_eq!(
        Calendar::new().slots(&一日(), 0, 10).unwrap_err(),
        Error::Malformed
    );
}

#[test]
fn 終わりが始まりより前の区間は作れない() {
    assert!(Span::new(夜, 朝).is_err());
    assert!(Span::new(朝, 朝).is_err(), "長さ 0 の区間");
    assert!(Span::new(朝, 夜).is_ok());
}
