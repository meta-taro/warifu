//! 受信箱。**未読を取るが、読んだ印は付けない。**

use core::fmt;

use async_imap::Session;
use futures::StreamExt as _;
use tokio::io::{AsyncRead, AsyncWrite};
use warifu_read::Received;

use crate::{Account, Error, to_received};

/// 一度に取り込む上限の既定。
///
/// 上限を置くのは、**受信箱が大きい人が最初の 1 回で止まらないため**。
/// 全部取るという選択肢は置かない。
pub const DEFAULT_LIMIT: usize = 50;

/// 繋がった受信箱。
///
/// 流れを差し替えられるようにしてあるので、**繋がらない環境でも試験が走る**（baseline §4）。
pub struct Mailbox<T: AsyncRead + AsyncWrite + Unpin + fmt::Debug + Send> {
    session: Session<T>,
}

impl<T: AsyncRead + AsyncWrite + Unpin + fmt::Debug + Send> Mailbox<T> {
    /// 繋ぎ先を使ってログインする。
    ///
    /// **パスワードは [`Account`] の外へ出ない。**呼ぶ側が触らずに済む形にしてある。
    pub async fn open(stream: T, account: &Account) -> Result<Self, Error> {
        Self::login(stream, account.user(), account.password()).await
    }

    /// 利用者名とパスワードを直に渡してログインする。
    ///
    /// 通常は [`Mailbox::open`] を使うこと。**こちらは秘密情報が呼ぶ側の変数に載る。**
    ///
    /// # 失敗
    ///
    /// [`Error::Imap`]。**理由に資格情報を載せない。**
    pub async fn login(stream: T, user: &str, password: &str) -> Result<Self, Error> {
        let client = async_imap::Client::new(stream);
        let session = client
            .login(user, password)
            .await
            // 下の層の文言には利用者名が入りうるので、**そのまま載せない**
            .map_err(|(e, _)| Error::Imap(伏せる(&e.to_string(), user)))?;
        Ok(Self { session })
    }

    /// 未読を取り込む。
    ///
    /// `received_at` は**こちらの時計**で、呼ぶ側が渡す（`Date` ヘッダを使わない・**D22**）。
    /// 1 回の取り込みぶんは同じ時刻になる。
    ///
    /// # 読んだ印を付けない
    ///
    /// `BODY.PEEK[]` を使う。`BODY[]` にすると**サーバ側で既読になる。**
    /// 人がまだ見ていないものを、取り込んだだけで既読にしてはいけない。
    ///
    /// # 失敗
    ///
    /// [`Error::Imap`] / [`Error::Unparsable`] / [`Error::NoSender`]。
    /// **1 通が読めなくても、そこで全部を止める。**黙って捨てると、
    /// 取り込めていない通が誰にも見えなくなる。
    pub async fn fetch_unseen(
        &mut self,
        limit: usize,
        received_at: u64,
    ) -> Result<Vec<Received>, Error> {
        self.session
            .select("INBOX")
            .await
            .map_err(|e| Error::Imap(e.to_string()))?;

        let mut 番号: Vec<u32> = self
            .session
            .search("UNSEEN")
            .await
            .map_err(|e| Error::Imap(e.to_string()))?
            .into_iter()
            .collect();
        if 番号.is_empty() {
            return Ok(Vec::new());
        }
        // 新しいほうから取る。古い順に並べてから後ろを切る
        番号.sort_unstable();
        let 取る = &番号[番号.len().saturating_sub(limit)..];
        let 並び = 取る
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(",");

        let mut 流れ = self
            .session
            .fetch(並び, "BODY.PEEK[]")
            .await
            .map_err(|e| Error::Imap(e.to_string()))?;

        let mut 届いた = Vec::new();
        while let Some(一通) = 流れ.next().await {
            let 一通 = 一通.map_err(|e| Error::Imap(e.to_string()))?;
            let Some(生) = 一通.body() else { continue };
            届いた.push(to_received(生, received_at)?);
        }
        Ok(届いた)
    }

    /// 閉じる。
    pub async fn logout(&mut self) -> Result<(), Error> {
        self.session
            .logout()
            .await
            .map_err(|e| Error::Imap(e.to_string()))
    }
}

/// 文言に混ざった利用者名を伏せる。
///
/// 失敗の理由は残すが、**誰の資格情報かはログに残さない。**
fn 伏せる(文言: &str, 利用者: &str) -> String {
    文言.replace(利用者, "（伏せる）")
}
