//! TURN を 2 つ建てて、間をメモリの中の運び手でつなぐ。
//!
//! **iroh も WebRTC も使わない。**運び手を差し替えれば同じ形で iroh に載る（D125）。

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use tokio::net::UdpSocket;
use turn::client::{Client, ClientConfig};
use warifu_turn::{交換所, 立てる, 運び手};
use webrtc_util::Conn;

/// 相手の交換所へ、そのまま渡すだけの運び手（iroh の代わり）。
struct 橋(交換所);
impl 運び手 for 橋 {
    fn 運ぶ(&self, 宛: SocketAddr, 元: SocketAddr, 中身: &[u8]) {
        self.0.届いた(宛, 元, 中身.to_vec());
    }
}

async fn 利用者(口: u16, 名前: &str, 合言葉: &str) -> Client {
    let conn = Arc::new(UdpSocket::bind("127.0.0.1:0").await.unwrap());
    let c = Client::new(ClientConfig {
        stun_serv_addr: String::new(),
        turn_serv_addr: format!("127.0.0.1:{口}"),
        username: 名前.to_owned(),
        password: 合言葉.to_owned(),
        realm: "warifu".to_owned(),
        software: String::new(),
        rto_in_ms: 0,
        conn,
        vnet: None,
    })
    .await
    .unwrap();
    c.listen().await.unwrap();
    c
}

#[tokio::test(flavor = "multi_thread")]
async fn 二つの機械の中のturnが_運び手を通して届く() {
    let a_の交換所 = 交換所::default();
    let b_の交換所 = 交換所::default();
    let a = 立てる(
        &[1; 32],
        Arc::new(橋(b_の交換所.clone())),
        a_の交換所.clone(),
    )
    .await
    .unwrap();
    let b = 立てる(
        &[2; 32],
        Arc::new(橋(a_の交換所.clone())),
        b_の交換所.clone(),
    )
    .await
    .unwrap();

    let a_利用者 = 利用者(a.口, &a.名前, &a.合言葉).await;
    let b_利用者 = 利用者(b.口, &b.名前, &b.合言葉).await;
    let a_中継 = a_利用者.allocate().await.expect("A の中継が取れない");
    let b_中継 = b_利用者.allocate().await.expect("B の中継が取れない");
    let a_番地 = a_中継.local_addr().unwrap();
    let b_番地 = b_中継.local_addr().unwrap();
    assert_ne!(a_番地.ip(), b_番地.ip(), "見せかけの番地は機械ごとに違う");

    // **TURN は許した相手からしか受けない。**先に B から送って、B 側の許しを作る
    let _ = b_中継.send_to(b"ping", a_番地).await.unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;
    let _ = a_中継.send_to(b"hello via bridge", b_番地).await.unwrap();

    let mut 受け = [0u8; 64];
    let (n, 元) = tokio::time::timeout(Duration::from_secs(5), b_中継.recv_from(&mut 受け))
        .await
        .expect("B に届かない")
        .unwrap();
    assert_eq!(&受け[..n], b"hello via bridge");
    assert_eq!(元, a_番地, "送り主は A の見せかけの番地に見える");
}
