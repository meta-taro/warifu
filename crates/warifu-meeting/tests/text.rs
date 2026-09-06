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
        from: 鍵(1),
        body: "こんにちは。聞こえますか".into(),
    };
    match 往復(&n) {
        Notice::Text { meeting, body, .. } => {
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
        from: 鍵(1),
        body: String::new(),
    };
    assert!(matches!(n.to_intent(), Err(Error::Malformed)));
}

#[test]
fn 長すぎる文字は組み立てない() {
    // **受け取る側でも数える**（D15）。相手が上限を守る保証は無い
    let n = Notice::Text {
        meeting: MeetingId::generate(),
        from: 鍵(1),
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
        from: 鍵(1),
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
        from: 鍵(1),
        body: "x".into(),
    };
    assert_eq!(n.meeting(), 会議);
    assert_eq!(往復(&n).meeting(), 会議);
}

/// **誰が言ったのかを、文字そのものが持つ**（**D48**）。
///
/// 三者会議は星形である —— 参加者どうしは繋がっておらず、**主催だけが全員と繋がっている**
/// （2026-09-06 に実測。B と C はお互いの発言を 1 通も受け取っていなかった）。
/// 主催が聞いた文字をほかの人へ配るには、**元の差出人が文字に載っていなければならない。**
/// 載っていないと、配られた側には**全部が主催の発言に見える。**
#[test]
fn 文字は差出人を持って往復する() {
    let 会議 = MeetingId::generate();
    let 言った人 = 鍵(7);
    let n = Notice::Text {
        meeting: 会議,
        from: 言った人,
        body: "B です。三者会議に入りました".into(),
    };
    match 往復(&n) {
        Notice::Text {
            meeting,
            from,
            body,
        } => {
            assert_eq!(meeting, 会議);
            assert_eq!(from, 言った人, "配ると差出人が消える");
            assert_eq!(body, "B です。三者会議に入りました");
        }
        other => panic!("文字として読めない: {other:?}"),
    }
}

/// 差出人だけで中身が無いものは通さない（**空は送れない**の筋を保つ）。
#[test]
fn 差出人だけの文字は送らない() {
    let n = Notice::Text {
        meeting: MeetingId::generate(),
        from: 鍵(7),
        body: String::new(),
    };
    assert!(matches!(n.to_intent(), Err(Error::Malformed)));
}

fn 鍵(seed: u8) -> warifu_core::PublicKey {
    warifu_core::Seed::from_bytes([seed; 32])
        .profile("Personal")
        .device("PC")
        .public_key()
}
