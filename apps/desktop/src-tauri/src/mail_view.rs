//! **受信箱を画面で読む**（0.1.45・DESIGN §1 の「受信箱」・読むだけ）。
//!
//! - **読むだけ。**受信箱は `EXAMINE` で開き、ヘッダも本文も `BODY.PEEK` で取る（既読にしない）
//! - 一覧は**ヘッダだけ**。札は `warifu-read` の仕分け（AI を呼ばない）
//! - 1 通を開くと**文字だけ**を返す。HTML の外の画像は読み込まず、数だけ
//! - **記録には中身を書かない。**数と、落ちた理由だけ
//! - 送る口は無い（D49）

use serde::Serialize;
use warifu_vault::{MailAccount, Vault};

use crate::{Answer, Failure};

/// 1 つの受信箱から取る通の数。
const 一覧の数: usize = 50;

/// 画面へ返す 1 通（本文は入らない）。
#[derive(Debug, Serialize)]
pub(crate) struct メールの行 {
    user: String,
    uid: u32,
    差出人: String,
    名: Option<String>,
    件名: String,
    届いた: Option<i64>,
    既読: bool,
    札: &'static str,
    理由: Vec<String>,
}

/// 一覧の答え。**落ちた受信箱は黙って外さず、理由を返す。**
#[derive(Debug, Serialize)]
pub(crate) struct 受信箱 {
    行たち: Vec<メールの行>,
    落ちた: Vec<String>,
    /// 控えたアドレスが 1 つも無い。
    控え無し: bool,
}

/// 開いた 1 通（文字だけ）。
#[derive(Debug, Serialize)]
pub(crate) struct 開いた {
    本文: String,
    外の画像: usize,
    添付: Vec<String>,
}

fn 失敗(message: String) -> Failure {
    Failure {
        message,
        code: None,
    }
}

async fn 開く(c: &MailAccount) -> Result<warifu_imap::Mailbox<warifu_imap::Stream>, String> {
    let パスワード = warifu_imap::keychain::読む(&c.user)
        .map_err(|_| "パスワードがキーチェーンにありません（warifu mail add でもう一度入れてください）".to_owned())?;
    let 繋ぎ先 = warifu_imap::Account::new(&c.host, c.port, &c.user, &パスワード)
        .map_err(|e| e.to_string())?;
    drop(パスワード);
    let 流れ = warifu_imap::connect(&繋ぎ先)
        .await
        .map_err(|e| e.to_string())?;
    warifu_imap::Mailbox::open(流れ, &繋ぎ先)
        .await
        .map_err(|e| e.to_string())
}

async fn 一つの一覧(c: &MailAccount) -> Result<Vec<メールの行>, String> {
    let mut 箱 = 開く(c).await?;
    let 見出し = 箱.一覧(一覧の数).await.map_err(|e| e.to_string())?;
    let _ = 箱.logout().await;
    let 受け手 = warifu_imap::受け手を推す(&c.host);
    Ok(見出し
        .into_iter()
        .map(|h| {
            let 分 = warifu_read::仕分ける(&h.手がかり, 受け手);
            メールの行 {
                user: c.user.clone(),
                uid: h.uid,
                差出人: h.差出人,
                名: h.名,
                件名: h.件名,
                届いた: h.届いた,
                既読: h.既読,
                札: 分.札.名(),
                理由: 分.理由,
            }
        })
        .collect())
}

/// **控えた全部の受信箱から、新しいほうをヘッダだけ取って仕分ける。**
#[tauri::command]
pub(crate) async fn mail_list() -> Answer<受信箱> {
    let 控え = Vault::default_location()
        .and_then(|v| v.inboxes())
        .map_err(|e| 失敗(format!("メールの控えを読めません: {e}")))?;
    let mut 行たち = Vec::new();
    let mut 落ちた = Vec::new();
    for c in &控え {
        match 一つの一覧(c).await {
            Ok(mut 分) => 行たち.append(&mut 分),
            Err(e) => {
                記録!("受信箱の画面: {} を読めませんでした（{e}）", c.user);
                落ちた.push(format!("{}: {e}", c.user));
            }
        }
    }
    記録!("受信箱の画面: {} 通を仕分けました", 行たち.len());
    Ok(受信箱 {
        行たち,
        落ちた,
        控え無し: 控え.is_empty(),
    })
}

/// **1 通を開く**（文字だけ・既読にしない）。
#[tauri::command]
pub(crate) async fn mail_open(user: String, uid: u32) -> Answer<開いた> {
    let 控え = Vault::default_location()
        .and_then(|v| v.inboxes())
        .map_err(|e| 失敗(format!("メールの控えを読めません: {e}")))?;
    let c = 控え
        .iter()
        .find(|c| c.user.eq_ignore_ascii_case(&user))
        .ok_or_else(|| 失敗("その受信箱は控えていません".to_owned()))?;
    let mut 箱 = 開く(c).await.map_err(失敗)?;
    let 通 = 箱.開く(uid).await.map_err(|e| 失敗(e.to_string()))?;
    let _ = 箱.logout().await;
    Ok(開いた {
        本文: 通.本文,
        外の画像: 通.外の画像,
        添付: 通.添付,
    })
}

/// 控えたメールの口 1 つ（画面の一覧・秘密は入らない）。
#[derive(Debug, Serialize)]
pub(crate) struct 控えた口 {
    user: String,
    kind: &'static str,
    繋ぎ先: String,
    読む: bool,
    送る: bool,
    秘密がある: bool,
}

/// **控えたメールの口を並べる**（0.1.45）。秘密は入っているかだけ。
#[tauri::command]
pub(crate) fn mail_accounts() -> Answer<Vec<控えた口>> {
    let 控え = Vault::default_location()
        .and_then(|v| v.mail_accounts())
        .map_err(|e| 失敗(format!("メールの控えを読めません: {e}")))?;
    Ok(控え
        .into_iter()
        .map(|c| 控えた口 {
            秘密がある: warifu_imap::keychain::入っているか(c.kind.secret_service(), &c.user),
            kind: c.kind.name(),
            繋ぎ先: format!("{}:{}", c.host, c.port),
            読む: c.kind.reads(),
            送る: c.kind.sends(),
            user: c.user,
        })
        .collect())
}

/// **人が画面で打った秘密を、キーチェーンに置く**（0.1.45）。
///
/// 字は `security` の標準入力だけを通る（引数・記録・返りに載せない）。エージェントの口からは呼べない
/// （画面の command で、机の線には無い）。
#[tauri::command]
pub(crate) fn mail_secret_set(user: String, secret: String) -> Answer<()> {
    let 控え = Vault::default_location()
        .and_then(|v| v.mail_accounts())
        .map_err(|e| 失敗(format!("メールの控えを読めません: {e}")))?;
    let c = 控え
        .iter()
        .find(|c| c.user.eq_ignore_ascii_case(&user))
        .ok_or_else(|| 失敗("その口は控えていません".to_owned()))?;
    let 名札 = match c.kind {
        warifu_vault::MailKind::Resend => "warifu 送る口（Resend）",
        _ => "warifu メール（IMAP）",
    };
    let 置けた = warifu_imap::keychain::字から置く(c.kind.secret_service(), &c.user, 名札, &secret);
    drop(secret);
    置けた.map_err(|e| 失敗(e.to_string()))?;
    記録!("メールの口: {} の秘密をキーチェーンに置きました（字は記録しない）", c.kind.name());
    Ok(())
}
