//! **相手が起動していない間に預けて、起動してから受け取る。**
//!
//! 中継は一切使わない（`bind_without_relay`）。外に出ないので、回線が無くても走る。

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Mutex;
use warifu_core::{Device, Revocations, Seed};
use warifu_net::Node;
use warifu_post::Box as 預かり所;
use warifu_postbox::{受け取る, 応える, 預ける, 頼みを聞ける形か};

const 待つ限度: Duration = Duration::from_secs(20);

fn 人(seed: u8, label: &str) -> Device {
    Seed::from_bytes([seed; 32])
        .profile("Personal")
        .device(label)
}

async fn 時間を切る<T>(f: impl Future<Output = T>) -> T {
    tokio::time::timeout(待つ限度, f)
        .await
        .expect("待つ限度を超えた")
}

/// 試験のあいだだけ立つ預かり所。**`warifu relay` と同じ応え方をする。**
fn 預かり所を立てる(
    node: Node, 箱: Arc<Mutex<預かり所>>
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            let Ok(mut session) = node.accept(&Revocations::new()).await else {
                continue;
            };
            let 相手 = session.peer();
            let 箱 = Arc::clone(&箱);
            tokio::spawn(async move {
                let Ok(塊) = session.recv().await else {
                    return;
                };
                let 返 = match 頼みを聞ける形か(&塊) {
                    Some(頼み) => 応える(&mut *箱.lock().await, 相手, 頼み, 100),
                    None => warifu_post::Reply::Refused,
                };
                let _ = session.send(&返.to_bytes()).await;
                let _ = session.finish().await;
            });
        }
    })
}

#[tokio::test]
async fn 起動していない相手へ預けて_起動してから受け取れる() {
    let 預かる人 = 人(1, "預かり所");
    let 出す人 = 人(2, "PC");
    let 受ける人 = 人(3, "Mac Air");

    let 所 = Node::bind_without_relay(&預かる人).await.unwrap();
    let 宛先 = 所.address().await.unwrap();
    let 箱 = Arc::new(Mutex::new(預かり所::new()));
    let 立った = 預かり所を立てる(所, Arc::clone(&箱));

    // 出す側 —— 受ける人はまだ起動していない
    let 出す = Node::bind_without_relay(&出す人).await.unwrap();
    時間を切る(預ける(
        &出す,
        &宛先,
        受ける人.public_key(),
        "こんばんは".as_bytes(),
    ))
    .await
    .unwrap();
    assert_eq!(箱.lock().await.len(), 1);

    // 受ける側 —— ここで初めて起動する
    let 受ける = Node::bind_without_relay(&受ける人).await.unwrap();
    let 届いた = 時間を切る(受け取る(&受ける, &受ける人, &宛先))
        .await
        .unwrap();
    assert_eq!(届いた, vec!["こんばんは".as_bytes().to_vec()]);

    // **渡したら手放す。**溜め込む場所にしない
    assert_eq!(箱.lock().await.len(), 0);

    立った.abort();
}

#[tokio::test]
async fn 他人あては受け取れない() {
    let 預かる人 = 人(4, "預かり所");
    let 出す人 = 人(5, "PC");
    let 受ける人 = 人(6, "Mac Air");
    let 横取りする人 = 人(7, "だれか");

    let 所 = Node::bind_without_relay(&預かる人).await.unwrap();
    let 宛先 = 所.address().await.unwrap();
    let 箱 = Arc::new(Mutex::new(預かり所::new()));
    let 立った = 預かり所を立てる(所, Arc::clone(&箱));

    let 出す = Node::bind_without_relay(&出す人).await.unwrap();
    時間を切る(預ける(
        &出す,
        &宛先,
        受ける人.public_key(),
        "ないしょ".as_bytes(),
    ))
    .await
    .unwrap();

    // **経路で確定した相手あてだけ**が渡る。名乗る余地が無い
    let 横取り = Node::bind_without_relay(&横取りする人).await.unwrap();
    let 届いた = 時間を切る(受け取る(&横取り, &横取りする人, &宛先))
        .await
        .unwrap();
    assert!(届いた.is_empty());
    assert_eq!(箱.lock().await.len(), 1);

    立った.abort();
}
