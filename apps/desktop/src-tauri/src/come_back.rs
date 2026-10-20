//! **ひとりになったら、自分から戻る**（0.1.28「ずっとつながる」）。
//!
//! ```text
//!   主催が上がり直す ──► ゲストは全員落ちる
//!   ゲストの画面に［前のルームに戻る］が出る ──► **人が押すまで、誰も戻らない**
//! ```
//!
//! 2026-10-20、主催の機械を上げ直したあと、ルームに残ったのは主催の機械の席だけだった。
//! 戻る口はあったが、**押す人が画面の前に居なかった。**
//!
//! だから**ゲストの機械が自分で戻る。**戻る先は帰り道（`rejoin.tsv`・入ったルームの数だけある）で、
//! **自分で抜けたルームには戻らない**（抜けたら、そのルームの帰り道を消している）。
//!
//! **通すかどうかは、相手の戸口が決める**（`rejoin_room` と同じ道を通る）。
//! 断られ続けたら、間を延ばして叩き続ける —— 主催がいつ戻るかは、こちらから分からない。

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use tauri::{AppHandle, Manager as _};

use crate::{Bridge, RoomRow};

/// 起動してから最初に見るまでの間。**画面と口が立ち上がるのを待つ。**
const 起動を待つ秒: u64 = 20;
/// 戻らなくてよいときに、見直す間。
const 見直す秒: u64 = 30;
/// 繋がってから、戻れたかを見直すまでの間。
const 落ち着くまでの秒: u64 = 5;
/// 戻れなかったときに延ばす上限。**10 分より長くは黙らない。**
const 待つ上限の秒: u64 = 600;

/// 戻っている最中か。**人の［戻る］と見張りが同時に叩かない**ようにする。
static 戻っている: AtomicBool = AtomicBool::new(false);

/// **戻るべきか。**
///
/// - 帰り道が無い …… 戻らない（入ったことが無いか、自分で抜けた）
/// - そのルームを持っていない …… 戻る（この機械が上がり直した）
/// - そのルームの主催が自分 …… 戻らない（自分の部屋で待てばよい）
/// - **主催と繋がっていない** …… 戻る（主催が上がり直した）
///
/// **人数では決めない。**ゲスト同士が繋がっていると 2 人以上残るが、
/// 主催が居なければ、新しく来る人とは会えない。
pub fn 戻るべきか(
    帰り道のルーム: Option<&str>,
    部屋たち: &[RoomRow],
    繋がっている: impl Fn(&str) -> bool,
) -> bool {
    let Some(帰り道) = 帰り道のルーム else {
        return false;
    };
    let Some(部屋) = 部屋たち.iter().find(|r| r.id == 帰り道) else {
        return true;
    };
    !部屋.host && !繋がっている(&部屋.host_key)
}

/// **戻れなかった回数から、次に叩くまでの秒を決める。**30 秒から倍にして、10 分で止める。
pub fn 次に待つ秒(しくじった: u32) -> u64 {
    let 倍 = 1u64 << しくじった.min(10);
    (見直す秒 * 倍).min(待つ上限の秒)
}

/// **戻る。**人の［戻る］も、見張りも、ここを通る。
///
/// 戻っている最中なら、**何もせずに返す**（同じ相手を 2 度叩かない）。
pub async fn 戻る(app: AppHandle, 鍵: String) -> crate::Answer<()> {
    if 戻っている.swap(true, Ordering::SeqCst) {
        記録!("戻る: もう戻っている最中です（重ねて叩きません）");
        return Ok(());
    }
    let 結果 = crate::前のルームへ戻る(app.clone(), app.state::<Bridge>(), 鍵).await;
    戻っている.store(false, Ordering::SeqCst);
    結果
}

/// ルームごとの、しくじった回数と、次に叩いてよい時刻。
type 控え = HashMap<String, (u32, Instant)>;

/// **見張る。**起動したら 1 本だけ立てる。
///
/// **入っているルームそれぞれを見る**（帰り道はルームの数だけある）。
/// 叩く間はルームごとに延ばす —— 1 つの主催が落ちたままでも、ほかの部屋へは早く戻る。
pub async fn 見張る(app: AppHandle) {
    tokio::time::sleep(Duration::from_secs(起動を待つ秒)).await;
    let mut 控え: 控え = HashMap::new();
    loop {
        let 戻る先 = いま戻る先(&app).await;
        // **戻らなくてよくなった部屋の控えは捨てる**（次に落ちたら 30 秒から）
        控え.retain(|id, _| 戻る先.iter().any(|(部屋, _)| 部屋 == id));
        for (id, 鍵) in 戻る先 {
            if 控え.get(&id).is_some_and(|(_, 次)| Instant::now() < *次) {
                continue;
            }
            一つ戻る(&app, &mut 控え, id, 鍵).await;
        }
        tokio::time::sleep(Duration::from_secs(見直す秒)).await;
    }
}

