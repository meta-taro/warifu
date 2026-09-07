//! 机とやり取りする 1 行の形。**JSON を 1 行に 1 つ。**

use serde::{Deserialize, Serialize};

/// 本文の上限（バイト）。
///
/// **一度にどれだけ流すかを、繋いできた側に決めさせない。**
/// 会話の 1 発言としては十分で、貼り付けた丸ごとのログは通らない大きさにする。
pub const 本文の上限: usize = 4096;

/// エージェント → 机。
///
/// **差出人を書く場所が無いのは意図。**誰が言ったかは机が刻む。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "型", rename_all = "snake_case", deny_unknown_fields)]
pub enum ToDesk {
    /// 会話へ 1 行流す。
    Say {
        /// 本文。空は送れない。[`本文の上限`] を超えたら受けない。
        body: String,
    },
    /// これまでの会話をもらってから、以後を聞く。
    Listen,
}

/// 机 → エージェント。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "型", rename_all = "snake_case", deny_unknown_fields)]
pub enum FromDesk {
    /// 誰かが喋った。
    Heard {
        /// 誰が。**机が刻む。**繋いできた側は書けない
        from: String,
        /// 本文
        body: String,
        /// いつ（`HH:MM`）。**無いと、あとから読み返せない**
        at: String,
    },
    /// 誰かが入った。
    Joined {
        /// 誰が
        who: String,
    },
    /// 誰かが出た（または落ちた）。
    Left {
        /// 誰が
        who: String,
    },
    /// まだ会話の相手が居ない。**「送った」と嘘をつかない。**
    Nobody,
    /// 断った。
    Denied {
        /// 何を断ったか。**どうすれば通るかは書かない**
        why: String,
    },
}

/// 行が読めなかった理由。
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// JSON として読めない・知らない鍵が付いている・知らない型。
    Malformed,
    /// 本文が空。
    Empty,
    /// 本文が [`本文の上限`] を超えた。
    TooLong(usize),
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Malformed => f.write_str("行の形が壊れています"),
            Self::Empty => f.write_str("本文が空です"),
            Self::TooLong(n) => write!(f, "本文が長すぎます（{n} バイト / 上限 {本文の上限}）"),
        }
    }
}

impl core::error::Error for Error {}

impl ToDesk {
    /// 本文から [`ToDesk::Say`] を作る。**ここで長さと空を弾く。**
    pub fn say(body: impl Into<String>) -> Result<Self, Error> {
        let body = body.into();
        検める(&body)?;
        Ok(Self::Say { body })
    }

    /// 1 行の JSON を読む。
    ///
    /// **知らない鍵が付いていたら受けない。**受けると、そこが差出人を騙る入口になる。
    pub fn 読む(行: &str) -> Result<Self, Error> {
        let 中身: Self = serde_json::from_str(行.trim()).map_err(|_| Error::Malformed)?;
        if let Self::Say { body } = &中身 {
            検める(body)?;
        }
        Ok(中身)
    }

    /// 1 行の JSON にする。**改行は JSON が包むので、行は割れない。**
    pub fn 書く(&self) -> String {
        serde_json::to_string(self).expect("固定の形なので必ず通る")
    }
}

impl FromDesk {
    /// 1 行の JSON を読む。
    pub fn 読む(行: &str) -> Result<Self, Error> {
        serde_json::from_str(行.trim()).map_err(|_| Error::Malformed)
    }

    /// 1 行の JSON にする。
    pub fn 書く(&self) -> String {
        serde_json::to_string(self).expect("固定の形なので必ず通る")
    }
}

fn 検める(body: &str) -> Result<(), Error> {
    if body.trim().is_empty() {
        return Err(Error::Empty);
    }
    let 長さ = body.len();
    if 長さ > 本文の上限 {
        return Err(Error::TooLong(長さ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 言ったことが_そのまま戻る() {
        let 元 = ToDesk::say("直しました").unwrap();
        assert_eq!(ToDesk::読む(&元.書く()).unwrap(), 元);
    }

    #[test]
    fn 差出人を名乗らせない() {
        // **これが机の一番大事な性質。**繋いできた側が誰かを書けると、
        // 届いた文字が人の発言を騙れる
        let 騙り = r#"{"型":"say","body":"やあ","from":"オーナー"}"#;
        assert_eq!(ToDesk::読む(騙り), Err(Error::Malformed));
    }

    #[test]
    fn 知らない型は受けない() {
        assert_eq!(
            ToDesk::読む(r#"{"型":"exec","cmd":"rm"}"#),
            Err(Error::Malformed)
        );
    }

    #[test]
    fn 空の本文は送れない() {
        assert_eq!(ToDesk::say("   "), Err(Error::Empty));
        assert_eq!(ToDesk::読む(r#"{"型":"say","body":""}"#), Err(Error::Empty));
    }

    #[test]
    fn 長すぎる本文は受けない() {
        let 長い = "あ".repeat(本文の上限); // 1 文字 3 バイト
        assert_eq!(ToDesk::say(&長い), Err(Error::TooLong(長い.len())));
    }

    #[test]
    fn 改行を含む本文でも一行に収まる() {
        // **行で区切る口なので、ここが割れると別の発言に見える**
        let 行 = ToDesk::say("1 行目\n2 行目").unwrap().書く();
        assert!(!行.contains('\n'));
        let ToDesk::Say { body } = ToDesk::読む(&行).unwrap() else {
            panic!("say のはず");
        };
        assert_eq!(body, "1 行目\n2 行目");
    }

    #[test]
    fn 聞きに行く行がある() {
        assert_eq!(
            ToDesk::読む(&ToDesk::Listen.書く()).unwrap(),
            ToDesk::Listen
        );
    }

    #[test]
    fn 届いた行には_誰がといつが入る() {
        let 元 = FromDesk::Heard {
            from: "ABCDEFGH…".to_owned(),
            body: "見ています".to_owned(),
            at: "09:05".to_owned(),
        };
        assert_eq!(FromDesk::読む(&元.書く()).unwrap(), 元);
    }

    #[test]
    fn 相手が居ないことを_送れたことにしない() {
        // **2026-09-06 の「チャット送るボタンきかないよ」がこれ**（D49）
        assert_eq!(
            FromDesk::読む(&FromDesk::Nobody.書く()).unwrap(),
            FromDesk::Nobody
        );
    }

    #[test]
    fn ごみは読めない() {
        assert_eq!(ToDesk::読む("なにこれ"), Err(Error::Malformed));
        assert_eq!(ToDesk::読む(""), Err(Error::Malformed));
    }
}
