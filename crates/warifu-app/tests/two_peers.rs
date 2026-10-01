//! 2 台を **1 プロセスの中で**繋いで、会議の進行が実経路の上で通ることを確かめる。
//!
//! **実機もカメラも要らない。**ここで通らないものは、実機でも通らない。
//! 逆に、ここが通っても映像が出るとは限らない（それは WebView の担当）。

use std::time::Duration;

use warifu_app::{Conference, Event};
use warifu_core::{Device, Revocations, Seed};
use warifu_intent::Channel;
use warifu_meeting::{Notice, Signal, Step};
use warifu_net::Node;

const 待つ限度: Duration = Duration::from_secs(20);

fn 端末(seed: u8, label: &str) -> Device {
    Seed::from_bytes([seed; 32])
        .profile("Personal")
        .device(label)
}

async fn 時間を切る<T>(f: impl std::future::Future<Output = T>) -> T {
    tokio::time::timeout(待つ限度, f)
        .await
        .expect("時間内に終わらない")
}

/// 2 台つないで、呼んだ側と受けた側の口を返す。
async fn つなぐ(a: &Device, b: &Device) -> (Channel, Channel) {
    let 受け手 = Node::bind_without_relay(a).await.unwrap();
    let 呼ぶ側 = Node::bind_without_relay(b).await.unwrap();
    let 宛先 = 受け手.address().await.unwrap();
    let 待ち受け = tokio::spawn(async move { 受け手.accept(&Revocations::new()).await });

    let 呼んだ側 = 時間を切る(呼ぶ側.connect(&宛先, &Revocations::new()))
        .await
        .expect("繋がらない");
    let 受けた側 = 時間を切る(待ち受け).await.unwrap().expect("受けられない");
    (Channel::new(呼んだ側), Channel::new(受けた側))
}

#[tokio::test]
async fn 招いてから下ごしらえが往復するまでが実経路で通る() {
    let 主催 = 端末(1, "PC");
    let 客 = 端末(2, "スマホ");
    let (mut 客側, mut 主催側) = つなぐ(&主催, &客).await;

    // 主催者が会議を作る
    let mut 主催の会議 = Conference::host(主催.public_key(), 12).unwrap();

    // 客が入ると言う
    let 入る = Notice::Join {
        meeting: 主催の会議.id(),
    };
    時間を切る(客側.send(&入る.to_intent().unwrap()))
        .await
        .unwrap();

    let 届いた = 時間を切る(主催側.recv()).await.unwrap();
    let 知らせ = Notice::from_intent(&届いた).unwrap();
    let events = 主催の会議.on_notice(客.public_key(), &知らせ).unwrap();

    assert_eq!(events, vec![Event::Joined(客.public_key())]);
    assert_eq!(主催の会議.members().len(), 2);

    // 下ごしらえを送る側は D38 で決まる
    let 主催が出す = 主催の会議.should_offer_to(&客.public_key());
    let 客の会議 = Conference::joined(
        客.public_key(),
        主催の会議.id(),
        // 招待が運ぶ名簿の代わりに、ここでは同じものを組み直す
        {
            let mut r = warifu_meeting::Roster::with_capacity(主催.public_key(), 12).unwrap();
            r.add(客.public_key()).unwrap();
            r
        },
    );
    let 客が出す = 客の会議.should_offer_to(&主催.public_key());
    assert_ne!(主催が出す, 客が出す, "offer を出すのは一方だけ（D38）");

    // 出す側から SDP を流す。**中身は解釈されない**
    let sdp = b"v=0\r\no=- 0 0 IN IP4 127.0.0.1\r\n".to_vec();
    let 下ごしらえ = Notice::Signal(Signal::new(主催の会議.id(), Step::Offer, sdp.clone()));
    時間を切る(主催側.send(&下ごしらえ.to_intent().unwrap()))
        .await
        .unwrap();

    let 届いた = 時間を切る(客側.recv()).await.unwrap();
    let 知らせ = Notice::from_intent(&届いた).unwrap();
    let mut 客の会議 = 客の会議;
    let events = 客の会議.on_notice(主催.public_key(), &知らせ).unwrap();

    assert_eq!(
        events,
        vec![Event::Signal {
            from: 主催.public_key(),
            step: Step::Offer,
            blob: sdp,
        }],
        "SDP がそのまま届く"
    );
}

#[tokio::test]
async fn 名簿に居ない相手の下ごしらえは実経路でも受け取らない() {
    let 主催 = 端末(1, "PC");
    let 他人 = 端末(9, "知らない端末");
    let (mut 他人側, mut 主催側) = つなぐ(&主催, &他人).await;

    let mut 会議 = Conference::host(主催.public_key(), 12).unwrap();

    let 下ごしらえ = Notice::Signal(Signal::new(会議.id(), Step::Offer, b"x".to_vec()));
    時間を切る(他人側.send(&下ごしらえ.to_intent().unwrap()))
        .await
        .unwrap();

    let 届いた = 時間を切る(主催側.recv()).await.unwrap();
    let 知らせ = Notice::from_intent(&届いた).unwrap();

    // **経路は通る。会議は受け取らない。**この 2 つを混ぜない（warifu-intent D5 と同じ構え）
    assert!(会議.on_notice(他人.public_key(), &知らせ).is_err());
}
