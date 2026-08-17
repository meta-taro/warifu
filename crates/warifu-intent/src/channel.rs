//! 経路に口を被せたもの。

use warifu_core::PublicKey;
use warifu_net::Session;

use crate::{Error, Intent};

/// 相手 1 人との、口をやり取りする路。
///
/// 下は [`warifu_net::Session`] そのもの。**この層が足すのは「何をしたいか」の形だけ**で、
/// 暗号も相手の確認も下でもう済んでいる。
///
/// # 受け取っても実行しない
///
/// [`Channel::recv`] は [`Intent`] を返すだけで、**何もしない**（`decisions.md` **D5**）。
/// 保存するか・開くか・捨てるかは、呼ぶ側が決める。
///
/// # 送ってすぐ落とさない
///
/// [`Channel::send`] が返るのは「送る列に積んだ」ところまで。
/// 送り終わりなら [`Channel::finish`] を呼ぶ（相手が受け取り切るまで待つ）。
#[derive(Debug)]
pub struct Channel {
    session: Session,
}

impl Channel {
    /// 繋がった経路に口を被せる。
    #[must_use]
    pub fn new(session: Session) -> Self {
        Self { session }
    }

    /// 繋がっている相手の公開鍵。**割符で確定した相手と一致するはず。**
    #[must_use]
    pub fn peer(&self) -> PublicKey {
        self.session.peer()
    }

    /// 生きているかどうか。
    #[must_use]
    pub fn is_open(&self) -> bool {
        self.session.is_open()
    }

    /// 口を 1 つ送る。
    ///
    /// # Errors
    /// 荷物が大きすぎたら [`Error::TooLarge`]。経路が切れたら [`Error::Route`]。
    pub async fn send(&mut self, intent: &Intent) -> Result<(), Error> {
        let 塊 = intent.encode()?;
        self.session.send(&塊).await?;
        Ok(())
    }

    /// 口を 1 つ受け取る。**受け取るだけで、何も実行しない。**
    ///
    /// # Errors
    /// 塊の形が壊れていたら [`Error::Malformed`]。経路が切れたら [`Error::Route`]。
    pub async fn recv(&mut self) -> Result<Intent, Error> {
        let 塊 = self.session.recv().await?;
        Intent::decode(&塊)
    }

    /// 送る側を閉じて、**相手が受け取り切るまで待つ。**
    ///
    /// # Errors
    /// 相手が受け取る前に経路が切れたら [`Error::Route`]。
    pub async fn finish(self) -> Result<(), Error> {
        self.session.finish().await?;
        Ok(())
    }

    /// 下の経路をそのまま取り出す。
    #[must_use]
    pub fn into_session(self) -> Session {
        self.session
    }
}

impl From<Session> for Channel {
    fn from(session: Session) -> Self {
        Self::new(session)
    }
}
