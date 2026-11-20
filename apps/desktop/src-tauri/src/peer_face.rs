//! **相手の画面にも顔を出す**（0.1.53）。この端末の人と席（エージェント）の顔を、同じルームの相手へ渡す。
//!
//! 名乗り（`Notice::Profile`）は名前と紹介だけなので、置いた顔は置いた機械の画面にしか出なかった。
//! 別の知らせ（`Notice::顔`）で渡す。古い版は知らない知らせとして読み飛ばすので、
//! **顔が出るのは、両方の機械が 0.1.53 以降のときだけ。**
//!
//! - 送る：ルームに入るとき（名乗りと一緒に）・顔を置き直したとき・席がつながったとき
//! - 受ける：差出人の確かめは文字と同じ（主催ごしの分だけ通す）。**中身を検めてから置く**
//! - 置き場所：`avatars/peer_<鍵>_<席の 16 進>.img`。網から来た名前をファイル名にしない
//! - 主催は、ほかの面々へそのまま配る（`話し手の場所` と同じ）

use tauri::{AppHandle, Emitter as _, Manager as _};
use warifu_core::PublicKey;
use warifu_meeting::{MeetingId, Notice};
use warifu_vault::{Vault, Who};

use crate::{Answer, Bridge, Failure};

/// 相手の顔が届いた・消えたことを画面へ知らせる（機械の鍵・席・中身。中身が空なら既定へ戻した）。
pub const EVENT_PEER_FACE: &str = "warifu://peer-face";

/// 1 台の相手について置いておく顔の数の上限（人 ＋ 席）。**網の向こうに置き場所を埋めさせない。**
const 一台の顔の上限: usize = 16;

/// 席の名乗り（場所）から、この端末の誰か。空ならこの端末の人。
fn 席から誰(席: &str) -> Who {
    if 席.is_empty() {
        Who::Me
    } else {
        Who::Desk(crate::desk::呼び方(Some(席)))
    }
}

/// いま置いてある顔で知らせを作る。**置いていなければ、`空も` のときだけ空の顔（既定へ戻した）を作る。**
fn 知らせを作る(
    meeting: MeetingId, 自分: PublicKey, 席: &str, 空も: bool
) -> Option<Notice> {
    let 中身 = crate::profile::置いてある顔(&席から誰(席)).ok().flatten();
    let 中身 = match 中身 {
        Some(中身) => 中身,
        None if 空も => Vec::new(),
        None => return None,
    };
    let 知らせ = Notice::顔 {
        meeting,
        from: 自分,
        席: 席.to_owned(),
        中身,
    };
    // **組めないもの（大きすぎる・行を壊す席の名）は送らない**
    知らせ.to_intent().is_ok().then_some(知らせ)
}

/// いまつながっている席の名乗り（場所）。名乗りのない席は入れない（相手の画面で結べない）。
fn つながっている席() -> Vec<String> {
    crate::desk::つながっている顔ぶれ()
        .iter()
        .filter_map(|呼び方| crate::speaker_place::呼び方から場所(呼び方).map(str::to_owned))
        .collect()
}

/// **ルームに入るときに添える顔**（名乗りの直後に送る）。置いてある分だけ。
pub(crate) fn 入るときの顔(
    meeting: MeetingId,
    自分: PublicKey,
) -> Vec<warifu_intent::Intent> {
    std::iter::once(String::new())
        .chain(つながっている席())
        .filter_map(|席| 知らせを作る(meeting, 自分, &席, false))
        .filter_map(|n| n.to_intent().ok())
        .collect()
}

/// **その人か席の顔を、居るルームすべての相手へ配る。**`空も` なら、置いていなくても「既定へ戻した」を配る。
pub(crate) async fn 顔を配る(bridge: &Bridge, 席: &str, 空も: bool) {
    let 自分 = bridge.device.public_key();
    let 部屋たち: Vec<MeetingId> = bridge.conferences.lock().await.keys().copied().collect();
    for meeting in 部屋たち {
        let Some(知らせ) = 知らせを作る(meeting, 自分, 席, 空も) else {
            return;
        };
        let 線たち =
            crate::そのルームの相手(&bridge.conferences, &bridge.outbound, meeting, 自分).await;
        for tx in 線たち {
            // 届かない相手が居ても止めない
            let _ = tx.send(知らせ.clone()).await;
        }
    }
}

/// **席が自分で顔を置いた**（`desk.rs`）。居るルームの相手へ配る。
pub(crate) async fn 席の顔を配る(bridge: &Bridge, 席: &str) {
    顔を配る(bridge, 席, true).await;
}

/// **席がつながった。**顔を置いていれば、居るルームの相手へ配る（入ったあとでつながった席の分）。
pub(crate) async fn つながった席の顔を配る(bridge: &Bridge, 席: &str) {
    顔を配る(bridge, 席, false).await;
}

