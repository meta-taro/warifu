//! 受信箱とのやり取り。**繋がらない環境でも走る**（baseline §4）。
//!
//! `issues/007` の完了条件「既存の IMAP アカウント 1 つだけで動く」の、
//! **サーバとのやり取りの側**をここで固定する。
//!
//! 実サーバへは繋がない。台本を読ませる流れを渡して、
//! **こちらが何を送るか**と**返ってきたものをどう読むか**を確かめる。

use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use warifu_imap::{Account, Error, Mailbox};

/// 台本どおりに返す流れ。**こちらが書いたものは全部取っておく。**
#[derive(Debug)]
struct 台本 {
    読ませる: Vec<u8>,
    位置: usize,
    書かれた: Arc<Mutex<Vec<u8>>>,
}

impl 台本 {
    fn new(読ませる: Vec<u8>) -> (Self, Arc<Mutex<Vec<u8>>>) {
        let 書かれた = Arc::new(Mutex::new(Vec::new()));
        (
            Self {
                読ませる,
                位置: 0,
                書かれた: Arc::clone(&書かれた),
            },
            書かれた,
        )
    }
}

impl AsyncRead for 台本 {
    fn poll_read(
        mut self: Pin<&mut Self>,
        _: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let 残り = &self.読ませる[self.位置..];
        let n = 残り.len().min(buf.remaining());
        buf.put_slice(&残り[..n]);
        self.位置 += n;
        Poll::Ready(Ok(()))
    }
}

