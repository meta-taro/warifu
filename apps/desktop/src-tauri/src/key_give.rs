//! **ルームキーを、その相手の機械にだけ手渡す**（2026-10-20）。
//!
//! ```text
//!   エージェント  room_key_give（誰へ・なぜ）
//!   この PC の人  帯の［渡す］／［渡さない］           ← **人が承認する**
//!   この機械      鍵を 1 本出し、その相手への線にだけ送る（Notice::Key）
//!   相手の機械    「〇〇からルームキーが届きました。入りますか？」 ← **入るかは相手の人が決める**
//! ```
//!
//! # なぜ割符の線で渡すのか
//!
//! 人が別の道（メール・チャットのアプリ）で渡すと、**鍵が外の網に出る。**
//! 割符の線は暗号化されていて、相手の機械に直接届く。
//!
//! # 鍵の文字を誰に見せるか
//!
//! - **エージェントには返さない。**返りに欄が無い（`warifu_desk::手渡しの返り`）
//! - **ルームの会話にも、記録（`warifu.log`）にも書かない。**誰へ・いつまでだけ書く
//! - 受け取った側の画面だけが持つ（入るときに使う）
//!
//! # いまつながっている相手だけ
//!
//! つながっていない相手へは、この線が無い。連絡先の相手なら「連絡先から呼ぶ」で
//! 鍵なしで入れられる（戸口が知り合いを通す）。

use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use tauri::{AppHandle, Emitter, Manager, State};
use warifu_core::PublicKey;
use warifu_desk::手渡しの返り;
use warifu_meeting::Notice;

use crate::{Answer, Bridge, Failure, key_to_string, 短く};

/// **前もって「聞かずに渡す」と決めておく札の名**（0.1.41）。持続の札（D129）と同じ控えに置く。
///
/// 人が帯の［今後は聞かずに渡す］を押すと、この PC では以後、人に聞かずに渡す。
/// 受け取る側の人が「入る」を押す所は変えない。取り消しは帯の下の一覧から。
/// **名は小文字・数字・点だけ**（`warifu-capability` の Action の決まり）。`room.key_give` にしたら
/// 口（`warifu mcp`）が控えを読むときに落ちた（10-29 に dev で踏んだ）。
pub const 聞かずに渡す札: &str = "room.handoff";

/// 「取り消すまで」の札の期限（秒）。持続の札の形（期限つき）のまま、実質ずっと効く。
pub const 取り消すまでの秒: u64 = 100 * 365 * 24 * 60 * 60;

/// 承認を待っている頼みが変わった（画面が帯を出し直す）。
pub const EVENT_HANDOFF: &str = "warifu://handoff";

/// 相手から鍵が届いた（画面が「入りますか？」を出す）。中身は `(鍵, 誰から)`。
pub const EVENT_KEY_GIFT: &str = "warifu://keygift";

/// 人の答えを待っている 1 件。**鍵はまだ出していない**（人が押してから出す）。
#[derive(Debug, Clone, serde::Serialize)]
pub struct 頼み {
    /// 答えるときに指す番号。
    pub 番号: u64,
    /// 誰へ（連絡先の名前、覚えていなければ鍵の頭）。
    pub 相手: String,
    /// **なぜ渡すか。**人が読んで決める。
    pub 訳: String,
    /// どのエージェントが頼んだか。
    pub 頼んだ人: String,
    /// 相手の鍵（画面へは出さない）。
    #[serde(skip)]
    相手の鍵: Option<PublicKey>,
}

static 頼みの棚: Mutex<Vec<頼み>> = Mutex::new(Vec::new());
static 次の番号: AtomicU64 = AtomicU64::new(1);

/// 答えが出た頼みの結果。**待っている机の口が取りに来る。**
static 結果たち: Mutex<Option<HashMap<u64, 手渡しの返り>>> = Mutex::new(None);

static 答えの知らせ: std::sync::OnceLock<tokio::sync::Notify> = std::sync::OnceLock::new();

fn 答えの知らせ口() -> &'static tokio::sync::Notify {
    答えの知らせ.get_or_init(tokio::sync::Notify::new)
}

/// 承認を待っている頼み（画面が読む）。
#[must_use]
pub fn 並べる() -> Vec<頼み> {
    頼みの棚.lock().expect("毒されていない").clone()
}

