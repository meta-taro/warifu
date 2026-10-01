//! 既存のメール（RFC 5322）を `warifu-read` の入口に合わせる。
//!
//! `issues/007` の完了条件「**既存の IMAP アカウント 1 つだけで動く**」の、
//! 通信を伴わない側をここで固定する。
//!
//! **MIME を解くのはこの層だけ。**`warifu-read` には入れない（`decisions.md` **D18**）。
//! 入れると、経路の数だけ同じものを作り直すことになる。

use warifu_imap::{Error, to_received};
use warifu_read::{Level, Reader, Source, View};

const 普通の一通: &[u8] = b"From: billing@example.com\r\n\
Subject: \xe8\xab\x8b\xe6\xb1\x82\xe6\x9b\xb8\r\n\
Date: Thu, 1 Jan 1970 00:00:00 +0000\r\n\
X-Priority: 1\r\n\
Importance: high\r\n\
Content-Type: text/plain; charset=utf-8\r\n\
\r\n\
\xe5\x90\x88\xe8\xa8\x88 12,000 \xe5\x86\x86\r\n";

#[test]
fn 差出人と本文が取り出せる() {
    let 届いた = to_received(普通の一通, 1_756_000_000).unwrap();

    assert_eq!(届いた.sender().as_str(), "billing@example.com");
    assert_eq!(届いた.source(), Source::Imap);

    match Reader::new().open_at(&届いた, Level::Raw).unwrap() {
        View::Raw { body, .. } => {
            assert!(String::from_utf8_lossy(body.as_bytes()).contains("合計 12,000 円"));
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn 相手の書いた日時を受け取った時刻にしない() {
    // Date は送る側が自由に書ける。**申し送りであって事実ではない。**
    let 届いた = to_received(普通の一通, 1_756_000_000).unwrap();

    assert_eq!(届いた.received_at(), 1_756_000_000);
}

#[test]
fn 送信者の優先度の申告が通らない() {
    // X-Priority: 1 と Importance: high が付いていても、読む側の判断は動かない。
    let 届いた = to_received(普通の一通, 1_756_000_000).unwrap();
    let 見え方 = Reader::new().read(&届いた);

    assert_eq!(見え方.metadata().priority(), warifu_read::Priority::Normal);
    assert!(!見え方.metadata().action_required());
}

#[test]
fn ヘッダは申し送りとして残る() {
    // 判断には使わないが、**何を無視したかが後から見えるように**残す。
    let 届いた = to_received(普通の一通, 1_756_000_000).unwrap();

    assert!(届いた.claims().len() >= 4, "ヘッダが落ちています");
}

#[test]
fn html_しかない本文はタグを落として渡す() {
    // HTML をそのまま読み手へ渡さない（PRD §12-2 の HTML sanitize）。
    let html = b"From: news@example.com\r\n\
Content-Type: text/html; charset=utf-8\r\n\
\r\n\
<html><body><script>alert('x')</script><p>Hello</p></body></html>\r\n";

    let 届いた = to_received(html, 100).unwrap();

    match Reader::new().open_at(&届いた, Level::Raw).unwrap() {
        View::Raw { body, .. } => {
            let 本文 = String::from_utf8_lossy(body.as_bytes()).into_owned();
            assert!(本文.contains("Hello"));
            assert!(!本文.contains("<p>"), "タグが残っています: {本文}");
            assert!(
                !本文.contains("alert"),
                "script の中身が残っています: {本文}"
            );
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn 添付が取り出せる() {
    let 添付あり = b"From: billing@example.com\r\n\
Content-Type: multipart/mixed; boundary=SEP\r\n\
\r\n\
--SEP\r\n\
Content-Type: text/plain\r\n\
\r\n\
\xe6\x9c\xac\xe6\x96\x87\r\n\
--SEP\r\n\
Content-Type: application/pdf\r\n\
Content-Disposition: attachment; filename=\"quote.pdf\"\r\n\
\r\n\
%PDF-1.4\r\n\
--SEP--\r\n";

    let 届いた = to_received(添付あり, 100).unwrap();

    match Reader::new().open_at(&届いた, Level::Attachments).unwrap() {
        View::Attachments { attachments, .. } => {
            assert_eq!(attachments.len(), 1);
            assert_eq!(attachments[0].name(), "quote.pdf");
            assert!(attachments[0].bytes().starts_with(b"%PDF"));
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn 添付の名前をそのまま渡す() {
    // **消毒しない。**したふりをすると、受け取る側が安全だと思って直にパスへ使う。
    // 隔離は Phase 2 の File Quarantine の仕事で、ここではない。
    let 危ない名 = b"From: x@example.com\r\n\
Content-Type: multipart/mixed; boundary=SEP\r\n\
\r\n\
--SEP\r\n\
Content-Type: application/octet-stream\r\n\
Content-Disposition: attachment; filename=\"../../etc/passwd\"\r\n\
\r\n\
root\r\n\
--SEP--\r\n";

    let 届いた = to_received(危ない名, 100).unwrap();

    match Reader::new().open_at(&届いた, Level::Attachments).unwrap() {
        View::Attachments { attachments, .. } => {
            assert_eq!(
                attachments[0].name(),
                "../../etc/passwd",
                "名前が書き換わっています"
            );
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn 差出人が無ければ受け取らない() {
    // 送信元が無いと規則の照合が成り立たない（誰の規則で読むのかが決まらない）。
    let 名無し = b"Subject: hello\r\n\r\nbody\r\n";

    assert_eq!(to_received(名無し, 100).unwrap_err(), Error::NoSender);
}

#[test]
fn 読めない塊は受け取らない() {
    assert_eq!(to_received(b"", 100).unwrap_err(), Error::Unparsable);
}

#[test]
fn 本文が無ければ空で渡す() {
    // 「本文が無い」を「読めなかった」にしない。
    let 本文無し = b"From: x@example.com\r\nSubject: hello\r\n\r\n";
    let 届いた = to_received(本文無し, 100).unwrap();

    match Reader::new().open_at(&届いた, Level::Raw).unwrap() {
        View::Raw { body, .. } => assert!(body.is_empty()),
        other => panic!("{other:?}"),
    }
}
