//! 2 台をつなぐところ。
//!
//! ここは **iroh の上に載る薄い層**であって、暗号も NAT 越えも自前では書かない。
//! この層が引き受けるのは 2 つだけ。
//!
//! 1. **相手が本当にその公開鍵の持ち主か**を、繋がった時点で確かめる
//! 2. **失効している相手を通さない**
//!
//! テストは中継を一切使わない（`bind_without_relay`）。
//! 外に出ないので、回線が無くても走る。

use std::time::Duration;

use warifu_core::{Revocations, Seed};
use warifu_net::{Error, Node};

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

#[tokio::test]
async fn 二つの結び目がつながる() {
    let alice = 端末(1, "PC");
    let bob = 端末(2, "スマホ");

    let 受け手 = Node::bind_without_relay(&alice).await.unwrap();
    let 呼ぶ側 = Node::bind_without_relay(&bob).await.unwrap();

    let 受け手の宛先 = 受け手.address().await.unwrap();

    let 待ち受け = tokio::spawn(async move {
        let 名簿 = Revocations::new();
        受け手.accept(&名簿).await
    });

    let こちら = 時間を切る(呼ぶ側.connect(&受け手の宛先, &Revocations::new()))
        .await
        .expect("繋がらない");
    let あちら = 時間を切る(待ち受け).await.unwrap().expect("受けられない");

    assert_eq!(こちら.peer(), alice.public_key(), "呼んだ側から見た相手");
    assert_eq!(あちら.peer(), bob.public_key(), "受けた側から見た相手");
}

#[tokio::test]
async fn 宛先は文字列にして渡せる() {
    // 割符と同じで、宛先も QR や貼り付けで渡る。読めない形だと配れない
    let alice = 端末(1, "PC");
    let node = Node::bind_without_relay(&alice).await.unwrap();

    let 宛先 = node.address().await.unwrap();
    let 文字列 = 宛先.to_string();

    assert!(文字列.is_ascii(), "読み上げ・手入力で壊れる形にしない");

    let 戻り: warifu_net::Address = 文字列.parse().expect("自分が出した文字列を読めない");

    assert_eq!(戻り.public_key(), alice.public_key());
    assert_eq!(戻り.to_string(), 文字列);
}

#[tokio::test]
async fn 宛先の公開鍵が相手の名前そのものになっている() {
    let alice = 端末(1, "PC");
    let node = Node::bind_without_relay(&alice).await.unwrap();

    assert_eq!(
        node.address().await.unwrap().public_key(),
        alice.public_key(),
        "宛先と Identity が別物だと、割符で確定した相手に繋いだことにならない"
    );
}

#[tokio::test]
async fn 割符で確定した相手にそのまま繋がる() {
    // これが warifu の芯。割符で相手が確定したら、その公開鍵だけで呼べる
    let alice = 端末(1, "PC");
    let bob = 端末(2, "スマホ");
    let 名簿 = Revocations::new();

    let (mut 控え, 渡す半分) = alice.issue_tally(1_755_000_000, 3600).unwrap();
    let 受諾 = bob.accept(&渡す半分, 1_755_000_010).unwrap();
    let 相手 = 控え.match_half(&受諾, 1_755_000_020, &名簿).unwrap();

    let 受け手 = Node::bind_without_relay(&bob).await.unwrap();
    let 呼ぶ側 = Node::bind_without_relay(&alice).await.unwrap();
    let 宛先 = 受け手.address().await.unwrap();

    let 待ち受け = tokio::spawn(async move { 受け手.accept(&Revocations::new()).await });

    let session = 時間を切る(呼ぶ側.connect(&宛先, &名簿)).await.unwrap();
    時間を切る(待ち受け).await.unwrap().unwrap();

    assert_eq!(
        session.peer(),
        相手.public_key(),
        "割符が指した相手と、実際に繋がった相手が一致しない"
    );
}

#[tokio::test]
async fn バイト列が往復する() {
    let alice = 端末(1, "PC");
    let bob = 端末(2, "スマホ");

    let 受け手 = Node::bind_without_relay(&alice).await.unwrap();
    let 呼ぶ側 = Node::bind_without_relay(&bob).await.unwrap();
    let 宛先 = 受け手.address().await.unwrap();

    let 待ち受け = tokio::spawn(async move {
        let mut s = 受け手.accept(&Revocations::new()).await.unwrap();
        let 届いた = s.recv().await.unwrap();
        s.send(&届いた).await.unwrap();
        // 送ってすぐ落とすと、まだ網に出ていない分が消える。
        // **送り終わりだと分かっているなら、相手が受け取り切るまで待つ**
        s.finish().await.unwrap();
        届いた
    });

    let mut session = 時間を切る(呼ぶ側.connect(&宛先, &Revocations::new()))
        .await
        .unwrap();

    let 送る = "割符".as_bytes();
    時間を切る(session.send(送る)).await.unwrap();
    let 返り = 時間を切る(session.recv()).await.unwrap();

    assert_eq!(返り, 送る);
    assert_eq!(時間を切る(待ち受け).await.unwrap(), 送る);
}

