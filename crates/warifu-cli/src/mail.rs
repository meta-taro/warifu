//! `warifu mail` —— **既存のメールの受信箱を、読むだけでつなぐ**（0.1.44）。
//!
//! ```text
//!   warifu mail add <アドレス> [--host <IMAP のホスト>] [--port <番号>]
//!   warifu mail check     つないで、受信箱の通の数と未読の数だけを出す
//!   warifu mail remove    控えとキーチェーンの項目を消す
//! ```
//!
//! # ここで守ること
//!
//! - **パスワードを割符が受け取らない。**macOS の `security` が人に直接たずね、キーチェーンに置く。
//!   引数にも標準入力にも取らない（シェルの履歴・`ps` に残る）
//! - **読むだけ。**受信箱は `EXAMINE` で開き、既読の印も付けない（`warifu_imap::Mailbox`）
//! - **中身を出さない。**`check` が出すのは数だけ（件名も本文も出さない）

use warifu_imap::{Account, Mailbox, keychain};
use warifu_vault::Vault;

/// IMAPS の既定のポート。
const 既定のポート: u16 = 993;

/// 何をするか。
#[derive(Debug, PartialEq, Eq)]
pub enum 設定 {
    /// 繋ぎ先を控え、パスワードをキーチェーンに置き、つないで確かめる。
    足す {
        /// メールアドレス（IMAP の利用者）。
        利用者: String,
        /// IMAP のホスト。
        ホスト: String,
        /// ポート。
        ポート: u16,
    },
    /// つないで、数だけを出す。
    確かめる,
    /// 控えとキーチェーンの項目を消す。
    消す,
}

/// ドメインからホストを決められるか（Gmail だけ）。**会社のドメインは決め打ちしない。**
fn ドメインのホスト(ドメイン: &str) -> Option<&'static str> {
    matches!(
        ドメイン.to_ascii_lowercase().as_str(),
        "gmail.com" | "googlemail.com"
    )
    .then_some("imap.gmail.com")
}

fn 決まったホスト(利用者: &str) -> Option<&'static str> {
    ドメインのホスト(利用者.rsplit_once('@')?.1)
}

/// 引数を読む。
pub fn 読む(args: &mut impl Iterator<Item = String>) -> Result<設定, String> {
    match args.next().as_deref() {
        Some("add") => {
            let 利用者 = args
                .next()
                .ok_or("add のあとにメールアドレスがありません")?;
            if !利用者.contains('@') || 利用者.starts_with('-') {
                return Err(format!("メールアドレスとして読めません: {利用者}"));
            }
            let (mut ホスト, mut ポート) = (None, 既定のポート);
            while let Some(a) = args.next() {
                match a.as_str() {
                    "--host" => ホスト = Some(args.next().ok_or("--host に値が付いていません")?),
                    "--port" => {
                        let v = args.next().ok_or("--port に値が付いていません")?;
                        ポート = v
                            .parse()
                            .map_err(|_| format!("--port の値を読めません: {v}"))?;
                    }
                    // **パスワードを引数に書かせない**（履歴に残る）
                    他 => {
                        return Err(format!(
                            "知らない指定です: {他}（パスワードは引数ではなく、このあと聞かれたときに入れてください）"
                        ));
                    }
                }
            }
            let ホスト = match ホスト.or_else(|| 決まったホスト(&利用者).map(str::to_owned))
            {
                Some(h) => h,
                None => {
                    return Err(
                        "--host で IMAP のホストを指定してください（Google Workspace なら --host imap.gmail.com）"
                            .into(),
                    );
                }
            };
            Ok(設定::足す {
                利用者,
                ホスト,
                ポート,
            })
        }
        Some("check") | None => Ok(設定::確かめる),
        Some("remove") => Ok(設定::消す),
        Some(他) => Err(format!("知らない指定です: {他}（add / check / remove）")),
    }
}

