//! **`@名前` の相手が、流す部屋に居るか**（11-11）。
//!
//! 席が返事を、相手の居ない部屋へ流していた。`room` は渡していたが、渡した部屋を間違えた
//! （直前に使った部屋から推して埋めた）。部屋に居ない相手宛ての発言は届かず、
//! 関係の無い席へ流れる。**呼んだ相手がその部屋に居ないと分かるなら、流さずに断る。**
//!
//! # 分かる範囲だけで断る
//!
//! 部屋の名簿は**機械の鍵**しか持たない。席の名（`PdM（ドスパラ）`）と鍵を結ぶのは、
//! その席の発言が `鍵の頭…（席の名）` の形で届いたことだけである。
//! **一度も発言を見ていない名は判断できないので、通す**（止めると、正しい呼びまで止まる）。
//! この機械の席は、どの部屋にも居るものとして扱う（同じ画面を通って流れる）。

use warifu_desk::{全員の呼び, 写しか, 宛てられたか};

/// 届いた発言の差出人（`鍵の頭…（席の名）`）を、鍵の頭と席の名に分ける。人の発言など、形が違えば `None`。
#[must_use]
pub fn 席の名札を分ける(差出人: &str) -> Option<(&str, &str)> {
    let 位置 = 差出人.find('（')?;
    if 位置 == 0 || !差出人.ends_with('）') {
        return None;
    }
    let 頭 = 差出人[..位置].trim_end_matches('…');
    let 札 = &差出人[位置 + '（'.len_utf8()..差出人.len() - '）'.len_utf8()];
    if 頭.is_empty() || 札.trim().is_empty() {
        return None;
    }
    Some((頭, 札))
}

/// 部屋ごとの面々（部屋の id の全文と、居る機械の鍵の全文）。
pub type 部屋の面々 = (String, Vec<String>);

/// **呼んだ相手が流す部屋に居なければ、断る文を返す。**居る・判断できないなら `None`。
///
/// - `名と頭`：見たことのある席の名と、その鍵の頭（同じ名が 2 台にあれば 2 行）
/// - `面々`：この機械が居る部屋すべて
/// - `部屋`：流す部屋の id（全文）
/// - `この機械の頭`：この機械の鍵の頭（この機械の席は、どの部屋にも居るものとする）
#[must_use]
pub fn 居ない宛て先(
    本文: &str,
    名と頭: &[(String, String)],
    面々: &[部屋の面々],
    部屋: &str,
    この機械の頭: &str,
) -> Option<String> {
    let 居る = |r: &部屋の面々, 頭: &str| r.1.iter().any(|k| k.starts_with(頭));
    let この部屋 = 面々.iter().find(|r| r.0 == 部屋)?;
    // **`@all` は誰にも当たるので、名を見る前に除く**（全員への呼びで、居ない席の名まで当たらないように）
    let 本文 = 全員の呼びを除く(本文);
    let 本文 = 本文.as_str();
    let mut 見た名: Vec<&str> = Vec::new();
    for (名, _) in 名と頭 {
        if 見た名.contains(&名.as_str()) {
            continue;
        }
        見た名.push(名);
        let 呼んだ = [名.clone()];
        if !宛てられたか(本文, &呼んだ) && !写しか(本文, &呼んだ) {
            continue;
        }
        let 頭たち: Vec<&str> = 名と頭
            .iter()
            .filter(|(n, _)| n == 名)
            .map(|(_, h)| h.as_str())
            .collect();
        if 頭たち
            .iter()
            .any(|h| !h.is_empty() && (この機械の頭.starts_with(h) || 居る(この部屋, h)))
        {
            continue;
        }
        let 居る部屋: Vec<String> = 面々
            .iter()
            .filter(|r| 頭たち.iter().any(|h| 居る(r, h)))
            .map(|r| crate::短く(&r.0))
            .collect();
        let 先 = if 居る部屋.is_empty() {
            "この機械が居る部屋のどこにも居ません".to_owned()
        } else {
            format!("居る部屋: {}", 居る部屋.join("・"))
        };
        return Some(format!(
            "{名} は部屋 {} に居ないので、流しませんでした（{先}）。room を確かめて送り直してください",
            crate::短く(部屋)
        ));
    }
    None
}

