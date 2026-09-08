//! `warifu relay` —— **預かり所を立てる。**
//!
//! これが LINE との本当の差だった —— LINE は中央のサーバが預かるから、
//! 相手が寝ていても届く。割符は**サーバーを 1 台も立てない**ので、
//! 相手が起動していなければ消えていた（**D60** の「B」）。
//!
//! ```text
//!   A ──封──▶ 預かり所（読めない）
//!                  │
//!   B が起動 ──────┘ 受け取って、預かり所からは消える
//! ```
//!
//! # ここで守ること
//!
//! - **中身を読まない。**封のまま持つ（**D69**）。開ける鍵を持っていない
//! - **鍵を持っている人だけ。**誰でも使える中継にしない ——
//!   すると、**立てた人が知らない誰かの通信を運ぶ**ことになる（`issues/004` の P5）
//! - **溜め込む場所にしない。**渡したら手放す。7 日で捨てる（**D70**）
//!
//! **これは「割符が用意する中央」ではない。**
//! **立てるのは導入した人**である（**D68**）。

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::Mutex;
use warifu_core::{PublicKey, Revocations};
use warifu_net::Node;
use warifu_post::{Box as 預かり所, Reply};
use warifu_postbox::{応える, 頼みを聞ける形か};

/// `warifu relay` の設定。
pub struct 設定 {
    /// 使ってよい人の一覧が書いてある所。
    ///
    /// **誰でも使える中継にしない。**書いていない相手は、繋いでも何も渡さない。
    pub 名簿: Option<PathBuf>,
}

/// 引数を読む。
pub fn 読む(args: &mut impl Iterator<Item = String>) -> Result<設定, String> {
    let mut 設 = 設定 { 名簿: None };
    while let Some(一つ) = args.next() {
        match 一つ.as_str() {
            "--allow-file" => {
                設.名簿 = Some(PathBuf::from(
                    args.next().ok_or("--allow-file のあとに場所がありません")?,
                ));
            }
            他 => return Err(format!("知らない指定です: {他}")),
        }
    }
    Ok(設)
}

/// 立てる。**閉じるまで戻らない。**
pub async fn 立てる(設: &設定) -> Result<(), Box<dyn std::error::Error>> {
    let 許す = 名簿を読む(設.名簿.as_deref())?;
    if 許す.is_empty() {
        // **誰でも使える中継にしない。**空で立てると、そうなってしまう
        return Err(
            "使ってよい人の一覧がありません。--allow-file <場所> に公開鍵を 1 行ずつ書いてください"
                .into(),
        );
    }

    let (_, device) = crate::identity::開く()?;
    let node = Node::bind_without_relay(&device).await?;
    let 宛先 = node.address().await?;

    println!("{宛先}");
    eprintln!(
        "warifu relay: 立ちました（使ってよい人 {} 名）。上の宛先を配ってください。",
        許す.len()
    );
    eprintln!(
        "預かるのは {} 日まで。中身は読めません。",
        warifu_post::預かれる日数
    );

    let 箱: Arc<Mutex<預かり所>> = Arc::new(Mutex::new(預かり所::new()));

    loop {
        let Ok(mut session) = node.accept(&Revocations::new()).await else {
            continue;
        };
        let 相手 = session.peer();
        // **名簿に無ければ、黙って落とす。**断る理由を返さない（**D31**）
        if !許す.contains(&相手.to_bytes()) {
            eprintln!("知らない相手が来ました（{}…）。落としました", 頭(相手));
            continue;
        }

        let 箱 = Arc::clone(&箱);
        tokio::spawn(async move {
            let 今 = crate::mcp::いま();
            let Ok(塊) = session.recv().await else {
                return;
            };
            // **応え方は `warifu-postbox` に 1 つだけ置く。**
            // ここに書き写すと、片方だけ直された形で残る
            let 返事 = match 頼みを聞ける形か(&塊) {
                Some(頼み) => 応える(&mut *箱.lock().await, 相手, 頼み, 今),
                None => Reply::Refused,
            };
            let _ = session.send(&返事.to_bytes()).await;
            let _ = session.finish().await;
        });
    }
}

/// 使ってよい人を読む。**1 行 1 公開鍵。**
fn 名簿を読む(
    場所: Option<&std::path::Path>,
) -> Result<HashSet<[u8; 32]>, Box<dyn std::error::Error>> {
    let Some(場所) = 場所 else {
        return Ok(HashSet::new());
    };
    let 中身 = std::fs::read_to_string(場所)?;
    Ok(中身
        .lines()
        .map(str::trim)
        // **書き間違いで全員を通さない。**読めない行は捨てる
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|l| l.parse::<PublicKey>().ok())
        .map(|k| k.to_bytes())
        .collect())
}

fn 頭(k: PublicKey) -> String {
    let s = k.to_string();
    s.chars().take(12).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use warifu_core::Seed;

    fn 鍵(seed: [u8; 32]) -> PublicKey {
        Seed::from_bytes(seed)
            .profile("Personal")
            .device("PC")
            .public_key()
    }

    /// 名簿を 1 つ書いて、その場所を返す。**試験ごとに別の名前にする**
    fn 置く(名: &str, 中身: &str) -> PathBuf {
        let 場所 =
            std::env::temp_dir().join(format!("warifu-relay-{}-{名}.txt", std::process::id()));
        std::fs::write(&場所, 中身).unwrap();
        場所
    }

    #[test]
    fn 名簿がなければ誰も居ない() {
        assert!(名簿を読む(None).unwrap().is_empty());
    }

    #[test]
    fn 一行一鍵で読む() {
        let 場所 = 置く("ふたり", &format!("{}\n{}\n", 鍵([1u8; 32]), 鍵([2u8; 32])));
        let 許す = 名簿を読む(Some(&場所)).unwrap();
        let _ = std::fs::remove_file(&場所);
        assert_eq!(許す.len(), 2);
        assert!(許す.contains(&鍵([1u8; 32]).to_bytes()));
    }

    #[test]
    fn 空行と注記は飛ばす() {
        let 場所 = 置く("注記", &format!("# 会社の分\n\n{}\n\n", 鍵([1u8; 32])));
        let 許す = 名簿を読む(Some(&場所)).unwrap();
        let _ = std::fs::remove_file(&場所);
        assert_eq!(許す.len(), 1);
    }

    #[test]
    fn 読めない行があっても他の行は生きる() {
        // **書き間違いで全員を締め出さない／全員を通さない**
        let 場所 = 置く("こわれ", &format!("こわれている\n{}\n", 鍵([1u8; 32])));
        let 許す = 名簿を読む(Some(&場所)).unwrap();
        let _ = std::fs::remove_file(&場所);
        assert_eq!(許す.len(), 1);
    }

    #[test]
    fn 名簿の場所を読み取る() {
        let mut args = ["--allow-file", "/tmp/だれ.txt"]
            .into_iter()
            .map(String::from);
        let 設 = 読む(&mut args).unwrap();
        assert_eq!(設.名簿, Some(PathBuf::from("/tmp/だれ.txt")));
    }

    #[test]
    fn 知らない指定は断る() {
        let mut args = ["--everyone"].into_iter().map(String::from);
        assert!(読む(&mut args).is_err());
    }
}
