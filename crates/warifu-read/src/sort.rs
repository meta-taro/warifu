//! **仕分け**（0.1.45）—— ヘッダだけから、AI を呼ばずに札を決める。
//!
//! - **本文は読まない。**ヘッダの手がかりだけで決める（段の札が要らない）
//! - **差出人が書ける所は「候補」にしかしない。**件名の語で決めた札は、なりすまし疑いに負ける
//! - **認証の結果は、受け取ったサーバ自身が付けたものだけを読む。**差出人も
//!   `Authentication-Results` を書き足せるので、いちばん上の 1 つで、受け手の名が合うものだけ
//! - 決まらなければ [`札::未分類`]。知らないものを知っているふりはしない

/// 仕分けの札。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum 札 {
    /// 認証に落ちた・表示名で別の番地を名乗っている。**人へは理由つきで伝える**
    なりすまし疑い,
    /// 受け取ったサーバが迷惑と印を付けた。**人へは伝えない**（数と理由は残す）
    迷惑,
    /// 請求・支払いの語が件名にある
    請求,
    /// 注文の語が件名にある（人が送ったもの）
    受注,
    /// 問い合わせの語が件名にある（人が送ったもの）
    問い合わせ,
    /// 自動で送られた知らせ（noreply など）
    知らせ,
    /// 一斉に送られたもの（配信停止の案内がある・bulk）
    広告,
    /// どれにも当たらない（エージェントが見る）
    未分類,
}

impl 札 {
    /// 口や記録に出す名前。
    #[must_use]
    pub const fn 名(self) -> &'static str {
        match self {
            Self::なりすまし疑い => "なりすまし疑い",
            Self::迷惑 => "迷惑",
            Self::請求 => "請求",
            Self::受注 => "受注",
            Self::問い合わせ => "問い合わせ",
            Self::知らせ => "知らせ",
            Self::広告 => "広告",
            Self::未分類 => "未分類",
        }
    }
}

/// ヘッダから拾った手がかり。**本文は入らない。**
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct 手がかり {
    /// `From` の番地。
    pub 差出人: String,
    /// `From` の表示名（相手の申告）。
    pub 名: Option<String>,
    /// 件名（相手の申告）。
    pub 件名: String,
    /// `Authentication-Results` を**上から順に**（いちばん上が、いちばん後に付いたもの）。
    pub 認証: Vec<String>,
    /// `List-Unsubscribe` があるか。
    pub 配信停止: bool,
    /// `Precedence` の値。
    pub 優先: Option<String>,
    /// `X-Spam-Flag` の値。
    pub 迷惑の印: Option<String>,
}

/// 仕分けた結果。理由は人が読める短い文。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct 仕分け {
    /// 札。
    pub 札: 札,
    /// なぜその札か（当たった規則）。
    pub 理由: Vec<String>,
}

/// **仕分ける。**`受け手` は、認証の結果を付けたと信じるサーバの名（Gmail なら `mx.google.com`）。
/// 分からなければ `None` —— そのときは認証の結果を読まない（読めないことを理由に書く）。
#[must_use]
pub fn 仕分ける(手: &手がかり, 受け手: Option<&str>) -> 仕分け {
    if 手
        .迷惑の印
        .as_deref()
        .is_some_and(|v| v.trim().eq_ignore_ascii_case("yes"))
    {
        return 決める(札::迷惑, vec!["受け取ったサーバが迷惑の印を付けた".into()]);
    }
    let mut 疑い = 認証を見る(&手.認証, 受け手);
    if let Some(r) = 表示名で別の番地を名乗るか(手) {
        疑い.push(r);
    }
    if !疑い.is_empty() {
        return 決める(札::なりすまし疑い, 疑い);
    }
    let 自動 = 自動で送ったか(手);
    let 件名 = 手.件名.to_lowercase();
    if let Some(語) = 当たる語(
        &件名,
        &["請求", "invoice", "お支払", "支払い", "ご入金", "振込"],
    ) {
        return 決める(札::請求, vec![format!("件名に「{語}」（差出人の申告）")]);
    }
    if 自動.is_none() {
        if let Some(語) = 当たる語(&件名, &["注文", "受注", "発注", "見積", "order"])
        {
            return 決める(札::受注, vec![format!("件名に「{語}」（差出人の申告）")]);
        }
        if let Some(語) = 当たる語(
            &件名,
            &[
                "お問い合わせ",
                "お問合せ",
                "問い合わせ",
                "問合せ",
                "inquiry",
            ],
        ) {
            return 決める(
                札::問い合わせ,
                vec![format!("件名に「{語}」（差出人の申告）")],
            );
        }
    }
    match 自動 {
        Some(自動の訳::返信不要) => {
            決める(札::知らせ, vec!["差出人が返信を受けない番地".into()])
        }
        Some(自動の訳::一斉) => 決める(
            札::広告,
            vec!["一斉配信の印（配信停止の案内・bulk）".into()],
        ),
        None => 決める(札::未分類, Vec::new()),
    }
}

