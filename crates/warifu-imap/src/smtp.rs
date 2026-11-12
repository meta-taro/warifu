//! **SMTP で 1 通送る**（D132・11-12）。エージェントが書いた返信を、人が画面で押してから送る道。
//!
//! - 繋ぐのは受信箱と同じ TLS（[`crate::connect`]）。Gmail は `smtp.gmail.com:465`（はじめから TLS）
//! - 資格は受信箱と同じアプリパスワード（`AUTH PLAIN`）。**新しい秘密を増やさない**
//! - **見出しに改行を入れさせない**（宛先・件名に改行があると、見出しを差し込まれる）
//! - 件名と本文は UTF-8。件名は MIME の encoded-word、本文は base64 で運ぶ（7bit の経路でも崩れない）
//! - 返信なら `In-Reply-To` と `References` を付け、相手のメーラーで同じスレッドに並ぶようにする

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as B64;
use tokio::io::{AsyncBufReadExt as _, AsyncRead, AsyncWrite, AsyncWriteExt as _, BufReader};

use crate::Error;

/// 送る 1 通。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct 送る通 {
    /// 差出人（口のアドレス）。
    pub 差出人: String,
    /// 宛先（1 つ以上）。
    pub 宛先: Vec<String>,
    /// 件名。
    pub 件名: String,
    /// 本文（文字だけ）。
    pub 本文: String,
    /// 返信なら、元の通の `Message-ID`（`<…>` 込み）。
    pub 返信先: Option<String>,
    /// 返信なら、元の通の `References`（無ければ `返信先` だけ）。
    pub 参照: Option<String>,
}

/// 見出しに入れてよい字か（改行・制御字を入れさせない）。
fn 見出しに入れてよいか(s: &str) -> bool {
    !s.chars().any(|c| c.is_control())
}

/// アドレスとして読めるか（`a@b` の形・空白や括弧や区切りを含まない）。**送り先を差し込ませない。**
fn アドレスか(s: &str) -> bool {
    let Some((前, 後)) = s.split_once('@') else {
        return false;
    };
    !前.is_empty()
        && 後.contains('.')
        && !s
            .chars()
            .any(|c| c.is_whitespace() || c.is_control() || "<>,;:\"()[]\\".contains(c))
}

/// **形を検める。**おかしければ人が読む理由で断る。
///
/// # Errors
/// 宛先が無い・アドレスとして読めない・見出しに改行がある・本文が空。
pub fn 検める(通: &送る通) -> Result<(), String> {
    if 通.宛先.is_empty() {
        return Err("宛先がありません".to_owned());
    }
    for a in std::iter::once(&通.差出人).chain(&通.宛先) {
        if !アドレスか(a) {
            return Err(format!("アドレスとして読めません: {a}"));
        }
    }
    for 見出し in [Some(&通.件名), 通.返信先.as_ref(), 通.参照.as_ref()]
        .into_iter()
        .flatten()
    {
        if !見出しに入れてよいか(見出し) {
            return Err("件名や返信先に改行は入れられません".to_owned());
        }
    }
    if 通.本文.trim().is_empty() {
        return Err("本文が空です".to_owned());
    }
    Ok(())
}

/// 件名を MIME の encoded-word にする（ASCII だけならそのまま）。
fn 件名を包む(件名: &str) -> String {
    if 件名.is_ascii() {
        return 件名.to_owned();
    }
    format!("=?UTF-8?B?{}?=", B64.encode(件名.as_bytes()))
}

/// 本文の base64 を 76 字ごとに折る（RFC 2045）。
fn 折る(字: &str) -> String {
    字.as_bytes()
        .chunks(76)
        .map(|c| String::from_utf8_lossy(c).into_owned())
        .collect::<Vec<_>>()
        .join("\r\n")
}

/// **1 通を RFC 5322 の形に組む**（純粋な関数・試験で見る）。`日時` は RFC 2822 の形、`id` は `Message-ID`。
#[must_use]
pub fn 組む(通: &送る通, 日時: &str, id: &str) -> String {
    let mut 見出し = vec![
        format!("From: {}", 通.差出人),
        format!("To: {}", 通.宛先.join(", ")),
        format!("Subject: {}", 件名を包む(&通.件名)),
        format!("Date: {日時}"),
        format!("Message-ID: {id}"),
    ];
    if let Some(先) = &通.返信先 {
        見出し.push(format!("In-Reply-To: {先}"));
        見出し.push(format!("References: {}", 通.参照.as_deref().unwrap_or(先)));
    }
    見出し.push("MIME-Version: 1.0".to_owned());
    見出し.push("Content-Type: text/plain; charset=UTF-8".to_owned());
    見出し.push("Content-Transfer-Encoding: base64".to_owned());
    let 本文 = 通.本文.replace("\r\n", "\n").replace('\n', "\r\n");
    format!(
        "{}\r\n\r\n{}\r\n",
        見出し.join("\r\n"),
        折る(&B64.encode(本文.as_bytes()))
    )
}

