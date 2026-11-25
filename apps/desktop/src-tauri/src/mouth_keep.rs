//! **口（`warifu mcp`）の読みを手元に残す**（0.1.62）。
//!
//! 0.1.61 で、切れた口の読みを名乗りごとに控え、同じ名乗りの口がつなぎ直したら継ぐようにした。
//! ところが控えは**記憶の中にしか無く**、本体（画面）ごと立ち上げ直すと消えた。
//! 版の入れ替えのあと、つなぎ直した口の読みは「つながる前」に戻り、読んでいない分が 99 件と出た（11-25）。
//!
//! - 残すのは、切れた口の控えと、**いまつながっている口の読み**（本体が先に落ちても次の起動で継げるように）
//! - 立ち上げたとき、つながっていた口は「本体が止まって切れた」として戻す。切れた時刻は最後に残した時刻
//! - **7 日より古い控えは捨てる**（同じ名乗りの口が二度と来ないこともある）
//! - 中身は読んだ番号と時刻と名乗りだけ。封はせず、この機械の利用者だけが読める権限で置く

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// 古すぎる控えを捨てるまでの秒（7 日）。
const 覚える秒: u64 = 7 * 24 * 60 * 60;

/// **切れた口の控え**（0.1.61・0.1.62 で手元に残す）。同じ名乗りの口がつなぎ直したら、ここから読みを継ぐ。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct 切れた口 {
    /// 切れたときに読んでいた所。
    pub 読んだ: u64,
    /// 切れたときの「最後に読んだ」。
    pub 読んだ時: Option<String>,
    /// 切れた時刻（秒）。
    pub 切れた秒: u64,
    /// 切れたときの、いちばん新しい発言の番号。これより後が「切れていた間」。
    pub 切れた時の番号: u64,
    /// **本体（画面）が止まって切れたか**（0.1.62）。口が切れたのではなく、本体ごと止まっていた。
    #[serde(default)]
    pub 本体が止まった: bool,
}

/// 手元に置く形。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct 残す形 {
    /// 書いた時刻（秒）。つながっていた口は、本体がこの頃に止まったとみる。
    pub 書いた秒: u64,
    /// 切れた口（名乗りごと）。
    #[serde(default)]
    pub 切れた: BTreeMap<String, 切れた口>,
    /// 書いたときにつながっていた口（名乗りごと）。`切れた秒` は使わない。
    #[serde(default)]
    pub 居た: BTreeMap<String, 切れた口>,
}

fn 控えの道(置き場: &Path) -> PathBuf {
    置き場.join("desk-mouths.json")
}

/// **立ち上げたときに戻す控え。**つながっていた口は「本体が止まって切れた」に直し、古いものは捨てる。
///
/// 同じ名乗りが両方にあれば、つながっていた方（新しい）を採る。
pub(crate) fn 起動で戻す(
    残した: 残す形, いまの秒: u64
) -> BTreeMap<String, 切れた口> {
    let mut 棚 = 残した.切れた;
    for (名, mut 口) in 残した.居た {
        口.切れた秒 = 残した.書いた秒;
        口.本体が止まった = true;
        棚.insert(名, 口);
    }
    棚.retain(|_, 口| いまの秒.saturating_sub(口.切れた秒) <= 覚える秒);
    棚
}

/// **置く。**先に別の名で書き、書き終えてから入れ替える。
pub(crate) fn 書く(置き場: &Path, 中身: &残す形) -> Result<(), String> {
    let 字 = serde_json::to_vec(中身).map_err(|e| format!("口の読みを形にできません: {e}"))?;
    std::fs::create_dir_all(置き場).map_err(|e| format!("置き場所を作れません: {e}"))?;
    let 途中 = 置き場.join("desk-mouths.json.tmp");
    crate::chat_keep::自分だけで書く(&途中, &字)
        .map_err(|e| format!("口の読みを書けません: {e}"))?;
    std::fs::rename(&途中, 控えの道(置き場)).map_err(|e| format!("口の読みを置けません: {e}"))
}

/// **読む。**無ければ空。読めなければ誤りを返す（上書きされる前に記録へ残せるように）。
pub(crate) fn 読む(置き場: &Path) -> Result<残す形, String> {
    match std::fs::read(控えの道(置き場)) {
        Ok(b) => serde_json::from_slice(&b).map_err(|e| format!("口の読みの控えを読めません: {e}")),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(残す形::default()),
        Err(e) => Err(format!("口の読みの控えを開けません: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn 試しの置き場(名: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("warifu-mouths-{名}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    fn 口(読んだ: u64, 切れた秒: u64) -> 切れた口 {
        切れた口 {
            読んだ,
            読んだ時: Some("11-25 15:00".to_owned()),
            切れた秒,
            切れた時の番号: 読んだ + 3,
            本体が止まった: false,
        }
    }

    #[test]
    fn 書いて読み戻すと同じ() {
        let d = 試しの置き場("roundtrip");
        let mut 中身 = 残す形 {
            書いた秒: 1_000,
            ..残す形::default()
        };
        中身.切れた.insert("席 A".to_owned(), 口(10, 900));
        中身.居た.insert("席 B".to_owned(), 口(20, 0));
        書く(&d, &中身).expect("書ける");
        assert_eq!(読む(&d).expect("読める"), 中身);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn 無ければ空で始める() {
        let d = 試しの置き場("none");
        assert_eq!(読む(&d).expect("読める"), 残す形::default());
    }

    #[test]
    fn つながっていた口は_本体が止まって切れたとして戻る() {
        let mut 中身 = 残す形 {
            書いた秒: 5_000,
            ..残す形::default()
        };
        中身.居た.insert("席 B".to_owned(), 口(20, 0));
        let 棚 = 起動で戻す(中身, 5_600);
        let 戻った = &棚["席 B"];
        assert_eq!(戻った.読んだ, 20);
        assert_eq!(戻った.切れた秒, 5_000);
        assert!(戻った.本体が止まった);
    }

    #[test]
    fn 同じ名乗りなら_つながっていた方を採る() {
        let mut 中身 = 残す形 {
            書いた秒: 5_000,
            ..残す形::default()
        };
        中身.切れた.insert("席 A".to_owned(), 口(10, 900));
        中身.居た.insert("席 A".to_owned(), 口(30, 0));
        assert_eq!(起動で戻す(中身, 5_100)["席 A"].読んだ, 30);
    }

    #[test]
    fn 七日より古い控えは捨てる() {
        let mut 中身 = 残す形 {
            書いた秒: 覚える秒 + 2_000,
            ..残す形::default()
        };
        中身.切れた.insert("古い".to_owned(), 口(1, 100));
        中身
            .切れた
            .insert("新しい".to_owned(), 口(2, 覚える秒 + 1_000));
        let 棚 = 起動で戻す(中身, 覚える秒 + 2_000);
        assert!(!棚.contains_key("古い"));
        assert!(棚.contains_key("新しい"));
    }

    #[test]
    fn 前の版の控え_本体の欄が無くても読める() {
        let 字 = r#"{"書いた秒":1,"切れた":{"席":{"読んだ":3,"読んだ時":null,"切れた秒":1,"切れた時の番号":4}}}"#;
        let 中身: 残す形 = serde_json::from_str(字).expect("読める");
        assert!(!中身.切れた["席"].本体が止まった);
    }
}
