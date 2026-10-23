//! 行でやり取りする口。**送る側と受ける側で、同じ枠を使う。**

use std::io;

use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader};

/// 1 行の上限（バイト）。
///
/// **本文の上限より少し大きいだけ。**JSON の飾りが乗る分しか足さない。
/// 上限を置かないと、改行を送らないだけで際限なく食わせられる。
const 行の上限: usize = crate::本文の上限 + 512;

/// **この機械からの返事の 1 行の上限**（バイト・0.1.38）。
///
/// 書き出し（`FromDesk::書き出し`）は控えた会話を 1 行にまとめて返す。17 行ほどで
/// [`行の上限`] を超え、受けた側が落として「この機械が返事をしません」になっていた（0.1.37）。
/// この機械は同じ利用者の手元のプロセスなので、大きめに取る。**上限そのものは残す**
/// （改行を送らないだけで際限なく食わせられないように）。この機械は返事をこれに収めて書く。
pub const 返事の上限: usize = 1024 * 1024;

/// 行で読み書きする口。
pub struct 口<S> {
    読み: BufReader<S>,
    行: String,
    上限: usize,
}

impl<S: AsyncRead + AsyncWrite + Unpin> 口<S> {
    /// 繋がった 1 本を、行の口にする。**エージェントから受ける側**（この機械）はこれ。
    pub fn 新しく(下: S) -> Self {
        Self {
            読み: BufReader::new(下),
            行: String::new(),
            上限: 行の上限,
        }
    }

    /// **この機械からの返事を受ける側**（MCP・CLI）。1 行の上限は [`返事の上限`]。
    pub fn 返事用(下: S) -> Self {
        Self {
            上限: 返事の上限,
            ..Self::新しく(下)
        }
    }

    /// 1 行送る。**行末は必ずここで付ける。**
    pub async fn 送る(&mut self, 中身: &str) -> io::Result<()> {
        let 下 = self.読み.get_mut();
        下.write_all(中身.as_bytes()).await?;
        下.write_all(b"\n").await?;
        下.flush().await
    }

    /// 1 行受ける。相手が閉じたら `None`。
    ///
    /// **長すぎる行は、切り詰めずに落とす。**切り詰めると
    /// 「途中まで通った」形になり、読み手が半端な JSON を見る。
    pub async fn 受ける(&mut self) -> io::Result<Option<String>> {
        self.行.clear();
        let mut 枠 = (&mut self.読み).take(self.上限 as u64 + 1);
        let 読んだ = 枠.read_line(&mut self.行).await?;
        if 読んだ == 0 {
            return Ok(None);
        }
        if 読んだ > self.上限 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "行が長すぎます"));
        }
        Ok(Some(self.行.trim_end().to_owned()))
    }
}

#[cfg(test)]
mod 試験 {
    use super::*;

    async fn 一行を流す(長さ: usize, 大きく: bool) -> io::Result<Option<String>> {
        let (送り, 受け) = tokio::io::duplex(長さ + 16);
        let mut 送る口 = 口::新しく(送り);
        let mut 受ける口 = if 大きく {
            口::返事用(受け)
        } else {
            口::新しく(受け)
        };
        let 行 = "あ".repeat(長さ / 3);
        let 書く = tokio::spawn(async move { 送る口.送る(&行).await });
        let 読んだ = 受ける口.受ける().await;
        let _ = 書く.await;
        読んだ
    }

    #[tokio::test]
    async fn エージェントから来る行は_本文の上限を少し超えたら落とす() {
        assert!(一行を流す(行の上限 + 300, false).await.is_err());
    }

    #[tokio::test]
    async fn この機械からの返事は_長い書き出しでも受ける() {
        // 0.1.37 まで、17 行ほどの会話を書き出すと 1 行が上限を超え、
        // MCP が返事を落として「この機械が返事をしません」になっていた
        let 読んだ = 一行を流す(64 * 1024, true).await.unwrap().unwrap();
        assert!(読んだ.len() > 行の上限);
    }

    #[tokio::test]
    async fn この機械からの返事にも_上限はある() {
        assert!(一行を流す(返事の上限 + 300, true).await.is_err());
    }
}
