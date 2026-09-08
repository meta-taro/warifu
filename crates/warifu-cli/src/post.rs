//! `warifu post` —— **預かり所を、画面なしで確かめる。**
//!
//! 立てた人が「**本当に預かれているか**」を、その場で見られるようにする。
//! 画面（`warifu.app`）を 2 台用意しなくても、1 台で往復が通せる。
//!
//! ```text
//!   warifu post put  --at <預かり所> --to <相手>   本文は標準入力から
//!   warifu post take --at <預かり所>               自分あてを受け取る（消える）
//! ```
//!
//! # ここで守ること
//!
//! - **封をしてから預ける。**預かり所は中身を読めない（**D69**）
//! - **受け取れるのは自分あてだけ。**尋ねる側に宛先は名乗らせない（**D71**）
//! - **署名で差出人を確かめる。**名乗りだけの言葉は通さない（**D72**）

use std::str::FromStr as _;

use warifu_core::PublicKey;
use warifu_net::{Address, Node};

/// `warifu post` の設定。
pub struct 設定 {
    /// 何をするか。
    pub 何を: 何を,
    /// 預かり所の宛先。
    pub 預かり所: String,
    /// 誰あてか（`put` のとき）。**呼び名でも公開鍵でも引ける。**
    pub 宛先: Option<String>,
}

/// `warifu post` でできること。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum 何を {
    /// 預ける。
    預ける,
    /// 受け取る。
    受け取る,
}

/// 引数を読む。
///
/// # Errors
/// 知らない指定・足りない指定のとき。
pub fn 読む(args: &mut impl Iterator<Item = String>) -> Result<設定, String> {
    let 何を = match args.next().as_deref() {
        Some("put") => 何を::預ける,
        Some("take") => 何を::受け取る,
        Some(他) => return Err(format!("put か take です（{他}）")),
        None => return Err("put か take を指定してください".to_owned()),
    };
    let mut 預かり所 = None;
    let mut 宛先 = None;
    while let Some(一つ) = args.next() {
        match 一つ.as_str() {
            "--at" => 預かり所 = args.next(),
            "--to" => 宛先 = args.next(),
            他 => return Err(format!("知らない指定です: {他}")),
        }
    }
    let 預かり所 = 預かり所.ok_or("--at に預かり所の宛先を指定してください")?;
    // **預ける先が無いまま預けさせない。**どこへ行くか分からないものを送らない
    if 何を == 何を::預ける && 宛先.is_none() {
        return Err("--to に相手（呼び名か公開鍵）を指定してください".to_owned());
    }
    Ok(設定 {
        何を,
        預かり所,
        宛先,
    })
}

/// 走らせる。
///
/// # Errors
/// 身元を開けないとき、繋がらないとき、預かり所が断ったとき。
pub async fn 走る(設: &設定) -> Result<(), Box<dyn std::error::Error>> {
    let (vault, device) = crate::identity::開く()?;
    let 所 =
        Address::from_str(&設.預かり所).map_err(|_| "預かり所の宛先として読めません".to_owned())?;
    let node = Node::bind_without_relay(&device).await?;

    match 設.何を {
        何を::預ける => {
            let 相手 = 相手を決める(&vault, 設.宛先.as_deref().unwrap_or_default())?;
            // **本文は標準入力から。**引数に置くと、履歴と `ps` に残る
            let 本文 = std::io::read_to_string(std::io::stdin())?;
            if 本文.trim().is_empty() {
                return Err("本文が空です（標準入力から渡してください）".into());
            }
            warifu_postbox::預ける(
                &node,
                &所,
                &device,
                相手,
                crate::mcp::いま(),
                本文.as_bytes(),
            )
            .await?;
            println!("預けました（{} バイト）", 本文.len());
        }
        何を::受け取る => {
            let 手紙たち = warifu_postbox::受け取る(&node, &device, &所).await?;
            println!("{} 通", 手紙たち.len());
            let 名簿 = vault.contacts().unwrap_or_default();
            for 手紙 in &手紙たち {
                // **誰が言ったかは署名で確かめたもの**（名乗りではない・D72）
                println!(
                    "{}\t{}\t{}",
                    いつ(手紙.時刻),
                    crate::identity::呼び名(&名簿, 手紙.差出人),
                    String::from_utf8_lossy(&手紙.本文)
                );
            }
        }
    }
    Ok(())
}

/// 出した側の時計を、人が読める形にする（**UTC**）。
///
/// **`Z` を付けて、どの時計かを言う。**手元の時刻に見せると、
/// 時差のある相手からの手紙が「未来から届いた」ように読める。
/// **これは出した側の時計**であって、受け取った時刻ではない（**D72**）。
#[must_use]
fn いつ(秒: u64) -> String {
    // 1970-01-01 からの日数と、その日の中の秒に割る
    let 日 = i64::try_from(秒 / 86_400).unwrap_or(0);
    let 中 = 秒 % 86_400;
    // Howard Hinnant の civil_from_days（うるう年の分岐を書かずに済む）
    let z = 日 + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!(
        "{y:04}-{m:02}-{d:02} {:02}:{:02}Z",
        中 / 3600,
        (中 % 3600) / 60
    )
}

/// 呼び名でも公開鍵でも引く。
fn 相手を決める(vault: &warifu_vault::Vault, 言葉: &str) -> Result<PublicKey, String> {
    let 名簿 = vault.contacts().map_err(|e| e.to_string())?;
    crate::identity::相手を引く(&名簿, 言葉).ok_or_else(|| format!("相手が分かりません（{言葉}）"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn 読ませる(引数: &[&str]) -> Result<設定, String> {
        読む(&mut 引数.iter().map(|s| (*s).to_owned()))
    }

    #[test]
    fn 預けるには_相手が要る() {
        // **どこへ行くか分からないものを送らない**
        assert!(読ませる(&["put", "--at", "WARIFU1-X"]).is_err());
    }

    #[test]
    fn 受け取るのに相手は要らない() {
        // **自分あてだけが返る。**尋ねる側は宛先を名乗れない（D71）
        let 設 = 読ませる(&["take", "--at", "WARIFU1-X"]).unwrap();
        assert_eq!(設.何を, 何を::受け取る);
        assert_eq!(設.宛先, None);
    }

    #[test]
    fn 預かり所を書かなければ断る() {
        assert!(読ませる(&["take"]).is_err());
    }

    #[test]
    fn 知らない指定は断る() {
        assert!(読ませる(&["peek", "--at", "X"]).is_err());
        assert!(読ませる(&["take", "--at", "X", "--all"]).is_err());
    }

    #[test]
    fn 時刻は人が読める形で出る() {
        // **どの時計かを言う。**手元の時刻に見せると、時差のある相手からの手紙が
        // 「未来から届いた」ように読める
        assert_eq!(いつ(0), "1970-01-01 00:00Z");
        assert_eq!(いつ(1_788_869_048), "2026-09-08 12:04Z");
        // うるう年の 2 月 29 日
        assert_eq!(いつ(1_709_164_800), "2024-02-29 00:00Z");
    }

    #[test]
    fn 呼び名でも公開鍵でも受ける() {
        // どちらで書けるかは `identity::相手を引く` が決める
        let 設 = 読ませる(&["put", "--at", "WARIFU1-X", "--to", "mac air"]).unwrap();
        assert_eq!(設.宛先.as_deref(), Some("mac air"));
    }
}
