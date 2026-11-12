//! **主催が客を部屋から外す**（11-12）。
//!
//! 外すと、名簿から除き、その相手に「居ない」を告げ、残りの面々へ名簿を配る。
//! 外した相手は**部屋ごとに締め出しへ控える**。客は帰り道から戻ろうとして叩くが、
//! 叩かれた時点で「その部屋はもう無い」と答える（`lib.rs` の待受）。客の側はこの答えで
//! 帰り道を消すので、戻ってこない（古い版の客も同じ）。
//!
//! 締め出しは**主催した部屋にだけ**効く。画面を立ち上げ直すと控えは消える（そのときは
//! 外した客が戻りうるので、もう一度外す）。

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use tauri::{AppHandle, Manager as _};
use warifu_core::PublicKey;
use warifu_meeting::{MeetingId, Notice};

use crate::{Answer, Bridge, Failure};

static 締め出し: Mutex<Option<HashMap<MeetingId, HashSet<[u8; 32]>>>> = Mutex::new(None);

/// その相手を、その部屋から締め出しているか。
#[must_use]
pub(crate) fn 締め出したか(部屋: MeetingId, 相手: &PublicKey) -> bool {
    締め出し
        .lock()
        .expect("毒されていない")
        .as_ref()
        .and_then(|棚| 棚.get(&部屋))
        .is_some_and(|面々| 面々.contains(&相手.to_bytes()))
}

fn 締め出す(部屋: MeetingId, 相手: &PublicKey) {
    締め出し
        .lock()
        .expect("毒されていない")
        .get_or_insert_with(HashMap::new)
        .entry(部屋)
        .or_default()
        .insert(相手.to_bytes());
}

/// **外してよいか**（純粋・試験で見る）。主催でなければ外せない。自分は外せない。名簿に居なければ外すものが無い。
///
/// # Errors
/// 人が読む訳。
pub(crate) fn 外してよいか(
    主催だ: bool,
    自分: &PublicKey,
    相手: &PublicKey,
    名簿に居る: bool,
) -> Result<(), String> {
    if !主催だ {
        return Err("この部屋の主催ではないので、外せません（外せるのは主催だけです）".to_owned());
    }
    if 相手 == 自分 {
        return Err("自分は外せません（抜けるなら room_leave）".to_owned());
    }
    if !名簿に居る {
        return Err("その相手は、この部屋の名簿に居ません".to_owned());
    }
    Ok(())
}

/// **外す。**名簿から除き、相手に「居ない」を告げ、残りへ名簿を配り、締め出しに控える。外した相手の呼び名を返す。
///
/// # Errors
/// 部屋に居ない・主催でない・名簿に居ない。人が読む訳。
pub(crate) async fn 外す(app: &AppHandle, 部屋: MeetingId, 相手: PublicKey) -> Result<(), String> {
    let bridge = app.state::<Bridge>();
    let me = bridge.device.public_key();
    let 主催 = bridge.主催たち.lock().await.get(&部屋).copied();
    let 主催だ = crate::roster_sync::主催か(主催.as_ref(), &me);
    let (出来事, 名簿, 面々) = {
        let mut 棚 = bridge.conferences.lock().await;
        let Some(c) = 棚.get_mut(&部屋) else {
            return Err("その部屋に居ません".to_owned());
        };
        外してよいか(主催だ, &me, &相手, c.members().contains(&相手))?;
        let 出来事 = c
            .on_notice(相手, &Notice::Leave { meeting: 部屋 })
            .map_err(|e| format!("名簿から外せませんでした: {e}"))?;
        (出来事, crate::名簿の写し(c), c.members().to_vec())
    };
    締め出す(部屋, &相手);
    // **相手に「居ない」を告げる**（届かなくても、戻ってきたときに締め出しが止める）
    let 線 = bridge.outbound.lock().await.get(&相手.to_bytes()).cloned();
    if let Some(tx) = 線 {
        let _ = tx.send(Notice::Leave { meeting: 部屋 }).await;
    }
    crate::emit_events(app, 部屋, &出来事);
    // **残りの面々へ、いまの名簿を配る**（外した相手は名簿に居ないので届かない）
    crate::名簿を配る(&bridge.outbound, 相手, me, 部屋, 名簿, &面々).await;
    記録!(
        "外す: {} を部屋 {} から外しました（締め出しに控えた）",
        crate::短く(&crate::key_to_string(相手)),
        crate::短く(&部屋.to_string())
    );
    Ok(())
}

/// 画面から：主催がその客を外す（名簿の［外す］）。
#[tauri::command]
pub(crate) async fn kick_member(app: AppHandle, room: String, key: String) -> Answer<()> {
    let 失敗 = |message: String| Failure { message, code: None };
    let 部屋: MeetingId = room.parse().map_err(|_| 失敗("部屋の id が読めません".to_owned()))?;
    let 相手: PublicKey = {
        use std::str::FromStr as _;
        PublicKey::from_str(&key).map_err(|_| 失敗("相手の鍵が読めません".to_owned()))?
    };
    外す(&app, 部屋, 相手).await.map_err(失敗)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn 鍵(b: u8) -> PublicKey {
        warifu_core::Seed::from_bytes([b; 32]).profile("Personal").device("PC").public_key()
    }

    #[test]
    fn 外せるのは主催だけで_自分と名簿に居ない相手は外せない() {
        assert!(外してよいか(true, &鍵(1), &鍵(2), true).is_ok());
        assert!(外してよいか(false, &鍵(1), &鍵(2), true).unwrap_err().contains("主催ではない"));
        assert!(外してよいか(true, &鍵(1), &鍵(1), true).unwrap_err().contains("自分は外せません"));
        assert!(外してよいか(true, &鍵(1), &鍵(2), false).unwrap_err().contains("名簿に居ません"));
    }

    #[test]
    fn 締め出しは_部屋ごとに効く() {
        let (a, b) = (MeetingId::generate(), MeetingId::generate());
        締め出す(a, &鍵(7));
        assert!(締め出したか(a, &鍵(7)));
        assert!(!締め出したか(b, &鍵(7)), "別の部屋には効かない");
        assert!(!締め出したか(a, &鍵(8)));
    }
}