#[tokio::test]
async fn 大きいバイト列も壊れない() {
    // 文書 1 個ぶん（md-business の TSV / 画像込みの md）が通らないと使い物にならない
    let alice = 端末(1, "PC");
    let bob = 端末(2, "スマホ");

    let 受け手 = Node::bind_without_relay(&alice).await.unwrap();
    let 呼ぶ側 = Node::bind_without_relay(&bob).await.unwrap();
    let 宛先 = 受け手.address().await.unwrap();

    let 待ち受け = tokio::spawn(async move {
        let mut s = 受け手.accept(&Revocations::new()).await.unwrap();
        s.recv().await.unwrap()
    });

    let mut session = 時間を切る(呼ぶ側.connect(&宛先, &Revocations::new()))
        .await
        .unwrap();

    let 送る: Vec<u8> = (0..4 * 1024 * 1024).map(|i| (i % 251) as u8).collect();
    時間を切る(session.send(&送る)).await.unwrap();

    assert_eq!(時間を切る(待ち受け).await.unwrap(), 送る);
}

#[tokio::test]
async fn 何度でも送れる() {
    let alice = 端末(1, "PC");
    let bob = 端末(2, "スマホ");

    let 受け手 = Node::bind_without_relay(&alice).await.unwrap();
    let 呼ぶ側 = Node::bind_without_relay(&bob).await.unwrap();
    let 宛先 = 受け手.address().await.unwrap();

    let 待ち受け = tokio::spawn(async move {
        let mut s = 受け手.accept(&Revocations::new()).await.unwrap();
        let mut 集めた = Vec::new();
        for _ in 0..10 {
            集めた.push(s.recv().await.unwrap());
        }
        集めた
    });

    let mut session = 時間を切る(呼ぶ側.connect(&宛先, &Revocations::new()))
        .await
        .unwrap();

    for i in 0..10u8 {
        時間を切る(session.send(&[i; 3])).await.unwrap();
    }

    let 集めた = 時間を切る(待ち受け).await.unwrap();
    assert_eq!(集めた.len(), 10);
    for (i, 中身) in 集めた.iter().enumerate() {
        assert_eq!(中身.as_slice(), &[i as u8; 3], "送った順に届いていない");
    }
}

#[tokio::test]
async fn 失効させた相手からは受けない() {
    let alice = 端末(1, "PC");
    let bob = 端末(2, "スマホ");

    let 受け手 = Node::bind_without_relay(&alice).await.unwrap();
    let 呼ぶ側 = Node::bind_without_relay(&bob).await.unwrap();
    let 宛先 = 受け手.address().await.unwrap();

    let mut 名簿 = Revocations::new();
    名簿.revoke_device(bob.public_key(), 1_755_000_000);

    let 待ち受け = tokio::spawn(async move { 受け手.accept(&名簿).await });

    // 呼ぶ側は繋いだつもりになるかもしれないが、受けた側は必ず断る
    let _ = 時間を切る(呼ぶ側.connect(&宛先, &Revocations::new())).await;

    assert!(
        matches!(時間を切る(待ち受け).await.unwrap(), Err(Error::Revoked)),
        "失効させた端末を通してしまっている"
    );
}

#[tokio::test]
async fn 失効させた相手へは呼びに行かない() {
    // 名簿は各自が持つ。呼ぶ側でも止まらないと、失くした端末を自分から呼びに行く
    let alice = 端末(1, "PC");
    let bob = 端末(2, "スマホ");

    let 受け手 = Node::bind_without_relay(&alice).await.unwrap();
    let 呼ぶ側 = Node::bind_without_relay(&bob).await.unwrap();
    let 宛先 = 受け手.address().await.unwrap();

    let mut 名簿 = Revocations::new();
    名簿.revoke_device(alice.public_key(), 1_755_000_000);

    assert!(matches!(
        時間を切る(呼ぶ側.connect(&宛先, &名簿)).await,
        Err(Error::Revoked)
    ));
}

#[tokio::test]
async fn 別人の鍵を名乗る宛先には繋がらない() {
    // 宛先の公開鍵だけ他人のものに差し替えられた場合。
    // 経路の暗号が相手の鍵に紐付いているので、ここで必ず落ちる
    let alice = 端末(1, "PC");
    let bob = 端末(2, "スマホ");
    let carol = 端末(3, "PC");

    let 受け手 = Node::bind_without_relay(&alice).await.unwrap();
    let 呼ぶ側 = Node::bind_without_relay(&bob).await.unwrap();

    let 偽の宛先 = 受け手
        .address()
        .await
        .unwrap()
        .with_public_key(carol.public_key());

    assert!(
        時間を切る(呼ぶ側.connect(&偽の宛先, &Revocations::new()))
            .await
            .is_err(),
        "alice を carol だと言われて、そのまま繋いでしまっている"
    );
}