fn 決める(札: 札, 理由: Vec<String>) -> 仕分け {
    仕分け { 札, 理由 }
}

fn 当たる語<'a>(件名: &str, 語たち: &[&'a str]) -> Option<&'a str> {
    語たち.iter().copied().find(|語| 件名.contains(語))
}

enum 自動の訳 {
    返信不要,
    一斉,
}

fn 自動で送ったか(手: &手がかり) -> Option<自動の訳> {
    let 頭 = 手.差出人.split('@').next().unwrap_or("").to_lowercase();
    let 返信不要 = [
        "noreply",
        "no-reply",
        "no_reply",
        "donotreply",
        "do-not-reply",
        "notification",
        "notify",
        "mailer-daemon",
    ];
    if 返信不要.iter().any(|w| 頭.contains(w)) {
        return Some(自動の訳::返信不要);
    }
    let 一斉 = 手.配信停止
        || 手.優先.as_deref().is_some_and(|p| {
            let p = p.trim().to_ascii_lowercase();
            p == "bulk" || p == "list" || p == "junk"
        });
    一斉.then_some(自動の訳::一斉)
}

/// 表示名にメールの番地を書き、それが本当の差出人と違う（`"社長 <boss@例>" <x@悪>` の形）。
fn 表示名で別の番地を名乗るか(手: &手がかり) -> Option<String> {
    let 名 = 手.名.as_deref()?;
    let 名乗り = 名
        .split(|c: char| c.is_whitespace() || "<>\"'()（）「」".contains(c))
        .find(|片| 片.contains('@') && 片.contains('.'))?;
    (!名乗り.eq_ignore_ascii_case(手.差出人.trim()))
        .then(|| "表示名で別のメール番地を名乗っている".to_owned())
}

/// **受け取ったサーバが付けた認証の結果**から、落ちたものを理由にする。
fn 認証を見る(認証: &[String], 受け手: Option<&str>) -> Vec<String> {
    let (Some(受け手), Some(上)) = (受け手, 認証.first()) else {
        return Vec::new();
    };
    // `mx.google.com; dkim=pass ...` —— 最初の `;` の前が付けたサーバの名（後ろに版の数字が付くことがある）
    let 名 = 上
        .split(';')
        .next()
        .unwrap_or("")
        .split_whitespace()
        .next()
        .unwrap_or("");
    if !名.eq_ignore_ascii_case(受け手) {
        return Vec::new();
    }
    let 結果 = |方式: &str| -> Option<String> {
        let 頭 = format!("{方式}=");
        上.split([';', ' ', '\t', '\n', '\r']).find_map(|片| {
            片.trim()
                .strip_prefix(頭.as_str())
                .map(str::to_ascii_lowercase)
        })
    };
    let (dmarc, spf, dkim) = (結果("dmarc"), 結果("spf"), 結果("dkim"));
    let mut 理由 = Vec::new();
    if dmarc.as_deref() == Some("fail") {
        理由.push("差出人のドメインの認証（DMARC）に落ちた".to_owned());
    }
    if spf.as_deref() != Some("pass") && dkim.as_deref() != Some("pass") {
        理由.push(format!(
            "送ったサーバの確かめ（SPF: {}・DKIM: {}）がどちらも通っていない",
            spf.as_deref().unwrap_or("無し"),
            dkim.as_deref().unwrap_or("無し")
        ));
    }
    理由
}

