//! **エージェントが書いたメールを、人が画面で押してから送る**（D132・11-12）。
//!
//! 机が頼みを受けると、ここに積んで画面へ帯を出し、人の答えを最長 `秒` 待つ。
//! **送るのは人が［送る］を押したときだけ**（`answer_mail_send`）。エージェントはこの口を呼べない。
//! 送る道は、いまは Gmail の口だけ（受信箱と同じアプリパスワードで `smtp.gmail.com:465`）。
//! 記録には「送った・宛先の数」だけを書く（宛先・件名・本文は書かない）。

use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use tauri::{AppHandle, Emitter as _, Manager as _};
use warifu_desk::メールの返り;
use warifu_vault::{MailAccount, MailKind};

use crate::{Answer, Failure};

/// 画面へ「送ってよいか」の頼みの並びを渡す。
pub const EVENT_MAIL_SEND: &str = "warifu://mail-send";

/// 人の答えを待っている 1 通。**まだ送っていない。**
#[derive(Debug, Clone, serde::Serialize)]
pub struct 頼み {
    /// 答えるときに指す番号。
    pub 番号: u64,
    /// どの口から（口のアドレス）。
    pub 口: String,
    /// 宛先。
    pub 宛先: Vec<String>,
    /// 件名。
    pub 件名: String,
    /// 本文（人が全文を読んで決める）。
    pub 本文: String,
    /// 返信か（元の通がある）。
    pub 返信: bool,
    /// どのエージェントが頼んだか。
    pub 頼んだ人: String,
    #[serde(skip)]
    返信先: Option<String>,
    #[serde(skip)]
    参照: Option<String>,
}

static 頼みの棚: Mutex<Vec<頼み>> = Mutex::new(Vec::new());
static 次の番号: AtomicU64 = AtomicU64::new(1);
static 結果たち: Mutex<Option<HashMap<u64, メールの返り>>> = Mutex::new(None);
static 答えの知らせ: std::sync::OnceLock<tokio::sync::Notify> = std::sync::OnceLock::new();

fn 答えの知らせ口() -> &'static tokio::sync::Notify {
    答えの知らせ.get_or_init(tokio::sync::Notify::new)
}

/// 承認を待っている頼み（画面が読む）。
#[must_use]
pub fn 並べる() -> Vec<頼み> {
    頼みの棚.lock().expect("毒されていない").clone()
}

fn 結果を置く(番号: u64, 結果: メールの返り) {
    結果たち
        .lock()
        .expect("毒されていない")
        .get_or_insert_with(HashMap::new)
        .insert(番号, 結果);
    答えの知らせ口().notify_waiters();
}

fn 結果を取る(番号: u64) -> Option<メールの返り> {
    結果たち.lock().expect("毒されていない").as_mut().and_then(|棚| 棚.remove(&番号))
}

async fn 答えを待つ(番号: u64, 秒: u64) -> メールの返り {
    let 終わり = tokio::time::Instant::now() + std::time::Duration::from_secs(秒);
    loop {
        let 起きた = 答えの知らせ口().notified();
        tokio::pin!(起きた);
        起きた.as_mut().enable();
        if let Some(結果) = 結果を取る(番号) {
            return 結果;
        }
        if tokio::time::timeout_at(終わり, 起きた).await.is_err() {
            return 結果を取る(番号).unwrap_or(メールの返り::まだ);
        }
    }
}

fn 送れない(訳: impl Into<String>) -> メールの返り {
    メールの返り::送れない { 訳: 訳.into() }
}

/// **どの口から送るか**を決める（純粋・試験で見る）。いま送れるのは Gmail の口だけ。
///
/// 指されていればその口、指されていなければ送れる口が 1 つだけのときそれ。
///
/// # Errors
/// 送れる口が無い・指した口が無い・2 つ以上あって決まらない。人が読む訳。
pub fn 口を決める(控え: &[MailAccount], 指定: &str) -> Result<MailAccount, String> {
    let 送れる: Vec<&MailAccount> = 控え.iter().filter(|c| c.kind == MailKind::Gmail).collect();
    let 指定 = 指定.trim();
    if !指定.is_empty() {
        return 控え
            .iter()
            .find(|c| c.user.eq_ignore_ascii_case(指定))
            .ok_or_else(|| format!("{指定} の口は控えていません（mail_account_list で並びます）"))
            .and_then(|c| {
                if c.kind == MailKind::Gmail {
                    Ok(c.clone())
                } else {
                    Err("いま送れるのは Gmail の口だけです（ほかの口は次の段で）".to_owned())
                }
            });
    }
    match 送れる.as_slice() {
        [一つ] => Ok((*一つ).clone()),
        [] => Err("送れる口がありません（いま送れるのは Gmail の口だけです）".to_owned()),
        _ => Err(format!(
            "送れる口が 2 つ以上あります。どの口から送るかを書いてください（{}）",
            送れる.iter().map(|c| c.user.as_str()).collect::<Vec<_>>().join("・")
        )),
    }
}

