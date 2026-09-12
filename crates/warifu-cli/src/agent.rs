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

/// 繋ぎ直すまでの、はじめの待ち（秒）。
///
/// **画面を落とすと、この機械の口は閉じる。**常駐なので繋ぎ直すが、
/// **すぐ叩き続けると閉じている間ずっと回り続ける。**
const 待ちのはじめ: u64 = 2;

/// 繋ぎ直すまでの待ちの上限（秒）。
///
/// **伸ばしすぎない。**人が画面を開け直したのに何分も黙っていては、
/// 「常駐しているのに反応しない」に見える。
const 待ちの上限: u64 = 30;

/// 何度目かに応じた待ち（秒）。**だんだん伸ばして、上限で止める。**
fn 繋ぎ直す間(何度目: u32) -> u64 {
    待ちのはじめ
        .saturating_mul(2_u64.saturating_pow(何度目.min(10)))
        .min(待ちの上限)
}

/// **差出人を、端末に出せる形にする。**
///
/// 呼び名が付いていればそのまま。付いていない相手は公開鍵のままで来るので、
/// **追える程度に切る**（画面と同じ切り方）——
/// 2026-09-11 に実物で出た。画面は `F7KROW4U2SNH…` と切っているのに、
/// **エージェントの端末には 52 桁が丸ごと出ていた。**
fn 読める差出人(差出人: &str) -> String {
    // base32 の公開鍵は 52 文字で、英大文字と数字しか使わない。
    // **それに当てはまるときだけ切る** —— 人が付けた名前は触らない
    let 鍵に見える = 差出人.len() == 52
        && 差出人
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit());
    if 鍵に見える {
        return 差出人.chars().take(12).collect::<String>() + "…";
    }
    差出人.to_owned()
}

/// **自分の発言か。**返した文字がまた自分へ返ると、止まらなくなる。
///
/// この機械は言った本人に返さないが、**別のエージェントとして映った自分**は別物である。
fn 自分の発言か(名乗り: Option<&str>, 差出人: &str) -> bool {
    名乗り.is_some_and(|名| 差出人 == format!("{名} の AI"))
}

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