#[cfg(test)]
mod tests {
    use super::*;

    const GMAIL: Option<&str> = Some("mx.google.com");

    fn 人から(差出人: &str, 件名: &str) -> 手がかり {
        手がかり {
            差出人: 差出人.into(),
            件名: 件名.into(),
            認証: vec!["mx.google.com; dkim=pass header.i=@example.com; spf=pass smtp.mailfrom=a@example.com; dmarc=pass header.from=example.com".into()],
            ..手がかり::default()
        }
    }

    #[test]
    fn 人からの注文と問い合わせは_件名の語で候補にする() {
        let 注文 = 仕分ける(&人から("buyer@example.com", "【発注】部品の件"), GMAIL);
        assert_eq!(注文.札, 札::受注);
        assert!(注文.理由[0].contains("差出人の申告"));
        assert_eq!(
            仕分ける(&人から("a@example.com", "お問い合わせ"), GMAIL).札,
            札::問い合わせ
        );
        assert_eq!(
            仕分ける(&人から("a@example.com", "ご請求書の送付"), GMAIL).札,
            札::請求
        );
        assert_eq!(
            仕分ける(&人から("a@example.com", "先日はどうも"), GMAIL).札,
            札::未分類
        );
    }

    #[test]
    fn 受け取ったサーバの認証に落ちたら_件名より先になりすまし疑い() {
        let mut 手 = 人から("ceo@example.com", "至急：振込先の変更");
        手.認証 = vec!["mx.google.com; spf=softfail smtp.mailfrom=x@example.net; dmarc=fail header.from=example.com".into()];
        let 分 = 仕分ける(&手, GMAIL);
        assert_eq!(分.札, 札::なりすまし疑い);
        assert!(分.理由.iter().any(|r| r.contains("DMARC")));
        assert!(
            分.理由
                .iter()
                .any(|r| r.contains("SPF: softfail・DKIM: 無し"))
        );
    }

    #[test]
    fn 差出人が書き足した認証の結果は読まない() {
        // いちばん上が受け手の名でない（差出人が書いた A-R が上に来ることはないが、名が違えば読まない）
        let mut 手 = 人から("a@example.com", "こんにちは");
        手.認証 = vec!["example.net; dmarc=fail".into()];
        assert_eq!(仕分ける(&手, GMAIL).札, 札::未分類);
        // 受け手が分からなければ読まない
        手.認証 = vec!["mx.google.com; dmarc=fail".into()];
        assert_eq!(仕分ける(&手, None).札, 札::未分類);
    }

    #[test]
    fn 表示名で別の番地を名乗っていれば_なりすまし疑い() {
        let mut 手 = 人から("x@example.net", "ご確認ください");
        手.名 = Some("boss@example.com".into());
        assert_eq!(仕分ける(&手, GMAIL).札, 札::なりすまし疑い);
        // 同じ番地なら疑わない
        手.名 = Some("x@example.net".into());
        assert_eq!(仕分ける(&手, GMAIL).札, 札::未分類);
    }

    #[test]
    fn 自動の知らせと一斉配信() {
        let mut 手 = 人から("no-reply@example.org", "ご注文ありがとうございます");
        // 自動で送られた「注文」は、こちらが買ったもの。受注にしない
        assert_eq!(仕分ける(&手, GMAIL).札, 札::知らせ);
        手.差出人 = "news@example.org".into();
        手.件名 = "今週のおすすめ".into();
        手.配信停止 = true;
        assert_eq!(仕分ける(&手, GMAIL).札, 札::広告);
        // 自動でも請求は請求
        手.件名 = "ご請求金額のお知らせ".into();
        assert_eq!(仕分ける(&手, GMAIL).札, 札::請求);
    }

    #[test]
    fn 迷惑の印はいちばん先() {
        let mut 手 = 人から("a@example.com", "注文");
        手.迷惑の印 = Some("YES".into());
        assert_eq!(仕分ける(&手, GMAIL).札, 札::迷惑);
    }
}
