//! 机の口の開け方。**OS ごとに違うのは、ここだけに閉じ込める。**
//!
//! - Unix — ドメインソケット。**所有者だけが読み書きできる（0600）**
//! - Windows — 名前付きパイプ
//!
//! **どちらも網には出ない。**繋げるのは同じ機械の中だけ。

use std::io;
use std::path::Path;

use tokio::io::{AsyncRead, AsyncWrite};

/// 机に繋がった 1 本。
pub trait 一本: AsyncRead + AsyncWrite + Unpin + Send + 'static {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send + 'static> 一本 for T {}

#[cfg(not(windows))]
mod 中身 {
    use super::{Path, io};
    use tokio::net::{UnixListener, UnixStream};

    /// 待ち受ける口。
    pub struct 受け口(UnixListener, std::path::PathBuf);

    impl 受け口 {
        /// 口を開く。**先に古い口を片付ける。**
        ///
        /// 落ちたあとのソケットが残っていると bind できない。
        /// 残骸を理由に机が開かないのは、直しようが無い形で止まる。
        pub async fn 開く(場所: &Path) -> io::Result<Self> {
            if let Some(親) = 場所.parent() {
                std::fs::create_dir_all(親)?;
            }
            // **繋がるなら消さない。**もう 1 つ動いている机を横取りしない
            if 場所.exists() {
                if UnixStream::connect(場所).await.is_ok() {
                    return Err(io::Error::new(
                        io::ErrorKind::AddrInUse,
                        "机はもう開いています",
                    ));
                }
                std::fs::remove_file(場所)?;
            }
            let 待ち = UnixListener::bind(場所)?;
            所有者だけにする(場所)?;
            Ok(Self(待ち, 場所.to_path_buf()))
        }

        /// 1 本受ける。
        pub async fn 受ける(&mut self) -> io::Result<UnixStream> {
            let (口, _) = self.0.accept().await?;
            Ok(口)
        }
    }

    impl Drop for 受け口 {
        fn drop(&mut self) {
            // 残骸を置いて出ない。**消せなくても落とさない**（次回 開く が片付ける）
            let _ = std::fs::remove_file(&self.1);
        }
    }

    fn 所有者だけにする(場所: &Path) -> io::Result<()> {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(場所, std::fs::Permissions::from_mode(0o600))
    }

    /// 机へ繋ぐ。
    pub async fn 繋ぐ(場所: &Path) -> io::Result<UnixStream> {
        UnixStream::connect(場所).await
    }
}

#[cfg(windows)]
mod 中身 {
    use super::{Path, io};
    use tokio::net::windows::named_pipe::{ClientOptions, NamedPipeServer, ServerOptions};

    /// 待ち受ける口。
    pub struct 受け口 {
        名: std::ffi::OsString,
        次: Option<NamedPipeServer>,
    }

    impl 受け口 {
        /// 口を開く。
        ///
        /// `first_pipe_instance` を立てる。**同じ名前の口を後から作られない**
        /// （立てないと、別のプロセスが同じ名前で待ち受けて横取りできる）。
        pub async fn 開く(場所: &Path) -> io::Result<Self> {
            let 名 = 場所.as_os_str().to_owned();
            let 次 = ServerOptions::new().first_pipe_instance(true).create(&名)?;
            Ok(Self {
                名, 次: Some(次)
            })
        }

        /// 1 本受ける。
        pub async fn 受ける(&mut self) -> io::Result<NamedPipeServer> {
            let 待つ = self
                .次
                .take()
                .ok_or_else(|| io::Error::new(io::ErrorKind::BrokenPipe, "机の口が閉じています"))?;
            待つ.connect().await?;
            // 次の 1 本を先に用意する。**用意しないと、次の客が繋げない**
            self.次 = Some(ServerOptions::new().create(&self.名)?);
            Ok(待つ)
        }
    }

    /// 机へ繋ぐ。
    pub async fn 繋ぐ(
        場所: &Path,
    ) -> io::Result<tokio::net::windows::named_pipe::NamedPipeClient> {
        ClientOptions::new().open(場所.as_os_str())
    }
}

pub use 中身::{受け口, 繋ぐ};

/// 机が開いているかを、繋いで確かめる。
///
/// **開いていないことと、開いているが応じないことを混ぜない。**
pub async fn 開いているか(場所: &Path) -> bool {
    繋ぐ(場所).await.is_ok()
}