/// サーバの返りを 1 つ読む（`250-…` の続きの行は読み飛ばし、最後の行の番号を返す）。
async fn 返りを読む<R: AsyncRead + Unpin>(
    読む: &mut BufReader<R>,
) -> Result<(u16, String), Error> {
    loop {
        let mut 行 = String::new();
        let n = 読む
            .read_line(&mut 行)
            .await
            .map_err(|e| Error::Network(e.to_string()))?;
        if n == 0 {
            return Err(Error::Smtp("サーバが途中で切りました".to_owned()));
        }
        let 番号: u16 = 行
            .get(..3)
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| Error::Smtp("返りが読めません".to_owned()))?;
        // `250-` は続きがある。`250 ` が最後の行
        if 行.as_bytes().get(3) != Some(&b'-') {
            // **返りの頭だけ残す**（長い文言に何が入っているか分からない）
            let 頭: String = 行.trim_end().chars().take(80).collect();
            return Ok((番号, 頭));
        }
    }
}

async fn 言って待つ<S: AsyncRead + AsyncWrite + Unpin>(
    流れ: &mut BufReader<S>,
    行: &str,
    待つ番号: &[u16],
    何を: &str,
) -> Result<(), Error> {
    流れ
        .get_mut()
        .write_all(format!("{行}\r\n").as_bytes())
        .await
        .map_err(|e| Error::Network(e.to_string()))?;
    let (番号, 頭) = 返りを読む(流れ).await?;
    if 待つ番号.contains(&番号) {
        Ok(())
    } else {
        Err(Error::Smtp(format!("{何を}で断られました（{頭}）")))
    }
}

