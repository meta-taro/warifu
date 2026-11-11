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

/// [`to_received`] に、**ヘッダだけから付けた仕分けの札**を添える。
///
/// 札は画面の受信箱と同じ `warifu_read::仕分ける`（AI を呼ばない）。`受け手` は
/// [`受け手を推す`] の値で、分からなければ `None`（認証の結果を読まない）。
///
/// # 失敗
///
/// [`to_received`] と同じ。
pub fn 仕分けて直す(
    raw: &[u8],
    received_at: u64,
    受け手: Option<&str>,
) -> Result<Received, Error> {
    let 一通 = to_received(raw, received_at)?;
    let msg = MessageParser::default()
        .parse_headers(raw)
        .ok_or(Error::Unparsable)?;
    let 先 = msg.from().and_then(|a| a.first());
    let 手 = 手がかりを拾う(
        &msg,
        一通.sender().as_str(),
        先.and_then(|a| a.name()),
        msg.subject().unwrap_or_default(),
    );
    Ok(一通.with_仕分け(warifu_read::仕分ける(&手, 受け手)))
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

/// 一覧の 1 行（0.1.44）。**本文は入らない**（ヘッダだけから作る）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct 見出し {
    /// サーバの UID。本文を開くときに使う。
    pub uid: u32,
    /// 差出人のアドレス。
    pub 差出人: String,
    /// 差出人の表示名（相手の申告。無ければ `None`）。
    pub 名: Option<String>,
    /// 件名（相手の申告。無ければ空）。
    pub 件名: String,
    /// **サーバが受け取った時刻**（epoch 秒）。`Date` ヘッダは使わない（相手の申告のため）。
    pub 届いた: Option<i64>,
    /// 既読の印がサーバにあるか。
    pub 既読: bool,
    /// **仕分けの手がかり**（0.1.45）。ヘッダだけから拾う。本文は入らない。
    pub 手がかり: warifu_read::手がかり,
}

/// 認証の結果（`Authentication-Results`）を付けたと信じるサーバの名を、IMAP の繋ぎ先から推す（0.1.45）。
///
/// 分からない繋ぎ先は `None` —— 仕分けは認証の結果を読まない（誰が付けた結果か分からないため）。
#[must_use]
pub fn 受け手を推す(imap_host: &str) -> Option<&'static str> {
    let h = imap_host.trim().to_ascii_lowercase();
    (h == "imap.gmail.com" || h == "imap.googlemail.com").then_some("mx.google.com")
}

/// ヘッダから仕分けの手がかりを拾う。`Authentication-Results` は**上から順**に持つ。
fn 手がかりを拾う(
    msg: &mail_parser::Message<'_>,
    差出人: &str,
    名: Option<&str>,
    件名: &str,
) -> warifu_read::手がかり {
    let mut 手 = warifu_read::手がかり {
        差出人: 差出人.to_owned(),
        名: 名.map(str::to_owned),
        件名: 件名.to_owned(),
        ..warifu_read::手がかり::default()
    };
    for (名前, 値) in msg.headers_raw() {
        let 値 = 値.split_whitespace().collect::<Vec<_>>().join(" ");
        match 名前.to_ascii_lowercase().as_str() {
            "authentication-results" => 手.認証.push(値),
            "list-unsubscribe" => 手.配信停止 = true,
            "precedence" => 手.優先 = Some(値),
            "x-spam-flag" => 手.迷惑の印 = Some(値),
            _ => {}
        }
    }
    手
}

/// ヘッダだけのバイト列から、一覧の 1 行を作る。
///
/// # 失敗
///
/// [`Error::Unparsable`] / [`Error::NoSender`]。**差出人の無い通を黙って飛ばさない。**
pub fn 見出しにする(
    uid: u32,
    ヘッダ: &[u8],
    届いた: Option<i64>,
    既読: bool,
) -> Result<見出し, Error> {
    let msg = MessageParser::default()
        .parse_headers(ヘッダ)
        .ok_or(Error::Unparsable)?;
    let 先 = msg.from().and_then(|a| a.first()).ok_or(Error::NoSender)?;
    let 差出人 = 先.address().ok_or(Error::NoSender)?.to_owned();
    let 名 = 先
        .name()
        .map(str::to_owned)
        .filter(|s| !s.trim().is_empty());
    let 件名 = msg.subject().unwrap_or_default().to_owned();
    let 手がかり = 手がかりを拾う(&msg, &差出人, 名.as_deref(), &件名);
    Ok(見出し {
        uid,
        差出人,
        名,
        件名,
        届いた,
        既読,
        手がかり,
    })
}

/// **人が読むために開いた 1 通**（0.1.44）。
///
/// エージェントへ渡す道（[`to_received`] → `warifu-read` の段）とは分ける。
/// こちらは画面の前の人が読む本文で、**文字だけ**（HTML は描かない）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct 開いた通 {
    /// 本文（文字）。HTML しか無い通はタグを落としたもの。
    pub 本文: String,
    /// HTML に入っていた外の画像の数（**読み込んでいない**）。
    pub 外の画像: usize,
    /// 添付の名前（**中は読まない**・名前も書き換えない）。
    pub 添付: Vec<String>,
}

