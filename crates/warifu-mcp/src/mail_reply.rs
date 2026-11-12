//! **返信の下ごしらえ**（D132・11-12）。元の通の見出しから、宛先・件名・スレッドの見出しを引く。
//!
//! 見出しは送り手の申し送り（`Claims`）。**判断には使わない**（仕分け・優先度には使わない）のは変わらず、
//! ここでは返す先と、相手のメーラーで同じスレッドに並べるためだけに読む。宛先は人が画面で確かめてから送る。

use warifu_read::Received;

/// 返信に要るもの。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct 返信の下ごしらえ {
    /// 返す先（`Reply-To` があればそれ、無ければ差出人）。
    pub 宛先: String,
    /// 件名（`Re: ` を 1 つだけ付ける）。
    pub 件名: String,
    /// 元の通の `Message-ID`（`<…>` 込み）。無ければ `None`。
    pub 返信先: Option<String>,
    /// `References`（元の References に元の Message-ID を足したもの）。
    pub 参照: Option<String>,
}

/// `<…>` で囲む（囲まれていなければ）。空なら `None`。
fn 囲む(id: &str) -> Option<String> {
    let id = id
        .trim()
        .trim_start_matches('<')
        .trim_end_matches('>')
        .trim();
    (!id.is_empty() && !id.contains(char::is_whitespace)).then(|| format!("<{id}>"))
}

/// 件名に `Re: ` を 1 つだけ付ける（もう付いていれば付けない）。
fn 件名に_re(件名: &str) -> String {
    let 件名 = 件名.trim();
    if 件名.len() >= 3 && 件名[..3].eq_ignore_ascii_case("re:") {
        件名.to_owned()
    } else {
        format!("Re: {件名}")
    }
}

/// **元の通から、返信の下ごしらえを作る。**
#[must_use]
pub fn 下ごしらえ(元: &Received) -> 返信の下ごしらえ {
    let 見出し = 元.claims();
    let 宛先 = 見出し
        .get("Reply-To")
        .and_then(|v| v.split_whitespace().next())
        .map_or_else(|| 元.sender().as_str().to_owned(), str::to_owned);
    let 返信先 = 見出し.get("Message-ID").and_then(囲む);
    let mut 参照: Vec<String> = 見出し
        .get("References")
        .map(|v| v.split_whitespace().filter_map(囲む).collect())
        .unwrap_or_default();
    if let Some(先) = &返信先
        && !参照.contains(先)
    {
        参照.push(先.clone());
    }
    返信の下ごしらえ {
        宛先,
        件名: 件名に_re(見出し.get("Subject").unwrap_or_default()),
        返信先,
        参照: (!参照.is_empty()).then(|| 参照.join(" ")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use warifu_read::{Body, Claims, SenderId, Source};

    fn 通(見出し: &[(&str, &str)]) -> Received {
        let mut c = Claims::new();
        for (n, v) in 見出し {
            c = c.with(n, v);
        }
        Received::new(
            Source::Imap,
            SenderId::new("sales@example.com").unwrap(),
            1,
            Body::new(Vec::new()),
        )
        .with_claims(c)
    }

    #[test]
    fn 返信は_返す先と_re_の件名と_スレッドの見出しを引く() {
        let 下 = 下ごしらえ(&通(&[
            ("Subject", "見積の件"),
            ("Message-ID", "abc@example.com"),
            ("References", "<r1@example.com> r2@example.com"),
        ]));
        assert_eq!(下.宛先, "sales@example.com");
        assert_eq!(下.件名, "Re: 見積の件");
        assert_eq!(下.返信先.as_deref(), Some("<abc@example.com>"));
        assert_eq!(
            下.参照.as_deref(),
            Some("<r1@example.com> <r2@example.com> <abc@example.com>")
        );
    }

    #[test]
    fn reply_to_があればそちらへ返し_re_は重ねない() {
        let 下 = 下ごしらえ(&通(&[
            ("Subject", "RE: 見積"),
            ("Reply-To", "desk@example.com"),
        ]));
        assert_eq!(下.宛先, "desk@example.com");
        assert_eq!(下.件名, "RE: 見積");
        assert_eq!(下.返信先, None);
        assert_eq!(下.参照, None);
    }
}
