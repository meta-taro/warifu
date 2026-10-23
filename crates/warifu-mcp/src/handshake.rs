//! **握手の前に来た頼みを、rmcp の手前で断る**（0.1.38・10-23）。
//!
//! Claude Code は、まず 2026-07-28 版の `server/discover` で尋ね、断られると `initialize` へ戻る。
//! **rmcp 3.2 は、最初の頼みが握手でないと「以後の頼みはすべて `_meta` に版を載せること」と覚え、
//! 握手し直しても忘れない。**そのため握手のあとの `tools/list` が断られ、道具が 1 つも出なかった。
//!
//! ここで握手より前の頼み（`ping` を除く）に「その版は無い」と返し、**rmcp には握手から見せる。**

use rmcp::RoleServer;
use rmcp::model::{
    ClientJsonRpcMessage, ClientRequest, ErrorData, GetMeta as _, ProtocolVersion,
    ServerJsonRpcMessage,
};
use rmcp::transport::Transport;
use rmcp::transport::async_rw::AsyncRwTransport;
use tokio::io::{AsyncRead, AsyncWrite};

/// 読み口と書き口を、握手より前の頼みを断る口にする。`serve` にそのまま渡せる。
pub fn 握手へ戻す<R, W>(
    読み: R,
    書き: W,
) -> 握手を待つ口<AsyncRwTransport<RoleServer, R, W>>
where
    R: AsyncRead + Send + Unpin + 'static,
    W: AsyncWrite + Send + Unpin + 'static,
{
    握手を待つ口 {
        中: AsyncRwTransport::new_server(読み, 書き),
        握手した: false,
    }
}

/// [`握手へ戻す`] が返す口。握手が済むまで、握手でない頼みを断る。
pub struct 握手を待つ口<T> {
    中: T,
    握手した: bool,
}

/// 握手の前に来ても、そのまま rmcp へ通す頼みか。
fn 通すか(頼み: &ClientRequest) -> bool {
    matches!(
        頼み,
        ClientRequest::InitializeRequest(_) | ClientRequest::PingRequest(_)
    )
}

impl<T: Transport<RoleServer>> Transport<RoleServer> for 握手を待つ口<T> {
    type Error = T::Error;

    fn send(
        &mut self,
        item: ServerJsonRpcMessage,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send + 'static {
        self.中.send(item)
    }

    async fn receive(&mut self) -> Option<ClientJsonRpcMessage> {
        loop {
            let 届いた = self.中.receive().await?;
            if self.握手した {
                return Some(届いた);
            }
            let ClientJsonRpcMessage::Request(頼み) = &届いた else {
                return Some(届いた);
            };
            if 通すか(&頼み.request) {
                self.握手した = matches!(頼み.request, ClientRequest::InitializeRequest(_));
                return Some(届いた);
            }
            let 頼まれた版 = 頼み
                .request
                .get_meta()
                .protocol_version()
                .unwrap_or(ProtocolVersion::V_2026_07_28);
            let 断り = ServerJsonRpcMessage::error(
                ErrorData::unsupported_protocol_version(頼まれた版, crate::server::話せる版),
                Some(頼み.id.clone()),
            );
            // 断りを書けないなら、口はもう使えない。rmcp に閉じたと伝える。
            if self.中.send(断り).await.is_err() {
                return None;
            }
        }
    }

    fn close(&mut self) -> impl Future<Output = Result<(), Self::Error>> + Send {
        self.中.close()
    }
}
