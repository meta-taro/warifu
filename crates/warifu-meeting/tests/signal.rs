//! 映像を張るための下ごしらえ（SDP / ICE）を運ぶところ。
//!
//! **warifu は SDP を読まない。**段（申し出か・返事か・経路の候補か）だけを見て、
//! 中身はそのまま相手に渡す。読み始めると Codec の話が warifu に入り込み、
//! 「Codec を自前で書かない」（`issues/005` 満たすこと 3）が守れなくなる。

use warifu_meeting::{Error, MAX_SIGNAL, MeetingId, Notice, Signal, Step};

#[test]
fn 段と中身がそのまま往復する() {
    let 会議 = MeetingId::generate();

    for 段 in [Step::Offer, Step::Answer, Step::Candidate, Step::End] {
        let 元 = Notice::Signal(Signal::new(
            会議,
            段,
            b"v=0\r\no=- 0 0 IN IP4 0.0.0.0".to_vec(),
        ));
        let 塊 = 元.to_intent().unwrap();

        assert_eq!(塊.kind().as_str(), "meeting.signal");
        assert_eq!(Notice::from_intent(&塊).unwrap(), 元);
    }
}

#[test]
fn 中身は解釈しない() {
    // SDP でないものを入れても通る。**warifu から見れば全部ただのバイト列**
    let 会議 = MeetingId::generate();
    let 元 = Notice::Signal(Signal::new(会議, Step::Offer, vec![0xff, 0x00, 0xfe]));

    let 戻り = Notice::from_intent(&元.to_intent().unwrap()).unwrap();
    assert_eq!(戻り, 元);
}

#[test]
fn 相手を指す欄が無い() {
    // 宛先欄を作ると「自分宛でない下ごしらえを預かって渡す」形が書けてしまう。
    // それは **D7（他人の通信を中継する）への入口**。フルメッシュでは
    // 相手ごとに経路が 1 本ずつあり、**誰宛かは経路そのものが決めている**
    let 会議 = MeetingId::generate();
    let 中身 = b"candidate:1 1 UDP".to_vec();
    let 塊 = Notice::Signal(Signal::new(会議, Step::Candidate, 中身.clone()))
        .to_intent()
        .unwrap();

    assert_eq!(
        塊.payload().len(),
        1 + 中身.len(),
        "段 1 バイトと中身だけ。宛先の 32 バイトが入っていない"
    );
}

#[test]
fn 知らない段は受け取らない() {
    // 口の名前（`Kind`）は知らないものも通すが、**段は通さない**。
    // 口は増やせる場所として開けてある。段は 4 つで閉じた手順で、
    // 知らない段を「たぶん申し出だろう」と扱うと繋ぎ間違える
    let 会議 = MeetingId::generate();
    let 塊 = warifu_intent::Intent::with_correlation(
        warifu_intent::Kind::new("meeting.signal").unwrap(),
        会議.into(),
        vec![99, 1, 2, 3],
    );

    assert!(matches!(Notice::from_intent(&塊), Err(Error::Malformed)));
}

#[test]
fn 空の下ごしらえは受け取らない() {
    let 会議 = MeetingId::generate();
    let 塊 = warifu_intent::Intent::with_correlation(
        warifu_intent::Kind::new("meeting.signal").unwrap(),
        会議.into(),
        Vec::new(),
    );

    assert!(matches!(Notice::from_intent(&塊), Err(Error::Malformed)));
}

#[test]
fn 大きすぎる中身は組み立てない() {
    // SDP は数 KB。16 MiB の「SDP」を送りつけられて確保させられる筋を塞ぐ
    let 会議 = MeetingId::generate();
    let 元 = Notice::Signal(Signal::new(会議, Step::Offer, vec![0u8; MAX_SIGNAL + 1]));

    assert!(matches!(元.to_intent(), Err(Error::TooLarge)));
}

#[test]
fn 上限ちょうどは通る() {
    let 会議 = MeetingId::generate();
    let 元 = Notice::Signal(Signal::new(会議, Step::Offer, vec![0u8; MAX_SIGNAL]));

    assert!(元.to_intent().is_ok(), "上限を 1 バイト読み違えていない");
}

#[test]
fn debug_に下ごしらえの中身が出ない() {
    // **SDP と ICE には端末のローカル IP が入っている。**
    // ログに落ちれば、直接つながる相手以外にも住所が漏れる
    let 会議 = MeetingId::generate();
    let 中身 = b"candidate:1 1 UDP 2130706431 192.168.1.42 50000 typ host".to_vec();
    let 文字列 = format!("{:?}", Signal::new(会議, Step::Candidate, 中身));

    assert!(
        !文字列.contains("192.168"),
        "ローカル IP が出ている: {文字列}"
    );
    assert!(!文字列.contains("candidate"), "中身が出ている: {文字列}");
    assert!(文字列.contains("Candidate"), "段は分かるべき: {文字列}");
    assert!(
        文字列.contains("56"),
        "何バイト来たかは分かるべき: {文字列}"
    );
}

#[test]
fn 別の会議の下ごしらえは別物として分かる() {
    let 甲 = MeetingId::generate();
    let 乙 = MeetingId::generate();

    let 塊 = Notice::Signal(Signal::new(甲, Step::Offer, b"x".to_vec()))
        .to_intent()
        .unwrap();
    let 戻り = Notice::from_intent(&塊).unwrap();

    match 戻り {
        Notice::Signal(s) => {
            assert_eq!(s.meeting(), 甲);
            assert_ne!(s.meeting(), 乙);
        }
        その他 => panic!("下ごしらえとして読めない: {その他:?}"),
    }
}
