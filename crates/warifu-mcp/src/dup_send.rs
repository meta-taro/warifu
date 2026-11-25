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

/// 二重送信を見るとき、1 つの口で遡る最初の通の数。
pub const 最初に見る通: usize = 20;
/// 二重送信を見るとき、1 つの口で遡る上限の通の数。
pub const 遡る上限: usize = 200;

/// **二重送信を見るために遡った送信済み**（[`crate::Warifu::二重送信を見るには`] の手の返り）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct 遡った送信済み {
    /// 読めた口の通（新しい順でなくてよい）。
    pub 通: Vec<送信済みの通>,
    /// 読めなかった口と、その訳。
    pub 読めなかった口: Vec<(String, String)>,
    /// [`遡る上限`] まで遡っても、24 時間より古い通に届かなかった口。
    pub 遡りきれなかった口: Vec<String>,
}

/// **もっと遡るか。**`頼んだ数` 通を頼んで `数` 通返り、いちばん古い通の時刻が `最古` のとき、
/// まだ `いつから` より古い通に届いていなければ真（頼んだより少なく返ったなら、箱の底まで見たので偽）。
pub fn もっと遡るか(
    数: usize, 頼んだ数: usize, 最古: Option<i64>, いつから: i64
) -> bool {
    数 >= 頼んだ数 && 最古.is_none_or(|t| t >= いつから)
}

/// **見立て**（[`見立てる`] の返り）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum 見立て {
    /// 同じ通を送っている。送らずに止める。
    当たった(送信済みの通),
    /// 当たらなかった。
    無い {
        /// 見きれなかった所（返りの頭に添える）。全部見られたなら `None`。
        注: Option<String>,
    },
}

/// **遡った送信済みから、止めるかを決める。**
///
/// - 当たれば止める（ほかの口が読めなくても）
/// - `口` を書いていて、その口が読めなかったなら、**全体が読めないときと同じ**注にする
/// - 見きれなかった口（読めない・遡りきれない）は、注に口と訳を名指しで書く（黙って飛ばさない）。
///   `口` を書いたときは、その口のことだけを書く
pub fn 見立てる(
    遡り: &遡った送信済み,
    口: Option<&str>,
    宛先: &[String],
    件名: &str,
    今: i64,
) -> 見立て {
    if let Some(前) = 同じ通を探す(&遡り.通, 口, 宛先, 件名, 今) {
        return 見立て::当たった(前.clone());
    }
    let 口 = 口.map(str::trim).filter(|s| !s.is_empty());
    let 関わる = |k: &str| 口.is_none_or(|x| k.trim().eq_ignore_ascii_case(x));
    let mut 欠け: Vec<String> = 遡り
        .読めなかった口
        .iter()
        .filter(|(k, _)| 関わる(k))
        .map(|(k, 訳)| format!("{k} の送信済みが読めません（{訳}）"))
        .collect();
    欠け.extend(
        遡り
            .遡りきれなかった口
            .iter()
            .filter(|k| 関わる(k))
            .map(|k| format!("{k} は {遡る上限} 通遡っても 24 時間前に届きませんでした")),
    );
    let 注 = (!欠け.is_empty()).then(|| {
        format!(
            "（送信済みを確かめられませんでした：{}。その分は、同じ通を前に送ったかを確かめずに進めます）\n",
            欠け.join("・")
        )
    });
    見立て::無い { 注 }
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

    #[test]
    fn 頼んだだけ返り_最古がまだ_24_時間の内なら_もっと遡る() {
        let いつから = 今 - 見る間;
        assert!(もっと遡るか(20, 20, Some(今 - 60), いつから));
        assert!(もっと遡るか(20, 20, None, いつから));
        // 24 時間より古い通に届いた
        assert!(!もっと遡るか(20, 20, Some(いつから - 1), いつから));
        // 頼んだより少ない＝箱の底まで見た
        assert!(!もっと遡るか(7, 20, Some(今 - 60), いつから));
    }

    fn 遡り(
        通: Vec<送信済みの通>,
        読めない: &[(&str, &str)],
        届かない: &[&str],
    ) -> 遡った送信済み {
        遡った送信済み {
            通,
            読めなかった口: 読めない
                .iter()
                .map(|(k, w)| ((*k).to_owned(), (*w).to_owned()))
                .collect(),
            遡りきれなかった口: 届かない.iter().map(|k| (*k).to_owned()).collect(),
        }
    }

    #[test]
    fn 当たれば_ほかの口が読めなくても止める() {
        let r = 遡り(
            vec![通("me@example.com", 60, &["a@example.com"], "見積")],
            &[("other@example.com", "繋がらない")],
            &[],
        );
        assert!(matches!(
            見立てる(&r, None, &宛(&["a@example.com"]), "見積", 今),
            見立て::当たった(_)
        ));
    }

    #[test]
    fn 一つの口が読めなければ_口と訳を名指しで注に書く() {
        let r = 遡り(Vec::new(), &[("other@example.com", "繋がらない")], &[]);
        let 見立て::無い { 注: Some(注) } =
            見立てる(&r, None, &宛(&["a@example.com"]), "見積", 今)
        else {
            panic!("注が要る");
        };
        assert!(注.contains("other@example.com"), "{注}");
        assert!(注.contains("繋がらない"), "{注}");
    }

    #[test]
    fn 口を書けば_その口のことだけを注に書く() {
        let r = 遡り(
            Vec::new(),
            &[("other@example.com", "繋がらない")],
            &["third@example.com"],
        );
        assert_eq!(
            見立てる(
                &r,
                Some("me@example.com"),
                &宛(&["a@example.com"]),
                "見積",
                今
            ),
            見立て::無い { 注: None }
        );
        let 見立て::無い { 注: Some(注) } = 見立てる(
            &r,
            Some("Other@example.com"),
            &宛(&["a@example.com"]),
            "見積",
            今,
        ) else {
            panic!("書いた口が読めないなら注が要る");
        };
        assert!(
            注.contains("other@example.com の送信済みが読めません"),
            "{注}"
        );
        assert!(!注.contains("third@example.com"), "{注}");
    }

    #[test]
    fn 遡りきれなかった口は_上限の数と一緒に注に書く() {
        let r = 遡り(Vec::new(), &[], &["me@example.com"]);
        let 見立て::無い { 注: Some(注) } =
            見立てる(&r, None, &宛(&["a@example.com"]), "見積", 今)
        else {
            panic!("注が要る");
        };
        assert!(注.contains("me@example.com"), "{注}");
        assert!(注.contains(&format!("{遡る上限} 通遡っても")), "{注}");
    }

    #[test]
    fn 全部読めて当たらなければ注は無い() {
        let r = 遡り(
            vec![通("me@example.com", 60, &["a@example.com"], "別")],
            &[],
            &[],
        );
        assert_eq!(
            見立てる(&r, None, &宛(&["a@example.com"]), "見積", 今),
            見立て::無い { 注: None }
        );
    }
}
