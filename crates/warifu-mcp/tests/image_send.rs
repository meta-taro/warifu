//! **画像を会話へ流す口**（`chat_image_send`・0.1.56）。
//!
//! 席が撮った画面を、部屋へ見せられるようにする（「できた」を文字だけで返さない）。
//! 画像は口が読んで欠片に分け、机が継いで検めてから、その席の発言として流す。

use rmcp::handler::server::wrapper::Parameters;
use warifu_capability::{Action, Gate, Grant};
use warifu_desk::{FromDesk, ToDesk, 受け口, 口 as 行の口};
use warifu_mcp::{ImageSendArgs, ToolError, Warifu, subject, 会話の画像を読む};
use warifu_read::RuleStore;

fn 用意(動作: &[&str]) -> Warifu {
    let mut 関所 = Gate::new();
    for a in 動作 {
        関所.issue(Grant::new(
            subject(),
            Action::new(a).unwrap(),
            1_798_761_600,
        ));
    }
    Warifu::new(Vec::new(), RuleStore::new(), 関所, 1_756_000_000)
}

fn 試験の机(名: &str) -> std::path::PathBuf {
    let 走り = std::process::id();
    #[cfg(windows)]
    {
        std::path::PathBuf::from(format!(r"\\.\pipe\warifu-mcp-{名}-{走り}"))
    }
    #[cfg(not(windows))]
    {
        std::env::temp_dir().join(format!("warifu-mcp-{名}-{走り}.sock"))
    }
}

fn 試験の控え(名: &str) -> std::path::PathBuf {
    let 控え = std::env::temp_dir().join(format!("warifu-mcp-控え-{名}-{}", std::process::id()));
    std::fs::create_dir_all(&控え).expect("フォルダが作れること");
    控え
}

/// 試験のための置き場所に、名前を付けて書く。
fn 置く(名: &str, 中身: &[u8]) -> std::path::PathBuf {
    let 場所 = std::env::temp_dir().join(format!("warifu-mcp-画像-{}-{名}", std::process::id()));
    std::fs::write(&場所, 中身).expect("書けること");
    場所
}

/// PNG の頭（8 バイト ＋ IHDR の縦横）だけを持つ、小さな絵。
fn 小さな_png() -> Vec<u8> {
    let mut v = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
    v.extend_from_slice(&[0, 0, 0, 13]);
    v.extend_from_slice(b"IHDR");
    v.extend_from_slice(&64u32.to_be_bytes());
    v.extend_from_slice(&64u32.to_be_bytes());
    v.resize(5000, 0);
    v
}

fn 引数の断りか(r: Result<Vec<u8>, ToolError>, 含む: &str) {
    match r {
        Err(ToolError::BadArgs(文)) => assert!(文.contains(含む), "{文}"),
        他 => panic!("引数の断りではない: {他:?}"),
    }
}

#[test]
fn 顔より大きい画像も_画像の上限までは読む() {
    let mut 中身 = 小さな_png();
    中身.resize(warifu_desk::顔の上限 * 4, 0);
    let 場所 = 置く("shot.png", &中身);
    assert_eq!(会話の画像を読む(場所.to_str().unwrap()).unwrap(), 中身);
}

#[test]
fn 空のパスと_大きすぎる画像と_画像でないものは断る() {
    引数の断りか(会話の画像を読む("  "), "空");
    let mut 大 = 小さな_png();
    大.resize(warifu_desk::画像の上限 + 1, 0);
    let 場所 = 置く("big.png", &大);
    引数の断りか(会話の画像を読む(場所.to_str().unwrap()), "512 KiB");
    let 文 = 置く("text.png", b"hello, this is not an image");
    引数の断りか(会話の画像を読む(文.to_str().unwrap()), "PNG でも");
}

#[tokio::test]
async fn 札が無ければ_画像も流せない() {
    let 口 = 用意(&["chat.read", "profile.write"]);
    let 出た = 口
        .chat_image_send(Parameters(ImageSendArgs {
            path: "x.png".to_owned(),
            room: None,
            reply_to: None,
        }))
        .await;
    let 文 = format!("{:?}", 出た.unwrap_err());
    assert!(文.contains("chat.send"), "{文}");
}

