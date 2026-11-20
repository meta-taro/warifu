//! `warifu pass` —— **この PC の人が、端末から札を出す**（11-20）。
//!
//! 札は画面の帯で人が押して出す。ところが人が割符の画面の前に居ないと、帯は押されず、
//! 運用の席（届いたメールを読んで返信案を作る）が止まった。人はエージェントの端末（Claude Code）の
//! 前には居ることがあるので、**その場で別の端末を開いて 1 行打てば通せる**ようにする。
//!
//! **端末の前の人だけが通せる形にする。**標準入力が端末でないとき（エージェントの道具から
//! 走らせたとき）は断り、端末では動作と効き目を見せて `y` を打たせる。
//! これは境界ではなく歯止めである —— 同じ利用者で動くプログラムは、置き場所の `pass.tsv` を直に書ける。

use std::io::{BufRead as _, IsTerminal as _, Write as _};

use warifu_vault::Vault;

/// 「24 時間」の札の効き目（秒）。画面の［24 時間聞かない］と同じ。
const 一日の秒: u64 = 24 * 60 * 60;

/// 「取り消すまで」の札の効き目（秒）。期限つきの形のまま、実質ずっと効く（画面と同じ）。
const 取り消すまでの秒: u64 = 100 * 365 * 24 * 60 * 60;

/// **取り消すまで出せる動作**。画面の `取り消すまで出せる札` と同じ。送る札は入れない。
const 取り消すまで出せる: &[&str] = &["room.autojoin", "inbox.open.structured", "inbox.open.raw"];

/// 何をするか。
#[derive(Debug, PartialEq, Eq)]
pub enum 設定 {
    /// 札を出す。
    出す {
        /// 動作の名。
        動作: String,
        /// 取り消すまで効かせるか（無ければ 24 時間）。
        取り消すまで: bool,
    },
    /// 出している札を並べる。
    並べる,
    /// 札を取り消す。
    取り消す {
        /// 動作の名。
        動作: String,
    },
}

/// 引数を読む。
pub fn 読む(args: &mut impl Iterator<Item = String>) -> Result<設定, String> {
    match args.next().as_deref() {
        Some("allow") => {
            let 動作 = args
                .next()
                .ok_or("allow のあとに動作の名がありません（例: inbox.open.raw）")?;
            let mut 取り消すまで = false;
            for a in args {
                match a.as_str() {
                    "--until-revoked" => 取り消すまで = true,
                    他 => return Err(format!("知らない指定です: {他}")),
                }
            }
            動作を確かめる(&動作, 取り消すまで)?;
            Ok(設定::出す {
                動作, 取り消すまで
            })
        }
        Some("list") | None => Ok(設定::並べる),
        Some("revoke") => {
            let 動作 = args.next().ok_or("revoke のあとに動作の名がありません")?;
            Ok(設定::取り消す { 動作 })
        }
        Some(他) => Err(format!("知らない指定です: {他}（allow / list / revoke）")),
    }
}

/// 出せる動作か。**綴り違いを黙って通さない。**
fn 動作を確かめる(動作: &str, 取り消すまで: bool) -> Result<(), String> {
    if !crate::mcp::許せる動作.contains(&動作) && 動作 != "room.autojoin" {
        return Err(format!(
            "知らない動作です: {動作}。出せるのは: {}",
            crate::mcp::許せる動作.join(" ")
        ));
    }
    if 取り消すまで && !取り消すまで出せる.contains(&動作) {
        return Err(format!(
            "{動作} は取り消すまでの札にできません（できるのは {}）。24 時間なら --until-revoked を外してください",
            取り消すまで出せる.join(" ")
        ));
    }
    Ok(())
}

/// **控えに 1 行を書き入れる**（同じ動作は 1 行だけ・期限も書き換える）。
fn 書き入れる(一覧: &mut Vec<(String, bool, Option<u64>)>, 動作: &str, まで: u64) {
    if let Some(在る) = 一覧.iter_mut().find(|(名, _, _)| 名 == 動作) {
        在る.1 = true;
        在る.2 = Some(まで);
    } else {
        一覧.push((動作.to_owned(), true, Some(まで)));
    }
}

