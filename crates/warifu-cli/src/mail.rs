//! `warifu mail` —— **既存のメールの受信箱を、読むだけでつなぐ**（0.1.44）。
//!
//! ```text
//!   warifu mail add <アドレス> [--host <IMAP のホスト>] [--port <番号>]
//!   warifu mail list                 控えたアドレスを並べる（つながない）
//!   warifu mail check [<アドレス>]    つないで、受信箱の通の数と未読の数だけを出す（省けば全部）
//!   warifu mail remove [<アドレス>]   控えとキーチェーンの項目を消す（控えが 2 つ以上なら要る）
//!
//! アドレスはいくつでも控えられる。同じアドレスを add し直すと、繋ぎ先を置き換える。
//! ```
//!
//! # ここで守ること
//!
//! - **パスワードを割符が受け取らない。**macOS の `security` が人に直接たずね、キーチェーンに置く。
//!   引数にも標準入力にも取らない（シェルの履歴・`ps` に残る）
//! - **読むだけ。**受信箱は `EXAMINE` で開き、既読の印も付けない（`warifu_imap::Mailbox`）
//! - **中身を出さない。**`check` が出すのは数だけ（件名も本文も出さない）

use warifu_imap::{Account, Mailbox, keychain};
use warifu_vault::{MailAccount, Vault};

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
    /// 控えたアドレスを並べる。
    並べる,
    /// つないで、数だけを出す。`None` なら控えた全部。
    確かめる(Option<String>),
    /// 控えとキーチェーンの項目を消す。`None` は控えが 1 つのときだけ通る。
    消す(Option<String>),
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
        Some("list") => Ok(設定::並べる),
        Some("check") => Ok(設定::確かめる(args.next())),
        None => Ok(設定::確かめる(None)),
        Some("remove") => Ok(設定::消す(args.next())),
        Some(他) => Err(format!(
            "知らない指定です: {他}（add / list / check / remove）"
        )),
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
            一つを確かめる(&vault.mail_accounts()?, 利用者).await
        }
        設定::並べる => {
            let 控え = vault.mail_accounts()?;
            if 控え.is_empty() {
                println!("控えはありません（warifu mail add <アドレス> で控えます）");
            }
            for c in 控え {
                println!("{}\t{}:{}", c.user, c.host, c.port);
            }
            Ok(())
        }
        設定::確かめる(どれ) => {
            let 控え = vault.mail_accounts()?;
            if 控え.is_empty() {
                return Err(
                    "メールの繋ぎ先を控えていません（warifu mail add <アドレス> で控えます）"
                        .into(),
                );
            }
            if let Some(利用者) = どれ {
                return 一つを確かめる(&控え, 利用者).await;
            }
            // **1 つが落ちても、残りは確かめる。**落ちた数を最後に返す
            let mut 落ちた = 0;
            for c in &控え {
                if let Err(e) = 確かめる(c).await {
                    println!("{}: つながりませんでした（{e}）", c.user);
                    落ちた += 1;
                }
            }
            if 落ちた > 0 {
                return Err(
                    format!("{} 件のうち {落ちた} 件がつながりませんでした", 控え.len()).into(),
                );
            }
            Ok(())
        }
        設定::消す(どれ) => {
            let 控え = vault.mail_accounts()?;
            let 利用者 = match (どれ, 控え.as_slice()) {
                (Some(u), _) => u.clone(),
                (None, []) => {
                    println!("控えはありません");
                    return Ok(());
                }
                (None, [一つ]) => 一つ.user.clone(),
                (None, _) => {
                    return Err(format!(
                        "控えが {} 件あります。消すアドレスを書いてください（warifu mail list で並びます）",
                        控え.len()
                    )
                    .into());
                }
            };
            if !vault.forget_mail(&利用者)? {
                return Err(format!("{利用者} は控えていません").into());
            }
            let 消えた = keychain::消す(&利用者)?;
            println!(
                "{利用者} の控えを消しました（キーチェーンの項目: {}）",
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

/// 控えの中から 1 つを選んで確かめる。
async fn 一つを確かめる(
    控え: &[MailAccount],
    利用者: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let c = 控え
        .iter()
        .find(|c| c.user.eq_ignore_ascii_case(利用者))
        .ok_or_else(|| format!("{利用者} は控えていません（warifu mail list で並びます）"))?;
    確かめる(c).await
}

/// つないで、数だけを出す。
async fn 確かめる(c: &MailAccount) -> Result<(), Box<dyn std::error::Error>> {
    let パスワード = keychain::読む(&c.user).map_err(|_| {
        format!(
            "{} のパスワードがキーチェーンにありません（warifu mail add でもう一度入れてください）",
            c.user
        )
    })?;
    let 繋ぎ先 = Account::new(&c.host, c.port, &c.user, &パスワード)?;
    drop(パスワード);
    let 流れ = warifu_imap::connect(&繋ぎ先).await?;
    let mut 受信箱 = Mailbox::open(流れ, &繋ぎ先).await?;
    let 数 = 受信箱.count().await?;
    // 閉じられなくても、数は読めている。閉じられなかったことだけ言う
    if let Err(e) = 受信箱.logout().await {
        eprintln!("（閉じるときに落ちました: {e}）");
    }
    println!(
        "{}: つながりました（読むだけ・{}）。受信箱 {} 通、うち未読 {} 通",
        c.user, c.host, 数.全部, 数.未読
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
        assert_eq!(読ませる(&[]), Ok(設定::確かめる(None)));
        assert_eq!(読ませる(&["remove"]), Ok(設定::消す(None)));
        assert_eq!(読ませる(&["list"]), Ok(設定::並べる));
        assert_eq!(
            読ませる(&["check", "b@example.com"]),
            Ok(設定::確かめる(Some("b@example.com".into())))
        );
        assert_eq!(
            読ませる(&["remove", "b@example.com"]),
            Ok(設定::消す(Some("b@example.com".into())))
        );
        assert!(読ませる(&["add", "not-an-address"]).is_err());
    }
}
