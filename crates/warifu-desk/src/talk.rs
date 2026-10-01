//! 行でやり取りする口。**送る側と受ける側で、同じ枠を使う。**

use std::io;

use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader};

/// 1 行の上限（バイト）。
///
/// **本文の上限より少し大きいだけ。**JSON の飾りが乗る分しか足さない。
/// 上限を置かないと、改行を送らないだけで際限なく食わせられる。
const 行の上限: usize = crate::本文の上限 + 512;

/// 行で読み書きする口。
pub struct 口<S> {
    読み: BufReader<S>,
    行: String,
}

impl<S: AsyncRead + AsyncWrite + Unpin> 口<S> {
    /// 繋がった 1 本を、行の口にする。
    pub fn 新しく(下: S) -> Self {
        Self {
            読み: BufReader::new(下),
            行: String::new(),
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
        let mut 枠 = (&mut self.読み).take(行の上限 as u64 + 1);
        let 読んだ = 枠.read_line(&mut self.行).await?;
        if 読んだ == 0 {
            return Ok(None);
        }
        if 読んだ > 行の上限 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "行が長すぎます"));
        }
        Ok(Some(self.行.trim_end().to_owned()))
    }
}
