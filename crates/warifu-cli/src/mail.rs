//! `warifu mail` —— **既存のメールの受信箱を、読むだけでつなぐ**（0.1.44）。
//!
//! ```text
//!   warifu mail add <アドレス> [--host <IMAP のホスト>] [--port <番号>]
//!   warifu mail list                 控えたアドレスを並べる（つながない）
//!   warifu mail check [<アドレス>]    つないで、受信箱の通の数と未読の数だけを出す（省けば全部）
//!   warifu mail remove [<アドレス>]   控えとキーチェーンの項目を消す（控えが 2 つ以上なら要る）
//!   warifu mail add <アドレス> --resend   送るだけの口（Resend）を控え、API キーをキーチェーンに置く（0.1.45）
//!   warifu mail secret <アドレス>      パスワード・API キーを入れ直す（macOS が聞く・0.1.45）
//!   warifu mail sort [<アドレス>] [--last <通>]  新しいほうからヘッダだけを読み、札ごとの数を出す（0.1.45）
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
use warifu_vault::{MailAccount, MailKind, MailNotify, Vault};

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
        /// 種類（0.1.45）。Gmail・IMAP は繋ぎ先から、Resend は `--resend` で。
        種類: MailKind,
        /// 新しい通の知らせ方（0.1.46・`--notify`）。省けば今の設定のまま（新しい口は important）。
        知らせ: Option<MailNotify>,
    },
    /// 新しい通の知らせ方を変える（0.1.46・`warifu mail notify <アドレス> <all|important|none>`）。
    知らせ方 {
        /// どのアドレスか。
        利用者: String,
        /// 知らせ方。
        知らせ: MailNotify,
    },
    /// 秘密（パスワード・API キー）を入れ直す（0.1.45）。
    秘密を入れる(String),
    /// 控えたアドレスを並べる。
    並べる,
    /// つないで、数だけを出す。`None` なら控えた全部。
    確かめる(Option<String>),
    /// 控えとキーチェーンの項目を消す。`None` は控えが 1 つのときだけ通る。
    消す(Option<String>),
    /// 新しいほうから `件数` 通のヘッダだけを読み、札ごとの数を出す（0.1.45）。`None` なら控えた全部。
    仕分けを数える {
        /// どのアドレスか。
        利用者: Option<String>,
        /// 何通見るか。
        件数: usize,
    },
}

/// `sort` で見る既定の通の数。
const 仕分けの既定の通: usize = 200;

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

