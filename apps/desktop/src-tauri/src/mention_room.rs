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
    if 人の行か(差出人) {
        return None;
    }
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

/// **返信したら、返信先の書き手を呼ぶ**（0.1.52）。返信先の発言の差出人（`Heard` の `from`）から呼ぶ名を引き、
/// 本文がまだ呼んでいなければ頭に `@名 ` を足す。
///
/// 返信（`reply_to`）で返しても、返信先の席は `@` で呼ばれていないので起きなかった。
/// **呼べるのはエージェントだけ**（相手の機械の `鍵の頭…（席の名）`・この機械の席の名札）。
/// 人の発言は差出人が鍵なので、相手の機械がその人への呼びと読める名を、ここでは決められない。
/// 自分（`自分の名札`）への返信と、`@all`・写し（`cc@`）で既に呼んでいる本文には足さない。
#[must_use]
pub fn 返信先の書き手を呼ぶ(
    本文: &str, 差出人: &str, 自分の名札: &str
) -> String {
    let Some(名) = 返信先の書き手の名(差出人) else {
        return 本文.to_owned();
    };
    if 名 == 自分の名札 {
        return 本文.to_owned();
    }
    let 名たち = warifu_desk::呼べる名に広げる(&[名.to_owned()]);
    if 宛てられたか(本文, &名たち) || 写しか(本文, &名たち) {
        return 本文.to_owned();
    }
    format!("@{名} {本文}")
}

/// 差出人から、`@` で呼べる名。相手の機械の席なら席の名、この機械の席なら名札そのもの。人・知らせは `None`。
fn 返信先の書き手の名(差出人: &str) -> Option<&str> {
    // 人の発言（`名（鍵の頭…）`）は呼ばない。人の名は `@` で呼べる名ではない
    if 人の行か(差出人) {
        return None;
    }
    if let Some((頭, 札)) = 席の名札を分ける(差出人)
        && 差出人.contains('…')
        && 鍵の字だけか(頭)
    {
        return Some(札);
    }
    // 鍵（人の発言）・この機械の知らせ（`割符`）・鍵の頭だけの差出人は呼ばない
    if 差出人 == "割符" || 鍵の字だけか(差出人.trim_end_matches('…')) || 差出人.trim().is_empty()
    {
        return None;
    }
    Some(差出人)
}

/// **人の発言の差出人か**（`名（鍵の頭…）`・11-20 `claimed`）。席の発言は `鍵の頭…（札）` で、括弧の中が鍵ではない。
pub fn 人の行か(差出人: &str) -> bool {
    // **最後の括弧を見る**（0.1.58）。人の名に括弧が入る（`MMJ（galleria）（鍵の頭…）`）と、
    // 最初の括弧では鍵に当たらず、人の発言を席と読んで `@MMJ（galleria）（鍵…）` を本文の頭に足し、
    // 「呼んだ相手が部屋に居ない」と断って返信できなかった
    let Some(位置) = 差出人.rfind('（') else {
        return false;
    };
    if 位置 == 0 || !差出人.ends_with('）') {
        return false;
    }
    let 中 = &差出人[位置 + '（'.len_utf8()..差出人.len() - '）'.len_utf8()];
    中.strip_suffix('…').is_some_and(鍵の字だけか)
}

