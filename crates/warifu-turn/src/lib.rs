//! **機械の中だけの TURN**（**D125**）。
//!
//! WebRTC の映像は、割符の経路（iroh）とは**別の口**を使う。その口は WebView が決め、
//! ルーターが UPnP で開けてくれた口（iroh の口）は通らない。外部の STUN / TURN は使わない
//! （D13 / D124）ので、**アプリの中の 127.0.0.1 に TURN を置き、中身を割符の経路で運ぶ。**
//!
//! ```text
//! WebRTC ─ この TURN（127.0.0.1）─ 運び手（iroh）─ 相手のアプリのこの TURN ─ WebRTC
//! ```
//!
//! # 中継先の番地は見せかけ
//!
//! WebRTC は中継先の番地（relayed address）へ直接は送らない。**必ず自分の TURN を通す。**
//! だから番地は実在しなくてよい。機械ごとに [`見せかけの番地`] を決め（公開鍵から作る）、
//! TURN は宛先の見せかけの番地から相手を引いて、[`運び手`] に渡す。
//!
//! この crate は iroh も画面も知らない。**運ぶ所だけを外から差し込む。**

use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;
use turn::auth::{AuthHandler, generate_auth_key};
use turn::relay::RelayAddressGenerator;
use turn::server::Server;
use turn::server::config::{ConnConfig, ServerConfig};
use webrtc_util::Conn;

/// TURN の結果（`turn` は別名を出していない）。
type TurnResult<T> = Result<T, turn::Error>;

/// TURN の realm。**外に出ない**ので何でもよいが、両側で揃える。
pub const REALM: &str = "warifu";

/// 1 つの中継口に溜めておく包みの数。**映像は落ちてよい**（溜めすぎると遅れる）。
const 溜める包み: usize = 256;

/// 中継した包みを、相手のアプリへ運ぶもの（**iroh を差し込む所**）。
///
/// `宛` は相手の見せかけの番地、`元` は自分の見せかけの番地。
/// **落としてよい**（映像の包みは、遅れて届くより落ちたほうがよい）。
pub trait 運び手: Send + Sync + 'static {
    /// 包みを 1 つ運ぶ。
    fn 運ぶ(&self, 宛: SocketAddr, 元: SocketAddr, 中身: &[u8]);
}

/// 機械ごとの**見せかけの番地**（中継先の番地に使う）。
///
/// `100.64.0.0/10`（CGNAT の帯）から、公開鍵で決める。**インターネットでは使われない帯**なので、
/// 本物の番地と取り違えない。同じ鍵なら同じ番地になるので、相手の鍵から相手の番地が引ける。
#[must_use]
pub fn 見せかけの番地(公開鍵: &[u8; 32]) -> IpAddr {
    IpAddr::V4(Ipv4Addr::new(
        100,
        64 | (公開鍵[0] & 0x3f),
        公開鍵[1],
        公開鍵[2],
    ))
}

/// **割符の経路で運ぶ包みの頭の長さ**（宛 6 バイト ＋ 元 6 バイト）。
pub const 頭の長さ: usize = 12;

/// 運ぶ包みを作る：**宛（v4・口）＋元（v4・口）＋中身**。
///
/// 見せかけの番地は v4 だけなので、**v6 が来たら `None`**（落とす）。
#[must_use]
pub fn 包む(宛: SocketAddr, 元: SocketAddr, 中身: &[u8]) -> Option<Vec<u8>> {
    let (SocketAddr::V4(宛), SocketAddr::V4(元)) = (宛, 元) else {
        return None;
    };
    let mut 包み = Vec::with_capacity(頭の長さ + 中身.len());
    for 番地 in [宛, 元] {
        包み.extend_from_slice(&番地.ip().octets());
        包み.extend_from_slice(&番地.port().to_be_bytes());
    }
    包み.extend_from_slice(中身);
    Some(包み)
}

/// 運ばれてきた包みをほどく。**短すぎれば `None`。**
#[must_use]
pub fn ほどく(包み: &[u8]) -> Option<(SocketAddr, SocketAddr, &[u8])> {
    if 包み.len() < 頭の長さ {
        return None;
    }
    let 番地 = |b: &[u8]| {
        SocketAddr::from((
            Ipv4Addr::new(b[0], b[1], b[2], b[3]),
            u16::from_be_bytes([b[4], b[5]]),
        ))
    };
    Some((番地(&包み[..6]), 番地(&包み[6..12]), &包み[頭の長さ..]))
}