/// **宛先を探す帳**を作る。連絡先に、いまつながっている相手（鍵の頭を名前に）を足す。
///
/// 連絡先に無くても、**いまつながっている相手には渡せる**ようにする ——
/// 名前を付ける前の相手にも、鍵の頭で指せる。
#[must_use]
pub fn 宛先の帳(
    連絡先: Vec<(String, String)>,
    つながっている: &[String],
) -> Vec<(String, String)> {
    let mut 帳 = 連絡先;
    for 鍵 in つながっている {
        if !帳.iter().any(|(k, _)| k == 鍵) {
            帳.push((鍵.clone(), 短く(鍵)));
        }
    }
    帳
}

/// 結果を置いて、待っている口を起こす。
fn 結果を置く(番号: u64, 結果: 手渡しの返り) {
    結果たち
        .lock()
        .expect("毒されていない")
        .get_or_insert_with(HashMap::new)
        .insert(番号, 結果);
    答えの知らせ口().notify_waiters();
}

/// 結果を取り出す（**取り出したら消える**）。
fn 結果を取る(番号: u64) -> Option<手渡しの返り> {
    結果たち
        .lock()
        .expect("毒されていない")
        .as_mut()
        .and_then(|棚| 棚.remove(&番号))
}

/// 人の答えを、最長 `秒` 待つ。**待ちきれなければ `まだ`**（頼みは画面に残る）。
async fn 答えを待つ(番号: u64, 秒: u64) -> 手渡しの返り {
    let 終わり = tokio::time::Instant::now() + std::time::Duration::from_secs(秒);
    loop {
        // **構えてから見る。**見たあとに構えると、その間の答えを取りこぼす
        let 起きた = 答えの知らせ口().notified();
        tokio::pin!(起きた);
        起きた.as_mut().enable();
        if let Some(結果) = 結果を取る(番号) {
            return 結果;
        }
        if tokio::time::timeout_at(終わり, 起きた).await.is_err() {
            return 結果を取る(番号).unwrap_or(手渡しの返り::まだ);
        }
    }
}

fn 渡せない(訳: impl Into<String>) -> 手渡しの返り {
    手渡しの返り::渡せない { 訳: 訳.into() }
}

/// **机から：渡してよいかを人に聞き、答えを待つ**（`ToDesk::鍵を渡す`）。
pub async fn 頼む(
    app: &AppHandle,
    指定: &str,
    訳: &str,
    頼んだ人: &str,
    待つ秒: u64,
) -> 手渡しの返り {
    if 訳.trim().is_empty() {
        return 渡せない("なぜ渡すか（why）が空です。人が読んで決めるので、必ず書いてください");
    }
    let bridge = app.state::<Bridge>();
    let つながっている: Vec<String> = bridge
        .outbound
        .lock()
        .await
        .keys()
        .filter_map(|k| PublicKey::from_bytes(*k).ok())
        .map(key_to_string)
        .collect();
    let 帳 = 宛先の帳(crate::desk::連絡先の組(), &つながっている);
    let 鍵 = match crate::contacts::呼ぶ相手を決める(&帳, 指定) {
        Ok(鍵) => 鍵,
        Err(訳) => return 渡せない(訳),
    };
    if !つながっている.contains(&鍵) {
        return 渡せない(
            "その相手の機械とは、いま割符でつながっていません（渡せるのは、つながっている相手だけです）",
        );
    }
    let Ok(相手の鍵) = key_from(&鍵) else {
        return 渡せない("相手の鍵が読めません");
    };
    let 相手 = 帳
        .iter()
        .find(|(k, _)| *k == 鍵)
        .map_or_else(|| 短く(&鍵), |(_, 名)| 名.clone());
    let 番号 = 次の番号.fetch_add(1, Ordering::Relaxed);
    // **人が前もって「聞かずに渡す」と決めていれば、聞かずに渡す**（0.1.41）
    if crate::desk::持続の札()
        .iter()
        .any(|(a, _)| a == 聞かずに渡す札)
    {
        記録!(
            "手渡し: 人が前もって許しているので、聞かずに渡します（{相手} へ・頼んだのは {頼んだ人}）"
        );
        let 頼 = 頼み {
            番号,
            相手,
            訳: 訳.trim().to_owned(),
            頼んだ人: 頼んだ人.to_owned(),
            相手の鍵: Some(相手の鍵),
        };
        return 送る(&bridge, &頼).await;
    }
    頼みの棚.lock().expect("毒されていない").push(頼み {
        番号,
        相手: 相手.clone(),
        訳: 訳.trim().to_owned(),
        頼んだ人: 頼んだ人.to_owned(),
        相手の鍵: Some(相手の鍵),
    });
    記録!("手渡し: 人に聞きます（{相手} へ・頼んだのは {頼んだ人}）");
    // **画面へ知らせる。**落ちても頼みは棚に在る（画面が起き上がったら取りに来る）
    if let Err(e) = app.emit(EVENT_HANDOFF, 並べる()) {
        記録!("手渡し: 画面へ知らせられませんでした: {e}");
    }
    crate::notify::承認を頼む(
        app,
        &format!("{相手} へルームキーを渡してよいか、聞かれています"),
    );
    答えを待つ(番号, 待つ秒).await
}