/// **届いた。**検めて置き、画面へ出し、主催ならほかの面々へ配る。差出人の確かめは呼ぶ側が済ませてある。
pub(crate) async fn 届いた(
    app: &AppHandle, 線の相手: PublicKey, 知らせ: &Notice, 主催だ: bool
) {
    let Notice::顔 {
        meeting,
        from,
        席,
        中身,
    } = 知らせ
    else {
        return;
    };
    let bridge = app.state::<Bridge>();
    let 自分 = bridge.device.public_key();
    // 自分の顔が回ってきたものは置かない（自分の顔は自分の置き場所にある）
    if *from == 自分 {
        return;
    }
    let 鍵 = crate::key_to_string(*from);
    if let Err(訳) = 置く(&鍵, 席, 中身) {
        記録!("受信: 顔を置きませんでした（{}・{訳}）", crate::短く(&鍵));
        return;
    }
    記録!(
        "受信: 顔（{}・{}・{} バイト）",
        crate::短く(&鍵),
        if 席.is_empty() { "人" } else { "席" },
        中身.len()
    );
    let _ = app.emit(EVENT_PEER_FACE, (鍵, 席.clone(), 中身.clone()));
    if !主催だ {
        return;
    }
    // **主催は、ほかの面々へそのまま配る**（差出人は書き換えない）
    let 面々 =
        crate::そのルームの面々(&bridge.conferences, &bridge.outbound, *meeting, 自分).await;
    for (相手, 線) in 面々 {
        if 相手 == 線の相手 || 相手 == *from {
            continue;
        }
        if let Some(tx) = 線 {
            let _ = tx.send(知らせ.clone()).await;
        }
    }
}

/// 相手の顔を置き場所へ置く（空なら消す）。**中身を信じない** —— 画像として読めなければ置かない。
fn 置く(鍵: &str, 席: &str, 中身: &[u8]) -> Result<(), String> {
    let 名 = warifu_vault::相手の顔のファイル名(鍵, 席)
        .ok_or_else(|| "鍵の形が読めません".to_owned())?;
    let 置き場 = Vault::default_location()
        .map_err(|e| e.to_string())?
        .avatars_dir();
    if 中身.is_empty() {
        // 既定へ戻した。無くても構わない
        let _ = std::fs::remove_file(置き場.join(&名));
        return Ok(());
    }
    warifu_vault::顔として読む(中身).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&置き場).map_err(|e| format!("置き場所を作れません（{e}）"))?;
    if !置き場.join(&名).exists() && その相手の顔の数(&置き場, 鍵) >= 一台の顔の上限
    {
        return Err(format!("1 台の顔は {一台の顔の上限} までです"));
    }
    std::fs::write(置き場.join(&名), 中身).map_err(|e| format!("置けません（{e}）"))
}

/// 置き場所にある、その相手の顔の数。
fn その相手の顔の数(置き場: &std::path::Path, 鍵: &str) -> usize {
    相手の顔を並べる(置き場)
        .into_iter()
        .filter(|(k, _, _)| k == 鍵)
        .count()
}

/// 置き場所にある相手の顔（鍵・席・ファイルの場所）。形の合わないファイルは並べない。
fn 相手の顔を並べる(
    置き場: &std::path::Path
) -> Vec<(String, String, std::path::PathBuf)> {
    let Ok(中) = std::fs::read_dir(置き場) else {
        return Vec::new();
    };
    中.filter_map(Result::ok)
        .filter_map(|e| {
            let 名 = e.file_name().into_string().ok()?;
            let (鍵, 席) = warifu_vault::相手の顔のファイル名を読む(&名)?;
            Some((鍵, 席, e.path()))
        })
        .collect()
}

/// 画面へ渡す、相手の顔 1 つ。
#[derive(Debug, serde::Serialize)]
pub struct PeerFace {
    /// 機械の鍵（全桁）。
    key: String,
    /// 席の名乗り（場所）。空ならその機械の人。
    seat: String,
    /// 画像。
    bytes: Vec<u8>,
}

/// **置いてある相手の顔をすべて**画面へ渡す（立ち上げたとき・画面を読み直したとき）。
///
/// 置き場所の中身でも、**渡す前にもう一度検める**（置いたあとで入れ替えられていても、絵でないものは渡さない）。
#[tauri::command]
pub async fn peer_faces() -> Answer<Vec<PeerFace>> {
    let 置き場 = Vault::default_location()
        .map_err(|e| Failure {
            message: e.to_string(),
            code: None,
        })?
        .avatars_dir();
    Ok(相手の顔を並べる(&置き場)
        .into_iter()
        .filter_map(|(key, seat, 場所)| {
            let bytes = std::fs::read(場所).ok()?;
            warifu_vault::顔として読む(&bytes).ok()?;
            Some(PeerFace { key, seat, bytes })
        })
        .collect())
}

#[cfg(test)]
mod 試験 {
    use super::*;

    #[test]
    fn 席の名乗りから_この端末の誰かを決める() {
        assert_eq!(席から誰(""), Who::Me);
        assert_eq!(
            席から誰("docs"),
            Who::Desk("docs のエージェント".to_owned())
        );
    }

    #[test]
    fn 置き場所の相手の顔だけを並べる() {
        let 置き場 = std::env::temp_dir().join(format!("warifu-peer-face-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&置き場);
        std::fs::create_dir_all(&置き場).unwrap();
        let 名 = warifu_vault::相手の顔のファイル名("ABC", "docs").unwrap();
        std::fs::write(置き場.join(&名), b"x").unwrap();
        std::fs::write(置き場.join("me.img"), b"x").unwrap();
        std::fs::write(置き場.join("desk_docs.img"), b"x").unwrap();
        let 並び = 相手の顔を並べる(&置き場);
        assert_eq!(並び.len(), 1);
        assert_eq!((並び[0].0.as_str(), 並び[0].1.as_str()), ("ABC", "docs"));
        assert_eq!(その相手の顔の数(&置き場, "ABC"), 1);
        assert_eq!(その相手の顔の数(&置き場, "XYZ"), 0);
        let _ = std::fs::remove_dir_all(&置き場);
    }
}
