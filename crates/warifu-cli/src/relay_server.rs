//! `warifu relay-server` —— **自前の中継の URL とトークンを、この機械に控える**（0.1.31）。
//!
//! 直接つながれないときだけ使う、自社の予備の中継（2026-10-21）。
//! 画面版は Finder から起動すると環境変数を読まないので、置き場所の `relay-server.tsv` に持たせる。
//!
//! **トークンは秘密である。**引数に取らない（シェルの履歴に残る）。標準入力から 1 行で受ける。
//! 控えは 0600。`show` も URL と「トークンあり／なし」だけを出す。

use std::io::{BufRead as _, IsTerminal as _};

use warifu_vault::Vault;

/// 何をするか。
#[derive(Debug, PartialEq, Eq)]
pub enum 設定 {
    /// URL を控える。トークンは標準入力から。
    置く {
        /// 中継の URL。
        url: String,
    },
    /// 控えを出す（トークンは中身を出さない）。
    見る,
    /// 控えを消す。
    消す,
}

/// 引数を読む。
pub fn 読む(args: &mut impl Iterator<Item = String>) -> Result<設定, String> {
    match args.next().as_deref() {
        Some("set") => {
            let url = args.next().ok_or("set のあとに中継の URL がありません")?;
            if let Some(他) = args.next() {
                // **トークンを引数に書かせない**（履歴に残る）
                return Err(format!(
                    "知らない指定です: {他}（トークンは引数ではなく標準入力から渡してください）"
                ));
            }
            Ok(設定::置く { url })
        }
        Some("show") | None => Ok(設定::見る),
        Some("clear") => Ok(設定::消す),
        Some(他) => Err(format!(
            "知らない指定です: {他}（set <URL> / show / clear）"
        )),
    }
}

/// 行う。
pub fn 行う(設: &設定) -> Result<(), Box<dyn std::error::Error>> {
    let vault = Vault::default_location()?;
    match 設 {
        設定::置く { url } => {
            if std::io::stdin().is_terminal() {
                eprintln!("トークンを貼って Enter を押してください（無ければ空のまま Enter）。");
                eprintln!(
                    "（打った字は画面に出ます。人の目があるときは `pbpaste | warifu relay-server set <URL>` で渡してください）"
                );
            }
            let mut 行 = String::new();
            std::io::stdin().lock().read_line(&mut 行)?;
            let トークン = 行.trim();
            vault.save_relay_server(url, (!トークン.is_empty()).then_some(トークン))?;
            eprintln!(
                "自前の中継を控えました（トークン{}）。画面版は次の起動から使います。",
                if トークン.is_empty() {
                    "なし"
                } else {
                    "あり"
                }
            );
        }
        設定::見る => match vault.relay_server()? {
            Some((url, トークン)) => println!(
                "{url}（トークン{}）",
                if トークン.is_some() {
                    "あり"
                } else {
                    "なし"
                }
            ),
            None => println!("自前の中継は控えていません"),
        },
        設定::消す => {
            vault.forget_relay_server()?;
            eprintln!("自前の中継の控えを消しました。画面版は次の起動から使いません。");
        }
    }
    Ok(())
}

#[cfg(test)]
mod 試験 {
    use super::*;

    fn 読ませる(a: &[&str]) -> Result<設定, String> {
        読む(&mut a.iter().map(|s| (*s).to_owned()))
    }

    #[test]
    fn set_は_url_だけを取る() {
        assert_eq!(
            読ませる(&["set", "https://relay.example.org/relay"]),
            Ok(設定::置く {
                url: "https://relay.example.org/relay".to_owned()
            })
        );
    }

    #[test]
    fn トークンを引数に書かせない() {
        assert!(読ませる(&["set", "https://relay.example.org", "t0ken"]).is_err());
    }

    #[test]
    fn 何も無ければ見る() {
        assert_eq!(読ませる(&[]), Ok(設定::見る));
        assert_eq!(読ませる(&["clear"]), Ok(設定::消す));
        assert!(読ませる(&["setx"]).is_err());
    }
}