/// 机の真似。名乗りを受けたあと、画像の行を `答え` で返していく。届いた画像の行を返す。
async fn 机を立てる(
    名: &str,
    答え: fn(&ToDesk) -> FromDesk,
) -> (std::path::PathBuf, tokio::task::JoinHandle<Vec<ToDesk>>) {
    let 場所 = 試験の机(名);
    let mut 待ち = 受け口::開く(&場所).await.expect("この机が開くこと");
    let 机 = tokio::spawn(async move {
        let mut 口 = 行の口::新しく(待ち.受ける().await.unwrap());
        let _挨拶 = 口.受ける().await.unwrap().unwrap();
        口.送る(
            &FromDesk::Seated {
                at: "12:00".to_owned(),
                who: "docs のエージェント".to_owned(),
            }
            .書く(),
        )
        .await
        .unwrap();
        let mut 届いた = Vec::new();
        while let Ok(Some(行)) = 口.受ける().await {
            let 行 = ToDesk::読む(&行).expect("机の読める形で届く");
            let 返事 = 答え(&行);
            let 終わり = !matches!(返事, FromDesk::画像の途中 { .. });
            届いた.push(行);
            口.送る(&返事.書く()).await.unwrap();
            if 終わり {
                break;
            }
        }
        届いた
    });
    (場所, 机)
}

#[tokio::test]
async fn 画像は欠片に分けて運び_部屋と返信は終わりの行に載り_届いた先を返す() {
    let mut 中身 = 小さな_png();
    中身.resize(20_000, 7);
    let 画像 = 置く("seat.png", &中身);
    let (場所, 机) = 机を立てる("image-ok", |行| match 行 {
        ToDesk::画像 {
            終わり: false, ..
        } => FromDesk::画像の途中 { 受けた: 0 },
        _ => FromDesk::Sent {
            to: 2,
            id: 41,
            届いた: Vec::new(),
            room: Some("ABCDEFGH".to_owned()),
        },
    })
    .await;
    let 口 = 用意(&["chat.send"])
        .この機械につながる_控えは(&場所, &試験の控え("image-ok"))
        .await
        .expect("つながれること");
    let 返り = 口
        .chat_image_send(Parameters(ImageSendArgs {
            path: 画像.to_str().unwrap().to_owned(),
            room: Some("ABCDEFGH".to_owned()),
            reply_to: Some(5),
        }))
        .await
        .expect("流せること");
    assert!(返り.contains("#41"), "{返り}");
    assert!(返り.contains("2 人"), "{返り}");

    let mut 継ぎ = warifu_desk::顔の継ぎ::画像用();
    let mut 継いだ = None;
    let 届いた = 机.await.unwrap();
    assert!(届いた.len() >= 7, "20000 バイトは 7 行に分かれる");
    let 数 = 届いた.len();
    for (i, 行) in 届いた.into_iter().enumerate() {
        let ToDesk::画像 {
            欠片,
            終わり,
            room,
            reply_to,
        } = 行
        else {
            panic!("画像の行ではない: {行:?}");
        };
        if i + 1 == 数 {
            assert_eq!(room.as_deref(), Some("ABCDEFGH"));
            assert_eq!(reply_to, Some(5));
        } else {
            assert!(room.is_none() && reply_to.is_none());
        }
        継いだ = 継ぎ.足す(&欠片, 終わり).unwrap();
    }
    assert_eq!(継いだ, Some(中身));
}

#[tokio::test]
async fn 古い画面は_更新してくださいと返す() {
    let 画像 = 置く("old.png", &小さな_png());
    let (場所, 机) = 机を立てる("image-old", |_| FromDesk::Denied {
        why: "行の形が壊れています".to_owned(),
    })
    .await;
    let 口 = 用意(&["chat.send"])
        .この機械につながる_控えは(&場所, &試験の控え("image-old"))
        .await
        .expect("つながれること");
    let 出た = 口
        .chat_image_send(Parameters(ImageSendArgs {
            path: 画像.to_str().unwrap().to_owned(),
            room: Some("ABCDEFGH".to_owned()),
            reply_to: None,
        }))
        .await;
    let 文 = format!("{:?}", 出た.unwrap_err());
    assert!(文.contains("0.1.56"), "{文}");
    let _ = 机.await;
}
