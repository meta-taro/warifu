//! 2 台のあいだで、映像を張るための下ごしらえが往復するところ（M4）。
//!
//! **ここではまだ映像を出さない。**確かめるのは
//! 「**外部のシグナリングサーバを 1 台も使わずに** SDP / ICE を交換できる」ことだけ
//! （`issues/005` 満たすこと 4）。映像は M5。
//!
//! テストは中継を一切使わない（`bind_without_relay`）。外に出ないので回線が無くても走る。

use std::time::Duration;

use warifu_core::{Revocations, Seed};
use warifu_intent::Channel;
use warifu_meeting::{MeetingId, Notice, Roster, Signal, Step};
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

/// 2 台つないで、呼んだ側と受けた側の口を返す。
async fn つなぐ() -> (Channel, Channel) {
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
    (Channel::new(呼んだ側), Channel::new(受けた側))
}

/// 口を 1 つ受け取って、会議の知らせとして読む。
async fn 受け取る(ch: &mut Channel) -> Notice {
    let 塊 = 時間を切る(ch.recv()).await.expect("届かない");
    Notice::from_intent(&塊).expect("会議の知らせとして読めない")
}

#[tokio::test]
async fn 招集から下ごしらえの往復までが通る() {
    let (mut 主催, mut 相手) = つなぐ().await;
    let 会議 = MeetingId::generate();

    // 1. 招集。**入る側は他の参加者が誰かを知らないと繋ぎに行けない**
    let mut 名簿 = Roster::new(主催.peer());
    名簿.add(相手.peer()).unwrap();
    主催
        .send(
            &Notice::Invite {
                meeting: 会議,
                roster: 名簿.clone(),
            }
            .to_intent()
            .unwrap(),
        )
        .await
        .unwrap();

    match 受け取る(&mut 相手).await {
        Notice::Invite { meeting, roster } => {
            assert_eq!(meeting, 会議);
            assert!(roster.contains(&相手.peer()), "自分が名簿に載っている");
        }
        その他 => panic!("招集が届いていない: {その他:?}"),
    }

    // 2. 参加
    相手
        .send(&Notice::Join { meeting: 会議 }.to_intent().unwrap())
        .await
        .unwrap();
    assert_eq!(受け取る(&mut 主催).await, Notice::Join { meeting: 会議 });

    // 3. 申し出 → 返事 → 経路の候補。**中身は warifu が作ったものではない**
    let 申し出 = b"v=0\r\no=- 4611731400430051336 2 IN IP4 127.0.0.1\r\n".to_vec();
    主催
        .send(
            &Notice::Signal(Signal::new(会議, Step::Offer, 申し出.clone()))
                .to_intent()
                .unwrap(),
        )
        .await
        .unwrap();

    match 受け取る(&mut 相手).await {
        Notice::Signal(s) => {
            assert_eq!(s.step(), Step::Offer);
            assert_eq!(s.blob(), 申し出, "SDP が 1 バイトも変わらずに届く");
        }
        その他 => panic!("申し出が届いていない: {その他:?}"),
    }

    let 返事 = b"v=0\r\na=recvonly\r\n".to_vec();
    相手
        .send(
            &Notice::Signal(Signal::new(会議, Step::Answer, 返事.clone()))
                .to_intent()
                .unwrap(),
        )
        .await
        .unwrap();
    let 候補 = b"candidate:1 1 UDP 2130706431 10.0.0.2 50000 typ host".to_vec();
    相手
        .send(
            &Notice::Signal(Signal::new(会議, Step::Candidate, 候補.clone()))
                .to_intent()
                .unwrap(),
        )
        .await
        .unwrap();

    match 受け取る(&mut 主催).await {
        Notice::Signal(s) => assert_eq!((s.step(), s.blob()), (Step::Answer, 返事.as_slice())),
        その他 => panic!("返事が届いていない: {その他:?}"),
    }
    match 受け取る(&mut 主催).await {
        Notice::Signal(s) => assert_eq!((s.step(), s.blob()), (Step::Candidate, 候補.as_slice())),
        その他 => panic!("候補が届いていない: {その他:?}"),
    }

    // 4. 退出
    相手
        .send(&Notice::Leave { meeting: 会議 }.to_intent().unwrap())
        .await
        .unwrap();
    assert_eq!(受け取る(&mut 主催).await, Notice::Leave { meeting: 会議 });

    名簿.remove(&相手.peer());
    assert_eq!(名簿.len(), 1, "残るのは主催者だけ");
}

#[tokio::test]
async fn 同じ会議のやり取りが一本の話として辿れる() {
    // 会議 id がそのまま相関。**複数の会議を同時に開いても取り違えない**
    let (mut 送る, mut 受ける) = つなぐ().await;
    let 甲 = MeetingId::generate();
    let 乙 = MeetingId::generate();

    for 会議 in [甲, 乙, 甲] {
        送る
            .send(
                &Notice::Signal(Signal::new(会議, Step::Candidate, b"c".to_vec()))
                    .to_intent()
                    .unwrap(),
            )
            .await
            .unwrap();
    }

    let mut 届いた = Vec::new();
    for _ in 0..3 {
        let 塊 = 時間を切る(受ける.recv()).await.expect("届かない");
        届いた.push(塊.correlation());
    }

    assert_eq!(届いた, vec![甲.into(), 乙.into(), 甲.into()]);
}

#[tokio::test]
async fn 会議の下ごしらえは相手の経路にしか流れない() {
    // **中継しない。**A と B の経路に流したものが、C の経路には現れない
    let (mut 送る, mut 受ける) = つなぐ().await;
    let (_別の呼んだ側, mut 別の受け手) = つなぐ().await;
    let 会議 = MeetingId::generate();

    送る
        .send(
            &Notice::Signal(Signal::new(会議, Step::Offer, b"secret sdp".to_vec()))
                .to_intent()
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(matches!(受け取る(&mut 受ける).await, Notice::Signal(_)));

    let 別の経路 = tokio::time::timeout(Duration::from_millis(300), 別の受け手.recv()).await;
    assert!(別の経路.is_err(), "関係ない経路には何も流れない");
}
