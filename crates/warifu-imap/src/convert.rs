//! 生のメールを `warifu-read` の入口に合わせる。

use mail_parser::{MessageParser, MimeHeaders};
use warifu_read::{Attachment, Body, Claims, Received, SenderId, Source};

use crate::Error;

/// 生の RFC 5322 バイト列を 1 通に直す。
///
/// `received_at` は**呼ぶ側が渡す**。**`Date` ヘッダを使わない。**
/// 相手が書いてきた日時は申し送りであって、こちらが受け取った事実ではない。
///
/// # この関数が引き受けること
///
/// - 差出人を取り出す（無ければ [`Error::NoSender`]）
/// - 本文を **text にして**渡す。HTML しか無ければタグを落とす
///   （HTML をそのまま読み手へ渡さない・PRD §12-2）
/// - ヘッダを [`Claims`] として全部残す。**判断には使われない**
/// - 添付を取り出す。**名前は書き換えない**（下記）
///
/// # 添付の名前を消毒しない
///
/// `../../etc/passwd` のような名前も、そのまま渡す。
/// **消毒したふりをすると、受け取る側が安全だと思って直にパスへ使う。**
/// 隔離は File Quarantine（roadmap Phase 2）の仕事で、ここではない。
///
/// # 失敗
///
/// [`Error::Unparsable`] / [`Error::NoSender`]。
pub fn to_received(raw: &[u8], received_at: u64) -> Result<Received, Error> {
    let msg = MessageParser::default()
        .parse(raw)
        .ok_or(Error::Unparsable)?;

    let 差出人 = msg
        .from()
        .and_then(|a| a.first())
        .and_then(|a| a.address())
        .ok_or(Error::NoSender)?;

    // body_text は text 部を返し、HTML しか無ければタグを落として返す。
    // **HTML のまま渡さない**のが要点で、ここを緩めると読み手が HTML を解釈することになる
    let 本文 = msg.body_text(0).unwrap_or_default().into_owned();

    let mut 申し送り = Claims::new();
    for h in msg.headers() {
        申し送り = 申し送り.with(h.name(), &値を文字列にする(&h.value));
    }

    let 添付 = msg
        .attachments()
        .map(|p| {
            Attachment::new(
                p.attachment_name().unwrap_or("（名前なし）"),
                p.contents().to_vec(),
            )
        })
        .collect();

    Ok(Received::new(
        Source::Imap,
        SenderId::new(差出人)?,
        received_at,
        Body::new(本文.into_bytes()),
    )
    .with_claims(申し送り)
    .with_attachments(添付))
}

/// ヘッダの値を、**申し送りとして残すためだけ**の文字列にする。
///
/// ここで意味を取らない。判断に使わないものを丁寧に解釈しても、
/// 使わないことに変わりはない。
fn 値を文字列にする(v: &mail_parser::HeaderValue<'_>) -> String {
    use mail_parser::HeaderValue;
    match v {
        HeaderValue::Text(t) => t.to_string(),
        HeaderValue::TextList(l) => l.join(" "),
        HeaderValue::Address(a) => a
            .iter()
            .filter_map(|x| x.address())
            .collect::<Vec<_>>()
            .join(" "),
        HeaderValue::DateTime(d) => d.to_rfc3339(),
        _ => String::new(),
    }
}
