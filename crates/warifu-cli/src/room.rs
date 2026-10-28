//! **`warifu room` —— この機械の割符が居る部屋を、端末から片付ける**（0.1.41）。
//!
//! 画面を開かずに「どの部屋に居るか」「抜ける」「名前を付ける」ができる。
//! 口（`warifu mcp`）の `room_list` / `room_leave` / `room_rename` と同じ行（[`ToDesk`]）を机へ送る。
//! **抜ける・名前を付けるには札（`room.leave` / `room.rename`）が要る** —— 人が画面で［通す］を押すか、
//! ［24 時間聞かない］を出しておく。無ければ机が断り、その理由をそのまま出す。

use std::time::Duration;

use warifu_desk::{FromDesk, ToDesk, この機械の場所, 口, 繋ぐ};

/// 机の返事を待つ上限。
const 返事を待つ秒: u64 = 5;

/// 何をするか。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum 指示 {
    /// 居る部屋を並べる。
    並べる,
    /// 部屋を抜ける（id の全文か頭）。
    抜ける(String),
    /// 部屋に名前を付ける。
    名前を付ける(String, String),
}

/// 引数を読む。
///
/// # Errors
/// 指定が足りない・知らない指定。
pub fn 読む(args: &mut impl Iterator<Item = String>) -> Result<指示, String> {
    match args.next().as_deref() {
        None | Some("list") => Ok(指示::並べる),
        Some("leave") => args
            .next()
            .filter(|s| !s.trim().is_empty())
            .map(|s| 指示::抜ける(s.trim().to_owned()))
            .ok_or_else(|| "使い方: warifu room leave <部屋の id か頭>".to_owned()),
        Some("rename") => match (args.next(), args.next()) {
            (Some(部屋), Some(名前)) if !部屋.trim().is_empty() && !名前.trim().is_empty() => {
                Ok(指示::名前を付ける(
                    部屋.trim().to_owned(),
                    名前.trim().to_owned(),
                ))
            }
            _ => Err("使い方: warifu room rename <部屋の id か頭> <名前>".to_owned()),
        },
        Some(other) => Err(format!(
            "知らない指定です: {other}（list / leave / rename）"
        )),
    }
}

/// 机へ送る行。
fn 行にする(指示: &指示) -> ToDesk {
    match 指示 {
        指示::並べる => ToDesk::部屋たち,
        指示::抜ける(部屋) => ToDesk::抜ける {
            部屋: 部屋.clone()
        },
        指示::名前を付ける(部屋, 名前) => ToDesk::名前を付ける {
            部屋: 部屋.clone(),
            名前: 名前.clone(),
        },
    }
}

/// その指示への返事か（会話の行や入退室は読み飛ばす）。
fn 返事か(指示: &指示, 中身: &FromDesk) -> bool {
    match 指示 {
        指示::並べる => matches!(中身, FromDesk::部屋たち { .. } | FromDesk::Denied { .. }),
        指示::抜ける(_) => matches!(中身, FromDesk::抜けた { .. } | FromDesk::Denied { .. }),
        指示::名前を付ける(..) => {
            matches!(中身, FromDesk::付けた { .. } | FromDesk::Denied { .. })
        }
    }
}

/// 返事を人の言葉にする。`Err` は断られた理由。
fn 言い方(中身: FromDesk) -> Result<String, String> {
    match 中身 {
        FromDesk::部屋たち { 部屋 } => Ok(warifu_mcp::部屋の一覧の言い方(&部屋)),
        FromDesk::抜けた { 部屋 } => Ok(format!("部屋 {} を抜けました。", 頭(&部屋))),
        FromDesk::付けた { 部屋, 名前 } => {
            Ok(format!("部屋 {} に「{名前}」と付けました。", 頭(&部屋)))
        }
        FromDesk::Denied { why } => Err(why),
        他 => Err(format!("この機械が想定しない返事をしました: {他:?}")),
    }
}

fn 頭(id: &str) -> String {
    format!("{}…", id.chars().take(12).collect::<String>())
}