fn key_from(鍵: &str) -> Result<PublicKey, ()> {
    use std::str::FromStr as _;
    PublicKey::from_str(鍵).map_err(|_| ())
}

/// **人が答えた**（画面の帯の［渡す］／［渡さない］）。
///
/// **ここが鍵を出して送る唯一の所である。**エージェントはこの口を呼べない（机には無い）。
#[tauri::command]
pub async fn answer_handoff(
    app: AppHandle,
    bridge: State<'_, Bridge>,
    番号: u64,
    渡す: bool,
    今後も: Option<bool>,
) -> Answer<String> {
    // **［今後は聞かずに渡す］**（0.1.41）。この PC では以後、人に聞かずに渡す
    if 渡す && 今後も.unwrap_or(false) {
        let まで = crate::now_secs().saturating_add(取り消すまでの秒);
        if let Err(e) = crate::desk::人が答えた(聞かずに渡す札, true, Some(まで)) {
            記録!("手渡し: 「今後は聞かずに渡す」を控えられませんでした: {e}");
        }
    }
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
    let _ = app.emit(EVENT_HANDOFF, 並べる());
    let 結果 = if 渡す {
        送る(&bridge, &頼).await
    } else {
        記録!("手渡し: 画面で渡さないと決めました（{} へ）", 頼.相手);
        手渡しの返り::断った
    };
    let 言い方 = match &結果 {
        手渡しの返り::渡した {
            相手, いつまで
        } => {
            format!("{相手} の機械へルームキーを渡しました（{いつまで} まで）")
        }
        手渡しの返り::断った => format!("{} へは渡しませんでした", 頼.相手),
        手渡しの返り::渡せない { 訳 } => format!("渡せませんでした：{訳}"),
        手渡しの返り::まだ => String::new(),
    };
    結果を置く(番号, 結果);
    Ok(言い方)
}

/// 鍵を 1 本出し、**その相手への線にだけ**送る。
async fn 送る(bridge: &Bridge, 頼: &頼み) -> 手渡しの返り {
    let Some(相手の鍵) = 頼.相手の鍵 else {
        return 渡せない("相手の鍵が読めません");
    };
    // **経路の宛先にする会議**（その相手と共にしているもの）。鍵の行き先とは限らない
    let Some(meeting) = crate::相手が居るルーム(&bridge.conferences, 相手の鍵).await
    else {
        return 渡せない("その相手と同じルームに居ません");
    };
    let Some(tx) = bridge
        .outbound
        .lock()
        .await
        .get(&相手の鍵.to_bytes())
        .cloned()
    else {
        return 渡せない("その相手の機械とは、いまつながっていません");
    };
    let 開始 = crate::now_secs();
    let 寿命 = crate::desk::既定の鍵の寿命;
    let 鍵 = match crate::鍵を出す(bridge, 寿命, Some(開始)).await {
        Ok(鍵) => 鍵,
        Err(e) => return 渡せない(e.message),
    };
    let 知らせ = Notice::Key {
        meeting,
        from: bridge.device.public_key(),
        鍵,
    };
    // **組めないものを送らない**（形が悪ければここで止まる）
    if 知らせ.to_intent().is_err() {
        return 渡せない("鍵を運べる形にできませんでした");
    }
    if tx.send(知らせ).await.is_err() {
        return 渡せない("送る途中で線が切れました");
    }
    let いつまで = crate::時刻の言い方(開始.saturating_add(寿命));
    // **鍵を書かない。**誰へ・いつまでだけ
    記録!(
        "手渡し: {} へルームキーを 1 本渡しました（{いつまで} まで）",
        頼.相手
    );
    手渡しの返り::渡した {
        相手: 頼.相手.clone(),
        いつまで,
    }
}

