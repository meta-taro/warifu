//! **受信箱の見張り**（0.1.44）—— 控えたアドレスを数分おきに見て、新しい通が来たら、
//! この機械のエージェントに「何通来た」とだけ知らせる。
//!
//! - **読むだけ。**受信箱は `EXAMINE` で開き、ヘッダだけを取る（既読にしない）
//! - **知らせに差出人も件名も載せない。**エージェントが中を見るのは `inbox_list` から（札が要る）
//! - **立ち上げた直後は数えるだけ。**前からある通を「新しい」と言わない
//! - パスワードは macOS のキーチェーンから、つなぐときだけ読む（記録に出さない）

use std::collections::BTreeMap;
use std::time::Duration;

use tauri::{AppHandle, Manager as _};

use crate::Bridge;

/// 見に行く間。
const 間: Duration = Duration::from_secs(5 * 60);
/// 立ち上げてから最初に見に行くまで（網が落ち着くのを待つ）。
const 最初まで: Duration = Duration::from_secs(60);
/// 1 回に見る見出しの数。これより多く一度に届いたら「N 通以上」と言う。
const 見る数: usize = 50;

/// 見張りの覚え。**アドレスごとに、いちばん新しい UID。**
#[derive(Debug, Default)]
pub(crate) struct 見張りの覚え(BTreeMap<String, u32>);

impl 見張りの覚え {
    /// 見た UID を足し、**新しく来た数**を返す。初めて見るアドレスは数えるだけ（0 を返す）。
    pub(crate) fn 見た(&mut self, アドレス: &str, uid: &[u32]) -> usize {
        let いちばん = uid.iter().copied().max();
        let 前 = self.0.get(アドレス).copied();
        if let Some(新) = いちばん {
            self.0.insert(アドレス.to_owned(), 新.max(前.unwrap_or(0)));
        }
        match 前 {
            None => 0,
            Some(前) => uid.iter().filter(|u| **u > 前).count(),
        }
    }
}

/// エージェントへの知らせの文。**差出人も件名も入れない。**
#[must_use]
pub(crate) fn 知らせの文(アドレス: &str, 数: usize) -> String {
    let 数 = if 数 >= 見る数 {
        format!("{数} 通以上")
    } else {
        format!("{数} 通")
    };
    format!(
        "メール: {アドレス} に新しい通が {数}届きました（中は inbox_list で見られます。札が要ります）"
    )
}

/// 見張りを始める。控えが無ければ、見に行くたびに何もしない。
pub(crate) async fn 見張る(app: AppHandle) {
    tokio::time::sleep(最初まで).await;
    let mut 覚え = 見張りの覚え::default();
    loop {
        一回り(&app, &mut 覚え).await;
        tokio::time::sleep(間).await;
    }
}

async fn 一回り(app: &AppHandle, 覚え: &mut 見張りの覚え) {
    let 控え = match warifu_vault::Vault::default_location().and_then(|v| v.mail_accounts()) {
        Ok(c) => c,
        Err(e) => {
            記録!("メールの見張り: 控えを読めません（{e}）");
            return;
        }
    };
    for c in 控え {
        match 見出しの_uid(&c).await {
            Ok(uid) => {
                let 新しい = 覚え.見た(&c.user, &uid);
                if 新しい > 0 {
                    記録!("メールの見張り: {} に新しい通 {新しい}", c.user);
                    crate::desk::配る(
                        &app.state::<Bridge>(),
                        crate::desk::聞いた("割符", &知らせの文(&c.user, 新しい)),
                    );
                }
            }
            // **落ちても次の回に見る。**記録には理由だけ（利用者名は imap の層で伏せてある）
            Err(e) => 記録!("メールの見張り: {} を見られませんでした（{e}）", c.user),
        }
    }
}

async fn 見出しの_uid(c: &warifu_vault::MailAccount) -> Result<Vec<u32>, String> {
    let パスワード = warifu_imap::keychain::読む(&c.user)
        .map_err(|_| "パスワードがキーチェーンにありません".to_owned())?;
    let 繋ぎ先 = warifu_imap::Account::new(&c.host, c.port, &c.user, &パスワード)
        .map_err(|e| e.to_string())?;
    drop(パスワード);
    let 流れ = warifu_imap::connect(&繋ぎ先)
        .await
        .map_err(|e| e.to_string())?;
    let mut 受信箱 = warifu_imap::Mailbox::open(流れ, &繋ぎ先)
        .await
        .map_err(|e| e.to_string())?;
    let 見出し = 受信箱.一覧(見る数).await.map_err(|e| e.to_string())?;
    let _ = 受信箱.logout().await;
    Ok(見出し.iter().map(|h| h.uid).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 立ち上げた直後は数えるだけで_そのあと増えた分だけ新しいと言う() {
        let mut 覚え = 見張りの覚え::default();
        assert_eq!(覚え.見た("a@example.com", &[10, 11, 12]), 0);
        assert_eq!(
            覚え.見た("a@example.com", &[11, 12]),
            0,
            "減っても新しくはない"
        );
        assert_eq!(覚え.見た("a@example.com", &[12, 13, 14]), 2);
        assert_eq!(覚え.見た("a@example.com", &[13, 14]), 0, "2 度は言わない");
        // アドレスごとに別に数える
        assert_eq!(覚え.見た("b@example.com", &[1]), 0);
        assert_eq!(覚え.見た("b@example.com", &[1, 2]), 1);
        // 空の受信箱を見ても、覚えた所は消さない
        assert_eq!(覚え.見た("b@example.com", &[]), 0);
        assert_eq!(覚え.見た("b@example.com", &[3]), 1);
    }

    #[test]
    fn 知らせには差出人も件名も入れず_見きれない数は以上と言う() {
        let 文 = 知らせの文("a@example.com", 2);
        assert_eq!(
            文,
            "メール: a@example.com に新しい通が 2 通届きました（中は inbox_list で見られます。札が要ります）"
        );
        assert!(知らせの文("a@example.com", 見る数).contains("通以上"));
    }
}
