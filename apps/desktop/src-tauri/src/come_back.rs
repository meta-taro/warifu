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

use tauri::{AppHandle, Emitter as _, Manager as _};

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
/// - **主催がそのルームの名簿に居ない** …… 戻る（0.1.37）。主催が上がり直したあと、
///   **別のルームの用事で**繋ぎ直してくると、線はあってもこのルームの名簿からは落ちたまま。
///   線だけを見ていたので、ゲストは戻らず、主催の部屋から消えたままだった（10-23 に 3 台で見た）
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
    if 部屋.host {
        return false;
    }
    let 主催が名簿に居る = 部屋.member_keys.contains(&部屋.host_key);
    !(繋がっている(&部屋.host_key) && 主催が名簿に居る)
}

/// **主催の部屋に居ないのに、居るつもりで送ってくる相手へ「居ない」と告げるか**（0.1.37）。
///
/// 主催が上がり直すと、主催の名簿からゲストは消える。ところが主催が**別のルームの用事で**
/// すぐ繋ぎ直すと、ゲストの側では線が切れたことにならず（入り直しとして扱う）、
/// **ゲストは居るつもりのまま下ごしらえを送り続け、主催は黙って捨て続けた**
/// （10-23・6 分で 60 通）。主催が `Leave` を返せば、ゲストは主催を名簿から外し、
/// 見張り（[`戻るべきか`]）が戸口から入り直す。
///
/// 告げるのは**自分が主催している部屋**についてだけ。1 本の線で 1 部屋 1 度
/// （`まだ告げていない`）。**戸口の答えではない**（線は既に通っている相手）ので、D31 とは別。
pub fn 居ないと告げるか<T: PartialEq>(
    名簿に居ないから断った: bool,
    部屋の主催: Option<&T>,
    自分: &T,
    まだ告げていない: bool,
) -> bool {
    名簿に居ないから断った && まだ告げていない && 部屋の主催.is_none_or(|主催| 主催 == 自分)
}

/// **戻れなかった回数から、次に叩くまでの秒を決める。**30 秒から倍にして、10 分で止める。
pub fn 次に待つ秒(しくじった: u32) -> u64 {
    let 倍 = 1u64 << しくじった.min(10);
    (見直す秒 * 倍).min(待つ上限の秒)
}

/// **戻る。**人の［戻る］も、見張りも、ここを通る。
///
/// 戻っている最中なら、**何もせずに返す**（同じ相手を 2 度叩かない）。
pub async fn 戻る(
    app: AppHandle,
    部屋: Option<warifu_meeting::MeetingId>,
    鍵: String,
) -> crate::Answer<()> {
    if 戻っている.swap(true, Ordering::SeqCst) {
        記録!("戻る: もう戻っている最中です（重ねて叩きません）");
        return Ok(());
    }
    let 結果 = crate::前のルームへ戻る(app.clone(), app.state::<Bridge>(), 部屋, 鍵).await;
    戻っている.store(false, Ordering::SeqCst);
    結果
}

/// ルームごとの、しくじった回数と、次に叩いてよい時刻。
/// しくじった回数・次に叩いてよい時刻・**主催に繋がったのに別の部屋を教えられた回数**（続けて）。
type 控え = HashMap<String, (u32, Instant, u32)>;

/// 主催に「その部屋はもう無い」と言われた返りか（0.1.41・`call::もう無いの印`）。
pub(crate) fn もう無いと言われたか(結果: &crate::Answer<()>) -> bool {
    matches!(結果, Err(e) if e.code.as_deref() == Some(crate::call::もう無いの印))
}

/// **主催が「その部屋は知らない」と続けて言ったら、見切る回数**（0.1.36）。
const 見切る回数: u32 = 3;

/// **その部屋は、もう主催の居ない部屋か。**
///
/// 主催に繋がれたのに別の部屋を教えられるのは、主催がその部屋を主催していないとき。
/// 0.1.32 までの取り違えで、**2 台が互いに「主催は相手」と覚えた部屋**が残っていた
/// （10-22 に mac mini と mac air で見た）。そのままだと、10 分ごとに叩き合い続ける。
pub fn 見切るか(続けて違った: u32) -> bool {
    続けて違った >= 見切る回数
}

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
            if 控え.get(&id).is_some_and(|(_, 次, _)| Instant::now() < *次) {
                continue;
            }
            一つ戻る(&app, &mut 控え, id, 鍵).await;
        }
        tokio::time::sleep(Duration::from_secs(見直す秒)).await;
    }
}

