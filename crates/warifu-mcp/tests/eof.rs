//! **頼みを書いてすぐ入力を閉じても、返事は返る**（2026-10-07・#50）。
//!
//! 口が返事を書く前に終わっていたので、呼ぶ側が同じ頼みを出し直し、
//! **`voice_say` の声が相手の機械で何度も流れた。**
//!
//! **遅れて答える机**を立てる —— rmcp は入力が閉じたあと、走っている返事を 5 秒しか待たない。
//! 読み上げは 6 秒かかることがあり、そこで返事が捨てられていた。

use std::time::Duration;

use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use warifu_capability::{Action, Gate, Grant};
use warifu_desk::{FromDesk, ToDesk, 受け口, 口 as 行の口};
use warifu_mcp::{Warifu, subject, 閉じを待つ入力};
use warifu_read::RuleStore;

fn 試験の机() -> std::path::PathBuf {
    let 走り = std::process::id();
    #[cfg(windows)]
    {
        std::path::PathBuf::from(format!(r"\\.\pipe\warifu-mcp-eof-{走り}"))
    }
    #[cfg(not(windows))]
    {
        std::env::temp_dir().join(format!("warifu-mcp-eof-{走り}.sock"))
    }
}

fn 試験の控え() -> std::path::PathBuf {
    std::env::temp_dir().join(format!("warifu-mcp-eof-控え-{}", std::process::id()))
}

/// **声を 7 秒かけて流す机**（読み上げは時間がかかる）。
///
/// **5 秒より長くする** —— rmcp は入力が閉じると、走っている返事を **5 秒だけ**待って捨てる。
/// 1 秒や 3 秒の呼びは、包まなくても間に合ってしまう（実際に 1 秒で試して、穴を捕まえなかった）。
fn 遅い机を立てる(mut 待ち: 受け口) -> tokio::task::JoinHandle<usize> {
    tokio::spawn(async move {
        let mut 口 = 行の口::新しく(待ち.受ける().await.expect("繋がること"));
        let mut 声の数 = 0;
        while let Ok(Some(行)) = 口.受ける().await {
            if let Ok(ToDesk::声 { .. }) = ToDesk::読む(&行) {
                声の数 += 1;
                tokio::time::sleep(Duration::from_secs(7)).await;
                let 返し = FromDesk::声の返り {
                    結果: warifu_desk::声の結果::流した,
                };
                if 口.送る(&返し.書く()).await.is_err() {
                    break;
                }
            }
        }
        声の数
    })
}

#[tokio::test(flavor = "multi_thread")]
async fn 頼んですぐ閉じても_返事が返る() {
    use rmcp::ServiceExt as _;

    let 控え = 試験の控え();
    std::fs::create_dir_all(&控え).expect("フォルダが作れること");
    let 場所 = 試験の机();
    let 待ち = 受け口::開く(&場所).await.expect("この机が開くこと");
    let 机 = 遅い机を立てる(待ち);

    let mut 関所 = Gate::new();
    関所.issue(Grant::new(
        subject(),
        Action::new("voice.say").unwrap(),
        1_798_761_600,
    ));
    let 口 = Warifu::new(Vec::new(), RuleStore::new(), 関所, 1_756_000_000)
        .この機械につながる_控えは(&場所, &控え)
        .await
        .expect("つながれること");

    let 呼び = 口.呼び中();
    let (mut 頼む側, 口の入力) = tokio::io::duplex(64 * 1024);
    let (口の出力, mut 聞く側) = tokio::io::duplex(64 * 1024);
    let 入力 = 閉じを待つ入力::new(口の入力, 呼び, Duration::from_secs(10));

    // **口は別の走り場で回し、`waiting` が返ったら走り場ごと捨てる** ——
    // 実物は `main` が返るとプロセスが終わり、走っていた呼びも消える。
    // 同じ走り場で回すと、呼びが生き残って返事を書いてしまい、穴を捕まえない
    let 走る = std::thread::spawn(move || {
        let 走り場 = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        走り場.block_on(async move {
            let 務め = 口.serve((入力, 口の出力)).await.unwrap();
            let _ = 務め.waiting().await;
        });
        走り場.shutdown_background();
    });

    // 呼ぶ側の形: 3 行書いて、すぐ閉じる（返事を待たない）
    let 頼み = concat!(
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"t","version":"0"}}}"#,
        "\n",
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        "\n",
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"voice_say","arguments":{"text":"試験の声"}}}"#,
        "\n",
    );
    頼む側.write_all(頼み.as_bytes()).await.unwrap();
    頼む側.shutdown().await.unwrap();
    drop(頼む側);

    let mut 出た = String::new();
    tokio::time::timeout(Duration::from_secs(30), 聞く側.read_to_string(&mut 出た))
        .await
        .expect("口が閉じない")
        .unwrap();
    走る.join().unwrap();
    let _ = std::fs::remove_dir_all(&控え);

    assert!(
        出た.lines().any(|l| l.contains(r#""id":2"#)),
        "頼み（id 2）への返事が無い:\n{出た}"
    );
    // 机は口が閉じると終わる
    let 声の数 = tokio::time::timeout(Duration::from_secs(5), 机)
        .await
        .map_or(0, |r| r.unwrap_or(0));
    assert_eq!(声の数, 1, "声は 1 回だけ流す");
}