/// 中継口へ包みを渡す口（中身と、送り主の見せかけの番地）。
type 包みの口 = mpsc::Sender<(Vec<u8>, SocketAddr)>;

/// 相手から運ばれてきた包みを、宛先の中継口へ配る所。
#[derive(Clone, Default)]
pub struct 交換所 {
    口たち: Arc<Mutex<HashMap<SocketAddr, 包みの口>>>,
}

impl 交換所 {
    /// 運ばれてきた包みを渡す。**宛先の中継口が無い・溢れている**ときは `false`（落とす）。
    pub fn 届いた(&self, 宛: SocketAddr, 元: SocketAddr, 中身: Vec<u8>) -> bool {
        let 口たち = self
            .口たち
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        口たち
            .get(&宛)
            .is_some_and(|口| 口.try_send((中身, 元)).is_ok())
    }

    fn 足す(&self, 番地: SocketAddr, 口: 包みの口) {
        self.口たち
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(番地, 口);
    }

    fn 外す(&self, 番地: &SocketAddr) {
        self.口たち
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(番地);
    }
}

/// TURN が割り当てる中継口。**UDP の代わりに運び手で送る。**
struct 中継口 {
    自分: SocketAddr,
    受け: tokio::sync::Mutex<mpsc::Receiver<(Vec<u8>, SocketAddr)>>,
    運び手: Arc<dyn 運び手>,
    交換所: 交換所,
}

fn 使わない口() -> webrtc_util::Error {
    webrtc_util::Error::Other("機械の中の TURN の中継口は、宛先を指して送る".to_owned())
}

#[async_trait]
impl Conn for 中継口 {
    async fn connect(&self, _addr: SocketAddr) -> webrtc_util::Result<()> {
        Err(使わない口())
    }

    async fn recv(&self, buf: &mut [u8]) -> webrtc_util::Result<usize> {
        Ok(self.recv_from(buf).await?.0)
    }

    async fn recv_from(&self, buf: &mut [u8]) -> webrtc_util::Result<(usize, SocketAddr)> {
        let Some((中身, 元)) = self.受け.lock().await.recv().await else {
            return Err(webrtc_util::Error::ErrClosedListener);
        };
        let n = 中身.len().min(buf.len());
        buf[..n].copy_from_slice(&中身[..n]);
        Ok((n, 元))
    }

    async fn send(&self, _buf: &[u8]) -> webrtc_util::Result<usize> {
        Err(使わない口())
    }

    async fn send_to(&self, buf: &[u8], target: SocketAddr) -> webrtc_util::Result<usize> {
        self.運び手.運ぶ(target, self.自分, buf);
        Ok(buf.len())
    }

    fn local_addr(&self) -> webrtc_util::Result<SocketAddr> {
        Ok(self.自分)
    }

    fn remote_addr(&self) -> Option<SocketAddr> {
        None
    }

    async fn close(&self) -> webrtc_util::Result<()> {
        self.交換所.外す(&self.自分);
        Ok(())
    }

    fn as_any(&self) -> &(dyn std::any::Any + Send + Sync) {
        self
    }
}

/// 中継口を作る係。**番地は見せかけ、口の番号は数えて振る。**
struct 口の係 {
    番地: IpAddr,
    次の口: AtomicU16,
    運び手: Arc<dyn 運び手>,
    交換所: 交換所,
}

#[async_trait]
impl RelayAddressGenerator for 口の係 {
    fn validate(&self) -> TurnResult<()> {
        Ok(())
    }

    async fn allocate_conn(
        &self,
        _use_ipv4: bool,
        _requested_port: u16,
    ) -> TurnResult<(Arc<dyn Conn + Send + Sync>, SocketAddr)> {
        // **0 と、よく使われる低い番号は避ける**（見せかけでも、読んだ人が混乱しない）
        // `fetch_update` は Rust 1.99 で非推奨になり、代わりの `try_update` は rust-version（1.85）に無い。
        // どちらの版でも警告の出ない比べて入れ替える形で書く
        let mut 口 = self.次の口.load(Ordering::Relaxed);
        loop {
            let 次 = if 口 == u16::MAX { 49152 } else { 口 + 1 };
            match self
                .次の口
                .compare_exchange_weak(口, 次, Ordering::Relaxed, Ordering::Relaxed)
            {
                Ok(_) => break,
                Err(いま) => 口 = いま,
            }
        }
        let 番地 = SocketAddr::new(self.番地, 口);
        let (送り, 受け) = mpsc::channel(溜める包み);
        self.交換所.足す(番地, 送り);
        let 口 = 中継口 {
            自分: 番地,
            受け: tokio::sync::Mutex::new(受け),
            運び手: Arc::clone(&self.運び手),
            交換所: self.交換所.clone(),
        };
        Ok((Arc::new(口), 番地))
    }
}

