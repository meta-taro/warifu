//! 受信箱。**未読を取るが、読んだ印は付けない。**

use core::fmt;

use async_imap::Session;
use futures::StreamExt as _;
use tokio::io::{AsyncRead, AsyncWrite};
use warifu_read::Received;

use crate::convert::{人が読む形にする, 見出し, 見出しにする, 開いた通};
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
        self.読むだけで開く().await?;

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

    /// **一覧**（0.1.44）。新しいほうから `件数` 通の、**ヘッダだけ**を返す（新しい順）。
    ///
    /// 本文は取らない（`BODY.PEEK[HEADER]`）。既読の印も付けない。
    /// 時刻は**サーバが受け取った時刻**（`INTERNALDATE`）で、相手の `Date` ではない。
    ///
    /// # 失敗
    ///
    /// [`Error::Imap`]。ヘッダの読めない通は [`Error::Unparsable`] / [`Error::NoSender`] で止める。
    pub async fn 一覧(&mut self, 件数: usize) -> Result<Vec<見出し>, Error> {
        let 全部 = self.読むだけで開く().await?;
        if 全部 == 0 || 件数 == 0 {
            return Ok(Vec::new());
        }
        let 件数 = u32::try_from(件数).unwrap_or(u32::MAX);
        let 先頭 = 全部.saturating_sub(件数) + 1;
        let mut 流れ = self
            .session
            .fetch(
                format!("{先頭}:{全部}"),
                "(UID FLAGS INTERNALDATE BODY.PEEK[HEADER])",
            )
            .await
            .map_err(|e| Error::Imap(e.to_string()))?;
        let mut 行 = Vec::new();
        while let Some(一通) = 流れ.next().await {
            let 一通 = 一通.map_err(|e| Error::Imap(e.to_string()))?;
            let (Some(uid), Some(ヘッダ)) = (一通.uid, 一通.header()) else {
                return Err(Error::Imap("UID かヘッダが返りませんでした".to_owned()));
            };
            let 既読 = 一通
                .flags()
                .any(|f| matches!(f, async_imap::types::Flag::Seen));
            let 届いた = 一通.internal_date().map(|d| d.timestamp());
            行.push(見出しにする(uid, ヘッダ, 届いた, 既読)?);
        }
        行.reverse();
        Ok(行)
    }

    /// **1 通を開く**（0.1.44）。`BODY.PEEK[]` で取るので**既読にしない。**
    ///
    /// 返りは**人が読む形**（文字の本文・外の画像の数・添付の名前）。エージェントへは渡さない。
    ///
    /// # 失敗
    ///
    /// [`Error::Imap`]（その UID が無いときも）/ [`Error::Unparsable`] / [`Error::NoSender`]。
    pub async fn 開く(&mut self, uid: u32) -> Result<開いた通, Error> {
        self.読むだけで開く().await?;
        let mut 流れ = self
            .session
            .uid_fetch(uid.to_string(), "BODY.PEEK[]")
            .await
            .map_err(|e| Error::Imap(e.to_string()))?;
        while let Some(一通) = 流れ.next().await {
            let 一通 = 一通.map_err(|e| Error::Imap(e.to_string()))?;
            if let Some(生) = 一通.body() {
                return 人が読む形にする(生);
            }
        }
        Err(Error::Imap(format!("UID {uid} の通がありません")))
    }

    /// **受信箱の通の数と、未読の数**（0.1.44）。件名も本文も取らない。
    ///
    /// つないで読めるかを、人が中身を見ずに確かめるための口（`warifu mail check`）。
    ///
    /// # 失敗
    ///
    /// [`Error::Imap`]。
    pub async fn count(&mut self) -> Result<数, Error> {
        let 箱 = self.読むだけで開く().await?;
        let 未読 = self
            .session
            .search("UNSEEN")
            .await
            .map_err(|e| Error::Imap(e.to_string()))?
            .len();
        Ok(数 {
            全部: 箱, 未読
        })
    }

    /// **受信箱を読むだけで開く**（`EXAMINE`）。返りは通の数。
    ///
    /// `SELECT` で開くと、サーバは「最近届いた」印を落とし、書ける形で開く。
    /// 割符は読むだけなので、**受信箱に何の跡も付けない**開き方にする（0.1.44）。
    async fn 読むだけで開く(&mut self) -> Result<u32, Error> {
        self.session
            .examine("INBOX")
            .await
            .map(|箱| 箱.exists)
            .map_err(|e| Error::Imap(e.to_string()))
    }

    /// 閉じる。
    pub async fn logout(&mut self) -> Result<(), Error> {
        self.session
            .logout()
            .await
            .map_err(|e| Error::Imap(e.to_string()))
    }
}

/// 受信箱の数（[`Mailbox::count`]）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct 数 {
    /// 受信箱の通の数。
    pub 全部: u32,
    /// そのうち未読。
    pub 未読: usize,
}

/// 文言に混ざった利用者名を伏せる。
///
/// 失敗の理由は残すが、**誰の資格情報かはログに残さない。**
fn 伏せる(文言: &str, 利用者: &str) -> String {
    文言.replace(利用者, "（伏せる）")
}
