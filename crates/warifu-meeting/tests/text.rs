//! 会議の中で文字を送る（チャット）。
//!
//! **画面が無くても使える口**にする。エージェントどうしが直接やり取りでき、
//! 2 台の疎通を人手なしで確かめられるようになる。

use warifu_meeting::{Error, MeetingId, Notice};

fn 往復(n: &Notice) -> Notice {
    Notice::from_intent(&n.to_intent().unwrap()).unwrap()
}

#[test]
fn 文字がそのまま往復する() {
    let 会議 = MeetingId::generate();
    let n = Notice::Text {
        meeting: 会議,
        body: "こんにちは。聞こえますか".into(),
    };
    match 往復(&n) {
        Notice::Text { meeting, body } => {
            assert_eq!(meeting, 会議);
            assert_eq!(body, "こんにちは。聞こえますか");
        }
        other => panic!("文字として読めない: {other:?}"),
    }
}

#[test]
fn 空の文字は送らない() {
    // 空を送れると、**中身の無い通知で相手の注意を消費できる**（D31 と同じ筋）
    let n = Notice::Text {
        meeting: MeetingId::generate(),
        body: String::new(),
    };
    assert!(matches!(n.to_intent(), Err(Error::Malformed)));
}

#[test]
fn 長すぎる文字は組み立てない() {
    // **受け取る側でも数える**（D15）。相手が上限を守る保証は無い
    let n = Notice::Text {
        meeting: MeetingId::generate(),
        body: "あ".repeat(20_000),
    };
    assert!(matches!(n.to_intent(), Err(Error::Malformed)));
}

#[test]
fn 改行も絵文字もそのまま通る() {
    // **中身を検めない。**warifu は文字の意味を知らない
    let 中身 = "1 行目\n2 行目\t— 🙂";
    let n = Notice::Text {
        meeting: MeetingId::generate(),
        body: 中身.into(),
    };
    match 往復(&n) {
        Notice::Text { body, .. } => assert_eq!(body, 中身),
        other => panic!("{other:?}"),
    }
}

#[test]
fn 会議_id_がそのまま相関になる() {
    let 会議 = MeetingId::generate();
    let n = Notice::Text {
        meeting: 会議,
        body: "x".into(),
    };
    assert_eq!(n.meeting(), 会議);
    assert_eq!(往復(&n).meeting(), 会議);
}
