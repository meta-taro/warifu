//! **会話を手元に残す**（D127・0.1.44）。
//!
//! 0.1.43 まで、会話は画面の記憶の中にしか無く、**立ち上げ直す（版の入れ替えを含む）たびに消えていた。**
//! 画面がルームごとの会話をまとめて渡し、ここで上限を当て、**この機械の鍵で封をして** 1 つのファイルへ置く。
//!
//! - 上限は**ルームごとに 30 日または 5,000 行の早いほう**（D127）。画面が何を渡しても、ここで刈る
//! - 封は `warifu-seal`（自分あて）。ファイルを持ち出されても、この機械の鍵が無ければ読めない
//! - **記録（`warifu.log`）には中身を書かない。**数だけを書く
//! - 読めない控えは**上書きしない。**脇へ退けてから空で始める（鍵を取り違えた時に、残した会話を潰さない）

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde_json::{Map, Value};
use warifu_core::{Device, PublicKey};

use crate::{Answer, Bridge, Failure};

/// 1 つのルームに残す行の数（D127）。
pub(crate) const 一部屋の行数: usize = 5_000;
/// 残す日数（D127）。
pub(crate) const 残す日数: u64 = 30;
const 一日のミリ秒: u64 = 24 * 60 * 60 * 1000;

/// 書くのは 1 度に 1 つ。**途中の書き込みどうしが混ざらない**ように
static 書く錠: Mutex<()> = Mutex::new(());

fn 控えの道(置き場: &Path) -> PathBuf {
    置き場.join("chat.seal")
}

/// **上限を当てる。**30 日より古い行を落とし、残りの新しいほうから 5,000 行まで。空になったルームは消す。
///
/// いつの行か分からない行（`時` が無い）は、日数では落とさない（行数では落ちる）。
pub(crate) fn 刈る(会話: Map<String, Value>, 今: u64) -> Map<String, Value> {
    let 境 = 今.saturating_sub(残す日数 * 一日のミリ秒);
    let mut 残す = Map::new();
    for (ルーム, 行たち) in 会話 {
        let Value::Array(行たち) = 行たち else {
            continue;
        };
        let 新しい: Vec<Value> = 行たち
            .into_iter()
            .filter(|行| {
                行.get("時")
                    .and_then(Value::as_u64)
                    .is_none_or(|時| 時 >= 境)
            })
            .collect();
        let 飛ばす = 新しい.len().saturating_sub(一部屋の行数);
        let 残り: Vec<Value> = 新しい.into_iter().skip(飛ばす).collect();
        if !残り.is_empty() {
            残す.insert(ルーム, Value::Array(残り));
        }
    }
    残す
}

/// 行の数（記録に書くため。中身は書かない）。
fn 行の数(会話: &Map<String, Value>) -> usize {
    会話
        .values()
        .filter_map(Value::as_array)
        .map(Vec::len)
        .sum()
}

/// **封をして置く。**先に別の名で書き、書き終えてから入れ替える（途中で落ちても前の控えが残る）。
pub(crate) fn 書く(
    置き場: &Path,
    自分: PublicKey,
    会話: &Map<String, Value>,
) -> Result<(), String> {
    let 中身 = serde_json::to_vec(会話).map_err(|e| format!("会話を形にできません: {e}"))?;
    let 封 = warifu_seal::seal(自分, &中身).map_err(|e| format!("封ができません: {e}"))?;
    std::fs::create_dir_all(置き場).map_err(|e| format!("置き場所を作れません: {e}"))?;
    let 途中 = 置き場.join("chat.seal.tmp");
    自分だけで書く(&途中, &封.to_bytes()).map_err(|e| format!("会話を書けません: {e}"))?;
    std::fs::rename(&途中, 控えの道(置き場)).map_err(|e| format!("会話を置けません: {e}"))
}

#[cfg(unix)]
fn 自分だけで書く(道: &Path, 中身: &[u8]) -> std::io::Result<()> {
    use std::io::Write as _;
    use std::os::unix::fs::{OpenOptionsExt as _, PermissionsExt as _};
    // 中身を入れる前に権限を絞る（逆にすると、絞るまでの一瞬だけ他人に読める）
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(道)?;
    f.set_permissions(std::fs::Permissions::from_mode(0o600))?;
    f.write_all(中身)?;
    f.sync_all()
}

#[cfg(not(unix))]
fn 自分だけで書く(道: &Path, 中身: &[u8]) -> std::io::Result<()> {
    // Windows の ACL は unix の mode とは別物。中身は封をしてあるので、この機械の鍵が無ければ読めない
    std::fs::write(道, 中身)
}

/// **読む。**控えが無ければ空。開けなければ、控えを `chat.seal.unreadable` へ退けて誤りを返す。
pub(crate) fn 読む(置き場: &Path, 自分: &Device) -> Result<Map<String, Value>, String> {
    let 道 = 控えの道(置き場);
    let 中身 = match std::fs::read(&道) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Map::new()),
        Err(e) => return Err(format!("残した会話を読めません: {e}")),
    };
    let 開いた = warifu_seal::Sealed::from_bytes(&中身)
        .ok_or(())
        .and_then(|封| warifu_seal::open(自分, &封).map_err(|_| ()))
        .and_then(|b| serde_json::from_slice::<Map<String, Value>>(&b).map_err(|_| ()));
    match 開いた {
        Ok(会話) => Ok(会話),
        Err(()) => {
            let 退け先 = 置き場.join("chat.seal.unreadable");
            let 退けた = std::fs::rename(&道, &退け先).is_ok();
            Err(if 退けた {
                "残した会話を開けません（この機械の鍵で封をしたものではありません）。chat.seal.unreadable へ退けました".into()
            } else {
                "残した会話を開けません（この機械の鍵で封をしたものではありません）".into()
            })
        }
    }
}

