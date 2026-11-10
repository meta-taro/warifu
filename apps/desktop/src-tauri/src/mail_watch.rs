//! **受信箱の見張り**（0.1.44）—— 控えたアドレスを数分おきに見て、新しい通が来たら、
//! この機械のエージェントに「何通来た」とだけ知らせる。
//!
//! - **読むだけ。**受信箱は `EXAMINE` で開き、ヘッダだけを取る（既読にしない）
//! - **知らせに差出人も件名も載せない。**載せるのは仕分けの札ごとの数だけ（0.1.46）。
//!   エージェントが中を見るのは `inbox_list` から（札が要る）
//! - **口ごとに知らせ方を持つ**（0.1.46・`warifu mail add --notify`）。既定は人が見るべき札だけ
//!   （知らせ・広告・迷惑は数えるだけ）。`none` の口は見に行かない
//! - **立ち上げた直後は数えるだけ。**前からある通を「新しい」と言わない
//! - パスワードは OS の鍵の置き場（macOS はキーチェーン・Windows は資格情報マネージャー）から、つなぐときだけ読む（記録に出さない）

use std::collections::BTreeMap;
use std::time::Duration;

use tauri::{AppHandle, Manager as _};

use warifu_read::札;
use warifu_vault::MailNotify;

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
    /// 見た UID を足し、**新しく来た UID** を返す。初めて見るアドレスは覚えるだけ（空を返す）。
    pub(crate) fn 見た(&mut self, アドレス: &str, uid: &[u32]) -> Vec<u32> {
        let いちばん = uid.iter().copied().max();
        let 前 = self.0.get(アドレス).copied();
        if let Some(新) = いちばん {
            self.0.insert(アドレス.to_owned(), 新.max(前.unwrap_or(0)));
        }
        match 前 {
            None => Vec::new(),
            Some(前) => uid.iter().copied().filter(|u| *u > 前).collect(),
        }
    }
}

/// **エージェントへの知らせの文**（0.1.46）。差出人も件名も入れず、札ごとの数だけ。
///
/// 知らせ方が `important` なら人が見るべき札だけを数え、無ければ知らせない（`None`）。
#[must_use]
pub(crate) fn 知らせの文(
    アドレス: &str,
    知らせ方: MailNotify,
    新しい札: &[札],
) -> Option<String> {
    let 数える = |札: &&札| match 知らせ方 {
        MailNotify::All => true,
        MailNotify::Important => 札.人へ知らせるか(),
        MailNotify::Off => false,
    };
    let mut 札ごと: BTreeMap<札, usize> = BTreeMap::new();
    for 札 in 新しい札.iter().filter(数える) {
        *札ごと.entry(*札).or_default() += 1;
    }
    let 数: usize = 札ごと.values().sum();
    if 数 == 0 {
        return None;
    }
    let 内訳 = 札ごと
        .iter()
        .map(|(札, n)| format!("{} {n}", 札.名()))
        .collect::<Vec<_>>()
        .join("・");
    let 以上 = if 新しい札.len() >= 見る数 {
        "以上"
    } else {
        ""
    };
    let 外した = 新しい札.len() - 数;
    let 外した = if 外した > 0 {
        format!("。ほかに知らせ・広告・迷惑が {外した} 通（数えただけ）")
    } else {
        String::new()
    };
    Some(format!(
        "メール: {アドレス} に新しい通が {数} 通{以上}届きました（{内訳}）{外した}（中は inbox_list で見られます。札が要ります）"
    ))
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
    // 秘密を置けない OS では見に行けない（毎回同じ失敗を記録に積まない）
    if !warifu_imap::keychain::使えるか() {
        return;
    }
    let 控え = match warifu_vault::Vault::default_location().and_then(|v| v.inboxes()) {
        Ok(c) => c,
        Err(e) => {
            記録!("メールの見張り: 控えを読めません（{e}）");
            return;
        }
    };
    for c in 控え.into_iter().filter(|c| c.notify != MailNotify::Off) {
        match 見出しの札(&c).await {
            Ok(見た) => {
                let uid: Vec<u32> = 見た.iter().map(|(u, _)| *u).collect();
                let 新しい = 覚え.見た(&c.user, &uid);
                if 新しい.is_empty() {
                    continue;
                }
                let 札たち: Vec<札> = 見た
                    .iter()
                    .filter(|(u, _)| 新しい.contains(u))
                    .map(|(_, 札)| *札)
                    .collect();
                記録!("メールの見張り: {} に新しい通 {}", c.user, 札たち.len());
                if let Some(文) = 知らせの文(&c.user, c.notify, &札たち) {
                    crate::desk::配る(&app.state::<Bridge>(), crate::desk::聞いた("割符", &文));
                }
            }
            // **落ちても次の回に見る。**記録には理由だけ（利用者名は imap の層で伏せてある）
            Err(e) => 記録!("メールの見張り: {} を見られませんでした（{e}）", c.user),
        }
    }
}