async fn 一つ戻る(app: &AppHandle, 控え: &mut 控え, id: String, 鍵: String) {
    let 頭 = crate::短く(&id);
    記録!("戻る: ルーム {頭} の主催と繋がっていないので、帰り道から戻ります");
    let 結果 = 戻る(app.clone(), 鍵).await;
    // **繋がった直後に断られることがある**（主催が別の部屋を建て直していると、
    // 割符を見てから口を閉じる）。すぐ見直すと「戻れた」と言い、30 秒ごとに叩き続けた
    if 結果.is_ok() {
        tokio::time::sleep(Duration::from_secs(落ち着くまでの秒)).await;
    }
    // **繋がっても、戻る先とは別のルームへ招かれることがある**
    // （割符なしで叩くと、主催がいま見ているルームへ招かれる）。戻れたかは見直して決める
    let まだ = いま戻る先(app).await.iter().any(|(部屋, _)| *部屋 == id);
    if !まだ {
        記録!("戻る: ルーム {頭} へ戻れました");
        控え.remove(&id);
        return;
    }
    let しくじった = 控え.get(&id).map_or(0, |(n, _)| *n).saturating_add(1);
    let 秒 = 次に待つ秒(しくじった);
    match 結果 {
        Ok(()) => 記録!(
            "戻る: 繋がりましたが、ルーム {頭} ではありませんでした。{秒} 秒あとにもう一度叩きます"
        ),
        Err(e) => 記録!(
            "戻る: ルーム {頭} へ戻れませんでした（{}）。{秒} 秒あとにもう一度叩きます",
            e.message
        ),
    }
    控え.insert(id, (しくじった, Instant::now() + Duration::from_secs(秒)));
}

/// **戻るべきルームの帰り道**（ルーム id と鍵）。
async fn いま戻る先(app: &AppHandle) -> Vec<(String, String)> {
    let Ok(vault) = warifu_vault::Vault::default_location() else {
        return Vec::new();
    };
    // 読めないときは戻らない。**半端な帰り道で叩かない**
    let Ok(帰り道) = vault.rejoins() else {
        return Vec::new();
    };
    let bridge = app.state::<Bridge>();
    let 部屋たち = crate::ルームの並び(&bridge).await;
    let 繋がっている: Vec<String> = bridge
        .outbound
        .lock()
        .await
        .keys()
        .filter_map(|k| warifu_core::PublicKey::from_bytes(*k).ok())
        .map(crate::key_to_string)
        .collect();
    帰り道
        .into_iter()
        .filter(|(id, _)| {
            戻るべきか(Some(id), &部屋たち, |鍵| 繋がっている.iter().any(|k| k == 鍵))
        })
        .collect()
}

#[cfg(test)]
mod 試験 {
    use super::*;

    fn 部屋(id: &str, host: bool, host_key: &str) -> RoomRow {
        RoomRow {
            id: id.into(),
            members: 1,
            host,
            host_key: host_key.into(),
        }
    }

    #[test]
    fn 帰り道が無ければ戻らない() {
        assert!(!戻るべきか(None, &[], |_| false));
    }

    #[test]
    fn ルームを持っていなければ戻る() {
        assert!(戻るべきか(Some("R"), &[部屋("別", true, "私")], |_| false));
    }

    #[test]
    fn 自分の部屋なら戻らない() {
        assert!(!戻るべきか(Some("R"), &[部屋("R", true, "私")], |_| false));
    }

    #[test]
    fn 主催と繋がっていれば戻らない() {
        assert!(!戻るべきか(Some("R"), &[部屋("R", false, "主")], |k| k == "主"));
    }

    #[test]
    fn 主催が落ちていれば_ゲストが残っていても戻る() {
        // 主催以外とは繋がっている（ゲスト同士）
        assert!(戻るべきか(Some("R"), &[部屋("R", false, "主")], |k| k == "客"));
    }

    #[test]
    fn 待つ間は倍に延びて_10分で止まる() {
        assert_eq!(次に待つ秒(0), 30);
        assert_eq!(次に待つ秒(1), 60);
        assert_eq!(次に待つ秒(2), 120);
        assert_eq!(次に待つ秒(5), 600);
        assert_eq!(次に待つ秒(u32::MAX), 600);
    }
}