fn いま() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// 行う。
pub fn 行う(設: &設定) -> Result<(), String> {
    let vault = Vault::default_location().map_err(|e| e.to_string())?;
    match 設 {
        設定::出す {
            動作, 取り消すまで
        } => {
            // **端末の前の人だけ**。エージェントの道具から走らせると、標準入力は端末でない
            if !std::io::stdin().is_terminal() {
                return Err(
                    "札は、この PC の人が端末で打ったときだけ出せます（標準入力が端末ではありません）。\
                     エージェントの道具からは出せません"
                        .to_owned(),
                );
            }
            let 効き目 = if *取り消すまで {
                "取り消すまで"
            } else {
                "24 時間"
            };
            print!(
                "この PC のエージェントに「{動作}」を、{効き目}聞かずに通します。よろしいですか？ [y/N] "
            );
            let _ = std::io::stdout().flush();
            let mut 答え = String::new();
            std::io::stdin()
                .lock()
                .read_line(&mut 答え)
                .map_err(|e| e.to_string())?;
            if !matches!(答え.trim(), "y" | "Y" | "yes") {
                println!("通しませんでした。");
                return Ok(());
            }
            let 秒 = if *取り消すまで {
                取り消すまでの秒
            } else {
                一日の秒
            };
            let mut 一覧 = vault.passes_with_until().unwrap_or_default();
            書き入れる(&mut 一覧, 動作, いま().saturating_add(秒));
            vault
                .save_passes_with_until(&一覧)
                .map_err(|e| e.to_string())?;
            println!(
                "通しました（{効き目}）。エージェントは次に呼んだときから使えます。取り消すには: warifu pass revoke {動作}"
            );
            Ok(())
        }
        設定::並べる => {
            let 今 = いま();
            let 一覧 = vault.passes_with_until().unwrap_or_default();
            let 効いている: Vec<_> = 一覧
                .iter()
                .filter(|(_, 許した, まで)| *許した && まで.is_some_and(|t| t > 今))
                .collect();
            if 効いている.is_empty() {
                println!("聞かずに通している札はありません。");
            }
            for (名, _, まで) in 効いている {
                let 残り = まで.unwrap_or(0).saturating_sub(今);
                if 残り > 一日の秒 * 365 {
                    println!("{名}\t取り消すまで");
                } else {
                    println!("{名}\tあと {} 時間", 残り / 3600);
                }
            }
            Ok(())
        }
        設定::取り消す { 動作 } => {
            let mut 一覧 = vault.passes_with_until().unwrap_or_default();
            let 前 = 一覧.len();
            一覧.retain(|(名, _, _)| 名 != 動作);
            if 一覧.len() == 前 {
                println!("{動作} の札は出ていません。");
                return Ok(());
            }
            vault
                .save_passes_with_until(&一覧)
                .map_err(|e| e.to_string())?;
            println!("取り消しました: {動作}");
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn 読ませる(列: &[&str]) -> Result<設定, String> {
        読む(&mut 列.iter().map(|s| (*s).to_owned()))
    }

    #[test]
    fn メールの本文は取り消すまで出せる() {
        assert_eq!(
            読ませる(&["allow", "inbox.open.raw", "--until-revoked"]),
            Ok(設定::出す {
                動作: "inbox.open.raw".to_owned(),
                取り消すまで: true
            })
        );
    }

    #[test]
    fn 送る札は取り消すまでにはできない_24時間ならできる() {
        assert!(読ませる(&["allow", "mail.send", "--until-revoked"]).is_err());
        assert!(読ませる(&["allow", "mail.send"]).is_ok());
    }

    #[test]
    fn 知らない動作と知らない指定は断る() {
        assert!(読ませる(&["allow", "inbox.open.rwa"]).is_err());
        assert!(読ませる(&["allow", "chat.read", "--forever"]).is_err());
        assert!(読ませる(&["grant", "chat.read"]).is_err());
    }

    #[test]
    fn 同じ動作は1行だけで_期限を書き換える() {
        let mut 一覧 = vec![("inbox.open.raw".to_owned(), false, None)];
        書き入れる(&mut 一覧, "inbox.open.raw", 100);
        書き入れる(&mut 一覧, "chat.read", 50);
        assert_eq!(
            一覧,
            vec![
                ("inbox.open.raw".to_owned(), true, Some(100)),
                ("chat.read".to_owned(), true, Some(50))
            ]
        );
    }
}
