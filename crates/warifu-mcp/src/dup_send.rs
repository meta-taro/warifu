//! **同じメールを二度送らない**（0.1.60・`mail_send` の前に送信済みを見る）。
//!
//! 送れていた通を「まだ送れていない」と取り違えて送り直すと、相手には同じメールが 2 通届く。
//! 送ったものは取り消せないので、**送る前に**、同じ口・同じ宛先・同じ件名の通が
//! 直近 24 時間の送信済みにあるかを見て、あれば止める。

use std::collections::BTreeSet;

use crate::server::送信済みの通;

/// 二重送信とみなす間（秒）。
pub const 見る間: i64 = 24 * 60 * 60;

/// **前に送った同じ通を探す。**新しい順の並びから、最初に当たったものを返す。
///
/// - 口：`口` を書いたときだけ、その口の通に絞る（大文字小文字は見ない）。省けばどの口でも当てる
/// - 宛先：アドレスの集合で比べる（小文字にし、前後の空白を落とし、並びと重なりは見ない）
/// - 件名：前後の空白だけ落として比べる。`Re:` などの前置きは**そのまま**比べる（返信の送り直しを止めたいので）
/// - 時刻：`今` から [`見る間`] 秒以内（ちょうど 24 時間は当てる）。時刻の分からない通は当てない。
///   `今` より先の時刻（時計のずれ）は当てる
pub fn 同じ通を探す<'a>(
    並び: &'a [送信済みの通],
    口: Option<&str>,
    宛先: &[String],
    件名: &str,
    今: i64,
) -> Option<&'a 送信済みの通> {
    let 口 = 口.map(str::trim).filter(|s| !s.is_empty());
    let 宛先 = 宛先の集合(宛先);
    let 件名 = 件名.trim();
    並び.iter().find(|通| {
        口.is_none_or(|k| 通.口.trim().eq_ignore_ascii_case(k))
            && 通.送った.is_some_and(|t| 今 - t <= 見る間)
            && 通.件名.trim() == 件名
            && 宛先の集合(&通.宛先) == 宛先
    })
}

fn 宛先の集合(宛先: &[String]) -> BTreeSet<String> {
    宛先
        .iter()
        .map(|a| a.trim().to_lowercase())
        .filter(|a| !a.is_empty())
        .collect()
}

#[cfg(test)]
mod 試験 {
    use super::*;

    const 今: i64 = 1_795_600_000;

    fn 通(口: &str, 前: i64, 宛先: &[&str], 件名: &str) -> 送信済みの通 {
        送信済みの通 {
            口: 口.to_owned(),
            uid: 1,
            送った: Some(今 - 前),
            宛先: 宛先.iter().map(|s| (*s).to_owned()).collect(),
            件名: 件名.to_owned(),
            印: Some("<1@example.com>".to_owned()),
        }
    }

    fn 宛(a: &[&str]) -> Vec<String> {
        a.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn 同じ口_宛先_件名で_24_時間以内なら当たる() {
        let 並び = [通(
            "me@example.com",
            3600,
            &["you@example.com"],
            "Re: 見積",
        )];
        assert!(
            同じ通を探す(
                &並び,
                Some("me@example.com"),
                &宛(&["you@example.com"]),
                "Re: 見積",
                今
            )
            .is_some()
        );
    }

    #[test]
    fn ちょうど_24_時間は当て_1_秒過ぎたら当てない() {
        let 境 = [通("me@example.com", 見る間, &["you@example.com"], "見積")];
        let 過ぎ = [通(
            "me@example.com",
            見る間 + 1,
            &["you@example.com"],
            "見積",
        )];
        let 宛先 = 宛(&["you@example.com"]);
        assert!(同じ通を探す(&境, None, &宛先, "見積", 今).is_some());
        assert!(同じ通を探す(&過ぎ, None, &宛先, "見積", 今).is_none());
    }

    #[test]
    fn 宛先は大文字小文字と並びと空白を見ない() {
        let 並び = [通(
            "me@example.com",
            60,
            &["B@Example.com", "a@example.com"],
            "見積",
        )];
        assert!(
            同じ通を探す(
                &並び,
                None,
                &宛(&[" a@EXAMPLE.com", "b@example.com"]),
                "見積",
                今
            )
            .is_some()
        );
    }

    #[test]
    fn 宛先が一人でも違えば当てない() {
        let 並び = [通("me@example.com", 60, &["a@example.com"], "見積")];
        let 足した = 宛(&["a@example.com", "c@example.com"]);
        assert!(同じ通を探す(&並び, None, &足した, "見積", 今).is_none());
        assert!(同じ通を探す(&並び, None, &宛(&["c@example.com"]), "見積", 今).is_none());
    }

    #[test]
    fn 件名は前後の空白だけ落とし_前置きの有無は別の件名とみる() {
        let 並び = [通("me@example.com", 60, &["a@example.com"], "Re: 見積")];
        let 宛先 = 宛(&["a@example.com"]);
        assert!(同じ通を探す(&並び, None, &宛先, "  Re: 見積 ", 今).is_some());
        assert!(同じ通を探す(&並び, None, &宛先, "見積", 今).is_none());
    }

    #[test]
    fn 口を書けばその口だけ_省けばどの口でも当てる() {
        let 並び = [通("Other@example.com", 60, &["a@example.com"], "見積")];
        let 宛先 = 宛(&["a@example.com"]);
        assert!(同じ通を探す(&並び, Some("me@example.com"), &宛先, "見積", 今).is_none());
        assert!(同じ通を探す(&並び, Some("other@example.com"), &宛先, "見積", 今).is_some());
        assert!(同じ通を探す(&並び, None, &宛先, "見積", 今).is_some());
        assert!(同じ通を探す(&並び, Some(""), &宛先, "見積", 今).is_some());
    }

    #[test]
    fn 時刻の分からない通は当てず_先の時刻は当てる() {
        let mut 分からない = 通("me@example.com", 0, &["a@example.com"], "見積");
        分からない.送った = None;
        let 先 = 通("me@example.com", -600, &["a@example.com"], "見積");
        let 宛先 = 宛(&["a@example.com"]);
        assert!(同じ通を探す(&[分からない], None, &宛先, "見積", 今).is_none());
        assert!(同じ通を探す(&[先], None, &宛先, "見積", 今).is_some());
    }

    #[test]
    fn 送信済みが空なら当てない() {
        assert!(同じ通を探す(&[], None, &宛(&["a@example.com"]), "見積", 今).is_none());
    }
}