/// この機械につながって待つ。**閉じるまで戻らない。**
pub async fn 待つ(設: &設定) -> Result<(), Box<dyn std::error::Error>> {
    // **はじめの 1 回だけは、繋がらなければ理由を出して終わる。**
    // 画面を開いていない人に、黙って回り続ける物を渡さない
    let mut 口 = 口::新しく(繋ぐ(&設.この機械).await.map_err(|e| {
        format!("この機械が開いていません（この PC で割符の画面を開いてください）: {e}")
    })?);
    // **初めて繋ぐときは、過去をもらわない**（頼まれてもいない過去を押し付けない）
    let mut 最後に聞いた: Option<u64> = None;
    口.送る(
        &ToDesk::Listen {
            場所: 設.名乗り.clone(),
            どこから: 最後に聞いた,
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
    // **閉じたら繋ぎ直す。**画面を建て直すたびに常駐が死ぬのでは、常駐ではない
    // （2026-09-11 にオーナーの手元で実際に落ちた）
    let mut 何度目 = 0_u32;
    loop {
        match 一巡(設, &mut 口, &mut 数, &mut 最後に聞いた).await {
            // 人が「止まれ」と言った／自分から降りた
            Ok(降りる::止まる) => return Ok(()),
            Ok(降りる::切れた) => {}
            Err(e) => eprintln!("この機械との行き違い: {e}"),
        }
        let 待ち = 繋ぎ直す間(何度目);
        eprintln!("（この機械が閉じました。{待ち} 秒後に繋ぎ直します）");
        tokio::time::sleep(std::time::Duration::from_secs(待ち)).await;
        match 繋ぐ(&設.この機械).await {
            Ok(一本) => {
                口 = 口::新しく(一本);
                // **どこまで聞いたかを名乗る**（`.claude/issues/019`）。
                // これが無いと、**切れている間に届いた言葉を永久に聞けない。**
                // 繋ぎ直せてはいるのに落ちている、という**見えない取りこぼし**になっていた
                if let Err(e) = 口
                    .送る(
                        &ToDesk::Listen {
                            場所: 設.名乗り.clone(),
                            どこから: 最後に聞いた,
                        }
                        .書く(),
                    )
                    .await
                {
                    eprintln!("（名乗れませんでした: {e}）");
                    何度目 = 何度目.saturating_add(1);
                    continue;
                }
                eprintln!("（繋ぎ直しました）");
                何度目 = 0;
            }
            Err(_) => 何度目 = 何度目.saturating_add(1),
        }
    }
}

/// 一巡の終わり方。
enum 降りる {
    /// 人が「止まれ」と言った。**繋ぎ直さない。**
    止まる,
    /// この機械が閉じた。**繋ぎ直す。**
    切れた,
}

/// この機械が閉じるまで、届いた行で命令を起こし続ける。
async fn 一巡(
    設: &設定,
    口: &mut 口<impl warifu_desk::一本>,
    数: &mut 回数,
    最後に聞いた: &mut Option<u64>,
) -> Result<降りる, Box<dyn std::error::Error>> {
    while let Some(行) = 口.受ける().await? {
        let (from, body, at) = match FromDesk::読む(&行) {
            Ok(FromDesk::Heard { id, from, body, at }) => {
                // **どこまで聞いたかを覚える。**繋ぎ直したときにここから続ける。
                // **番号が無い（0）机とも繋がる** —— そのときは覚えない
                if id > 0 {
                    *最後に聞いた = Some(id);
                }
                (from, body, at)
            }
            // **人が画面から止めた。**落とすしか止め方が無い状態にしない
            Ok(FromDesk::Stop) => {
                eprintln!("止まれと言われました。降ります。");
                return Ok(降りる::止まる);
            }
            // 入退室・断りは動く理由にしない
            _ => continue,
        };
        eprintln!("[{at}] {}: {body}", 読める差出人(&from));
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
        if 自分の発言か(設.名乗り.as_deref(), &from) {
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
            Ok(_) => 言い残す(口, "（動きましたが、何も言いませんでした）").await?,
            Err(e) => {
                eprintln!("命令が失敗しました: {e}");
                言い残す(口, &format!("（動きましたが、失敗しました: {e}）")).await?;
            }
        }
    }
    // 受ける口が閉じた＝画面が落ちた
    Ok(降りる::切れた)
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

#[cfg(test)]
mod tests {
    use super::*;

    // `.claude/issues/019` —— **繋ぎ直したエージェントが、切れている間の言葉を聞けない。**
    // 繋ぎ直せてはいるのに落ちている、という**見えない取りこぼし**だった。
    #[tokio::test]
    async fn 聞いた番号を覚える() {
        let (こちら, むこう) = tokio::io::duplex(4096);
        let 行 = FromDesk::Heard {
            id: 7,
            from: "だれか".to_owned(),
            body: "やあ".to_owned(),
            at: "18:00".to_owned(),
        }
        .書く();
        // **相手側を書いて閉じる。**閉じないと `一巡` が待ち続ける
        tokio::spawn(async move {
            use tokio::io::AsyncWriteExt;
            let mut むこう = むこう;
            let _ = むこう.write_all(行.as_bytes()).await;
            let _ = むこう.write_all(b"\n").await;
            let _ = むこう.shutdown().await;
        });
        let 設 = 設定 {
            名乗り: Some("試し".to_owned()),
            この機械: std::path::PathBuf::from("/dev/null"),
            // **命令は持たせない。**ここで見たいのは番号を覚えるかだけ
            命令: None,
        };
        let mut 口 = 口::新しく(こちら);
        let mut 数 = 回数::新しく();
        let mut 最後に聞いた = None;
        let 終わり = 一巡(&設, &mut 口, &mut 数, &mut 最後に聞いた).await;
        assert!(matches!(終わり, Ok(降りる::切れた)));
        assert_eq!(最後に聞いた, Some(7));
    }

    #[tokio::test]
    async fn 番号の無い机では覚えない() {
        // **古い机は 0 を返す。**0 は「番号が無い」と読む約束（`line.rs`）——
        // **0 を覚えると、次に繋いだとき全部もらい直すことになる**
        let (こちら, むこう) = tokio::io::duplex(4096);
        let 行 = r#"{"型":"heard","id":0,"from":"だれか","body":"やあ","at":"18:00"}"#;
        tokio::spawn(async move {
            use tokio::io::AsyncWriteExt;
            let mut むこう = むこう;
            let _ = むこう.write_all(行.as_bytes()).await;
            let _ = むこう.write_all(b"\n").await;
            let _ = むこう.shutdown().await;
        });
        let 設 = 設定 {
            名乗り: None,
            この機械: std::path::PathBuf::from("/dev/null"),
            命令: None,
        };
        let mut 口 = 口::新しく(こちら);
        let mut 数 = 回数::新しく();
        let mut 最後に聞いた = None;
        let _ = 一巡(&設, &mut 口, &mut 数, &mut 最後に聞いた).await;
        assert_eq!(最後に聞いた, None);
    }

    #[test]
    fn 繋ぎ直す間は_だんだん伸ばして_上限で止める() {
        // **画面を落とすと、この機械の口は閉じる。**
        // 2026-09-11 に実物で出た —— オーナーが画面を建て直したら、
        // **常駐していた `warifu agent` が一緒に落ちていた**
        // （`chat_wait` は繋ぎ直すのに、こちらは落ちたまま）。
        //
        // すぐ叩き続けると、閉じている間ずっと回り続ける ——
        // **だんだん伸ばして、上限で止める。**
        assert_eq!(繋ぎ直す間(0), 待ちのはじめ);
        assert_eq!(繋ぎ直す間(1), 待ちのはじめ * 2);
        assert_eq!(繋ぎ直す間(2), 待ちのはじめ * 4);
        // **上限を超えて伸びない**（何時間も黙らない）
        assert_eq!(繋ぎ直す間(99), 待ちの上限);
        assert!(繋ぎ直す間(99) <= 待ちの上限);
    }

    #[test]
    fn 名前の無い差出人は_短く出す() {
        // **2026-09-11 に実物で出た。**画面は `F7KROW4U2SNH…` と切っているのに、
        // エージェントの端末には**52 桁の生の公開鍵**が出ていた。
        // 追える程度に残して、**全桁は出さない**（画面と同じ切り方）。
        let 鍵 = "F7KROW4U2SNHTL3ER3233NVAS325AHQX57XJVP5OBBAWHU35XHNQ";
        assert_eq!(読める差出人(鍵), "F7KROW4U2SNH…");
    }

    #[test]
    fn 呼び名が付いている差出人は_そのまま出す() {
        // **人が付けた名前を切らない。**切るのは鍵だけである
        assert_eq!(読める差出人("B役"), "B役");
        assert_eq!(読める差出人("souta の AI"), "souta の AI");
        // 32 文字を超える名前でも、鍵でなければ触らない
        let 長い名 = "とてもながいよびなをつけたひとのなまえですここまでくる";
        assert_eq!(読める差出人(長い名), 長い名);
    }

    #[test]
    fn 自分の発言では動かない() {
        // 返した文字がまた自分へ返ると、止まらなくなる
        assert!(自分の発言か(Some("souta"), "souta の AI"));
        assert!(!自分の発言か(Some("souta"), "taro の AI"));
        assert!(!自分の発言か(None, "souta の AI"));
    }
}
