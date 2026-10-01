//! 繋ぐところ。**TLS は自前で書かない。**

use std::sync::Arc;

use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
use tokio_rustls::client::TlsStream;
use tokio_rustls::rustls::pki_types::ServerName;
use tokio_rustls::rustls::{ClientConfig, RootCertStore};

use crate::{Account, Error};

/// TLS で繋ぐ（IMAPS。既定は 993）。
///
/// 使うのは **既にこのワークスペースの木にあるもの**（`rustls` / `ring` / `webpki-roots`。
/// iroh 経由で入っている）。既定の `aws-lc-rs` は cmake で C を建てるので採らない。
/// **ゲートに新しいビルド系を持ち込まない。**
///
/// # 失敗
///
/// [`Error::Network`]。**下の層の理由を捨てない。**
pub async fn connect(account: &Account) -> Result<TlsStream<TcpStream>, Error> {
    let mut 根 = RootCertStore::empty();
    根.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

    let 設定 = ClientConfig::builder_with_provider(Arc::new(
        tokio_rustls::rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .map_err(|e| Error::Network(e.to_string()))?
    .with_root_certificates(根)
    .with_no_client_auth();

    let 宛先 = ServerName::try_from(account.host().to_owned())
        .map_err(|e| Error::Network(format!("ホスト名が読めません: {e}")))?;

    let tcp = TcpStream::connect((account.host(), account.port()))
        .await
        .map_err(|e| Error::Network(e.to_string()))?;

    TlsConnector::from(Arc::new(設定))
        .connect(宛先, tcp)
        .await
        .map_err(|e| Error::Network(e.to_string()))
}