#[tokio::test]
async fn 読めない宛先の文字列は受け取らない() {
    use warifu_net::Address;

    assert!("".parse::<Address>().is_err());
    assert!("ふつうの文字列".parse::<Address>().is_err());
    assert!("WARIFU1-AAAA".parse::<Address>().is_err());
}

#[tokio::test]
async fn 結び目を落としても経路は生きている() {
    // 呼び出す側は「繋がったら Session だけ持ち回す」と書きたくなる。
    // そこで経路が黙って死ぬと、**送った側は成功が返り、受ける側は永久に待つ**。
    // 落ちたことにすら気づけないので、Session は自分で結び目を生かす。
    let alice = 端末(1, "PC");
    let bob = 端末(2, "スマホ");

    let 受け手 = Node::bind_without_relay(&alice).await.unwrap();
    let 呼ぶ側 = Node::bind_without_relay(&bob).await.unwrap();
    let 受け手の宛先 = 受け手.address().await.unwrap();

    let 待ち受け = {
        let 受け手 = 受け手.clone();
        tokio::spawn(async move { 受け手.accept(&Revocations::new()).await })
    };
    let mut こちら = 時間を切る(呼ぶ側.connect(&受け手の宛先, &Revocations::new()))
        .await
        .expect("繋がらない");
    let mut あちら = 時間を切る(待ち受け).await.unwrap().expect("受けられない");

    // ここで結び目を手放す。**経路はまだ使う**
    drop(受け手);
    drop(呼ぶ側);

    時間を切る(こちら.send(b"warifu")).await.expect("送れない");
    let 届いた = 時間を切る(あちら.recv()).await.expect("届かない");
    assert_eq!(届いた, b"warifu", "結び目を落とした後も往復する");
}

/// **「正しく閉じた」と「落ちた」を混ぜない。**
///
/// 主催は、相手が挨拶して帰ったのか、回線が切れて消えたのかで、
/// **待ち直すべきかどうかが変わる**（予定に紐づく会議キー・D43）。
/// どちらも `Network` にしてしまうと、上の層には区別する手がかりが残らない。
#[tokio::test]
async fn 相手が正しく閉じたのは_落ちたのとは別の誤りになる() {
    let alice = 端末(1, "PC");
    let bob = 端末(2, "スマホ");

    let 受け手 = Node::bind_without_relay(&alice).await.unwrap();
    let 呼ぶ側 = Node::bind_without_relay(&bob).await.unwrap();
    let 受け手の宛先 = 受け手.address().await.unwrap();

    let 待ち受け = tokio::spawn(async move { 受け手.accept(&Revocations::new()).await });
    let こちら = 時間を切る(呼ぶ側.connect(&受け手の宛先, &Revocations::new()))
        .await
        .expect("繋がらない");
    let mut あちら = 時間を切る(待ち受け).await.unwrap().expect("受けられない");

    // **挨拶して帰る**（送る側を閉じて、相手が受け取り切るのを待つ）
    時間を切る(こちら.finish()).await.expect("閉じられない");

    let 誤り = 時間を切る(あちら.recv())
        .await
        .expect_err("閉じた後も届いてしまった");
    assert!(
        matches!(誤り, Error::Closed),
        "正しく閉じたのに「落ちた」と言っている: {誤り}"
    );
}

#[tokio::test]
async fn 相手が黙って消えたら_落ちたと分かる() {
    let alice = 端末(1, "PC");
    let bob = 端末(2, "スマホ");

    let 受け手 = Node::bind_without_relay(&alice).await.unwrap();
    let 呼ぶ側 = Node::bind_without_relay(&bob).await.unwrap();
    let 受け手の宛先 = 受け手.address().await.unwrap();

    let 待ち受け = tokio::spawn(async move { 受け手.accept(&Revocations::new()).await });
    let こちら = 時間を切る(呼ぶ側.connect(&受け手の宛先, &Revocations::new()))
        .await
        .expect("繋がらない");
    let mut あちら = 時間を切る(待ち受け).await.unwrap().expect("受けられない");

    // **挨拶せずに消える。**結び目は生かしたまま経路だけ手放す
    // （結び目ごと落とすと閉じる合図すら飛ばず、相手が気づくのは QUIC の
    // idle timeout まで待った後になる——実測 33 秒。テストで待つには長い）
    drop(こちら);

    let 誤り = 時間を切る(あちら.recv())
        .await
        .expect_err("消えた後も届いてしまった");
    assert!(
        matches!(誤り, Error::Network { .. }),
        "落ちたのに「正しく閉じた」と言っている: {誤り}"
    );
}
