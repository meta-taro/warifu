//! **`--relay` を付けなければ、今までどおり**（**D78**）。
//!
//! ここが落ちたら、いま同じ網で動いている試験が全部止まる。
//! **既定を固定するためのテストである。**

use warifu_core::{Revocations, Seed};
use warifu_net::{Node, 中継の使い方};

fn 端末(seed: u8, label: &str) -> warifu_core::Device {
    Seed::from_bytes([seed; 32])
        .profile("Personal")
        .device(label)
}

#[test]
fn 既定は中継を使わない() {
    assert_eq!(中継の使い方::default(), 中継の使い方::使わない);
}

#[tokio::test]
async fn 中継を使わない結び目は外向きを名乗らない() {
    // 回線が無くても走る。**中継を使わないので、外へ出ない**
    let node = Node::bind(&端末(1, "mini"), 中継の使い方::使わない)
        .await
        .expect("結べる");
    let addr = node.address().await.expect("宛先");
    assert_eq!(addr.relay(), None, "中継を名乗ってはいけない");
}

#[tokio::test]
async fn 明示しない結び方は今までどおり() {
    // `bind_without_relay` を呼ぶ既存の口が、そのままであること
    let node = Node::bind_without_relay(&端末(2, "air"))
        .await
        .expect("結べる");
    let addr = node.address().await.expect("宛先");
    assert_eq!(addr.relay(), None);
}

#[tokio::test]
async fn 中継を使わない同士は今までどおり繋がる() {
    let 迎える = Node::bind_without_relay(&端末(3, "host"))
        .await
        .expect("結べる");
    let 呼ぶ = Node::bind_without_relay(&端末(4, "guest"))
        .await
        .expect("結べる");
    let 宛先 = 迎える.address().await.expect("宛先");

    let 名簿 = Revocations::default();
    let 待つ =
        tokio::spawn(async move { 迎える.accept(&Revocations::default()).await.map(|_| ()) });
    let 繋いだ = 呼ぶ.connect(&宛先, &名簿).await;

    assert!(繋いだ.is_ok(), "{:?}", 繋いだ.err());
    assert!(待つ.await.expect("待てる").is_ok());
}
