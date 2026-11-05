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

#[cfg(test)]
mod 試験 {
    use super::*;

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