impl AsyncWrite for 台本 {
    fn poll_write(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        self.書かれた.lock().unwrap().extend_from_slice(buf);
        Poll::Ready(Ok(buf.len()))
    }
    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

const 一通: &str = "From: billing@example.com\r\n\
Subject: invoice\r\n\
Content-Type: text/plain; charset=utf-8\r\n\
\r\n\
goukei 12000\r\n";

/// 未読が 1 通ある受信箱の台本。
fn 未読が一通ある() -> Vec<u8> {
    format!(
        "* OK IMAP4rev1 ready\r\n\
         A0001 OK LOGIN completed\r\n\
         * 1 EXISTS\r\n\
         A0002 OK [READ-ONLY] EXAMINE completed\r\n\
         * SEARCH 1\r\n\
         A0003 OK SEARCH completed\r\n\
         * 1 FETCH (BODY[] {{{}}}\r\n{})\r\n\
         A0004 OK FETCH completed\r\n\
         A0005 OK LOGOUT completed\r\n",
        一通.len(),
        一通
    )
    .into_bytes()
}

#[tokio::test]
async fn 未読を取り込んで読み取り層に渡せる() {
    let (流れ, _) = 台本::new(未読が一通ある());
    let mut 受信箱 = Mailbox::login(流れ, "watashi@例", "himitsu").await.unwrap();

    let 届いた = 受信箱.fetch_unseen(50, 1_756_000_000).await.unwrap();

    assert_eq!(届いた.len(), 1);
    assert_eq!(届いた[0].sender().as_str(), "billing@example.com");
    assert_eq!(届いた[0].received_at(), 1_756_000_000, "こちらの時計が入る");
}

#[tokio::test]
async fn 読んだ印を付けない() {
    // BODY[] にするとサーバ側で既読になる。**人がまだ見ていないものを既読にしない。**
    let (流れ, 書かれた) = 台本::new(未読が一通ある());
    let mut 受信箱 = Mailbox::login(流れ, "watashi@例", "himitsu").await.unwrap();
    受信箱.fetch_unseen(50, 100).await.unwrap();

    let 送ったもの = String::from_utf8_lossy(&書かれた.lock().unwrap()).into_owned();

    assert!(
        送ったもの.contains("BODY.PEEK[]"),
        "送った命令: {送ったもの}"
    );
    assert!(
        !送ったもの.contains("FETCH 1 BODY[]"),
        "既読になる形で取りに行っています: {送ったもの}"
    );
}

#[tokio::test]
async fn 受信箱は読むだけで開き_書ける形で開かない() {
    // SELECT は書ける形で開き、「最近届いた」印を落とす。割符は読むだけ（0.1.44）
    let (流れ, 書かれた) = 台本::new(未読が一通ある());
    let mut 受信箱 = Mailbox::login(流れ, "watashi@例", "himitsu").await.unwrap();
    受信箱.fetch_unseen(50, 100).await.unwrap();
    let 送ったもの = String::from_utf8_lossy(&書かれた.lock().unwrap()).into_owned();
    assert!(
        送ったもの.contains("EXAMINE \"INBOX\""),
        "送った命令: {送ったもの}"
    );
    assert!(
        !送ったもの.contains("SELECT"),
        "書ける形で開いています: {送ったもの}"
    );
}

#[tokio::test]
async fn 数を聞くと_通の数と未読の数だけを返し_中身は取らない() {
    let 台 = "* OK ready\r\n\
              A0001 OK LOGIN completed\r\n\
              * 12 EXISTS\r\n\
              A0002 OK [READ-ONLY] EXAMINE completed\r\n\
              * SEARCH 3 7 12\r\n\
              A0003 OK SEARCH completed\r\n";
    let (流れ, 書かれた) = 台本::new(台.as_bytes().to_vec());
    let mut 受信箱 = Mailbox::login(流れ, "watashi@例", "himitsu").await.unwrap();
    let 数 = 受信箱.count().await.unwrap();
    assert_eq!(
        数,
        warifu_imap::数 {
            全部: 12, 未読: 3
        }
    );
    assert!(!String::from_utf8_lossy(&書かれた.lock().unwrap()).contains("FETCH"));
}

#[tokio::test]
async fn 未読が無ければ空で返る() {
    let 台 = "* OK ready\r\n\
              A0001 OK LOGIN completed\r\n\
              * 0 EXISTS\r\n\
              A0002 OK [READ-ONLY] EXAMINE completed\r\n\
              * SEARCH\r\n\
              A0003 OK SEARCH completed\r\n";
    let (流れ, 書かれた) = 台本::new(台.as_bytes().to_vec());
    let mut 受信箱 = Mailbox::login(流れ, "watashi@例", "himitsu").await.unwrap();

    assert!(受信箱.fetch_unseen(50, 100).await.unwrap().is_empty());
    // 取るものが無いのに FETCH を投げない
    assert!(!String::from_utf8_lossy(&書かれた.lock().unwrap()).contains("FETCH"));
}

#[tokio::test]
async fn ログインに失敗したら理由に利用者名を載せない() {
    // 失敗の理由は残すが、**誰の資格情報かはログに残さない**（baseline §14）。
    let 台 = "* OK ready\r\nA0001 NO [AUTHENTICATIONFAILED] Authentication failed for watashi\r\n";
    let (流れ, _) = 台本::new(台.as_bytes().to_vec());

    let 結果 = Mailbox::login(流れ, "watashi", "himitsu").await;

    match 結果 {
        Err(Error::Imap(理由)) => {
            assert!(
                !理由.contains("watashi"),
                "利用者名が理由に残っています: {理由}"
            );
            assert!(
                !理由.contains("himitsu"),
                "パスワードが理由に残っています: {理由}"
            );
        }
        Err(e) => panic!("別の理由で落ちました: {e:?}"),
        Ok(_) => panic!("ログインが失敗しませんでした"),
    }
}

#[test]
fn パスワードが表示に出ない() {
    let 繋ぎ先 = Account::new("imap.example.com", 993, "watashi@例", "HIMITSU-9f3a").unwrap();

    let 出力 = format!("{繋ぎ先:?}");
    assert!(
        !出力.contains("HIMITSU-9f3a"),
        "パスワードが出ています: {出力}"
    );
    assert!(
        出力.contains("imap.example.com"),
        "繋ぎ先は見えてよい: {出力}"
    );
}

#[test]
fn 資格情報が欠けていれば受け取らない() {
    assert_eq!(
        Account::new("", 993, "u", "p").unwrap_err(),
        Error::NoCredentials
    );
    assert_eq!(
        Account::new("h", 993, "", "p").unwrap_err(),
        Error::NoCredentials
    );
    assert_eq!(
        Account::new("h", 993, "u", "").unwrap_err(),
        Error::NoCredentials
    );
}

/// 3 通ある受信箱で、新しい 2 通の見出しを返す台本（0.1.44）。
fn 見出しの台本() -> Vec<u8> {
    let h2 = "From: Yamada <a@example.com>\r\nSubject: one\r\n\r\n";
    let h3 = "From: b@example.com\r\nSubject: two\r\n\r\n";
    format!(
        "* OK ready\r\n\
         A0001 OK LOGIN completed\r\n\
         * 3 EXISTS\r\n\
         A0002 OK [READ-ONLY] EXAMINE completed\r\n\
         * 2 FETCH (UID 20 FLAGS (\\Seen) INTERNALDATE \"05-Nov-2026 11:42:00 +0900\" BODY[HEADER] {{{}}}\r\n{})\r\n\
         * 3 FETCH (UID 30 FLAGS () INTERNALDATE \"05-Nov-2026 12:00:00 +0900\" BODY[HEADER] {{{}}}\r\n{})\r\n\
         A0003 OK FETCH completed\r\n",
        h2.len(),
        h2,
        h3.len(),
        h3
    )
    .into_bytes()
}

#[tokio::test]
async fn 一覧は新しい順に見出しだけを返し_本文を取らず既読にもしない() {
    let (流れ, 書かれた) = 台本::new(見出しの台本());
    let mut 受信箱 = Mailbox::login(流れ, "watashi@例", "himitsu").await.unwrap();
    let 行 = 受信箱.一覧(2).await.unwrap();

    assert_eq!(行.len(), 2);
    assert_eq!(
        (行[0].uid, 行[0].件名.as_str(), 行[0].既読),
        (30, "two", false)
    );
    assert_eq!(
        (行[1].uid, 行[1].名.as_deref(), 行[1].既読),
        (20, Some("Yamada"), true)
    );
    // サーバが受け取った時刻（2026-11-05 03:00:00Z）
    assert_eq!(行[0].届いた, Some(1_793_847_600));

    let 送った = String::from_utf8_lossy(&書かれた.lock().unwrap()).into_owned();
    assert!(
        送った.contains("FETCH 2:3 (UID FLAGS INTERNALDATE BODY.PEEK[HEADER])"),
        "{送った}"
    );
    assert!(
        !送った.contains("BODY[]"),
        "本文を取りに行っています: {送った}"
    );
}

#[tokio::test]
async fn 受信箱が空なら一覧は何も取りに行かない() {
    let 台 = "* OK ready\r\nA0001 OK LOGIN completed\r\n* 0 EXISTS\r\nA0002 OK [READ-ONLY] EXAMINE completed\r\n";
    let (流れ, 書かれた) = 台本::new(台.as_bytes().to_vec());
    let mut 受信箱 = Mailbox::login(流れ, "watashi@例", "himitsu").await.unwrap();
    assert!(受信箱.一覧(50).await.unwrap().is_empty());
    assert!(!String::from_utf8_lossy(&書かれた.lock().unwrap()).contains("FETCH"));
}

#[tokio::test]
async fn 一通を開いても既読にせず_外の画像は数えるだけ() {
    let 生 = "From: a@example.com\r\nContent-Type: text/html; charset=utf-8\r\n\r\n<p>mitsumori</p><img src=\"https://t.example.com/o.gif\">\r\n";
    let 台 = format!(
        "* OK ready\r\nA0001 OK LOGIN completed\r\n* 3 EXISTS\r\nA0002 OK [READ-ONLY] EXAMINE completed\r\n\
         * 3 FETCH (UID 30 BODY[] {{{}}}\r\n{})\r\nA0003 OK FETCH completed\r\n",
        生.len(),
        生
    );
    let (流れ, 書かれた) = 台本::new(台.into_bytes());
    let mut 受信箱 = Mailbox::login(流れ, "watashi@例", "himitsu").await.unwrap();
    let 開いた = 受信箱.開く(30).await.unwrap();

    assert_eq!(開いた.外の画像, 1);
    assert!(
        開いた.本文.contains("mitsumori") && !開いた.本文.contains("<img"),
        "{}",
        開いた.本文
    );
    let 送った = String::from_utf8_lossy(&書かれた.lock().unwrap()).into_owned();
    assert!(送った.contains("UID FETCH 30 BODY.PEEK[]"), "{送った}");
}
