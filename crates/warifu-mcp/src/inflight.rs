//! **返事を返し終えるまで、入力の終わりを伏せる**（2026-10-07・#50）。
//!
//! # なぜ要るのか
//!
//! 呼ぶ側が「頼みを書いて、すぐ入力を閉じる」と、口は**返事を書く前に終わっていた。**
//! 呼ぶ側には返事が来ないので、**同じ頼みを出し直す。**
//! `voice_say` では、**同じ声が相手の機械で何度も流れた**（声は外へ出る）。
//!
//! # 形
//!
//! - [`呼び中`] —— いま走っている呼びの数。`call_tool` の前後で数える
//! - [`閉じを待つ入力`] —— 下の入力が終わっても、**呼びが 0 になるまで終わりを返さない**
//!
//! **待つのは上限まで**（[`待つ上限`]）。返らない呼びのせいで、口が残り続けないようにする。

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll};
use std::time::Duration;

use tokio::io::{AsyncRead, ReadBuf};
use tokio::sync::Notify;

/// 入力が閉じたあと、返事を待つ上限。
///
/// **いちばん長い呼びより長くする** —— `pass_wait` と `chat_wait` は最長 60 秒、
/// `voice_say` は読み上げの長さだけかかる。
pub const 待つ上限: Duration = Duration::from_secs(160);

/// 入力が閉じたあと、読んだ頼みが走り出すまで待つ間。
const 走り出す間: Duration = Duration::from_millis(300);

/// いま走っている呼びの数。
#[derive(Debug, Default)]
pub struct 呼び中 {
    数: AtomicUsize,
    減った: Notify,
}

/// 呼びが終わったら（落ちても）数を戻す。
#[derive(Debug)]
pub struct 呼びの札(Arc<呼び中>);

impl Drop for 呼びの札 {
    fn drop(&mut self) {
        self.0.数.fetch_sub(1, Ordering::SeqCst);
        self.0.減った.notify_waiters();
    }
}

impl 呼び中 {
    /// 呼びを 1 つ数える。**返した札を落とすまで**数に入る。
    #[must_use]
    pub fn 始める(self: &Arc<Self>) -> 呼びの札 {
        self.数.fetch_add(1, Ordering::SeqCst);
        呼びの札(Arc::clone(self))
    }

    /// いまの数。
    #[must_use]
    pub fn いくつ(&self) -> usize {
        self.数.load(Ordering::SeqCst)
    }

    /// 0 になるまで待つ。
    pub async fn 無くなるまで待つ(&self) {
        loop {
            // 先に待ちを立ててから数を見る（見たあとに減った分を取り逃がさない）
            let 待ち = self.減った.notified();
            if self.いくつ() == 0 {
                return;
            }
            待ち.await;
        }
    }
}

type 待ち = Pin<Box<dyn Future<Output = ()> + Send>>;

/// **呼びが 0 になるまで、終わりを返さない入力。**
pub struct 閉じを待つ入力<R> {
    下: R,
    呼び: Arc<呼び中>,
    上限: Duration,
    待っている: Option<待ち>,
    終えた: bool,
}

impl<R> 閉じを待つ入力<R> {
    /// 包む。`上限` は [`待つ上限`] を渡す（試験では短くする）。
    pub fn new(下: R, 呼び: Arc<呼び中>, 上限: Duration) -> Self {
        Self {
            下,
            呼び,
            上限,
            待っている: None,
            終えた: false,
        }
    }
}

impl<R: AsyncRead + Unpin> AsyncRead for 閉じを待つ入力<R> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        if self.終えた {
            return Poll::Ready(Ok(()));
        }
        if self.待っている.is_none() {
            let 前 = buf.filled().len();
            match Pin::new(&mut self.下).poll_read(cx, buf) {
                Poll::Ready(Ok(())) if buf.filled().len() == 前 => {}
                other => return other,
            }
            // 下が終わった。**呼びが残っていれば、終わりを伏せて待つ**
            let 呼び = Arc::clone(&self.呼び);
            let 上限 = self.上限;
            self.待っている = Some(Box::pin(async move {
                // **読んだ頼みが走り出すのを待つ** —— 頼みと終わりは同時に届くので、
                // 終わりを読んだ時点では、まだ数に入っていない呼びがある（試験で踏んだ）
                tokio::time::sleep(走り出す間).await;
                let 残り = 呼び.いくつ();
                if tokio::time::timeout(上限, 呼び.無くなるまで待つ())
                    .await
                    .is_err()
                {
                    eprintln!(
                        "warifu mcp: 入力が閉じたあと {} 秒待ちましたが、返事を返し終えない呼びが残っていました（閉じた時点で {残り} 件）",
                        上限.as_secs()
                    );
                }
            }));
        }
        let 待ち = self.待っている.as_mut().expect("直前に置いた");
        match 待ち.as_mut().poll(cx) {
            Poll::Ready(()) => {
                self.終えた = true;
                Poll::Ready(Ok(()))
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

#[cfg(test)]
mod 試験 {
    use super::*;
    use tokio::io::AsyncReadExt as _;

    #[tokio::test]
    async fn 呼びが無ければ_すぐ終わる() {
        let 呼び = Arc::new(呼び中::default());
        let mut 入力 = 閉じを待つ入力::new(&b"abc"[..], 呼び, Duration::from_secs(5));
        let mut 読んだ = Vec::new();
        入力.read_to_end(&mut 読んだ).await.unwrap();
        assert_eq!(読んだ, b"abc");
    }

    #[tokio::test]
    async fn 呼びが残っていれば_終わるまで終わりを伏せる() {
        let 呼び = Arc::new(呼び中::default());
        let 札 = 呼び.始める();
        let mut 入力 =
            閉じを待つ入力::new(&b""[..], Arc::clone(&呼び), Duration::from_secs(5));
        let 読む = tokio::spawn(async move {
            let mut 読んだ = Vec::new();
            入力.read_to_end(&mut 読んだ).await.map(|_| ())
        });
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(!読む.is_finished(), "呼びが残っているのに終わりを返した");
        drop(札);
        tokio::time::timeout(Duration::from_secs(2), 読む)
            .await
            .expect("呼びが終わったのに終わりを返さない")
            .unwrap()
            .unwrap();
    }

    #[tokio::test]
    async fn 返らない呼びは_上限で見切る() {
        let 呼び = Arc::new(呼び中::default());
        let _札 = 呼び.始める();
        let mut 入力 = 閉じを待つ入力::new(&b""[..], 呼び, Duration::from_millis(50)); // 走り出す間 + 上限 で見切る
        let mut 読んだ = Vec::new();
        tokio::time::timeout(Duration::from_secs(2), 入力.read_to_end(&mut 読んだ))
            .await
            .expect("上限で見切らない")
            .unwrap();
    }
}