/// 生の 1 通を、人が読む形にする。
///
/// # 失敗
///
/// [`Error::Unparsable`]。
pub fn 人が読む形にする(raw: &[u8]) -> Result<開いた通, Error> {
    let msg = MessageParser::default()
        .parse(raw)
        .ok_or(Error::Unparsable)?;
    Ok(開いた通 {
        本文: msg.body_text(0).unwrap_or_default().into_owned(),
        外の画像: 外の画像の数(raw),
        添付: msg
            .attachments()
            .map(|p| p.attachment_name().unwrap_or("（名前なし）").to_owned())
            .collect(),
    })
}

/// **外から読み込む画像の数**（0.1.44・3-c の最初）。
///
/// 割符は本文を文字にして見せ、HTML を描かない（[`to_received`]）ので、
/// 外の画像は**もともと読み込まない**。ここは「止めたものがある」と人へ言うための数である
/// （開いたことを送り手に知らせる画像が、メールにはよく入っている）。
pub fn 外の画像の数(raw: &[u8]) -> usize {
    let Some(msg) = MessageParser::default().parse(raw) else {
        return 0;
    };
    (0..msg.html_body_count())
        .filter_map(|i| msg.body_html(i))
        .map(|html| 外の画像を数える(&html))
        .sum()
}

/// HTML の中の `<img src="http…">`（と `//…`）を数える。
fn 外の画像を数える(html: &str) -> usize {
    let 小 = html.to_ascii_lowercase();
    小.split("<img")
        .skip(1)
        .filter(|tag| {
            let 中 = tag.split('>').next().unwrap_or_default();
            let Some(i) = 中.find("src") else {
                return false;
            };
            let 値 = 中[i + 3..]
                .trim_start()
                .trim_start_matches('=')
                .trim_start()
                .trim_start_matches(['"', '\'']);
            値.starts_with("http://") || 値.starts_with("https://") || 値.starts_with("//")
        })
        .count()
}

#[cfg(test)]
mod 試験 {
    use super::*;

    #[test]
    fn ヘッダから一覧の行を作り_表示名と件名を取る() {
        let h = b"From: =?UTF-8?B?5bGx5pys?= <mitsumori@example.com>\r\nSubject: hello\r\n\r\n";
        let 行 = 見出しにする(42, h, Some(1_700_000_000), false).unwrap();
        assert_eq!(行.uid, 42);
        assert_eq!(行.差出人, "mitsumori@example.com");
        assert_eq!(行.名.as_deref(), Some("山本"));
        assert_eq!(行.件名, "hello");
        assert_eq!(行.届いた, Some(1_700_000_000));
        assert!(!行.既読);
    }

    #[test]
    fn ヘッダから仕分けの手がかりを拾い_認証の結果は上から順に持つ() {
        let h = b"Authentication-Results: mx.google.com;\r\n       dmarc=fail header.from=example.com\r\n\
Authentication-Results: example.net; dmarc=pass\r\n\
List-Unsubscribe: <mailto:u@example.com>\r\n\
Precedence: bulk\r\n\
X-Spam-Flag: NO\r\n\
From: a@example.com\r\nSubject: s\r\n\r\n";
        let 手 = 見出しにする(1, h, None, false).unwrap().手がかり;
        assert_eq!(手.認証.len(), 2);
        assert_eq!(
            手.認証[0],
            "mx.google.com; dmarc=fail header.from=example.com"
        );
        assert!(手.配信停止);
        assert_eq!(手.優先.as_deref(), Some("bulk"));
        assert_eq!(手.迷惑の印.as_deref(), Some("NO"));
        assert_eq!(
            warifu_read::仕分ける(&手, 受け手を推す("imap.gmail.com")).札,
            warifu_read::札::なりすまし疑い
        );
        assert_eq!(受け手を推す("imap.example.com"), None);
    }

    #[test]
    fn 差出人が無い通は黙って飛ばさず_誤りにする() {
        let r = 見出しにする(1, b"Subject: x\r\n\r\n", None, true);
        assert!(matches!(r, Err(Error::NoSender)));
    }

    #[test]
    fn 外の画像だけを数え_埋め込みの画像は数えない() {
        let raw = "From: a@example.com\r\n\
Content-Type: text/html; charset=utf-8\r\n\
\r\n\
<p>hi</p><IMG SRC=\"https://t.example.com/open.gif\"><img src='//cdn.example.com/a.png'>\
<img src=\"cid:logo\"><img alt=x src = http://example.com/b.png>\r\n";
        assert_eq!(外の画像の数(raw.as_bytes()), 3);
    }

    #[test]
    fn 文字だけの通は画像が無い() {
        let raw =
            b"From: a@example.com\r\nContent-Type: text/plain\r\n\r\n<img src=\"https://x\">\r\n";
        // text/plain の中の文字は、画像として読み込まれることがない
        assert_eq!(外の画像の数(raw), 0);
    }
}
