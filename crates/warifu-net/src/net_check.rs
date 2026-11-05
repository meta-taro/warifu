//! **網の調べ**（0.1.44）—— iroh が起動のあとに調べた網の様子を、人が読める 1 行にする。
//!
//! 自前の中継に QAD（7842/UDP）を入れたとき、**効いたかがどこにも出ていなかった。**
//! いまの iroh は STUN を使わないので、QAD が通らない間は「UDP が通る」も
//! 「外から見た自分の番地」も出ない。入れる前と後で、この 1 行を見比べる。
//!
//! iroh の `unstable-net-report` は版の約束の外にある。iroh を上げて壊れたら、ここだけを直す。
//!
//! **番地そのものは持たない**（記録は人に見せて貼られる。見えたかどうかだけで足りる）。

use std::fmt;
use std::time::Duration;

/// 網の調べの要約。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct 網の調べ {
    /// UDP（v4）が外へ通ったか。
    pub udp_v4: bool,
    /// UDP（v6）が外へ通ったか。
    pub udp_v6: bool,
    /// 外から見た自分の番地が分かったか（v4 か v6 のどちらか）。**中継の QAD が答えたときだけ真。**
    pub 外の番地が見えた: bool,
    /// いちばん近い中継までの往復。
    pub 中継まで: Option<Duration>,
}

impl 網の調べ {
    /// iroh の調べから作る。
    #[must_use]
    pub fn from_report(r: &iroh::unstable_net_report::NetReport) -> Self {
        Self {
            udp_v4: r.udp_v4,
            udp_v6: r.udp_v6,
            外の番地が見えた: r.global_v4.is_some() || r.global_v6.is_some(),
            中継まで: r.relay_latency.iter().map(|(_, _, d)| d).min(),
        }
    }
}

impl fmt::Display for 網の調べ {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let 通る = |b: bool| if b { "通る" } else { "通らない" };
        write!(
            f,
            "UDP v4 {}・v6 {}・外から見た番地 {}・中継まで {}",
            通る(self.udp_v4),
            通る(self.udp_v6),
            if self.外の番地が見えた {
                "見えた（中継の QAD）"
            } else {
                "見えない"
            },
            self.中継まで.map_or_else(
                || "測れていない".to_owned(),
                |d| format!("{} ms", d.as_millis())
            )
        )
    }
}

/// **中継との線の様子**（0.1.44）。変わったときだけ記録に 1 行書くために使う。
///
/// 中継に断られても、0.1.40 までは記録に何も出なかった（許可リストから漏れた機械が、
/// 中継を使えていないことに本人も画面の前の人も気づけない）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum 中継の様子 {
    /// まだ一度もつなぎに行っていない。
    まだ,
    /// つながっている。
    つながった,
    /// **中継に断られた**（許可リストに無い・トークンが違う）。iroh はつなぎ直し続けるが、同じ身元では通らない。
    断られた(String),
    /// それ以外の理由でつながっていない（網が届かない など）。
    落ちた(String),
}

impl 中継の様子 {
    /// iroh の見えるもの（つながっているか・最後の誤りの文）から決める。
    ///
    /// **断られたかを、誤りの文で見分けている**（`The relay denied our authentication (…)`）。
    /// いま使っている iroh（1.0 / 1.1）には断られた理由を返す口が無いため。iroh を上げたら
    /// `RelayStatus::auth_denied_reason` に替える（文の言い回しは版の約束の外）。
    #[must_use]
    pub fn 決める(つながっている: bool, 最後の誤り: Option<String>) -> Self {
        if つながっている {
            return Self::つながった;
        }
        match 最後の誤り {
            Some(訳) if 訳.contains("denied our authentication") => {
                Self::断られた(断られた訳(&訳))
            }
            Some(訳) => Self::落ちた(訳),
            None => Self::まだ,
        }
    }

    /// 記録に書く 1 行（`中継: ` に続く所）。`まだ` は書かない。
    #[must_use]
    pub fn 記録の文(&self) -> Option<String> {
        match self {
            Self::まだ => None,
            Self::つながった => Some("つながりました".to_owned()),
            Self::断られた(訳) => Some(format!(
                "断られました（{訳}）。この機械の鍵が中継の許可リストに入っていない見込みです。`warifu id` の 1 行目を中継を立てた人に渡してください"
            )),
            Self::落ちた(訳) => Some(format!(
                "つながっていません（{訳}）。直接つながれない相手には届きません"
            )),
        }
    }
}

/// `… denied our authentication (not authorized) …` から、括弧の中（中継が言った理由）を取る。無ければ全文。
fn 断られた訳(文: &str) -> String {
    文.split_once("denied our authentication (")
        .and_then(|(_, 後)| 後.split_once(')'))
        .map_or_else(|| 文.to_owned(), |(訳, _)| 訳.to_owned())
}

#[cfg(test)]
mod 試験 {
    use super::*;

    #[test]
    fn 中継の様子は_つながっていればつながった_断られたが落ちたより先() {
        assert_eq!(
            中継の様子::決める(true, Some("x".into())),
            中継の様子::つながった
        );
        // ZenBook の中継で実際に出た文（11-05）
        assert_eq!(
            中継の様子::決める(
                false,
                Some(
                    "connecting to relay: The relay denied our authentication (not authorized)"
                        .into()
                )
            ),
            中継の様子::断られた("not authorized".into())
        );
        assert_eq!(
            中継の様子::決める(false, Some("timeout".into())),
            中継の様子::落ちた("timeout".into())
        );
        assert_eq!(中継の様子::決める(false, None), 中継の様子::まだ);
        assert!(中継の様子::まだ.記録の文().is_none());
        let 文 = 中継の様子::断られた("not authorized".into())
            .記録の文()
            .unwrap();
        assert!(
            文.contains("断られました（not authorized）") && 文.contains("許可リスト"),
            "{文}"
        );
    }

    #[test]
    fn qad_が答えたら_見えたと言う_番地そのものは出さない() {
        let 調べ = 網の調べ {
            udp_v4: true,
            udp_v6: false,
            外の番地が見えた: true,
            中継まで: Some(Duration::from_millis(23)),
        };
        assert_eq!(
            調べ.to_string(),
            "UDP v4 通る・v6 通らない・外から見た番地 見えた（中継の QAD）・中継まで 23 ms"
        );
    }

    #[test]
    fn qad_が通らないときは_見えないと言い_中継の遅れだけが残る() {
        let 調べ = 網の調べ {
            udp_v4: false,
            udp_v6: false,
            外の番地が見えた: false,
            中継まで: None,
        };
        assert_eq!(
            調べ.to_string(),
            "UDP v4 通らない・v6 通らない・外から見た番地 見えない・中継まで 測れていない"
        );
    }
}