/// **SMTP で送る。**`流れ` は繋いだ TLS（[`crate::connect`]）。`利用者`・`パスワード` は口の資格。
///
/// # Errors
/// 形がおかしい（[`Error::Smtp`]・送る前に止める）・認証で断られた・宛先で断られた・経路で落ちた。
/// **パスワードは誤りの文に載せない。**
pub async fn 送る<S: AsyncRead + AsyncWrite + Unpin>(
    流れ: S,
    利用者: &str,
    パスワード: &str,
    通: &送る通,
    日時: &str,
    id: &str,
) -> Result<(), Error> {
    検める(通).map_err(Error::Smtp)?;
    let mut 流れ = BufReader::new(流れ);
    let (番号, 頭) = 返りを読む(&mut 流れ).await?;
    if 番号 != 220 {
        return Err(Error::Smtp(format!("挨拶が違います（{頭}）")));
    }
    言って待つ(&mut 流れ, "EHLO warifu", &[250], "挨拶").await?;
    let 資格 = B64.encode(format!("\0{利用者}\0{パスワード}").as_bytes());
    言って待つ(&mut 流れ, &format!("AUTH PLAIN {資格}"), &[235], "認証").await?;
    言って待つ(
        &mut 流れ,
        &format!("MAIL FROM:<{}>", 通.差出人),
        &[250],
        "差出人",
    )
    .await?;
    for 先 in &通.宛先 {
        言って待つ(&mut 流れ, &format!("RCPT TO:<{先}>"), &[250, 251], "宛先").await?;
    }
    言って待つ(&mut 流れ, "DATA", &[354], "本文の前").await?;
    // 本文は base64 なので、行頭の `.` は出てこない（点の重ねは要らない）
    let 中身 = 組む(通, 日時, id);
    言って待つ(&mut 流れ, &format!("{中身}."), &[250], "本文").await?;
    // 閉じるときの返りは待たない（送れたことは本文の 250 で決まっている）
    let _ = 流れ.get_mut().write_all(b"QUIT\r\n").await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::pin::Pin;
    use std::sync::{Arc, Mutex};
    use std::task::{Context, Poll};
    use tokio::io::ReadBuf;

    fn 通() -> 送る通 {
        送る通 {
            差出人: "me@example.com".to_owned(),
            宛先: vec!["you@example.org".to_owned()],
            件名: "Re: 見積の件".to_owned(),
            本文: "確かめました。\n明日お送りします。".to_owned(),
            返信先: Some("<abc@example.org>".to_owned()),
            参照: None,
        }
    }

    #[test]
    fn 返信は_同じスレッドに並ぶ見出しを付け_件名と本文は_utf8_で運ぶ() {
        let 組んだ = 組む(&通(), "Thu, 12 Nov 2026 14:00:00 +0900", "<1@warifu>");
        assert!(組んだ.contains("In-Reply-To: <abc@example.org>\r\n"));
        assert!(組んだ.contains("References: <abc@example.org>\r\n"));
        assert!(組んだ.contains("Subject: =?UTF-8?B?"));
        assert!(組んだ.contains("Content-Transfer-Encoding: base64"));
        let 本文の字 = 組んだ.split("\r\n\r\n").nth(1).unwrap().replace("\r\n", "");
        let 戻した = String::from_utf8(B64.decode(本文の字).unwrap()).unwrap();
        assert_eq!(戻した, "確かめました。\r\n明日お送りします。");
    }

    #[test]
    fn 見出しに改行や_読めないアドレスがあれば_送らない() {
        let mut 件名に改行 = 通();
        件名に改行.件名 = "a\r\nBcc: x@example.net".to_owned();
        assert!(検める(&件名に改行).is_err());
        for 悪い in [
            "you@example.org\r\nBcc: x@example.net",
            "a b@example.org",
            "<x@example.net>",
            "nobody",
        ] {
            let mut 宛先 = 通();
            宛先.宛先 = vec![悪い.to_owned()];
            assert!(検める(&宛先).is_err(), "{悪い:?}");
        }
        let mut 空 = 通();
        空.宛先.clear();
        assert!(検める(&空).is_err());
        assert!(検める(&通()).is_ok());
    }

    struct 台本 {
        読ませる: Vec<u8>,
        位置: usize,
        書かれた: Arc<Mutex<Vec<u8>>>,
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
            b: &[u8],
        ) -> Poll<std::io::Result<usize>> {
            self.書かれた.lock().unwrap().extend_from_slice(b);
            Poll::Ready(Ok(b.len()))
        }
        fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<std::io::Result<()>> {
            Poll::Ready(Ok(()))
        }
        fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<std::io::Result<()>> {
            Poll::Ready(Ok(()))
        }
    }

    fn 台本で(返り: &str) -> (台本, Arc<Mutex<Vec<u8>>>) {
        let 書かれた = Arc::new(Mutex::new(Vec::new()));
        (
            台本 {
                読ませる: 返り.as_bytes().to_vec(),
                位置: 0,
                書かれた: Arc::clone(&書かれた),
            },
            書かれた,
        )
    }

    #[tokio::test]
    async fn 認証して_差出人と宛先と本文を渡し_250_で送れたとする() {
        let (流れ, 書かれた) = 台本で(
            "220 smtp.example ready\r\n250-smtp.example\r\n250-AUTH PLAIN\r\n250 OK\r\n235 ok\r\n250 ok\r\n250 ok\r\n354 go\r\n250 queued\r\n221 bye\r\n",
        );
        送る(
            流れ,
            "me@example.com",
            "himitsu",
            &通(),
            "Thu, 12 Nov 2026 14:00:00 +0900",
            "<1@warifu>",
        )
        .await
        .unwrap();
        let 送った = String::from_utf8(書かれた.lock().unwrap().clone()).unwrap();
        assert!(送った.contains("EHLO warifu\r\n"));
        assert!(送った.contains(&format!(
            "AUTH PLAIN {}\r\n",
            B64.encode("\0me@example.com\0himitsu")
        )));
        assert!(送った.contains("MAIL FROM:<me@example.com>\r\n"));
        assert!(送った.contains("RCPT TO:<you@example.org>\r\n"));
        assert!(送った.contains("\r\n.\r\n"), "本文の終わりの点");
    }

    #[tokio::test]
    async fn 認証で断られたら_送らずに理由を返し_パスワードを載せない() {
        let (流れ, 書かれた) =
            台本で("220 ready\r\n250 OK\r\n535 5.7.8 Username and Password not accepted\r\n");
        let 誤り = 送る(流れ, "me@example.com", "himitsu", &通(), "x", "<1@warifu>")
            .await
            .unwrap_err()
            .to_string();
        assert!(誤り.contains("認証"), "{誤り}");
        assert!(!誤り.contains("himitsu"));
        let 送った = String::from_utf8(書かれた.lock().unwrap().clone()).unwrap();
        assert!(!送った.contains("MAIL FROM"), "断られたら先へ進まない");
    }
}