/// **受信で捨ててよい誤り**（2026-09-29）。
///
/// Windows の UDP は、**閉じた相手へ送ったあとの受信で `ConnectionReset`** を返す
/// （ICMP の「届かない」を誤りとして返す）。TURN の受信の輪はそれを致命的と読んで終わり、
/// **機械の中の TURN の口が、黙って消えた**。
/// 相手が 1 人閉じただけなので、**捨てて受信を続ける。**
fn 捨ててよい受信の誤りか(誤り: &std::io::Error) -> bool {
    matches!(
        誤り.kind(),
        std::io::ErrorKind::ConnectionReset | std::io::ErrorKind::ConnectionRefused
    )
}

/// TURN が待つ UDP の口。**相手が閉じた知らせで止まらない**（[`捨ててよい受信の誤りか`]）。
struct 待ち口(UdpSocket);

#[async_trait]
impl Conn for 待ち口 {
    async fn connect(&self, addr: SocketAddr) -> webrtc_util::Result<()> {
        Ok(self.0.connect(addr).await?)
    }

    async fn recv(&self, buf: &mut [u8]) -> webrtc_util::Result<usize> {
        Ok(self.recv_from(buf).await?.0)
    }

    async fn recv_from(&self, buf: &mut [u8]) -> webrtc_util::Result<(usize, SocketAddr)> {
        loop {
            match self.0.recv_from(buf).await {
                Err(e) if 捨ててよい受信の誤りか(&e) => {}
                他 => return Ok(他?),
            }
        }
    }

    async fn send(&self, buf: &[u8]) -> webrtc_util::Result<usize> {
        Ok(self.0.send(buf).await?)
    }

    async fn send_to(&self, buf: &[u8], target: SocketAddr) -> webrtc_util::Result<usize> {
        Ok(self.0.send_to(buf, target).await?)
    }

    fn local_addr(&self) -> webrtc_util::Result<SocketAddr> {
        Ok(self.0.local_addr()?)
    }

    fn remote_addr(&self) -> Option<SocketAddr> {
        self.0.peer_addr().ok()
    }

    async fn close(&self) -> webrtc_util::Result<()> {
        Ok(())
    }

    fn as_any(&self) -> &(dyn std::any::Any + Send + Sync) {
        self
    }
}

/// 名前と合言葉を確かめる係。**合言葉は起動ごとに作り直す**（外に出ない）。
struct 合言葉の係 {
    名前: String,
    鍵: Vec<u8>,
}

impl AuthHandler for 合言葉の係 {
    fn auth_handle(
        &self,
        username: &str,
        _realm: &str,
        _src_addr: SocketAddr,
    ) -> TurnResult<Vec<u8>> {
        if username == self.名前 {
            Ok(self.鍵.clone())
        } else {
            Err(turn::Error::ErrNoSuchUser)
        }
    }
}

/// 建てた TURN。**WebRTC の `iceServers` に渡すもの**を持つ。
pub struct 機械の中のturn {
    /// 待っている口（127.0.0.1）。
    pub 口: u16,
    /// 名前（`username`）。
    pub 名前: String,
    /// 合言葉（`credential`）。**起動ごとに変わる。外に出さない。**
    pub 合言葉: String,
    server: Server,
}

impl 機械の中のturn {
    /// WebRTC の `iceServers` に書く URL。
    #[must_use]
    pub fn url(&self) -> String {
        format!("turn:127.0.0.1:{}?transport=udp", self.口)
    }

    /// 閉じる。
    ///
    /// # Errors
    /// TURN が閉じられなかったとき。
    pub async fn 閉じる(self) -> TurnResult<()> {
        self.server.close().await
    }
}

