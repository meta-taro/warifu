//! **席の顔を置く口**（`avatar_set`・0.1.53）。
//!
//! 席（この PC のエージェント）が、自分の顔（製品のアイコンなど）を自分で置けるようにする。
//! 置けるのは自分の席の顔だけ。画像は口が読んで欠片に分け、机が継いで検めてから置く。

use rmcp::handler::server::wrapper::Parameters;
use warifu_capability::{Action, Gate, Grant};
use warifu_desk::{FromDesk, ToDesk, 受け口, 口 as 行の口};
use warifu_mcp::{AvatarArgs, ToolError, Warifu, subject, 顔の画像を読む};
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
    let 場所 = std::env::temp_dir().join(format!("warifu-mcp-顔-{}-{名}", std::process::id()));
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
fn 画像は_読んでから運ぶ() {
    let 中身 = 小さな_png();
    let 場所 = 置く("ok.png", &中身);
    assert_eq!(顔の画像を読む(場所.to_str().unwrap()).unwrap(), 中身);
}

#[test]
fn 空のパスは_既定へ戻す() {
    assert_eq!(顔の画像を読む("  ").unwrap(), Vec::<u8>::new());
}

#[test]
fn 大きすぎる画像は_読まずに断り_縮め方を言う() {
    let mut 中身 = 小さな_png();
    中身.resize(warifu_desk::顔の上限 + 1, 0);
    let 場所 = 置く("big.png", &中身);
    引数の断りか(顔の画像を読む(場所.to_str().unwrap()), "64 KiB");
}

#[test]
fn 画像でないものは_拡張子が画像でも断る() {
    let 場所 = 置く("text.png", b"hello, this is not an image");
    引数の断りか(顔の画像を読む(場所.to_str().unwrap()), "PNG でも");
}

#[test]
fn 無いファイルとフォルダは断る() {
    引数の断りか(顔の画像を読む("/そこには/無い/顔.png"), "読めませんでした");
    引数の断りか(
        顔の画像を読む(std::env::temp_dir().to_str().unwrap()),
        "ファイルではありません",
    );
}

#[tokio::test]
async fn 札が無ければ_顔も置けない() {
    // **名乗りと同じ書く札**（profile.write）。読む札では通らない
    let 口 = 用意(&["chat.send", "chat.read"]);
    let 出た = 口
        .avatar_set(Parameters(AvatarArgs {
            path: String::new(),
        }))
        .await;
    let 文 = format!("{:?}", 出た.unwrap_err());
    assert!(文.contains("profile.write"), "{文}");
}

/// 机の真似。名乗りを受けたあと、顔の行を `答え` で返していく。届いた顔の行を返す。
async fn 机を立てる(
    名: &str,
    答え: fn(&ToDesk, usize) -> FromDesk,
) -> (std::path::PathBuf, tokio::task::JoinHandle<Vec<ToDesk>>) {
    let 場所 = 試験の机(名);
    let mut 待ち = 受け口::開く(&場所).await.expect("この机が開くこと");
    let 机 = tokio::spawn(async move {
        let mut 口 = 行の口::新しく(待ち.受ける().await.unwrap());
        let _挨拶 = 口.受ける().await.unwrap().unwrap();
        // 本物の机と同じく、名乗りを受けたら「着いた」を返す（着く前の断りは、名乗りの断りと読まれる）
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
            let 返事 = 答え(&行, 届いた.len());
            let 終わり = !matches!(返事, FromDesk::顔の途中 { .. });
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
async fn 顔は欠片に分けて運び_誰の顔として置いたかを返す() {
    let 中身 = 小さな_png();
    let 画像 = 置く("seat.png", &中身);
    let (場所, 机) = 机を立てる("avatar-ok", |行, _| match 行 {
        ToDesk::顔 {
            終わり: false, ..
        } => FromDesk::顔の途中 { 受けた: 0 },
        _ => FromDesk::Wrote {
            who: "docs のエージェント".to_owned(),
        },
    })
    .await;
    let 口 = 用意(&["profile.write"])
        .この機械につながる_控えは(&場所, &試験の控え("avatar-ok"))
        .await
        .expect("つながれること");
    let 返り = 口
        .avatar_set(Parameters(AvatarArgs {
            path: 画像.to_str().unwrap().to_owned(),
        }))
        .await
        .expect("置けること");
    assert!(返り.contains("docs のエージェント"), "{返り}");

    // **机に届いた欠片を継ぐと、読んだ画像に戻る**
    let mut 継ぎ = warifu_desk::顔の継ぎ::default();
    let mut 継いだ = None;
    let 届いた = 机.await.unwrap();
    assert!(届いた.len() >= 2, "5000 バイトは 2 行に分かれる");
    for 行 in 届いた {
        let ToDesk::顔 { 欠片, 終わり } = 行 else {
            panic!("顔の行ではない: {行:?}");
        };
        継いだ = 継ぎ.足す(&欠片, 終わり).unwrap();
    }
    assert_eq!(継いだ, Some(中身));
}

#[tokio::test]
async fn 古い画面は_更新してくださいと返す() {
    let 画像 = 置く("old.png", &小さな_png());
    // 古い机は知らない行を「行の形が壊れています」で断る
    let (場所, 机) = 机を立てる("avatar-old", |_, _| FromDesk::Denied {
        why: "行の形が壊れています".to_owned(),
    })
    .await;
    let 口 = 用意(&["profile.write"])
        .この機械につながる_控えは(&場所, &試験の控え("avatar-old"))
        .await
        .expect("つながれること");
    let 文 = format!(
        "{:?}",
        口.avatar_set(Parameters(AvatarArgs {
            path: 画像.to_str().unwrap().to_owned(),
        }))
        .await
        .unwrap_err()
    );
    assert!(文.contains("顔を置けません"), "{文}");
    assert!(文.contains("更新"), "{文}");
    assert_eq!(机.await.unwrap().len(), 1, "断られたら残りの欠片を送らない");
}
