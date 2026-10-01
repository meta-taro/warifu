//! 既定で本文を返さない、という約束。
//!
//! `issues/007` の完了条件の 2 つ目
//! 「**Level 0 で返した内容に本文が 1 文字も含まれていない**」をここで固定する。
//!
//! 送信者が優先度を申告できないことも、ここで固定する（`decisions.md` **D5**）。
//! 申告できるなら全員が「緊急」を付けるので、並べ替えが役に立たなくなる。

use warifu_read::{Body, Claims, Kind, Level, Priority, Reader, Received, SenderId, Source, View};

/// 本文に入れておく目印。**どこにも漏れてはいけない文字列。**
const 目印: &str = "SHIRUSHI-本文-9f3a";

fn 一通(claims: Claims) -> Received {
    Received::new(
        Source::Imap,
        SenderId::new("billing@例").unwrap(),
        1_756_000_000,
        Body::new(format!("請求書です {目印} 合計 12,000 円").into_bytes()),
    )
    .with_claims(claims)
}

#[test]
fn 既定では本文を返さない() {
    let 見え方 = Reader::new().read(&一通(Claims::new()));

    assert_eq!(見え方.level(), Level::Metadata);
    assert!(matches!(見え方, View::Metadata(_)), "既定は Level 0 だけ");
}

#[test]
fn level_0_の返りに本文が_1_文字も入らない() {
    // 型の上で入る場所が無いことに加えて、**表示にも出ないこと**を確かめる。
    // ここが破れると、Level 0 で返した意味がログ側で消える。
    let 見え方 = Reader::new().read(&一通(Claims::new()));

    let 出力 = format!("{見え方:?}");
    assert!(
        !出力.contains(目印),
        "Level 0 の Debug に本文が出ています: {出力}"
    );
}

#[test]
fn 受け取ったものの_debug_に本文が出ない() {
    let 届いた = 一通(Claims::new().with("Subject", 目印));

    let 出力 = format!("{届いた:?}");
    assert!(
        !出力.contains(目印),
        "Received の Debug に中身が出ています: {出力}"
    );
}

#[test]
fn 送信者は優先度を申告できない() {
    // 送る側が自由に書けるヘッダ。**読む側の判断は動かない。**
    let 申告 = Claims::new()
        .with("X-Priority", "1")
        .with("Importance", "high")
        .with("Priority", "urgent");

    let 見え方 = Reader::new().read(&一通(申告));

    assert_eq!(
        見え方.metadata().priority(),
        Priority::Normal,
        "送信者の申告で優先度が上がっています"
    );
}

#[test]
fn 送信者は人の判断を要求できない() {
    let 申告 = Claims::new()
        .with("X-Action-Required", "true")
        .with("Importance", "high");

    let 見え方 = Reader::new().read(&一通(申告));

    assert!(
        !見え方.metadata().action_required(),
        "送信者の申告で action_required が立っています"
    );
}

#[test]
fn 申し送りは捨てずに持つが判断には使わない() {
    // 無視した事実を後から人が見られるようにする（何を無視したかが分からないと直せない）。
    let 届いた = 一通(Claims::new().with("X-Priority", "1"));
    assert_eq!(届いた.claims().len(), 1);

    let 申し送り無し = Reader::new().read(&一通(Claims::new()));
    let 申し送り有り = Reader::new().read(&届いた);

    assert_eq!(
        申し送り無し.metadata(),
        申し送り有り.metadata(),
        "申し送りの有無で metadata が変わっています"
    );
}

#[test]
fn 規則が無ければ知らない種別のままにする() {
    // 知らないものを知っているふりはしない（warifu-intent::Kind::is_known と同じ姿勢）。
    let 見え方 = Reader::new().read(&一通(Claims::new()));

    assert_eq!(見え方.metadata().kind().as_str(), Kind::UNKNOWN);
    assert!(!見え方.metadata().kind().is_known());
}

#[test]
fn 経路が違っても同じ判断をする() {
    // 読み取りを Adapter の内側に書くと、経路の数だけ同じものを作り直すことになる。
    let 本文 = || Body::new(b"same".to_vec());
    let 差出人 = || SenderId::new("billing@例").unwrap();

    let 郵便 = Reader::new().read(&Received::new(Source::Imap, 差出人(), 100, 本文()));
    let 口 = Reader::new().read(&Received::new(Source::Intent, 差出人(), 100, 本文()));

    assert_eq!(郵便.metadata().kind(), 口.metadata().kind());
    assert_eq!(郵便.metadata().priority(), 口.metadata().priority());
    assert_eq!(
        郵便.metadata().action_required(),
        口.metadata().action_required()
    );
    // 経路そのものは残す（どこから来たかは、人が見るときに要る）
    assert_eq!(郵便.metadata().source(), Source::Imap);
    assert_eq!(口.metadata().source(), Source::Intent);
}

#[test]
fn 時刻はこちらの時計で入る() {
    // 相手が書いてきた日時は申し送りであって事実ではない。
    let 届いた = 一通(Claims::new().with("Date", "Thu, 1 Jan 1970 00:00:00 +0000"));
    let 見え方 = Reader::new().read(&届いた);

    assert_eq!(見え方.metadata().received_at(), 1_756_000_000);
}

#[test]
fn 送信元の形が壊れていれば受け取らない() {
    assert!(SenderId::new("").is_err(), "空の送信元は規則の照合を壊す");
    assert!(SenderId::new(&"a".repeat(321)).is_err());
}

#[test]
fn 種別は正規形しか受け取らない() {
    // 表記が 2 通りあると、同じ種別に規則が二重にできる。
    assert!(Kind::new("invoice.received").is_ok());
    assert!(Kind::new("Invoice.received").is_err(), "大文字");
    assert!(Kind::new(".invoice").is_err(), "先頭の点");
    assert!(Kind::new("invoice.").is_err(), "末尾の点");
    assert!(Kind::new("invoice..received").is_err(), "点の連続");
    assert!(Kind::new("invoice/received").is_err(), "区切り以外の記号");
    assert!(Kind::new("").is_err());
}