fn 今のミリ秒() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

fn 置き場() -> Result<PathBuf, Failure> {
    warifu_vault::Vault::default_location()
        .map(|v| v.dir().to_path_buf())
        .map_err(|e| Failure {
            message: format!("置き場所が分かりません: {e}"),
            code: None,
        })
}

/// **画面の会話を残す。**画面は変わるたびに（まとめて）全部を渡す。
#[tauri::command]
pub(crate) fn chat_keep(bridge: tauri::State<'_, Bridge>, rooms: Map<String, Value>) -> Answer<()> {
    let 置き場 = 置き場()?;
    let 刈った = 刈る(rooms, 今のミリ秒());
    let _錠 = 書く錠
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    書く(&置き場, bridge.device.public_key(), &刈った).map_err(|message| {
        記録!("会話を残せませんでした: {message}");
        Failure {
            message,
            code: None,
        }
    })
}

/// **残した会話を読む。**画面が立ち上がったときに 1 度だけ呼ぶ。
#[tauri::command]
pub(crate) fn chat_restore(bridge: tauri::State<'_, Bridge>) -> Answer<Map<String, Value>> {
    let 置き場 = 置き場()?;
    let _錠 = 書く錠
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    match 読む(&置き場, &bridge.device) {
        Ok(会話) => {
            let 刈った = 刈る(会話, 今のミリ秒());
            記録!(
                "残した会話を読みました（{} ルーム・{} 行）",
                刈った.len(),
                行の数(&刈った)
            );
            Ok(刈った)
        }
        Err(message) => {
            記録!("{message}");
            Err(Failure {
                message,
                code: Some("chat.unreadable".into()),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn 行(時: u64) -> Value {
        json!({ "who": "A", "body": "x", "mine": false, "時": 時 })
    }

    fn ルーム(会話: Map<String, Value>) -> Vec<String> {
        会話.keys().cloned().collect()
    }

    const 今: u64 = 100 * 一日のミリ秒;

    #[test]
    fn 三十日より古い行は落とし_空になったルームは消す() {
        let mut 会話 = Map::new();
        会話.insert("古い".into(), json!([行(今 - 31 * 一日のミリ秒)]));
        会話.insert(
            "混ざる".into(),
            json!([行(今 - 31 * 一日のミリ秒), 行(今 - 一日のミリ秒)]),
        );
        let 刈った = 刈る(会話, 今);
        assert_eq!(ルーム(刈った.clone()), vec!["混ざる".to_owned()]);
        assert_eq!(刈った["混ざる"].as_array().map(Vec::len), Some(1));
    }

    #[test]
    fn 一部屋_5000_行を超えたら_古いほうから落とす() {
        let 行たち: Vec<Value> = (0..一部屋の行数 as u64 + 3)
            .map(|i| 行(今 - 1000 + i))
            .collect();
        let mut 会話 = Map::new();
        会話.insert("R".into(), Value::Array(行たち));
        let 刈った = 刈る(会話, 今);
        let 残り = 刈った["R"].as_array().cloned().unwrap_or_default();
        assert_eq!(残り.len(), 一部屋の行数);
        // 落ちたのは最初の 3 行
        assert_eq!(残り[0]["時"].as_u64(), Some(今 - 1000 + 3));
    }

    #[test]
    fn いつの行か分からない行は_日数では落とさない() {
        let mut 会話 = Map::new();
        会話.insert(
            "R".into(),
            json!([{ "who": "A", "body": "x", "mine": false }]),
        );
        assert_eq!(刈る(会話, 今)["R"].as_array().map(Vec::len), Some(1));
    }

    fn 試しの置き場(名: &str) -> PathBuf {
        let 道 = std::env::temp_dir().join(format!("warifu-chat-keep-{名}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&道);
        道
    }

    fn 試しの身元() -> Device {
        warifu_core::Seed::generate()
            .expect("乱数")
            .profile("P")
            .device("この端末")
    }

    #[test]
    fn 封をして書いたものを_同じ鍵で読み戻せ_ファイルに平文は無い() {
        let 置き場 = 試しの置き場("往復");
        let 自分 = 試しの身元();
        let mut 会話 = Map::new();
        会話.insert(
            "R".into(),
            json!([{ "who": "A", "body": "ひみつの話", "mine": false }]),
        );
        書く(&置き場, 自分.public_key(), &会話).expect("書ける");
        let 生 = std::fs::read(控えの道(&置き場)).expect("ある");
        assert!(!String::from_utf8_lossy(&生).contains("ひみつ"));
        assert_eq!(読む(&置き場, &自分).expect("読める"), 会話);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            let mode = std::fs::metadata(控えの道(&置き場))
                .expect("ある")
                .permissions()
                .mode();
            assert_eq!(mode & 0o077, 0, "他人に読める置き方になっている");
        }
        let _ = std::fs::remove_dir_all(&置き場);
    }

    #[test]
    fn 控えが無ければ空() {
        let 置き場 = 試しの置き場("無い");
        assert_eq!(読む(&置き場, &試しの身元()).expect("空"), Map::new());
    }

    #[test]
    fn 別の鍵の控えは上書きせず_脇へ退ける() {
        let 置き場 = 試しの置き場("別の鍵");
        let mut 会話 = Map::new();
        会話.insert("R".into(), json!([]));
        書く(&置き場, 試しの身元().public_key(), &会話).expect("書ける");
        assert!(読む(&置き場, &試しの身元()).is_err());
        assert!(!控えの道(&置き場).exists());
        assert!(置き場.join("chat.seal.unreadable").exists());
        let _ = std::fs::remove_dir_all(&置き場);
    }
}