/// 端末から部屋を片付ける。
///
/// # Errors
/// この機械が開いていない・断られた・返事が来ない。
pub async fn 口(
    args: &mut impl Iterator<Item = String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let 指示 = 読む(args)?;
    let 場所 = この機械の場所();
    let mut 口 = 口::返事用(繋ぐ(&場所).await.map_err(|e| {
        format!("この機械が開いていません（この PC で割符の画面を開いてください）: {e}")
    })?);
    口.送る(
        &ToDesk::Listen {
            場所: Some("warifu room".to_owned()),
            どこから: None,
            部屋も聞く: false,
        }
        .書く(),
    )
    .await?;
    // 着いたことを確かめてから頼む。**名乗る前の行は机が断る**
    let 着いた = 返事を待つ(&mut 口, |中身| {
        matches!(中身, FromDesk::Seated { .. } | FromDesk::Denied { .. })
    })
    .await?;
    if let FromDesk::Denied { why } = 着いた {
        return Err(format!("この機械に着けませんでした: {why}").into());
    }
    口.送る(&行にする(&指示).書く()).await?;
    let 返事 = 返事を待つ(&mut 口, |中身| 返事か(&指示, 中身)).await?;
    let 文 = 言い方(返事).map_err(|why| format!("できませんでした: {why}"))?;
    println!("{文}");
    Ok(())
}

/// 条件に合う返事が来るまで読む。会話の行などは読み飛ばす。
async fn 返事を待つ(
    口: &mut 口<impl warifu_desk::一本>,
    合うか: impl Fn(&FromDesk) -> bool,
) -> Result<FromDesk, Box<dyn std::error::Error>> {
    let 限り = Duration::from_secs(返事を待つ秒);
    let 読む = async {
        while let Some(行) = 口.受ける().await? {
            if let Ok(中身) = FromDesk::読む(&行)
                && 合うか(&中身)
            {
                return Ok(中身);
            }
        }
        Err::<FromDesk, Box<dyn std::error::Error>>("この機械が閉じました".into())
    };
    tokio::time::timeout(限り, 読む)
        .await
        .map_err(|_| format!("この機械が {返事を待つ秒} 秒たっても返事をしません"))?
}

#[cfg(test)]
mod 試験 {
    use super::*;

    fn 引数(語: &[&str]) -> impl Iterator<Item = String> {
        語.iter()
            .map(|s| (*s).to_owned())
            .collect::<Vec<_>>()
            .into_iter()
    }

    #[test]
    fn 指定を読む_無しと_list_は並べる_leave_と_rename_は中身が要る() {
        assert_eq!(読む(&mut 引数(&[])).unwrap(), 指示::並べる);
        assert_eq!(読む(&mut 引数(&["list"])).unwrap(), 指示::並べる);
        assert_eq!(
            読む(&mut 引数(&["leave", "NBIW"])).unwrap(),
            指示::抜ける("NBIW".to_owned())
        );
        assert!(読む(&mut 引数(&["leave"])).is_err());
        assert!(読む(&mut 引数(&["leave", " "])).is_err());
        assert_eq!(
            読む(&mut 引数(&["rename", "NBIW", "開発6台"])).unwrap(),
            指示::名前を付ける("NBIW".to_owned(), "開発6台".to_owned())
        );
        assert!(読む(&mut 引数(&["rename", "NBIW"])).is_err());
        assert!(読む(&mut 引数(&["drop"])).is_err());
    }

    #[test]
    fn 返事は指示ごとに見分け_断りはどれにも返事() {
        let 断り = FromDesk::Denied {
            why: "x".to_owned(),
        };
        let 抜けた = FromDesk::抜けた {
            部屋: "NBIW".to_owned(),
        };
        assert!(返事か(&指示::並べる, &断り));
        assert!(返事か(&指示::抜ける("NBIW".to_owned()), &抜けた));
        assert!(!返事か(&指示::並べる, &抜けた));
        assert!(!返事か(
            &指示::抜ける("NBIW".to_owned()),
            &FromDesk::Stop
        ));
    }

    #[test]
    fn 言い方_抜けた_付けた_断り() {
        let 文 = 言い方(FromDesk::抜けた {
            部屋: "NBIW3PA2TUQ52DTJNVXSQOL3G4".to_owned(),
        })
        .unwrap();
        assert_eq!(文, "部屋 NBIW3PA2TUQ5… を抜けました。");
        let 文 = 言い方(FromDesk::付けた {
            部屋: "NBIW3PA2TUQ52DTJNVXSQOL3G4".to_owned(),
            名前: "開発6台".to_owned(),
        })
        .unwrap();
        assert!(文.contains("「開発6台」"));
        assert_eq!(
            言い方(FromDesk::Denied {
                why: "札が無い".to_owned()
            }),
            Err("札が無い".to_owned())
        );
    }
}
