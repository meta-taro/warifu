//! **ルームでのエージェントの扱い**（0.1.52）。
//!
//! 社外の人が居るルームで、エージェントが勝手に話さないようにする。
//! これまでは席ごとの心得（「外の席が居る部屋では @all に答えない」）でしか守っていなかった。
//!
//! ```text
//!   主催（人が画面で決める） ── Notice::RoomRule ──► ほかの人
//!   新しく入った人 ◄── 主催が、いまの扱いを渡す
//! ```
//!
//! **決めるのは主催だけ。**受け取る側は、主催から来たものだけを受ける（ゲストが緩めることはできない）。
//! **止めるのは各機械の割符**で、自分の機械のエージェントの発言だけを止める。
//! 古い版（〜0.1.51）の機械は知らせを読み飛ばすので、その機械のエージェントには効かない。

use std::collections::HashMap;

use tauri::{AppHandle, Emitter as _, State};
use warifu_core::PublicKey;
use warifu_meeting::{MeetingId, Notice, エージェントの扱い};

use crate::{Answer, Bridge, 短く};

/// 扱いが届いた・変わったことを画面へ知らせる（ルーム id・扱いの数）。
pub const EVENT_ROOM_RULE: &str = "warifu://room-rule";

/// **届いた扱いを受け取るか。**そのルームの主催から直に来たものだけ。
///
/// 主催を覚えていない（自分が建てた）ルームへ、他人が送ってきたものは捨てる。
pub fn 受け取るか(経路の相手: PublicKey, 主催: Option<PublicKey>) -> bool {
    主催 == Some(経路の相手)
}

/// **エージェントの発言を断る訳。**流してよければ `None`。
///
/// - 名指しだけ：`@名前` で呼ばれた発言への返信だけ通す。`@all` / `@全員` と写し（`cc@`）は名指しに数えない
/// - 読むだけ：何も通さない
///
/// `返信先の本文` は `reply_to` が指す発言（無ければ `None`）。`名前たち` はそのエージェントが呼ばれる名前。
#[must_use]
pub fn 断る訳(
    扱い: エージェントの扱い,
    返信先の本文: Option<&str>,
    名前たち: &[String],
) -> Option<String> {
    match 扱い {
        エージェントの扱い::ふつう => None,
        エージェントの扱い::読むだけ => Some(
            "このルームは、主催が「エージェントは読むだけ」にしています。発言は流せません（読むのは chat_read でできます）"
                .to_owned(),
        ),
        エージェントの扱い::名指しだけ => {
            let 名指しか = 返信先の本文.is_some_and(|本文| {
                warifu_desk::名指しで宛てられたか(本文, 名前たち)
            });
            (!名指しか).then(|| {
                "このルームは、主催が「エージェントは名指しにだけ答える」にしています。\
                 あなたを @名前 で呼んだ発言に、reply_to を付けて返すときだけ流せます（@all は名指しに数えません）"
                    .to_owned()
            })
        }
    }
}

/// 控えてある扱い（ルーム id → 扱い）。ふつうのルームは載らない。
pub fn 控え() -> HashMap<String, エージェントの扱い> {
    match warifu_vault::Vault::default_location().and_then(|v| v.room_rules()) {
        Ok(並び) => 並び
            .into_iter()
            .filter_map(|(id, 数)| エージェントの扱い::数から(数).map(|扱い| (id, 扱い)))
            .collect(),
        Err(e) => {
            // **握り潰さない。**読めなければ「ふつう」に倒れる（止めるはずの席が話す）ので、記録に残す
            記録!("ルームの扱いを読めませんでした: {e}");
            HashMap::new()
        }
    }
}

/// そのルームの扱い。控えに無ければふつう。
pub fn 扱い(id: &str) -> エージェントの扱い {
    控え().remove(id).unwrap_or_default()
}

fn 控える(id: &str, 扱い: エージェントの扱い) {
    if let Err(e) =
        warifu_vault::Vault::default_location().and_then(|v| v.save_room_rule(id, 扱い.数()))
    {
        記録!("ルームの扱いを書けませんでした: {e}");
    }
}

/// 画面が起き上がったときに読む、**ルームごとの扱い**（数）。ふつうのルームは載らない。
#[tauri::command]
pub fn room_rules() -> HashMap<String, u8> {
    控え()
        .into_iter()
        .map(|(id, 扱い)| (id, 扱い.数()))
        .collect()
}