/// **見張りが戻ったあと、どのルームを見るか。**前に見ていたルームがまだ在れば、そこに戻す。
fn 見張りのあとに見るルーム<T: Copy>(前: Option<T>, いま: Option<T>, 前は在る: bool) -> Option<T> {
    match 前 {
        Some(前) if 前は在る => Some(前),
        _ => いま,
    }
}

/// **戻るときに、頼んでいない部屋へ招かれた分**（0.1.36）。
///
/// 割符なしで叩くと、主催が選んだ部屋へ招かれる。戻りたい部屋でなければ、
/// その部屋は持ち続けない —— 叩き直すたびに知らない部屋が増え、通話も作り直していた。
pub(crate) fn 頼んでいない部屋<T: PartialEq + Copy>(前: &[T], 後: &[T], 戻る先: Option<T>) -> Vec<T> {
    後.iter()
        .filter(|id| !前.contains(id) && Some(**id) != 戻る先)
        .copied()
        .collect()
}

async fn 一つ戻る(app: &AppHandle, 控え: &mut 控え, id: String, 鍵: String) {
    let 頭 = crate::短く(&id);
    記録!("戻る: ルーム {頭} の主催と繋がっていないので、帰り道から戻ります");
    // **見ているルームを奪わない**（0.1.33）。戻る道は人が押したときと同じで、
    // 戻った先を「いま話すルーム」にしていた。人は別のルームを見たまま打つので、
    // **打った文字が、戻った先のルームへ流れていた**（10-22 に 3 台で確かめた）
    let bridge = app.state::<Bridge>();
    let 前 = crate::いま見ているルーム(&bridge.いまのルーム).await;
    let 前の部屋: Vec<_> = bridge.conferences.lock().await.keys().copied().collect();
    let 結果 = 戻る(app.clone(), id.parse().ok(), 鍵).await;
    {
        let 在る = match 前 {
            Some(前) => bridge.conferences.lock().await.contains_key(&前),
            None => false,
        };
        let mut いま = bridge.いまのルーム.lock().await;
        *いま = 見張りのあとに見るルーム(前, *いま, 在る);
        let 見る = *いま;
        drop(いま);
        if let Some(部屋) = 見る {
            crate::影へ(&bridge, warifu_app::room::入り口::見る { 部屋 }).await;
        }
    }
    // **画面にも読み直させる。**戻った瞬間に画面は戻った先を読んでいる
    let _ = app.emit(crate::EVENT_LOOK, ());
    // **繋がった直後に断られることがある**（主催が別の部屋を建て直していると、
    // 割符を見てから口を閉じる）。すぐ見直すと「戻れた」と言い、30 秒ごとに叩き続けた
    if 結果.is_ok() {
        tokio::time::sleep(Duration::from_secs(落ち着くまでの秒)).await;
    }
    // **頼んでいない部屋へ招かれていたら、抜ける**（0.1.36）
    let 後の部屋: Vec<_> = bridge.conferences.lock().await.keys().copied().collect();
    let 戻る先 = 後の部屋.iter().copied().find(|m| m.to_string() == id);
    for 部屋 in 頼んでいない部屋(&前の部屋, &後の部屋, 戻る先) {
        記録!("戻る: 頼んでいない部屋 {} へ招かれたので抜けます", crate::短く(&部屋.to_string()));
        crate::部屋を畳む(&bridge, 部屋).await;
    }
    // **主催が「その部屋はもう無い」と言った**（0.1.41）。3 回叩いて見切るのを待たず、1 回で帰り道を消す
    if もう無いと言われたか(&結果) {
        見切る(app, &id, "主催が「その部屋はもう無い」と言った").await;
        控え.remove(&id);
        return;
    }
    // **繋がっても、戻る先とは別のルームへ招かれることがある**
    // （割符なしで叩くと、主催がいま見ているルームへ招かれる）。戻れたかは見直して決める
    let まだ = いま戻る先(app).await.iter().any(|(部屋, _)| *部屋 == id);
    if !まだ {
        記録!("戻る: ルーム {頭} へ戻れました");
        控え.remove(&id);
        return;
    }
    let (前のしくじり, _, 前の違い) = 控え.get(&id).copied().unwrap_or((0, Instant::now(), 0));
    let しくじった = 前のしくじり.saturating_add(1);
    // 繋がれなかった回は数えない（主催が落ちているだけかもしれない）
    let 違った = if 結果.is_ok() { 前の違い.saturating_add(1) } else { 0 };
    if 見切るか(違った) {
        見切る(app, &id, &format!("主催が別の部屋を {見切る回数} 回続けて教えた")).await;
        控え.remove(&id);
        return;
    }
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
    控え.insert(id, (しくじった, Instant::now() + Duration::from_secs(秒), 違った));
}