/// 行う。
pub async fn 行う(設: &設定) -> Result<(), Box<dyn std::error::Error>> {
    let vault = Vault::default_location()?;
    match 設 {
        設定::足す {
            利用者,
            ホスト,
            ポート,
        } => {
            vault.save_mail(ホスト, *ポート, 利用者)?;
            eprintln!("このあと macOS がパスワードを 2 回たずねます（打った字は画面に出ません）。");
            eprintln!("Google のアカウントなら、アプリパスワード（16 字）を入れてください。");
            eprintln!("入れた字は割符を通らず、そのまま macOS のキーチェーンに置かれます。");
            keychain::人に聞いて置く(利用者)?;
            確かめる(&vault).await
        }
        設定::確かめる => 確かめる(&vault).await,
        設定::消す => {
            let Some((_, _, 利用者)) = vault.mail()? else {
                println!("控えはありません");
                return Ok(());
            };
            let 消えた = keychain::消す(&利用者)?;
            vault.forget_mail()?;
            println!(
                "控えを消しました（キーチェーンの項目: {}）",
                if 消えた {
                    "消しました"
                } else {
                    "ありませんでした"
                }
            );
            Ok(())
        }
    }
}

/// つないで、数だけを出す。
async fn 確かめる(vault: &Vault) -> Result<(), Box<dyn std::error::Error>> {
    let Some((ホスト, ポート, 利用者)) = vault.mail()? else {
        return Err(
            "メールの繋ぎ先を控えていません（warifu mail add <アドレス> で控えます）".into(),
        );
    };
    let パスワード = keychain::読む(&利用者).map_err(
        |_| "キーチェーンにパスワードがありません（warifu mail add でもう一度入れてください）",
    )?;
    let 繋ぎ先 = Account::new(&ホスト, ポート, &利用者, &パスワード)?;
    drop(パスワード);
    let 流れ = warifu_imap::connect(&繋ぎ先).await?;
    let mut 受信箱 = Mailbox::open(流れ, &繋ぎ先).await?;
    let 数 = 受信箱.count().await?;
    // 閉じられなくても、数は読めている。閉じられなかったことだけ言う
    if let Err(e) = 受信箱.logout().await {
        eprintln!("（閉じるときに落ちました: {e}）");
    }
    println!(
        "つながりました（読むだけ・{ホスト}）。受信箱 {} 通、うち未読 {} 通",
        数.全部, 数.未読
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn 読ませる(引数: &[&str]) -> Result<設定, String> {
        読む(&mut 引数.iter().map(|s| (*s).to_owned()))
    }

    #[test]
    fn gmail_ならホストを決め_ほかのドメインは_host_が要る() {
        assert_eq!(ドメインのホスト("gmail.com"), Some("imap.gmail.com"));
        assert_eq!(ドメインのホスト("GoogleMail.com"), Some("imap.gmail.com"));
        // 会社のドメインは、MX が Google でも決め打ちしない
        assert_eq!(ドメインのホスト("example.com"), None);
        assert!(読ませる(&["add", "a@example.com"]).is_err());
        assert_eq!(
            読ませる(&[
                "add",
                "a@example.com",
                "--host",
                "imap.gmail.com",
                "--port",
                "994"
            ]),
            Ok(設定::足す {
                利用者: "a@example.com".into(),
                ホスト: "imap.gmail.com".into(),
                ポート: 994
            })
        );
    }

    #[test]
    fn パスワードらしい引数は受けない() {
        // 履歴に残る。聞かれたときに入れてもらう
        let 誤り = 読ませる(&["add", "a@example.com", "abcd efgh ijkl mnop"]).unwrap_err();
        assert!(誤り.contains("パスワードは引数ではなく"));
    }

    #[test]
    fn 何も付けなければ確かめる() {
        assert_eq!(読ませる(&[]), Ok(設定::確かめる));
        assert_eq!(読ませる(&["remove"]), Ok(設定::消す));
        assert!(読ませる(&["add", "not-an-address"]).is_err());
    }
}