/// 見出しだけを取り、UID と仕分けの札を返す（本文は読まない）。
async fn 見出しの札(c: &warifu_vault::MailAccount) -> Result<Vec<(u32, 札)>, String> {
    let パスワード = warifu_imap::keychain::読む(&c.user)
        .map_err(|_| warifu_imap::keychain::無いとき().to_owned())?;
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
    let 受け手 = warifu_imap::受け手を推す(&c.host);
    Ok(見出し
        .iter()
        .map(|h| (h.uid, warifu_read::仕分ける(&h.手がかり, 受け手).札))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 立ち上げた直後は覚えるだけで_そのあと増えた分だけ新しいと言う() {
        let mut 覚え = 見張りの覚え::default();
        assert!(覚え.見た("a@example.com", &[10, 11, 12]).is_empty());
        assert!(
            覚え.見た("a@example.com", &[11, 12]).is_empty(),
            "減っても新しくはない"
        );
        assert_eq!(覚え.見た("a@example.com", &[12, 13, 14]), vec![13, 14]);
        assert!(
            覚え.見た("a@example.com", &[13, 14]).is_empty(),
            "2 度は言わない"
        );
        // アドレスごとに別に数える
        assert!(覚え.見た("b@example.com", &[1]).is_empty());
        assert_eq!(覚え.見た("b@example.com", &[1, 2]), vec![2]);
        // 空の受信箱を見ても、覚えた所は消さない
        assert!(覚え.見た("b@example.com", &[]).is_empty());
        assert_eq!(覚え.見た("b@example.com", &[3]), vec![3]);
    }

    #[test]
    fn 知らせには札ごとの数だけを載せ_差出人も件名も入れない() {
        let 札たち = [
            札::問い合わせ,
            札::広告,
            札::受注,
            札::問い合わせ,
            札::知らせ,
        ];
        assert_eq!(
            知らせの文("a@example.com", MailNotify::Important, &札たち).as_deref(),
            Some(
                "メール: a@example.com に新しい通が 3 通届きました（受注 1・問い合わせ 2）。ほかに知らせ・広告・迷惑が 2 通（数えただけ）（中は inbox_list で見られます。札が要ります）"
            )
        );
        assert_eq!(
            知らせの文("a@example.com", MailNotify::All, &札たち).as_deref(),
            Some(
                "メール: a@example.com に新しい通が 5 通届きました（受注 1・問い合わせ 2・知らせ 1・広告 1）（中は inbox_list で見られます。札が要ります）"
            )
        );
    }

    #[test]
    fn 人が見るべき札が無ければ_important_では知らせない() {
        let 札たち = [札::広告, 札::知らせ, 札::迷惑];
        assert_eq!(
            知らせの文("a@example.com", MailNotify::Important, &札たち),
            None
        );
        assert!(知らせの文("a@example.com", MailNotify::All, &札たち).is_some());
        assert_eq!(
            知らせの文("a@example.com", MailNotify::Off, &[札::受注]),
            None
        );
    }

    #[test]
    fn 見きれない数は以上と言う() {
        let 札たち = vec![札::未分類; 見る数];
        assert!(
            知らせの文("a@example.com", MailNotify::All, &札たち)
                .expect("知らせる")
                .contains("通以上")
        );
    }
}
