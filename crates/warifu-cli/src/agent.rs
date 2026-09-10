//! `warifu agent` —— **この機械につながって待ち、届いたら動く常駐。**
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
use warifu_desk::{FromDesk, ToDesk, この機械の場所, 口, 繋ぐ};

/// 起こした命令を待つ長さ（秒）。
///
/// **終わらない命令にこの機械を塞がせない。**
const 命令を待つ秒: u64 = 300;

/// 窓のあいだに動いてよい回数。
///
/// **人が居ない間に動くものに、上限を置かないわけにいかない。**
/// 洪水を送るだけで、この機械の上で命令を何度でも起こせることになる
/// （戸口が知らない相手の叩きに上限を置いているのと同じ理由・**D31**）。
const 窓のあいだに動ける回数: usize = 60;

/// 数える窓の長さ（秒）。
const 窓の秒: u64 = 60;

/// 動いた回数を、窓のあいだで数える。
struct 回数 {
    窓のはじまり: std::time::Instant,
    動いた: usize,
}

impl 回数 {
    fn 新しく() -> Self {
        Self {
            窓のはじまり: std::time::Instant::now(),
            動いた: 0,
        }
    }

    /// 動いてよいか。**上限を超えたら断る。**
    fn 動いてよい(&mut self) -> bool {
        if self.窓のはじまり.elapsed().as_secs() >= 窓の秒 {
            *self = Self::新しく();
        }
        self.動いた += 1;
        self.動いた <= 窓のあいだに動ける回数
    }
}

/// `warifu agent` の設定。
pub struct 設定 {
    /// この機械の場所。
    pub この機械: PathBuf,
    /// どこで動いているか（名乗り）。
    pub 名乗り: Option<String>,
    /// 届いたときに起こす命令。**人が書く。**
    pub 命令: Option<Vec<String>>,
}

/// 引数を読む。
pub fn 読む(args: &mut impl Iterator<Item = String>) -> Result<設定, String> {
    let mut 設 = 設定 {
        この機械: この機械の場所(),
        名乗り: crate::mcp::居場所から名乗る(),
        命令: None,
    };
    while let Some(一つ) = args.next() {
        match 一つ.as_str() {
            "--desk" => {
                設.この機械 = PathBuf::from(args.next().ok_or("--desk のあとに場所がありません")?)
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

/// この機械に着いて待つ。**閉じるまで戻らない。**
pub async fn 待つ(設: &設定) -> Result<(), Box<dyn std::error::Error>> {
    let mut 口 = 口::新しく(繋ぐ(&設.この機械).await.map_err(|e| {
        format!("この機械が開いていません（この PC で割符の画面を開いてください）: {e}")
    })?);
    口.送る(
        &ToDesk::Listen {
            場所: 設.名乗り.clone(),
        }
        .書く(),
    )
    .await?;

    eprintln!(
        "warifu agent: 名乗り {}／この機械 {}／届いたら {}",
        設.名乗り.as_deref().unwrap_or("（名乗らない）"),
        設.この機械.display(),
        設.命令.as_ref().map_or_else(
            || "（何もしない。記録するだけ）".to_owned(),
            |c| c.join(" ")
        )
    );

    let mut 数 = 回数::新しく();
    while let Some(行) = 口.受ける().await? {
        let (from, body, at) = match FromDesk::読む(&行) {
            Ok(FromDesk::Heard { from, body, at, .. }) => (from, body, at),
            // **人が画面から止めた。**落とすしか止め方が無い状態にしない
            Ok(FromDesk::Stop) => {
                eprintln!("止まれと言われました。降ります。");
                return Ok(());
            }
            // 入退室・断りは動く理由にしない
            _ => continue,
        };
        eprintln!("[{at}] {from}: {body}");
        let Some(命令) = &設.命令 else { continue };

        // **上限を超えたら動かない。**洪水で命令を起こし続けられないようにする
        if !数.動いてよい() {
            eprintln!(
                "（{}秒に{}回を超えたので動きません）",
                窓の秒, 窓のあいだに動ける回数
            );
            continue;
        }

        // **自分が言ったことで動かない。**返した文字がまた自分へ返ると、
        // 止まらなくなる（この機械は言った本人に返さないが、**別のエージェントの自分**は別物である）
        if 設
            .名乗り
            .as_deref()
            .is_some_and(|名| from == format!("{名} の AI"))
        {
            eprintln!("（自分の発言なので動きません）");
            continue;
        }

        // **一度に 1 つだけ。**受けている間は次を読まないので、重ならない
        match 起こす(命令, &body).await {
            Ok(出力) if !出力.trim().is_empty() => {
                // **命令が言ったことを、そのまま会話へ流す。**中身は読まない
                if let Ok(言う) = ToDesk::say(出力.trim()) {
                    口.送る(&言う.書く()).await?;
                }
            }
            // **何も言わなかったことも、会話に残す。**
            // 人が居ない間に動くので、**何も出ないと「動いたのか」が分からない**
            Ok(_) => 言い残す(&mut 口, "（動きましたが、何も言いませんでした）").await?,
            Err(e) => {
                eprintln!("命令が失敗しました: {e}");
                言い残す(&mut 口, &format!("（動きましたが、失敗しました: {e}）")).await?;
            }
        }
    }
    Ok(())
}

/// 会話に一言残す。**残せなくても止めない。**
///
/// 人が居ない間に動くので、**何も出ないと「動いたのか」が分からない。**
async fn 言い残す(
    口: &mut 口<impl warifu_desk::一本>,
    一言: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("{一言}");
    if let Ok(言う) = ToDesk::say(一言) {
        口.送る(&言う.書く()).await?;
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
