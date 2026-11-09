//! **机の直近の発言を手元に残す**（0.1.45）。
//!
//! 机は、繋ぎ直したエージェントへ「読んだところより後」を渡すために直近の発言を持つ（`issues/019`）。
//! これが**記憶の中にしか無かった**ので、画面を立ち上げ直す（版の入れ替えを含む）と空になり、
//! エージェントがまだ読んでいなかった発言は、繋ぎ直しても渡らずに消えていた。
//!
//! - 封は `warifu-seal`（自分あて）。会話の控え（`chat.seal`）と同じ置き方
//! - **記録には中身を書かない。**数だけ
//! - 読めない控えは**上書きしない。**脇へ退けてから空で始める

use std::path::{Path, PathBuf};

use warifu_core::{Device, PublicKey};
use warifu_desk::FromDesk;

/// 番号つきの直近の発言。
pub(crate) type 履歴 = Vec<(u64, FromDesk)>;

fn 控えの道(置き場: &Path) -> PathBuf {
    置き場.join("desk-recent.seal")
}

/// **封をして置く。**先に別の名で書き、書き終えてから入れ替える。
pub(crate) fn 書く(置き場: &Path, 自分: PublicKey, 行たち: &[(u64, FromDesk)]) -> Result<(), String> {
    let 中身 = serde_json::to_vec(行たち).map_err(|e| format!("直近の発言を形にできません: {e}"))?;
    let 封 = warifu_seal::seal(自分, &中身).map_err(|e| format!("封ができません: {e}"))?;
    std::fs::create_dir_all(置き場).map_err(|e| format!("置き場所を作れません: {e}"))?;
    let 途中 = 置き場.join("desk-recent.seal.tmp");
    crate::chat_keep::自分だけで書く(&途中, &封.to_bytes())
        .map_err(|e| format!("直近の発言を書けません: {e}"))?;
    std::fs::rename(&途中, 控えの道(置き場)).map_err(|e| format!("直近の発言を置けません: {e}"))
}

/// **読む。**控えが無ければ空。開けなければ `desk-recent.seal.unreadable` へ退けて誤りを返す。
pub(crate) fn 読む(置き場: &Path, 自分: &Device) -> Result<履歴, String> {
    let 道 = 控えの道(置き場);
    let 中身 = match std::fs::read(&道) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(format!("直近の発言を読めません: {e}")),
    };
    let 開いた = warifu_seal::Sealed::from_bytes(&中身)
        .ok_or(())
        .and_then(|封| warifu_seal::open(自分, &封).map_err(|_| ()))
        .and_then(|b| serde_json::from_slice::<履歴>(&b).map_err(|_| ()));
    開いた.map_err(|()| {
        let 退けた = std::fs::rename(&道, 置き場.join("desk-recent.seal.unreadable")).is_ok();
        if 退けた {
            "直近の発言の控えを開けません。desk-recent.seal.unreadable へ退けました".into()
        } else {
            "直近の発言の控えを開けません".into()
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use warifu_core::Seed;

    fn 試しの置き場(名: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("warifu-recent-{名}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    fn 身元(n: u8) -> Device {
        Seed::from_bytes([n; 32]).profile("Personal").device("PC")
    }

    fn 行(id: u64, 本文: &str) -> (u64, FromDesk) {
        (
            id,
            FromDesk::Heard {
                id,
                from: "相手".to_owned(),
                body: 本文.to_owned(),
                at: "11-09 12:00".to_owned(),
                room: None,
                任された: false,
            },
        )
    }

    #[test]
    fn 封をして書いたものを_同じ鍵で読み戻せ_ファイルに平文は無い() {
        let 置き場 = 試しの置き場("roundtrip");
        let 私 = 身元(1);
        let 行たち = vec![行(41, "読まれる前の発言"), 行(42, "もう一つ")];
        書く(&置き場, 私.public_key(), &行たち).unwrap();
        assert_eq!(読む(&置き場, &私).unwrap(), 行たち);
        let 生 = std::fs::read(控えの道(&置き場)).unwrap();
        assert!(!String::from_utf8_lossy(&生).contains("読まれる前の発言"));
    }

    #[test]
    fn 控えが無ければ空() {
        assert!(読む(&試しの置き場("none"), &身元(1)).unwrap().is_empty());
    }

    #[test]
    fn 別の鍵の控えは上書きせず_脇へ退ける() {
        let 置き場 = 試しの置き場("other");
        書く(&置き場, 身元(1).public_key(), &[行(1, "x")]).unwrap();
        assert!(読む(&置き場, &身元(2)).is_err());
        assert!(置き場.join("desk-recent.seal.unreadable").exists());
        assert!(!控えの道(&置き場).exists());
    }
}