/// **主催の人が扱いを決めた。**控えて、そのルームの相手へ渡す。主催でなければ断る。
#[tauri::command]
pub async fn set_room_rule(bridge: State<'_, Bridge>, id: String, rule: u8) -> Answer<()> {
    let 扱い =
        エージェントの扱い::数から(rule).ok_or_else(|| format!("知らない扱いです（{rule}）"))?;
    let meeting = id
        .parse::<MeetingId>()
        .map_err(|_| format!("部屋の id が読めません（{id}）"))?;
    let me = bridge.device.public_key();
    // **覚えていない部屋は、自分が建てたもの**（`ルームの並び` と同じ読み方）
    let 主催か = bridge
        .主催たち
        .lock()
        .await
        .get(&meeting)
        .is_none_or(|主催| *主催 == me);
    if !主催か {
        return Err("エージェントの扱いを決められるのは、ルームの主催だけです".into());
    }
    控える(&id, 扱い);
    let notice = Notice::RoomRule { meeting, 扱い };
    let 送り先 =
        crate::そのルームの相手(&bridge.conferences, &bridge.outbound, meeting, me).await;
    記録!(
        "ルームの扱い: {} 人へ渡します（ルーム {}・{扱い:?}）",
        送り先.len(),
        短く(&id)
    );
    for tx in &送り先 {
        // 届かない相手が居ても止めない
        let _ = tx.send(notice.clone()).await;
    }
    Ok(())
}

/// **扱いが届いた。**主催からのものだけを控えて、画面へ出す。
pub fn 届いた(
    app: &AppHandle,
    meeting: MeetingId,
    経路の相手: PublicKey,
    主催: Option<PublicKey>,
    扱い: エージェントの扱い,
) {
    if !受け取るか(経路の相手, 主催) {
        記録!(
            "ルームの扱いを捨てました（主催 {} からではない）",
            短く(&crate::key_to_string(経路の相手))
        );
        return;
    }
    let id = meeting.to_string();
    記録!("ルームの扱いが届きました（ルーム {}・{扱い:?}）", 短く(&id));
    控える(&id, 扱い);
    let _ = app.emit(EVENT_ROOM_RULE, (id, 扱い.数()));
}

/// **新しく入った人へ、いまの扱いを渡す。**主催が、名前を渡すのと同じ瞬間に呼ぶ。
///
/// ふつうでも渡す（前に別の扱いで居た人の控えを、ふつうに戻すため）。
pub async fn 新入りへ渡す(bridge: &Bridge, 相手: PublicKey, meeting: MeetingId) {
    let 扱い = 扱い(&meeting.to_string());
    let tx = bridge.outbound.lock().await.get(&相手.to_bytes()).cloned();
    let Some(tx) = tx else {
        return;
    };
    let _ = tx.send(Notice::RoomRule { meeting, 扱い }).await;
}

#[cfg(test)]
mod 試験 {
    use super::*;

    fn 鍵(n: u8) -> PublicKey {
        warifu_core::Seed::from_bytes([n; 32])
            .profile("Personal")
            .device("PC")
            .public_key()
    }

    fn 名(n: &str) -> Vec<String> {
        vec![n.to_owned()]
    }

    #[test]
    fn 主催から直に来たものだけ受け取る() {
        assert!(受け取るか(鍵(9), Some(鍵(9))));
        assert!(!受け取るか(鍵(2), Some(鍵(9))));
        // 自分が建てた部屋（主催を覚えていない）へ他人が送ってきたもの
        assert!(!受け取るか(鍵(2), None));
    }

    #[test]
    fn ふつうなら何でも流せる() {
        assert_eq!(断る訳(エージェントの扱い::ふつう, None, &名("割符")), None);
    }

    #[test]
    fn 読むだけなら名指しされても流せない() {
        let 訳 = 断る訳(
            エージェントの扱い::読むだけ,
            Some("@割符 どうですか"),
            &名("割符"),
        );
        assert!(訳.is_some_and(|w| w.contains("読むだけ")));
    }

    #[test]
    fn 名指しだけなら_名指しした発言への返信だけ流せる() {
        let 扱い = エージェントの扱い::名指しだけ;
        assert_eq!(断る訳(扱い, Some("@割符 どうですか"), &名("割符")), None);
        // 返信でない・ほかの席を呼んだ発言・全員を呼んだ発言・写しは断る
        assert!(断る訳(扱い, None, &名("割符")).is_some());
        assert!(断る訳(扱い, Some("@PdM どうですか"), &名("割符")).is_some());
        assert!(断る訳(扱い, Some("@all 見てください"), &名("割符")).is_some());
        assert!(断る訳(扱い, Some("@全員 見てください"), &名("割符")).is_some());
        assert!(断る訳(扱い, Some("済みました cc@割符"), &名("割符")).is_some());
    }

    #[test]
    fn 名指しと全員の呼びが両方あれば_名指しとして通す() {
        assert_eq!(
            断る訳(
                エージェントの扱い::名指しだけ,
                Some("@all @割符 お願いします"),
                &名("割符")
            ),
            None
        );
    }
}