/// **机から：送ってよいかを人に聞き、答えを待つ**（`ToDesk::メールを送る`）。
#[allow(clippy::too_many_arguments)]
pub async fn 頼む(
    app: &AppHandle,
    口: &str,
    宛先: Vec<String>,
    件名: String,
    本文: String,
    返信先: Option<String>,
    参照: Option<String>,
    頼んだ人: &str,
    待つ秒: u64,
) -> メールの返り {
    let 控え = match warifu_vault::Vault::default_location().and_then(|v| v.mail_accounts()) {
        Ok(c) => c,
        Err(e) => return 送れない(format!("メールの口の控えを読めません: {e}")),
    };
    let 口 = match 口を決める(&控え, 口) {
        Ok(c) => c,
        Err(訳) => return 送れない(訳),
    };
    let 番号 = 次の番号.fetch_add(1, Ordering::Relaxed);
    頼みの棚.lock().expect("毒されていない").push(頼み {
        番号,
        口: 口.user.clone(),
        宛先,
        件名,
        本文,
        返信: 返信先.is_some(),
        頼んだ人: 頼んだ人.to_owned(),
        返信先,
        参照,
    });
    記録!("メールを送る: 人に聞きます（頼んだのは {頼んだ人}）");
    if let Err(e) = app.emit(EVENT_MAIL_SEND, 並べる()) {
        記録!("メールを送る: 画面へ知らせられませんでした: {e}");
    }
    crate::notify::承認を頼む(app, "エージェントがメールを送ってよいか、聞いています");
    答えを待つ(番号, 待つ秒).await
}

/// **人が答えた**（画面の帯の［送る］／［やめる］）。**ここが送る唯一の所である。**
#[tauri::command]
pub async fn answer_mail_send(app: AppHandle, 番号: u64, 送る: bool) -> Answer<String> {
    let 取った = {
        let mut 棚 = 頼みの棚.lock().expect("毒されていない");
        let 位置 = 棚.iter().position(|頼| 頼.番号 == 番号);
        位置.map(|i| 棚.remove(i))
    };
    let Some(頼) = 取った else {
        return Err(Failure {
            message: "その頼みには、もう答えています".into(),
            code: None,
        });
    };
    let _ = app.emit(EVENT_MAIL_SEND, 並べる());
    let 結果 = if 送る {
        送り出す(&頼).await
    } else {
        記録!("メールを送る: 画面で送らないと決めました");
        メールの返り::断った
    };
    let 言い方 = match &結果 {
        メールの返り::送った { 宛先の数 } => format!("{宛先の数} 人へ送りました"),
        メールの返り::断った => "送りませんでした".to_owned(),
        メールの返り::送れない { 訳 } => format!("送れませんでした：{訳}"),
        メールの返り::まだ => String::new(),
    };
    // **エージェントにも知らせる**（待ちきれずに「まだ」を受け取った席が、結果を知れるように）
    crate::desk::配る(
        &app.state::<crate::Bridge>(),
        crate::desk::聞いた("割符", &format!("メールの送信: {言い方}")),
    );
    結果を置く(番号, 結果);
    Ok(言い方)
}

/// 画面が帯を出すための並び（立ち上げ直したときに取りに来る）。
#[tauri::command]
pub fn pending_mail_sends() -> Vec<頼み> {
    並べる()
}

/// **SMTP で送る。**資格は受信箱と同じアプリパスワード（キーチェーン）。
async fn 送り出す(頼: &頼み) -> メールの返り {
    let パスワード = match warifu_imap::keychain::読む(&頼.口) {
        Ok(p) => p,
        Err(_) => return 送れない(warifu_imap::keychain::無いとき()),
    };
    let 繋ぎ先 = match warifu_imap::Account::new("smtp.gmail.com", 465, &頼.口, &パスワード) {
        Ok(a) => a,
        Err(e) => return 送れない(e.to_string()),
    };
    let 流れ = match warifu_imap::connect(&繋ぎ先).await {
        Ok(s) => s,
        Err(e) => return 送れない(e.to_string()),
    };
    let 通 = warifu_imap::smtp::送る通 {
        差出人: 頼.口.clone(),
        宛先: 頼.宛先.clone(),
        件名: 頼.件名.clone(),
        本文: 頼.本文.clone(),
        返信先: 頼.返信先.clone(),
        参照: 頼.参照.clone(),
    };
    let 日時 = chrono::Local::now().to_rfc2822();
    let id = format!("<{}.{}@warifu.local>", chrono::Utc::now().timestamp_micros(), 頼.番号);
    match warifu_imap::smtp::送る(流れ, &頼.口, &パスワード, &通, &日時, &id).await {
        Ok(()) => {
            記録!("メールを送る: 送りました（宛先 {} 件）", 頼.宛先.len());
            メールの返り::送った {
                宛先の数: 頼.宛先.len(),
            }
        }
        Err(e) => {
            記録!("メールを送る: 送れませんでした（{e}）");
            送れない(e.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn 口(user: &str, kind: MailKind) -> MailAccount {
        MailAccount {
            host: "imap.example.com".to_owned(),
            port: 993,
            user: user.to_owned(),
            kind,
            notify: warifu_vault::MailNotify::default(),
        }
    }

    #[test]
    fn 送る口は_指したものか_送れる口が_1_つならそれ() {
        let 控え = vec![口("a@example.com", MailKind::Gmail), 口("b@example.com", MailKind::Imap)];
        assert_eq!(口を決める(&控え, "").unwrap().user, "a@example.com");
        assert_eq!(口を決める(&控え, "A@example.com").unwrap().user, "a@example.com");
        assert!(口を決める(&控え, "b@example.com").unwrap_err().contains("Gmail の口だけ"));
        assert!(口を決める(&控え, "c@example.com").unwrap_err().contains("控えていません"));
        let 二つ = vec![口("a@example.com", MailKind::Gmail), 口("c@example.com", MailKind::Gmail)];
        assert!(口を決める(&二つ, "").unwrap_err().contains("2 つ以上"));
        assert!(口を決める(&[], "").unwrap_err().contains("送れる口がありません"));
    }

    #[tokio::test]
    async fn 人が答えなければ_まだを返し_答えたらその場で返る() {
        assert_eq!(答えを待つ(9_001, 0).await, メールの返り::まだ);
        let 待つ = tokio::spawn(答えを待つ(9_002, 5));
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        結果を置く(9_002, メールの返り::断った);
        assert_eq!(待つ.await.unwrap(), メールの返り::断った);
    }
}
