//! **ルームの名前を、居る人みんなで持つ**（0.1.28「ずっとつながる」）。
//!
//! 前は付けた人の画面にしか出なかった。同じルームが機械ごとに別の名前で見え、
//! 「どのルームの話か」が人どうしで噛み合わなかった。
//!
//! ```text
//!   付けた人 ── Notice::RoomName ──► 主催 ── 配り直す ──► ほかの人
//!   新しく入った人 ◄── 主催が、いまの名前を渡す
//! ```
//!
//! **誰が付けてもよい。いちばん新しいものが勝つ。**
//!
//! **`room.tsv` には、主催しているルームの名前だけを書く。**
//! 他人のルームの id を書くと、次の起動で他人の id で建てることになる
//! （0.1.27 までは、ゲストが名前を付けるとそこへ書いていた）。

use std::collections::HashMap;

use tauri::{AppHandle, Emitter as _};
use warifu_core::PublicKey;
use warifu_meeting::{MeetingId, Notice};

use crate::{Answer, Bridge, key_to_string, 短く};

/// 名前が届いた・変わったことを画面へ知らせる（ルーム id・名前）。
pub const EVENT_ROOM_NAME: &str = "warifu://room-name";

/// **届いた名前を受け取るか。**
///
/// 付けた本人から直に来たもの、または**そのルームの主催が配り直したもの**だけ。
/// それ以外（ゲストが他人の名を騙って送るなど）は捨てる。
pub fn 受け取るか(
    付けた人: PublicKey,
    経路の相手: PublicKey,
    主催: Option<PublicKey>,
) -> bool {
    付けた人 == 経路の相手 || 主催 == Some(経路の相手)
}

/// 控えてある名前。
fn 控え() -> HashMap<String, String> {
    match warifu_vault::Vault::default_location().and_then(|v| v.room_names()) {
        Ok(並び) => 並び.into_iter().collect(),
        Err(e) => {
            記録!("ルームの名前を読めませんでした: {e}");
            HashMap::new()
        }
    }
}

fn 控える(id: &str, 名前: &str) {
    let 結果 = warifu_vault::Vault::default_location().and_then(|v| {
        v.save_room_name(id, 名前)?;
        // **主催しているルームなら、`room.tsv` の名前も揃える**（起動したときに読む）
        if v.my_room()?.is_some_and(|(自分の, _)| 自分の == id) {
            v.save_my_room(id, 名前)?;
        }
        Ok(())
    });
    if let Err(e) = 結果 {
        // **握り潰さない。**書けなければ、次の起動で名前が消える
        記録!("ルームの名前を書けませんでした: {e}");
    }
}

/// 画面が起き上がったときに読む、**ルームごとの名前**。
#[tauri::command]
pub fn room_names() -> HashMap<String, String> {
    控え()
}

/// **人が名前を付けた。**控えて、そのルームの相手へ渡す。
pub async fn 付ける(bridge: &Bridge, id: &str, 名前: &str) -> Answer<()> {
    控える(id, 名前);
    // まだ誰とも居ないルームなら、控えるだけでよい（入ってきた人へは主催が渡す）
    let Ok(meeting) = id.parse::<MeetingId>() else {
        return Ok(());
    };
    let me = bridge.device.public_key();
    let notice = Notice::RoomName {
        meeting,
        from: me,
        名前: 名前.trim().to_owned(),
    };
    // **組めないものは、送る前に断る**（長すぎる・改行を含む）
    notice.to_intent()?;
    let 送り先 =
        crate::そのルームの相手(&bridge.conferences, &bridge.outbound, meeting, me).await;
    記録!(
        "ルームの名前: {} 人へ渡します（ルーム {}）",
        送り先.len(),
        短く(id)
    );
    for tx in &送り先 {
        // 届かない相手が居ても止めない
        let _ = tx.send(notice.clone()).await;
    }
    Ok(())
}

/// **名前が届いた。**受け取るかを決め、控えて、画面へ出す。
///
/// 受け取ったら `true`（主催なら、呼ぶ側がほかの人へ配り直す）。
pub fn 届いた(
    app: &AppHandle,
    meeting: MeetingId,
    付けた人: PublicKey,
    経路の相手: PublicKey,
    主催: Option<PublicKey>,
    名前: &str,
) -> bool {
    if !受け取るか(付けた人, 経路の相手, 主催) {
        記録!(
            "ルームの名前を捨てました（付けた人と経路の相手 {} が違い、主催でもない）",
            短く(&key_to_string(経路の相手))
        );
        return false;
    }
    let id = meeting.to_string();
    記録!(
        "ルームの名前が届きました（ルーム {}・{} 文字）",
        短く(&id),
        名前.chars().count()
    );
    控える(&id, 名前);
    let _ = app.emit(EVENT_ROOM_NAME, (id, 名前.to_owned()));
    true
}

/// **新しく入った人へ、いまの名前を渡す。**主催が、紹介を配るのと同じ瞬間に呼ぶ。
pub async fn 新入りへ渡す(bridge: &Bridge, 相手: PublicKey, meeting: MeetingId) {
    let Some(名前) = 控え().remove(&meeting.to_string()) else {
        return;
    };
    let tx = bridge.outbound.lock().await.get(&相手.to_bytes()).cloned();
    let Some(tx) = tx else {
        return;
    };
    let _ = tx
        .send(Notice::RoomName {
            meeting,
            from: bridge.device.public_key(),
            名前,
        })
        .await;
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

    #[test]
    fn 付けた本人から直に来たものは受け取る() {
        assert!(受け取るか(鍵(1), 鍵(1), Some(鍵(9))));
    }

    #[test]
    fn 主催が配り直したものは受け取る() {
        assert!(受け取るか(鍵(1), 鍵(9), Some(鍵(9))));
    }

    #[test]
    fn 主催でない人が他人の名で送ったものは捨てる() {
        assert!(!受け取るか(鍵(1), 鍵(2), Some(鍵(9))));
        assert!(!受け取るか(鍵(1), 鍵(2), None));
    }
}
