//! **自己点検 —— 画面が見せているものと、本体が知っていることを見比べる**（計画 1-D・spec 04 §2・0.1.41）。
//!
//! 10-22、届いた文字が別のルームの欄に積まれていた。10-23、網を越えた相手が見出しの人数にも届く先にも
//! 出なかった。**人が見てすぐ分かるおかしさを、エージェントは見られなかった。**
//! 本体が自分で見比べて、食い違いを机（エージェント）と記録へ出す。
//!
//! ここは純粋な見比べだけ。画面の写しは `note_view` で画面が送り、本体の側は `ルームの並び` から作る。

use std::collections::BTreeSet;

/// 画面が見せている 1 部屋。
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct 画面の部屋 {
    /// 部屋の id（全桁）。
    pub id: String,
    /// 見出しに出している人数（自分を含む）。見出しを出していない部屋は `None`。
    #[serde(default)]
    pub 人数: Option<usize>,
    /// 届く先に出している相手の鍵（全桁・自分は入らない）。出していない部屋は `None`。
    #[serde(default)]
    pub 届く先: Option<Vec<String>>,
    /// 「あなたが建てた」と出しているか。
    #[serde(default)]
    pub 建てたと出している: bool,
}

/// 画面が見せているもの（`note_view`）。
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct 画面の写し {
    /// 見ている部屋の id。無ければ `None`。
    #[serde(default)]
    pub 見ている: Option<String>,
    /// 左の一覧に出している部屋。
    #[serde(default)]
    pub 部屋ごと: Vec<画面の部屋>,
}

/// 本体が知っている 1 部屋。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct 本体の部屋 {
    pub id: String,
    /// 居る人の鍵（全桁・自分を含む）。
    pub 面々: Vec<String>,
    pub 主催か: bool,
}

/// 本体が知っているもの。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct 本体の写し {
    pub 自分: String,
    pub 見ている: Option<String>,
    pub 部屋ごと: Vec<本体の部屋>,
}

fn 頭(id: &str) -> String {
    format!("{}…", id.chars().take(12).collect::<String>())
}

/// **見比べる**（S04-1〜S04-5）。合っていれば空。
///
/// 画面が「まだ出していない」もの（`None`）は食い違いにしない —— 分からないものを倒さない（DESIGN §2）。
#[must_use]
pub fn 見比べる(本体: &本体の写し, 画面: &画面の写し) -> Vec<String> {
    let mut 出 = Vec::new();
    // S04-3 見ている部屋
    if 画面.見ている != 本体.見ている {
        出.push(format!(
            "画面が見ている部屋（{}）と本体が見ている部屋（{}）が違う",
            画面.見ている.as_deref().map_or("なし".to_owned(), 頭),
            本体.見ている.as_deref().map_or("なし".to_owned(), 頭)
        ));
    }
    // S04-5 部屋の一覧
    let 画面の集合: BTreeSet<&str> = 画面.部屋ごと.iter().map(|r| r.id.as_str()).collect();
    let 本体の集合: BTreeSet<&str> = 本体.部屋ごと.iter().map(|r| r.id.as_str()).collect();
    for id in 画面の集合.difference(&本体の集合) {
        出.push(format!(
            "画面にだけ部屋 {} がある（本体の棚に無い）",
            頭(id)
        ));
    }
    for id in 本体の集合.difference(&画面の集合) {
        出.push(format!(
            "本体にだけ部屋 {} がある（画面の一覧に無い）",
            頭(id)
        ));
    }
    for 画 in &画面.部屋ごと {
        let Some(本) = 本体.部屋ごと.iter().find(|r| r.id == 画.id) else {
            continue;
        };
        // S04-2 見出しの人数 ＝ 相手の数 ＋ 1
        if let Some(人数) = 画.人数
            && 人数 != 本.面々.len()
        {
            出.push(format!(
                "部屋 {} の見出しの人数（{人数}）が本体の名簿（{} 人）と違う",
                頭(&画.id),
                本.面々.len()
            ));
        }
        // S04-1 届く先 ＝ 名簿から自分を除いたもの
        if let Some(届く先) = &画.届く先 {
            let 画の: BTreeSet<&str> = 届く先.iter().map(String::as_str).collect();
            let 本の: BTreeSet<&str> = 本
                .面々
                .iter()
                .filter(|k| **k != 本体.自分)
                .map(String::as_str)
                .collect();
            if 画の != 本の {
                let 足りない: Vec<String> = 本の.difference(&画の).map(|k| 頭(k)).collect();
                let 余計: Vec<String> = 画の.difference(&本の).map(|k| 頭(k)).collect();
                出.push(format!(
                    "部屋 {} の届く先が名簿と違う（画面に無い: {}／名簿に無い: {}）",
                    頭(&画.id),
                    if 足りない.is_empty() {
                        "なし".to_owned()
                    } else {
                        足りない.join("・")
                    },
                    if 余計.is_empty() {
                        "なし".to_owned()
                    } else {
                        余計.join("・")
                    }
                ));
            }
        }
        // S04-4 「あなたが建てた」 ⇔ 主催
        if 画.建てたと出している != 本.主催か {
            出.push(format!(
                "部屋 {} の題字（{}）と本体の主催（{}）が違う",
                頭(&画.id),
                if 画.建てたと出している {
                    "あなたが建てた"
                } else {
                    "建てていない"
                },
                if 本.主催か { "自分" } else { "相手" }
            ));
        }
    }
    出
}