/// 公開鍵の字（base32 の大文字と 2〜7）だけか。
fn 鍵の字だけか(字: &str) -> bool {
    !字.is_empty()
        && 字
            .chars()
            .all(|c| c.is_ascii_uppercase() || ('2'..='7').contains(&c))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 人の発言の差出人は_席として扱わず_返信でも呼ばない() {
        let 人 = "NOTE-PC の中の人（ABCDEFGHJKMN…）";
        assert!(人の行か(人));
        assert_eq!(席の名札を分ける(人), None);
        assert_eq!(返信先の書き手を呼ぶ("はい", 人, "割符（mac mini）"), "はい");
        // 席の発言は今までどおり
        assert!(!人の行か("ABCDEFGHJKMN…（docs）"));
        assert_eq!(
            席の名札を分ける("ABCDEFGHJKMN…（docs）"),
            Some(("ABCDEFGHJKMN", "docs"))
        );
    }

    #[test]
    fn 名に括弧の入った人の発言も人として扱い_返信で呼ばない() {
        let 人 = "MMJ（galleria）（BCDTEYTS4GQ2…）";
        assert!(人の行か(人));
        assert_eq!(席の名札を分ける(人), None);
        assert_eq!(返信先の書き手を呼ぶ("はい", 人, "割符（mac mini）"), "はい");
        // 札に括弧の入った席は、今までどおり席
        let 席 = "TLUTIE3HQGUD…（PdM（ドスパラ））";
        assert!(!人の行か(席));
        assert_eq!(席の名札を分ける(席), Some(("TLUTIE3HQGUD", "PdM（ドスパラ）")));
    }

    const ここ: &str = "NBIW3PA2TUQ52DTJNVXSQOL3G4";
    const あちら: &str = "4TOOFXPUM2QSAAAAAAAAAAAAAA";

    fn 面々() -> Vec<部屋の面々> {
        vec![
            (ここ.to_owned(), vec!["TLUTIE3HQGUDRKEZFULL".to_owned()]),
            (
                あちら.to_owned(),
                vec![
                    "TLUTIE3HQGUDRKEZFULL".to_owned(),
                    "WORKSKEY1234FULL".to_owned(),
                ],
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
        assert_eq!(席の名札を分ける("NOTE-PC"), None, "人の発言");
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
            居ない宛て先(
                "cc@WORKS（nighto 担当） 参考まで",
                &名と頭(),
                &面々(),
                ここ,
                "MACMINIKEY12"
            )
            .is_some()
        );
    }

    #[test]
    fn 部屋に居るなら通す() {
        assert_eq!(
            居ない宛て先(
                "@WORKS（nighto 担当） どうぞ",
                &名と頭(),
                &面々(),
                あちら,
                "MACMINIKEY12"
            ),
            None
        );
        assert_eq!(
            居ない宛て先(
                "@PdM（ドスパラ） どうぞ",
                &名と頭(),
                &面々(),
                ここ,
                "MACMINIKEY12"
            ),
            None
        );
    }

    #[test]
    fn 見たことの無い名と_この機械の席と_全員は_止めない() {
        let 通す = |本文: &str| 居ない宛て先(本文, &名と頭(), &面々(), ここ, "MACMINIKEY12");
        assert_eq!(通す("@知らない席 こんにちは"), None, "判断できないので通す");
        assert_eq!(通す("@割符（mac mini） 自分の機械の席"), None);
        assert_eq!(通す("@all みなさん"), None);
        assert!(
            通す("@ALL と @WORKS（nighto 担当）へ").is_some(),
            "全員と一緒でも、名指しは見る"
        );
        assert_eq!(
            通す("`@WORKS（nighto 担当）` と書くと呼べます"),
            None,
            "コードの中は呼びではない"
        );
    }

    #[test]
    fn どの部屋にも居なければ_そう言う() {
        let 名 = vec![("遠い席".to_owned(), "FARAWAYKEY12".to_owned())];
        let 断り =
            居ない宛て先("@遠い席 聞こえますか", &名, &面々(), ここ, "MACMINIKEY12").unwrap();
        assert!(断り.contains("どこにも居ません"), "{断り}");
    }

    #[test]
    fn 返信したら_返信先の席を呼ぶ() {
        let 相手 = "6L7QUIAG5PWH…（はじエス担当（kaji PC）（haji-s.com））";
        assert_eq!(
            返信先の書き手を呼ぶ("了解です", 相手, "割符（mac mini）"),
            "@はじエス担当（kaji PC）（haji-s.com） 了解です"
        );
        // この機械の席への返信は、その名札で呼ぶ
        assert_eq!(
            返信先の書き手を呼ぶ("見ました", "dbboard（mac mini）", "割符（mac mini）"),
            "@dbboard（mac mini） 見ました"
        );
    }

    #[test]
    fn 返信先を呼ばない_既に呼んだ_自分_人_知らせ() {
        let 相手 = "TLUTIE3HQGUD…（PdM（ドスパラ））";
        let 自分 = "割符（mac mini）";
        // 既に名前だけで呼んでいる・全員を呼んでいる・写しにしている
        assert_eq!(返信先の書き手を呼ぶ("@PdM 了解", 相手, 自分), "@PdM 了解");
        assert_eq!(返信先の書き手を呼ぶ("@all 了解", 相手, 自分), "@all 了解");
        assert_eq!(
            返信先の書き手を呼ぶ("済み cc@PdM", 相手, 自分),
            "済み cc@PdM"
        );
        // 自分の発言への返信
        assert_eq!(
            返信先の書き手を呼ぶ("補足", "AAAABBBBCCCC…（割符（mac mini））", 自分),
            "補足"
        );
        // 人（差出人が鍵）と、この機械の知らせ
        let 鍵 = "ABCDEFGHJKMNJRLEKLSYIH4NTNPRLEFQEZL7C2PU3Z464KG47DEQ";
        assert_eq!(返信先の書き手を呼ぶ("はい", 鍵, 自分), "はい");
        assert_eq!(返信先の書き手を呼ぶ("はい", "割符", 自分), "はい");
    }
}