/// **機械の中だけの TURN を建てる**（127.0.0.1 の空いた口）。
///
/// `自分の鍵` から見せかけの番地を決める。中継した包みは `運び手` へ、
/// 相手から運ばれてきた包みは `交換所` から受ける。
///
/// # Errors
/// 口が取れない・合言葉が作れない・TURN が起こせないとき。
pub async fn 立てる(
    自分の鍵: &[u8; 32],
    運び手: Arc<dyn 運び手>,
    交換所: 交換所,
) -> TurnResult<機械の中のturn> {
    let 口の実体 = UdpSocket::bind("127.0.0.1:0").await?;
    let 口 = 口の実体.local_addr()?.port();
    let conn: Arc<dyn Conn + Send + Sync> = Arc::new(待ち口(口の実体));

    let mut 種 = [0u8; 16];
    getrandom::fill(&mut 種).map_err(|e| turn::Error::Other(e.to_string()))?;
    let 合言葉: String = 種.iter().map(|b| format!("{b:02x}")).collect();
    let 名前 = "warifu".to_owned();

    let server = Server::new(ServerConfig {
        conn_configs: vec![ConnConfig {
            conn,
            relay_addr_generator: Box::new(口の係 {
                番地: 見せかけの番地(自分の鍵),
                次の口: AtomicU16::new(49152),
                運び手,
                交換所,
            }),
        }],
        realm: REALM.to_owned(),
        auth_handler: Arc::new(合言葉の係 {
            鍵: generate_auth_key(&名前, REALM, &合言葉),
            名前: 名前.clone(),
        }),
        channel_bind_timeout: std::time::Duration::from_secs(0),
        alloc_close_notify: None,
    })
    .await?;

    Ok(機械の中のturn {
        口,
        名前,
        合言葉,
        server,
    })
}

#[cfg(test)]
mod tests {
    use super::{ほどく, 包む, 見せかけの番地};

    #[test]
    fn 相手が閉じた知らせは_受け口を止めない() {
        // **2026-09-29、Windows で機械の中の TURN の口が、黙って消えた**。
        // Windows の UDP は、閉じた相手へ送ったあとの受信で ConnectionReset を返す。
        // それを致命的と読むと、TURN の受信の輪が終わり、口ごと止まる
        use std::io::{Error, ErrorKind};
        assert!(super::捨ててよい受信の誤りか(&Error::from(
            ErrorKind::ConnectionReset
        )));
        assert!(super::捨ててよい受信の誤りか(&Error::from(
            ErrorKind::ConnectionRefused
        )));
        assert!(!super::捨ててよい受信の誤りか(&Error::from(
            ErrorKind::PermissionDenied
        )));
    }

    #[test]
    fn 包んでほどくと_宛と元と中身が戻る() {
        let 宛: std::net::SocketAddr = "100.65.1.2:49152".parse().unwrap();
        let 元: std::net::SocketAddr = "100.66.3.4:49153".parse().unwrap();
        let 包み = 包む(宛, 元, b"rtp").expect("v4 は包める");
        assert_eq!(包み.len(), 12 + 3, "頭は 12 バイト");
        let (宛2, 元2, 中身) = ほどく(&包み).expect("ほどける");
        assert_eq!((宛2, 元2, 中身), (宛, 元, &b"rtp"[..]));
    }

    #[test]
    fn 短すぎる包みは_ほどかない() {
        assert!(ほどく(&[0; 11]).is_none());
    }

    #[test]
    fn v6_の番地は_包まない() {
        // 見せかけの番地は v4 だけ。**来たら落とす**（黙って別の宛先にしない）
        let 宛: std::net::SocketAddr = "[::1]:1".parse().unwrap();
        let 元: std::net::SocketAddr = "100.66.3.4:49153".parse().unwrap();
        assert!(包む(宛, 元, b"x").is_none());
    }

    #[test]
    fn 見せかけの番地は_インターネットで使われない帯に入る() {
        for 先頭 in [0u8, 0x3f, 0x40, 0xff] {
            let std::net::IpAddr::V4(v4) = 見せかけの番地(&[先頭; 32]) else {
                panic!("v4 のはず");
            };
            let [a, b, ..] = v4.octets();
            assert_eq!(a, 100);
            assert!((64..128).contains(&b), "100.64.0.0/10 の外: {v4}");
        }
    }

    #[test]
    fn 同じ鍵なら同じ番地_違う鍵なら違う番地() {
        assert_eq!(見せかけの番地(&[1; 32]), 見せかけの番地(&[1; 32]));
        assert_ne!(見せかけの番地(&[1; 32]), 見せかけの番地(&[2; 32]));
    }
}