#[cfg(test)]
mod 試験 {
    use super::*;

    fn 本体() -> 本体の写し {
        本体の写し {
            自分: "ME".to_owned(),
            見ている: Some("ROOMA".to_owned()),
            部屋ごと: vec![本体の部屋 {
                id: "ROOMA".to_owned(),
                面々: vec!["ME".to_owned(), "PEER1".to_owned(), "PEER2".to_owned()],
                主催か: true,
            }],
        }
    }

    fn 合っている画面() -> 画面の写し {
        画面の写し {
            見ている: Some("ROOMA".to_owned()),
            部屋ごと: vec![画面の部屋 {
                id: "ROOMA".to_owned(),
                人数: Some(3),
                届く先: Some(vec!["PEER2".to_owned(), "PEER1".to_owned()]),
                建てたと出している: true,
            }],
        }
    }

    #[test]
    fn 合っていれば空() {
        assert_eq!(見比べる(&本体(), &合っている画面()), Vec::<String>::new());
    }

    #[test]
    fn 見ている部屋が違えば言う() {
        let mut 画 = 合っている画面();
        画.見ている = None;
        let 出 = 見比べる(&本体(), &画);
        assert_eq!(出.len(), 1);
        assert!(出[0].contains("見ている部屋"));
    }

    #[test]
    fn 見出しの人数と届く先が名簿と違えば_どちらも言う() {
        let mut 画 = 合っている画面();
        画.部屋ごと[0].人数 = Some(2);
        画.部屋ごと[0].届く先 = Some(vec!["PEER1".to_owned()]);
        let 出 = 見比べる(&本体(), &画);
        assert_eq!(出.len(), 2, "{出:?}");
        assert!(出[0].contains("見出しの人数（2）") && 出[0].contains("3 人"));
        assert!(出[1].contains("画面に無い: PEER2…"));
    }

    #[test]
    fn まだ出していないものは食い違いにしない() {
        let mut 画 = 合っている画面();
        画.部屋ごと[0].人数 = None;
        画.部屋ごと[0].届く先 = None;
        assert_eq!(見比べる(&本体(), &画), Vec::<String>::new());
    }

    #[test]
    fn 部屋の一覧の増減と_題字の主催を言う() {
        let mut 画 = 合っている画面();
        画.部屋ごと[0].建てたと出している = false;
        画.部屋ごと.push(画面の部屋 {
            id: "GHOST".to_owned(),
            人数: None,
            届く先: None,
            建てたと出している: false,
        });
        let 出 = 見比べる(&本体(), &画);
        assert!(出.iter().any(|s| s.contains("画面にだけ部屋 GHOST…")));
        assert!(
            出.iter()
                .any(|s| s.contains("題字（建てていない）と本体の主催（自分）"))
        );
    }
}
