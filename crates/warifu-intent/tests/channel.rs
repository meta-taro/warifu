//! 経路の上で口をやり取りするところ。
//!
//! `warifu-net` が運ぶのはバイト列だけで、**それが何なのかは知らない**。
//! ここで初めて「何をしたいか」が形になる。
//!
//! テストは中継を一切使わない（`bind_without_relay`）。外に出ないので回線が無くても走る。

use std::time::Duration;

use warifu_core::{Revocations, Seed};
use warifu_intent::{Channel, Error, Intent, Kind};
use warifu_net::Node;

/// つながらないまま止まると、落ちたのか待っているのか分からなくなる。
const 待つ限度: Duration = Duration::from_secs(20);

fn 端末(seed: u8, label: &str) -> warifu_core::Device {
    Seed::from_bytes([seed; 32])
        .profile("Personal")
        .device(label)
}

async fn 時間を切る<T>(f: impl Future<Output = T>) -> T {
    tokio::time::timeout(待つ限度, f)
        .await
        .expect("待つ限度を超えた")
}

/// 2 台つないで、呼んだ側と受けた側の経路を返す。
async fn つなぐ() -> (warifu_net::Session, warifu_net::Session) {
    let alice = 端末(1, "PC");
    let bob = 端末(2, "スマホ");

    let 受け手 = Node::bind_without_relay(&alice).await.unwrap();
    let 呼ぶ側 = Node::bind_without_relay(&bob).await.unwrap();
    let 宛先 = 受け手.address().await.unwrap();
    let 待ち受け = tokio::spawn(async move { 受け手.accept(&Revocations::new()).await });

    let 呼んだ側 = 時間を切る(呼ぶ側.connect(&宛先, &Revocations::new()))
        .await
        .expect("繋がらない");
    let 受けた側 = 時間を切る(待ち受け).await.unwrap().expect("受けられない");
    (呼んだ側, 受けた側)
}

#[tokio::test]
async fn 口が相手に届く() {
    let (呼んだ側, 受けた側) = つなぐ().await;
    let mut 送る = Channel::new(呼んだ側);
    let mut 受ける = Channel::new(受けた側);

    let 元 = Intent::new(Kind::new("file.offer").unwrap(), b"design.md".to_vec());
    送る.send(&元).await.unwrap();

    let 届いた = 時間を切る(受ける.recv()).await.unwrap();

    assert_eq!(届いた.kind().as_str(), "file.offer");
    assert_eq!(届いた.payload(), b"design.md");
    assert_eq!(届いた.correlation(), 元.correlation());
}

#[tokio::test]
async fn 返事が同じ相関で戻る() {
    let (呼んだ側, 受けた側) = つなぐ().await;
    let mut a = Channel::new(呼んだ側);
    let mut b = Channel::new(受けた側);

    let 申し出 = Intent::new(Kind::new("file.offer").unwrap(), b"design.md".to_vec());
    a.send(&申し出).await.unwrap();

    let 届いた = 時間を切る(b.recv()).await.unwrap();
    b.send(&届いた.reply(Kind::new("file.accept").unwrap(), Vec::new()))
        .await
        .unwrap();

    let 返事 = 時間を切る(a.recv()).await.unwrap();

    assert_eq!(返事.kind().as_str(), "file.accept");
    assert_eq!(
        返事.correlation(),
        申し出.correlation(),
        "どの申し出への返事か"
    );
}

#[tokio::test]
async fn 相手が誰かは経路のまま分かる() {
    let (呼んだ側, 受けた側) = つなぐ().await;
    let a = Channel::new(呼んだ側);
    let b = Channel::new(受けた側);

    // 口を被せても、**割符で確定した相手が誰か**は消えない
    assert_eq!(a.peer(), 端末(1, "PC").public_key());
    assert_eq!(b.peer(), 端末(2, "スマホ").public_key());
}

#[tokio::test]
async fn 知らない口もそのまま届く() {
    // warifu は中身を解釈しない（D11）。知らない口を経路ごと落とすと、
    // 版が 1 つずれただけで繋がらなくなる
    let (呼んだ側, 受けた側) = つなぐ().await;
    let mut 送る = Channel::new(呼んだ側);
    let mut 受ける = Channel::new(受けた側);

    送る
        .send(&Intent::new(
            Kind::new("invoice.issue").unwrap(),
            b"{}".to_vec(),
        ))
        .await
        .unwrap();

    let 届いた = 時間を切る(受ける.recv()).await.unwrap();

    assert_eq!(届いた.kind().as_str(), "invoice.issue");
    assert!(!届いた.kind().is_known(), "知らないものは知らないまま渡す");
}

#[tokio::test]
async fn 壊れた塊は_malformed_で止まる() {
    // 相手が warifu とは限らない。**経路が暗号化されていることと、
    // 中身が正しいことは別**
    let (呼んだ側, 受けた側) = つなぐ().await;
    let mut 生 = 呼んだ側;
    let mut 受ける = Channel::new(受けた側);

    生.send(b"\x00").await.unwrap(); // 名前の長さが 0

    let 結果 = 時間を切る(受ける.recv()).await;

    assert!(
        matches!(結果, Err(Error::Malformed)),
        "壊れた塊を受け取ってしまった: {結果:?}"
    );
}

#[tokio::test]
async fn 大きい荷物が壊れずに届く() {
    let (呼んだ側, 受けた側) = つなぐ().await;
    let mut 送る = Channel::new(呼んだ側);
    let mut 受ける = Channel::new(受けた側);

    let 荷物: Vec<u8> = (0..1024 * 1024).map(|i| (i % 251) as u8).collect();
    let 元 = Intent::new(Kind::new("file.chunk").unwrap(), 荷物.clone());

    送る.send(&元).await.unwrap();
    let 届いた = 時間を切る(受ける.recv()).await.unwrap();

    assert_eq!(
        届いた.payload(),
        &荷物[..],
        "1 バイトでも違えば文書が壊れる"
    );
}

#[tokio::test]
async fn 送った順に届く() {
    let (呼んだ側, 受けた側) = つなぐ().await;
    let mut 送る = Channel::new(呼んだ側);
    let mut 受ける = Channel::new(受けた側);

    for i in 0u8..8 {
        送る
            .send(&Intent::new(Kind::new("file.chunk").unwrap(), vec![i; 4]))
            .await
            .unwrap();
    }

    for i in 0u8..8 {
        let 届いた = 時間を切る(受ける.recv()).await.unwrap();
        assert_eq!(届いた.payload(), &[i; 4], "{i} 番目が入れ替わった");
    }
}