/// `--notify` の値を読む。
fn 知らせ方を読む(v: &str) -> Result<MailNotify, String> {
    MailNotify::from_name(v)
        .ok_or_else(|| format!("知らせ方を読めません: {v}（all / important / none）"))
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
            let (mut ホスト, mut ポート, mut 送るだけ) = (None, 既定のポート, false);
            let mut 知らせ = None;
            while let Some(a) = args.next() {
                match a.as_str() {
                    "--resend" => 送るだけ = true,
                    "--notify" => {
                        知らせ = Some(知らせ方を読む(
                            &args.next().ok_or("--notify に値が付いていません")?,
                        )?);
                    }
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
            if 送るだけ {
                if 知らせ.is_some() {
                    return Err("--resend は送るだけの口なので、--notify は要りません".into());
                }
                return Ok(設定::足す {
                    利用者,
                    ホスト: "api.resend.com".to_owned(),
                    ポート: 443,
                    種類: MailKind::Resend,
                    知らせ,
                });
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
            let 種類 = if ホスト.eq_ignore_ascii_case("imap.gmail.com") {
                MailKind::Gmail
            } else {
                MailKind::Imap
            };
            Ok(設定::足す {
                利用者,
                ホスト,
                ポート,
                種類,
                知らせ,
            })
        }
        Some("notify") => {
            let 利用者 = args
                .next()
                .ok_or("notify のあとにメールアドレスがありません")?;
            let 知らせ = 知らせ方を読む(
                &args
                    .next()
                    .ok_or("notify のあとに知らせ方がありません（all / important / none）")?,
            )?;
            Ok(設定::知らせ方 {
                利用者, 知らせ
            })
        }
        Some("secret") => Ok(設定::秘密を入れる(
            args.next()
                .ok_or("secret のあとにメールアドレスがありません")?,
        )),
        Some("list") => Ok(設定::並べる),
        Some("check") => Ok(設定::確かめる(args.next())),
        None => Ok(設定::確かめる(None)),
        Some("remove") => Ok(設定::消す(args.next())),
        Some("sort") => {
            let (mut 利用者, mut 件数) = (None, 仕分けの既定の通);
            while let Some(a) = args.next() {
                if a == "--last" {
                    let v = args.next().ok_or("--last に値が付いていません")?;
                    件数 = v
                        .parse()
                        .map_err(|_| format!("--last の値を読めません: {v}"))?;
                } else {
                    利用者 = Some(a);
                }
            }
            Ok(設定::仕分けを数える { 利用者, 件数 })
        }
        Some(他) => Err(format!(
            "知らない指定です: {他}（add / list / check / remove / sort / secret / notify）"
        )),
    }
}

/// 行う。
pub async fn 行う(設: &設定) -> Result<(), Box<dyn std::error::Error>> {
    let vault = Vault::default_location()?;
    // **置けない OS では、控える前に止める**（0.1.46）。控えだけ残ると「もう一度入れてください」の行き止まりになる
    if matches!(設, 設定::足す { .. } | 設定::秘密を入れる(_)) && !keychain::使えるか()
    {
        return Err(keychain::置けない訳.into());
    }
    match 設 {
        設定::足す {
            利用者,
            ホスト,
            ポート,
            種類,
            知らせ,
        } => {
            vault.save_mail_kind(*種類, ホスト, *ポート, 利用者)?;
            if let Some(n) = 知らせ {
                vault.set_mail_notify(利用者, *n)?;
            }
            // **Windows は端末で字を伏せて聞けない**（0.1.48）。控えだけ置いて、画面の押し所を言う
            if !keychain::端末で聞けるか() {
                println!("{利用者}: 控えました。{}", keychain::画面から入れる案内);
                return Ok(());
            }
            秘密を聞く(*種類, 利用者)?;
            if 種類.reads() {
                一つを確かめる(&vault.inboxes()?, 利用者).await
            } else {
                println!(
                    "{利用者}: 送る口（{}）を控えました。送ることは 0.1.46 からできます",
                    種類.name()
                );
                Ok(())
            }
        }
        設定::知らせ方 {
            利用者, 知らせ
        } => {
            let 控え = vault.mail_accounts()?;
            let c = 控え
                .iter()
                .find(|c| c.user.eq_ignore_ascii_case(利用者))
                .ok_or_else(|| {
                    format!("{利用者} は控えていません（warifu mail list で並びます）")
                })?;
            if !c.kind.reads() {
                return Err(
                    format!("{} は送るだけの口なので、知らせることがありません", c.user).into(),
                );
            }
            vault.set_mail_notify(利用者, *知らせ)?;
            println!(
                "{}: 新しい通の知らせ方を {} にしました（割符の画面が次に見に行くときから効きます）",
                c.user,
                知らせ.name()
            );
            Ok(())
        }
        設定::秘密を入れる(利用者) => {
            let 控え = vault.mail_accounts()?;
            let c = 控え
                .iter()
                .find(|c| c.user.eq_ignore_ascii_case(利用者))
                .ok_or_else(|| {
                    format!("{利用者} は控えていません（warifu mail list で並びます）")
                })?;
            秘密を聞く(c.kind, &c.user)?;
            println!("{} の秘密を入れました（キーチェーン）", c.user);
            Ok(())
        }
        設定::並べる => {
            let 控え = vault.mail_accounts()?;
            if 控え.is_empty() {
                println!("控えはありません（warifu mail add <アドレス> で控えます）");
            }
            for c in 控え {
                let 入った = keychain::入っているか(c.kind.secret_service(), &c.user);
                let 知らせ = if c.kind.reads() {
                    format!("\t知らせ: {}", c.notify.name())
                } else {
                    String::new()
                };
                println!(
                    "{}\t{}\t{}:{}\t秘密: {}{知らせ}",
                    c.user,
                    c.kind.name(),
                    c.host,
                    c.port,
                    if 入った {
                        "入っている"
                    } else {
                        "まだ（warifu mail secret <アドレス>）"
                    }
                );
            }
            Ok(())
        }
        設定::確かめる(どれ) => {
            let 控え = vault.inboxes()?;
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
        設定::仕分けを数える { 利用者, 件数 } => {
            let 控え = vault.inboxes()?;
            let 見る: Vec<&MailAccount> = 控え
                .iter()
                .filter(|c| {
                    利用者
                        .as_ref()
                        .is_none_or(|u| c.user.eq_ignore_ascii_case(u))
                })
                .collect();
            if 見る.is_empty() {
                return Err(
                    "仕分けるアドレスを控えていません（warifu mail list で並びます）".into(),
                );
            }
            for c in 見る {
                仕分けを数える(c, *件数).await?;
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
            let 種類 = 控え
                .iter()
                .find(|c| c.user.eq_ignore_ascii_case(&利用者))
                .map_or(MailKind::Imap, |c| c.kind);
            if !vault.forget_mail(&利用者)? {
                return Err(format!("{利用者} は控えていません").into());
            }
            let 消えた = keychain::消す_口(種類.secret_service(), &利用者)?;
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

/// **MCP の口へ渡す、メールの口の控え**（0.1.45）。置き場所とキーチェーンから読み書きする。秘密は扱わない。
pub struct 控えの手;

/// 種類の名と、IMAP のホストから、控える形を決める（純粋）。
///
/// # Errors
/// 知らない種類・アドレスに見えない・imap でホストが無いとき。
pub fn 控える形(
    種類: &str,
    アドレス: &str,
    ホスト: Option<&str>,
    ポート: Option<u16>,
) -> Result<(MailKind, String, u16), String> {
    let 種類 = MailKind::from_name(種類)
        .ok_or_else(|| format!("知らない種類です: {種類}（gmail / imap / resend）"))?;
    if !アドレス.contains('@')
        || アドレス.starts_with('-')
        || アドレス.chars().any(char::is_whitespace)
    {
        return Err(format!("メールアドレスとして読めません: {アドレス}"));
    }
    Ok(match 種類 {
        MailKind::Gmail => (
            種類,
            "imap.gmail.com".to_owned(),
            ポート.unwrap_or(既定のポート),
        ),
        MailKind::Resend => (種類, "api.resend.com".to_owned(), 443),
        MailKind::Imap => (
            種類,
            ホスト
                .filter(|h| !h.trim().is_empty())
                .ok_or("imap には host（IMAP のホスト）が要ります")?
                .trim()
                .to_owned(),
            ポート.unwrap_or(既定のポート),
        ),
    })
}

fn 口にする(c: &MailAccount) -> warifu_mcp::メールの口 {
    warifu_mcp::メールの口 {
        アドレス: c.user.clone(),
        種類: c.kind.name().to_owned(),
        繋ぎ先: format!("{}:{}", c.host, c.port),
        読む: c.kind.reads(),
        送る: c.kind.sends(),
        秘密がある: keychain::入っているか(c.kind.secret_service(), &c.user),
        知らせ: if c.kind.reads() {
            c.notify.name().to_owned()
        } else {
            String::new()
        },
    }
}

impl warifu_mcp::メールの控え for 控えの手 {
    fn 並べる(&self) -> Result<Vec<warifu_mcp::メールの口>, String> {
        let vault = Vault::default_location().map_err(|e| e.to_string())?;
        Ok(vault
            .mail_accounts()
            .map_err(|e| e.to_string())?
            .iter()
            .map(口にする)
            .collect())
    }

    fn 足す(
        &self,
        種類: &str,
        アドレス: &str,
        ホスト: Option<&str>,
        ポート: Option<u16>,
        知らせ: Option<&str>,
    ) -> Result<warifu_mcp::メールの口, String> {
        if !keychain::使えるか() {
            return Err(keychain::置けない訳.to_owned());
        }
        let (種類, ホスト, ポート) = 控える形(種類, アドレス, ホスト, ポート)?;
        let 知らせ = 知らせ.map(知らせ方を読む).transpose()?;
        if 知らせ.is_some() && !種類.reads() {
            return Err("resend は送るだけの口なので、notify は要りません".into());
        }
        let vault = Vault::default_location().map_err(|e| e.to_string())?;
        vault
            .save_mail_kind(種類, &ホスト, ポート, アドレス)
            .map_err(|e| e.to_string())?;
        if let Some(n) = 知らせ {
            vault
                .set_mail_notify(アドレス, n)
                .map_err(|e| e.to_string())?;
        }
        let 控え = vault.mail_accounts().map_err(|e| e.to_string())?;
        控え
            .iter()
            .find(|c| c.user.eq_ignore_ascii_case(アドレス.trim()))
            .map(口にする)
            .ok_or_else(|| "控えたはずの口が見つかりません".to_owned())
    }

    fn 外す(&self, アドレス: &str) -> Result<bool, String> {
        let vault = Vault::default_location().map_err(|e| e.to_string())?;
        vault.forget_mail(アドレス).map_err(|e| e.to_string())
    }
}

/// **秘密を人に聞いて、キーチェーンに置く。**割符は字を見ない（`security` が聞く）。
fn 秘密を聞く(種類: MailKind, 利用者: &str) -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("このあと macOS が 2 回たずねます（打った字は画面に出ません）。");
    match 種類 {
        MailKind::Resend => eprintln!("Resend の API キー（re_ で始まる字）を入れてください。"),
        MailKind::Gmail => {
            eprintln!("Google のアカウントのアプリパスワード（16 字）を入れてください。")
        }
        MailKind::Imap => eprintln!("メールのパスワードを入れてください。"),
    }
    eprintln!("入れた字は割符を通らず、そのまま macOS のキーチェーンに置かれます。");
    let 名札 = match 種類 {
        MailKind::Resend => "warifu 送る口（Resend）",
        _ => "warifu メール（IMAP）",
    };
    keychain::人に聞いて置く_口(種類.secret_service(), 利用者, 名札)?;
    Ok(())
}

/// **エージェントの `inbox_list` のために、控えた全部の受信箱から新しい分を汲む**（0.1.44）。
///
/// 1 つのアドレスにつき新しいほうから `件数` 通。**読むだけ**（既読にしない）。
/// 全部が汲めなかったときだけ失敗にする。一部が落ちたら、汲めた分を返し、落ちたことは標準エラーへ言う
/// （標準出力は MCP のもの）。
pub async fn 汲む(件数: usize) -> Result<Vec<warifu_read::Received>, String> {
    let vault = Vault::default_location().map_err(|e| e.to_string())?;
    let 控え = vault.inboxes().map_err(|e| e.to_string())?;
    if 控え.is_empty() {
        return Err("メールのアドレスを控えていません（warifu mail add <アドレス>）".to_owned());
    }
    let 今 = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let mut 通 = Vec::new();
    let mut 落ちた = Vec::new();
    for c in &控え {
        match 一つから汲む(c, 件数, 今).await {
            Ok(mut 分) => 通.append(&mut 分),
            Err(e) => {
                eprintln!("warifu mcp: {} の受信箱を汲めませんでした（{e}）", c.user);
                落ちた.push(c.user.clone());
            }
        }
    }
    if 落ちた.len() == 控え.len() {
        return Err(format!(
            "どの受信箱にもつながりませんでした（{}）",
            落ちた.join("・")
        ));
    }
    // 新しい順にそろえる（アドレスをまたいで）
    通.sort_by_key(|r| std::cmp::Reverse(r.received_at()));
    Ok(通)
}

async fn 一つから汲む(
    c: &MailAccount,
    件数: usize,
    今: u64,
) -> Result<Vec<warifu_read::Received>, Box<dyn std::error::Error>> {
    let パスワード = keychain::読む(&c.user).map_err(
        |_| "パスワードがキーチェーンにありません（warifu mail add でもう一度入れてください）",
    )?;
    let 繋ぎ先 = Account::new(&c.host, c.port, &c.user, &パスワード)?;
    drop(パスワード);
    let 流れ = warifu_imap::connect(&繋ぎ先).await?;
    let mut 受信箱 = Mailbox::open(流れ, &繋ぎ先).await?;
    let 通 = 受信箱.新しい順に取る(件数, 今).await?;
    if let Err(e) = 受信箱.logout().await {
        eprintln!("（閉じるときに落ちました: {e}）");
    }
    Ok(通)
}

/// **ヘッダだけを読んで仕分け、札ごとの数と理由ごとの数を出す**（0.1.45）。
///
/// 出すのは数だけ。**差出人も件名も出さない**（理由に出るのは、当たった規則と件名の語だけ）。
async fn 仕分けを数える(
    c: &MailAccount,
    件数: usize,
) -> Result<(), Box<dyn std::error::Error>> {
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
    let 行たち = 受信箱.一覧(件数).await?;
    if let Err(e) = 受信箱.logout().await {
        eprintln!("（閉じるときに落ちました: {e}）");
    }
    let 受け手 = warifu_imap::受け手を推す(&c.host);
    let 分けた: Vec<warifu_read::仕分け> = 行たち
        .iter()
        .map(|行| warifu_read::仕分ける(&行.手がかり, 受け手))
        .collect();
    // **道具が読めているかの対照。**疑いが 0 でも、認証の結果を 1 通も読めていなければ「無い」とは言えない
    let 読めた = 受け手.map_or(0, |名| {
        行たち
            .iter()
            .filter(|行| {
                行.手がかり.認証.first().is_some_and(|a| {
                    a.split(';')
                        .next()
                        .unwrap_or("")
                        .trim()
                        .eq_ignore_ascii_case(名)
                })
            })
            .count()
    });
    print!("{}", 数の表(&c.user, 受け手, &分けた));
    println!("  認証の結果を読めた通: {読めた} / {}", 行たち.len());
    Ok(())
}

/// 札ごとの数と、理由ごとの数を並べる（多い順）。
fn 数の表(
    利用者: &str, 受け手: Option<&str>, 分けた: &[warifu_read::仕分け]
) -> String {
    use std::collections::BTreeMap;
    let mut 札の数: BTreeMap<warifu_read::札, usize> = BTreeMap::new();
    let mut 理由の数: BTreeMap<String, usize> = BTreeMap::new();
    for 分 in 分けた {
        *札の数.entry(分.札).or_default() += 1;
        for r in &分.理由 {
            *理由の数.entry(format!("{}: {r}", 分.札.名())).or_default() += 1;
        }
    }
    let mut 文 = format!(
        "{利用者}: 新しいほうから {} 通（ヘッダだけ・既読にしない・認証の結果を付けたサーバ: {}）\n",
        分けた.len(),
        受け手.unwrap_or("分からないので読まない")
    );
    for (札, 数) in &札の数 {
        文.push_str(&format!("  {}\t{数}\n", 札.名()));
    }
    let mut 理由: Vec<_> = 理由の数.into_iter().collect();
    理由.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    if !理由.is_empty() {
        文.push_str("  理由:\n");
    }
    for (r, 数) in 理由 {
        文.push_str(&format!("    {数}\t{r}\n"));
    }
    文
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
    use warifu_vault::MailKind;
    #[test]
    fn 数の表は札と理由の数だけで_差出人も件名も出さない() {
        let 手 = warifu_read::手がかり {
            差出人: "secret-sender@example.com".into(),
            件名: "秘密の件名 注文".into(),
            ..warifu_read::手がかり::default()
        };
        let 分 = vec![
            warifu_read::仕分ける(&手, None),
            warifu_read::仕分ける(&手, None),
        ];
        let 表 = super::数の表("a@example.com", None, &分);
        assert!(表.contains("受注\t2"));
        assert!(表.contains("2\t受注: 件名に「注文」"));
        assert!(!表.contains("secret-sender") && !表.contains("秘密の件名"));
    }

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
                ポート: 994,
                種類: MailKind::Gmail,
                知らせ: None,
            })
        );
    }

    #[test]
    fn 知らせ方は_add_の_notify_か_notify_で決め_知らない値と_resend_には断る() {
        assert_eq!(
            読ませる(&[
                "add",
                "a@example.com",
                "--host",
                "imap.gmail.com",
                "--notify",
                "all"
            ]),
            Ok(設定::足す {
                利用者: "a@example.com".into(),
                ホスト: "imap.gmail.com".into(),
                ポート: 993,
                種類: MailKind::Gmail,
                知らせ: Some(MailNotify::All),
            })
        );
        assert_eq!(
            読ませる(&["notify", "a@example.com", "none"]),
            Ok(設定::知らせ方 {
                利用者: "a@example.com".into(),
                知らせ: MailNotify::Off,
            })
        );
        assert!(読ませる(&["notify", "a@example.com", "sometimes"]).is_err());
        assert!(読ませる(&["notify", "a@example.com"]).is_err());
        assert!(
            読ませる(&[
                "add",
                "a@example.com",
                "--host",
                "imap.gmail.com",
                "--notify"
            ])
            .is_err()
        );
        assert!(
            読ませる(&["add", "b@example.com", "--resend", "--notify", "all"]).is_err(),
            "送るだけの口に知らせ方は無い"
        );
    }

    #[test]
    fn mcp_から控える形は種類で決まり_imap_はホストが要る() {
        use super::控える形;
        assert_eq!(
            控える形("gmail", "a@example.com", None, None),
            Ok((MailKind::Gmail, "imap.gmail.com".into(), 993))
        );
        assert_eq!(
            控える形("resend", "b@example.com", Some("x"), Some(1)),
            Ok((MailKind::Resend, "api.resend.com".into(), 443))
        );
        assert!(控える形("imap", "c@example.com", None, None).is_err());
        assert!(控える形("pop3", "c@example.com", None, None).is_err());
        assert!(控える形("gmail", "not-an-address", None, None).is_err());
    }

    #[test]
    fn resend_は送る口として控え_secret_は入れ直す() {
        assert_eq!(
            読ませる(&["add", "b@example.com", "--resend"]),
            Ok(設定::足す {
                利用者: "b@example.com".into(),
                ホスト: "api.resend.com".into(),
                ポート: 443,
                種類: MailKind::Resend,
                知らせ: None,
            })
        );
        assert_eq!(
            読ませる(&["secret", "b@example.com"]),
            Ok(設定::秘密を入れる("b@example.com".into()))
        );
        // API キーも引数では受けない
        assert!(読ませる(&["add", "b@example.com", "--resend", "re_123"]).is_err());
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
            読ませる(&["sort"]),
            Ok(設定::仕分けを数える {
                利用者: None,
                件数: 200
            })
        );
        assert_eq!(
            読ませる(&["sort", "b@example.com", "--last", "50"]),
            Ok(設定::仕分けを数える {
                利用者: Some("b@example.com".into()),
                件数: 50
            })
        );
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