/// **主催の居ない部屋を見切る。**帰り道を忘れ、自分のほかに誰も居なければ部屋も畳む。
async fn 見切る(app: &AppHandle, id: &str, 訳: &str) {
    let 頭 = crate::短く(id);
    記録!("戻る: {訳}ので、ルーム {頭} へはもう戻りません");
    let bridge = app.state::<Bridge>();
    let 部屋たち = crate::ルームの並び(&bridge).await;
    let 誰も居ない = 部屋たち
        .iter()
        .find(|r| r.id == id)
        .is_none_or(|r| r.member_keys.len() <= 1);
    let 部屋 = bridge
        .conferences
        .lock()
        .await
        .keys()
        .copied()
        .find(|m| m.to_string() == id);
    match 部屋 {
        // 畳むと帰り道も忘れる
        Some(部屋) if 誰も居ない => {
            記録!("戻る: ルーム {頭} には誰も居ないので畳みます");
            crate::部屋を畳む(&bridge, 部屋).await;
        }
        _ => {
            if let Ok(vault) = warifu_vault::Vault::default_location() {
                if let Err(e) = vault.forget_rejoin_room(id) {
                    記録!("帰り道を消せませんでした: {e}");
                }
            }
        }
    }
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
    #[test]
    fn もう無いと言われた返りだけを見分ける() {
        let もう無い: crate::Answer<()> = Err(crate::Failure {
            message: "x".into(),
            code: Some(crate::call::もう無いの印.into()),
        });
        assert!(super::もう無いと言われたか(&もう無い));
        let 居ない: crate::Answer<()> = Err(crate::Failure {
            message: "x".into(),
            code: Some("contact.unreachable".into()),
        });
        assert!(!super::もう無いと言われたか(&居ない));
        assert!(!super::もう無いと言われたか(&Ok(())));
    }


    #[test]
    fn 見張りが戻っても_前に見ていたルームを奪わない() {
        use super::見張りのあとに見るルーム;
        assert_eq!(見張りのあとに見るルーム(Some(1), Some(2), true), Some(1));
        // 前のルームが無くなっていれば、戻った先を見る
        assert_eq!(見張りのあとに見るルーム(Some(1), Some(2), false), Some(2));
        // 何も見ていなかったなら、戻った先を見る
        assert_eq!(見張りのあとに見るルーム(None, Some(2), false), Some(2));
    }

    use super::*;

    #[test]
    fn 主催が別の部屋を_3_回続けて教えたら見切る() {
        assert!(!見切るか(2));
        assert!(見切るか(3));
    }

    #[test]
    fn 戻る先とは別に増えた部屋は_頼んでいない部屋() {
        assert_eq!(頼んでいない部屋(&[1, 2], &[1, 2, 7], Some(5)), vec![7]);
    }

    #[test]
    fn 戻る先に戻れたなら_それは抜けない() {
        assert_eq!(頼んでいない部屋(&[1], &[1, 5], Some(5)), Vec::<i32>::new());
    }

    #[test]
    fn 前から居た部屋は抜けない() {
        assert_eq!(頼んでいない部屋(&[1, 7], &[1, 7], None), Vec::<i32>::new());
    }

    fn 部屋(id: &str, host: bool, host_key: &str) -> RoomRow {
        RoomRow {
            id: id.into(),
            members: 1,
            host,
            host_key: host_key.into(),
            // 主催がそのルームの名簿に居る（ふだんの形）
            member_keys: vec![host_key.into()],
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
    fn 主催と繋がっていても_そのルームの名簿に主催が居なければ戻る() {
        // 主催が上がり直したあと、**別のルームの用事で**繋ぎ直してきた。
        // 線はあるが、このルームの名簿からは落ちたまま（切れたときに外している）
        let mut r = 部屋("R", false, "主");
        r.member_keys = vec!["私".into()];
        assert!(戻るべきか(Some("R"), &[r], |k| k == "主"));
    }

    #[test]
    fn 自分の部屋に居ない相手からの下ごしらえには_1度だけ居ないと告げる() {
        assert!(居ないと告げるか(true, None, &"私", true)); // 覚えていない＝自分が建てた
        assert!(居ないと告げるか(true, Some(&"私"), &"私", true));
        assert!(!居ないと告げるか(true, Some(&"私"), &"私", false)); // もう告げた
    }

    #[test]
    fn 他人の部屋や_ほかの理由で断ったときは告げない() {
        assert!(!居ないと告げるか(true, Some(&"主"), &"私", true));
        assert!(!居ないと告げるか(false, Some(&"私"), &"私", true));
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