/// `@all` / `@全員`（全角の `＠` も・大文字小文字を問わない）を本文から除く。
fn 全員の呼びを除く(本文: &str) -> String {
    let mut 残り = 本文.to_owned();
    for 印 in ['@', '＠'] {
        for 名 in 全員の呼び {
            let 呼び = format!("{印}{名}");
            while let Some(i) = 残り.to_ascii_lowercase().find(&呼び) {
                残り.replace_range(i..i + 呼び.len(), "");
            }
        }
    }
    残り
}

#[cfg(test)]
mod tests {
    use super::*;

    const ここ: &str = "NBIW3PA2TUQ52DTJNVXSQOL3G4";
    const あちら: &str = "4TOOFXPUM2QSAAAAAAAAAAAAAA";

    fn 面々() -> Vec<部屋の面々> {
        vec![
            (ここ.to_owned(), vec!["TLUTIE3HQGUDRKEZFULL".to_owned()]),
            (
                あちら.to_owned(),
                vec!["TLUTIE3HQGUDRKEZFULL".to_owned(), "WORKSKEY1234FULL".to_owned()],
            ),
        ]
    }

    fn 名と頭() -> Vec<(String, String)> {
        vec![
            ("WORKS（nighto 担当）".to_owned(), "WORKSKEY1234".to_owned()),
            ("PdM（ドスパラ）".to_owned(), "TLUTIE3HQGUD".to_owned()),
            ("割符（mac mini）".to_owned(), "MACMINIKEY12".to_owned()),
        ]
    }

    #[test]
    fn 差出人を_鍵の頭と席の名に分ける() {
        assert_eq!(
            席の名札を分ける("TLUTIE3HQGUD…（PdM（ドスパラ））"),
            Some(("TLUTIE3HQGUD", "PdM（ドスパラ）"))
        );
        assert_eq!(席の名札を分ける("ZenBook"), None, "人の発言");
        assert_eq!(席の名札を分ける("（名だけ）"), None);
    }

    #[test]
    fn 呼んだ相手が部屋に居なければ_居る部屋を添えて断る() {
        let 断り = 居ない宛て先(
            "@WORKS（nighto 担当） #1776 をレビューの OK として",
            &名と頭(),
            &面々(),
            ここ,
            "MACMINIKEY12",
        )
        .expect("断る");
        assert!(断り.contains("WORKS（nighto 担当）"), "{断り}");
        assert!(断り.contains("NBIW3PA2TUQ5…"), "{断り}");
        assert!(断り.contains("居る部屋: 4TOOFXPUM2QS…"), "{断り}");
    }

    #[test]
    fn 写しで呼んでも_居なければ断る() {
        assert!(
            居ない宛て先("cc@WORKS（nighto 担当） 参考まで", &名と頭(), &面々(), ここ, "MACMINIKEY12").is_some()
        );
    }

    #[test]
    fn 部屋に居るなら通す() {
        assert_eq!(
            居ない宛て先("@WORKS（nighto 担当） どうぞ", &名と頭(), &面々(), あちら, "MACMINIKEY12"),
            None
        );
        assert_eq!(
            居ない宛て先("@PdM（ドスパラ） どうぞ", &名と頭(), &面々(), ここ, "MACMINIKEY12"),
            None
        );
    }

    #[test]
    fn 見たことの無い名と_この機械の席と_全員は_止めない() {
        let 通す = |本文: &str| 居ない宛て先(本文, &名と頭(), &面々(), ここ, "MACMINIKEY12");
        assert_eq!(通す("@知らない席 こんにちは"), None, "判断できないので通す");
        assert_eq!(通す("@割符（mac mini） 自分の機械の席"), None);
        assert_eq!(通す("@all みなさん"), None);
        assert!(通す("@ALL と @WORKS（nighto 担当）へ").is_some(), "全員と一緒でも、名指しは見る");
        assert_eq!(通す("`@WORKS（nighto 担当）` と書くと呼べます"), None, "コードの中は呼びではない");
    }

    #[test]
    fn どの部屋にも居なければ_そう言う() {
        let 名 = vec![("遠い席".to_owned(), "FARAWAYKEY12".to_owned())];
        let 断り = 居ない宛て先("@遠い席 聞こえますか", &名, &面々(), ここ, "MACMINIKEY12").unwrap();
        assert!(断り.contains("どこにも居ません"), "{断り}");
    }
}
