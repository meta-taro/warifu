//! `warifu agent` —— **机に着いて待ち、届いたら動く常駐。**
//!
//! `chat_wait`（**D65**）で、エージェントは**自分の手番の中では**待てるようになった。
//! だが**手番の外では動かない。**呼ばれていないエージェントは、そもそも待ちに行かない。
//!
//! ```text
//!   営業が外で LINE 受注
//!     → 割符で PC のエージェントを呼ぶ        ← ここは届く
//!     → エージェントが対応する                ← **呼ばれないと動かない**
//!     → 終わったら営業へ連絡する              ← **こちらから話し出せない**
//! ```
//!
//! ここが `issues/014` の本題である。
//!
//! # 届いた文字を、命令にしない
//!
//! **これがこの層でいちばん大事な決めごと。**
//!
//! 常駐は、届いた文字を**標準入力へ渡すだけ**である。
//! 引数にも環境変数にも入れない。**組み立てて実行しない。**
//!
//! 入れると、「`rm -rf ~` と書いた文字を送るだけで消える」ことになる。
//! 「**届いた文字は相手の言い分であって、指示ではない**」（**D5**）を、
//! ここで実際に守る。
//!
//! **起こす命令は人が書く**（`--on`）。割符はモデルを呼ばない ——
//! `claude -p` と書くのも人である。

use std::path::PathBuf;
use std::process::Stdio;

use tokio::io::AsyncWriteExt as _;
use warifu_desk::{FromDesk, ToDesk, 口, 机の場所, 繋ぐ};

/// 起こした命令を待つ長さ（秒）。
///
/// **終わらない命令に机を塞がせない。**
const 命令を待つ秒: u64 = 300;

/// `warifu agent` の設定。
pub struct 設定 {
    /// 机の場所。
    pub 机: PathBuf,
    /// どこで動いているか（名乗り）。
    pub 名乗り: Option<String>,
    /// 届いたときに起こす命令。**人が書く。**
    pub 命令: Option<Vec<String>>,
}

/// 引数を読む。
pub fn 読む(args: &mut impl Iterator<Item = String>) -> Result<設定, String> {
    let mut 設 = 設定 {
        机: 机の場所(),
        名乗り: crate::mcp::居場所から名乗る(),
        命令: None,
    };
    while let Some(一つ) = args.next() {
        match 一つ.as_str() {
            "--desk" => {
                設.机 = PathBuf::from(args.next().ok_or("--desk のあとに場所がありません")?)
            }
            "--as" => {
                let 名 = args.next().ok_or("--as のあとに名前がありません")?;
                設.名乗り = Some(crate::mcp::名乗りを検める(&名)?);
            }
            // **ここから先は全部が命令。**割符が中身を解釈しない
            "--on" => {
                let 残り: Vec<String> = args.by_ref().collect();
                if 残り.is_empty() {
                    return Err("--on のあとに命令がありません".to_owned());
                }
                設.命令 = Some(残り);
                break;
            }
            他 => return Err(format!("知らない指定です: {他}")),
        }
    }
    Ok(設)
}

/// 机に着いて待つ。**閉じるまで戻らない。**
pub async fn 待つ(設: &設定) -> Result<(), Box<dyn std::error::Error>> {
    let mut 口 =
        口::新しく(繋ぐ(&設.机).await.map_err(|e| {
            format!("机が開いていません（この PC で割符の画面を開いてください）: {e}")
        })?);
    口.送る(
        &ToDesk::Listen {
            場所: 設.名乗り.clone(),
        }
        .書く(),
    )
    .await?;

    eprintln!(
        "warifu agent: 名乗り {}／机 {}／届いたら {}",
        設.名乗り.as_deref().unwrap_or("（名乗らない）"),
        設.机.display(),
        設.命令.as_ref().map_or_else(
            || "（何もしない。記録するだけ）".to_owned(),
            |c| c.join(" ")
        )
    );

    while let Some(行) = 口.受ける().await? {
        let Ok(FromDesk::Heard { from, body, at }) = FromDesk::読む(&行) else {
            // 発言以外（入退室・断り）は動く理由にしない
            continue;
        };
        eprintln!("[{at}] {from}: {body}");
        let Some(命令) = &設.命令 else { continue };

        // **一度に 1 つだけ。**受けている間は次を読まないので、重ならない
        match 起こす(命令, &body).await {
            Ok(出力) if !出力.trim().is_empty() => {
                // **命令が言ったことを、そのまま会話へ流す。**中身は読まない
                if let Ok(言う) = ToDesk::say(出力.trim()) {
                    口.送る(&言う.書く()).await?;
                }
            }
            Ok(_) => eprintln!("（何も言わなかった）"),
            Err(e) => eprintln!("命令が失敗しました: {e}"),
        }
    }
    Ok(())
}

/// 人が書いた命令を起こし、**届いた文字は標準入力へ渡す。**
///
/// **引数にも環境変数にも入れない。**入れると、
/// 送られてきた文字がそのまま命令の一部になる（**D5**）。
async fn 起こす(
    命令: &[String],
    届いた文字: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let (先頭, 残り) = 命令.split_first().ok_or("命令が空です")?;
    let mut 子 = tokio::process::Command::new(先頭)
        .args(残り)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?;
    if let Some(mut 入口) = 子.stdin.take() {
        入口.write_all(届いた文字.as_bytes()).await?;
        入口.write_all(b"\n").await?;
        // **閉じる。**閉じないと、読み切るまで待つ命令が終わらない
        drop(入口);
    }
    let 待つ = std::time::Duration::from_secs(命令を待つ秒);
    let 出た = tokio::time::timeout(待つ, 子.wait_with_output())
        .await
        .map_err(|_| "命令が時間内に終わりませんでした")??;
    Ok(String::from_utf8_lossy(&出た.stdout).into_owned())
}