/// **承認を待っている頼み**を並べる（画面が起き上がったときに取りに来る）。
#[tauri::command]
pub fn pending_handoffs() -> Vec<頼み> {
    並べる()
}

/// **相手から鍵が届いた。**入るかは、この PC の人が決める（**D79** と同じ）。
pub fn 受け取った(app: &AppHandle, 鍵: &str, 誰から: &str) {
    記録!("手渡し: {誰から} からルームキーが届きました（人に尋ねます）");
    crate::link::届いた鍵を待たせる(鍵);
    let _ = app.emit(EVENT_KEY_GIFT, (鍵.to_owned(), 誰から.to_owned()));
    crate::notify::承認を頼む(app, &format!("{誰から} からルームキーが届きました"));
    // **エージェントには鍵を渡さない。**届いたことだけを 1 行で見せる
    crate::desk::配る(
        &app.state::<Bridge>(),
        crate::desk::聞いた(
            誰から,
            "（この機械へルームキーを渡しました。入るかは、この PC の人が画面で決めます）",
        ),
    );
}

#[cfg(test)]
mod tests {
    #[test]
    fn 聞かずに渡す札の名は_口が読める形() {
        // 口（warifu mcp）は控えの札の名を「小文字・数字・点」で読む。外れると読めない
        let 名 = super::聞かずに渡す札;
        assert!(!名.is_empty() && !名.starts_with('.') && !名.ends_with('.') && !名.contains(".."));
        assert!(
            名.bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'.')
        );
    }

    use super::*;

    #[test]
    fn 名前を付けていない相手にも_鍵の頭で渡せる() {
        let 連絡先 = vec![("AAAAMMMMKKKK1111".to_owned(), "相手A".to_owned())];
        let 帳 = 宛先の帳(
            連絡先,
            &["AAAAMMMMKKKK1111".to_owned(), "BBBBNNNNLLLL2222".to_owned()],
        );
        assert_eq!(帳.len(), 2, "覚えている相手を二重に足さない");
        assert_eq!(
            crate::contacts::呼ぶ相手を決める(&帳, "相手A").unwrap(),
            "AAAAMMMMKKKK1111"
        );
        assert_eq!(
            crate::contacts::呼ぶ相手を決める(&帳, "BBBBNNNNLLLL").unwrap(),
            "BBBBNNNNLLLL2222"
        );
    }

    #[tokio::test]
    async fn 人が答えなければ_まだを返す() {
        let 番号 = 次の番号.fetch_add(1, Ordering::Relaxed);
        assert_eq!(答えを待つ(番号, 1).await, 手渡しの返り::まだ);
    }

    #[tokio::test]
    async fn 人が答えたら_その場で返る() {
        let 番号 = 次の番号.fetch_add(1, Ordering::Relaxed);
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            結果を置く(番号, 手渡しの返り::断った);
        });
        let 始め = std::time::Instant::now();
        assert_eq!(答えを待つ(番号, 30).await, 手渡しの返り::断った);
        assert!(始め.elapsed().as_secs() < 5, "時間切れまで待った");
    }

    #[tokio::test]
    async fn ほかの頼みの答えは_受け取らない() {
        let 自分 = 次の番号.fetch_add(1, Ordering::Relaxed);
        let ほか = 次の番号.fetch_add(1, Ordering::Relaxed);
        結果を置く(ほか, 手渡しの返り::断った);
        assert_eq!(答えを待つ(自分, 1).await, 手渡しの返り::まだ);
        assert_eq!(結果を取る(ほか), Some(手渡しの返り::断った));
    }
}
