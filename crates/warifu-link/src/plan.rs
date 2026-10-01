//! 割り当て。**落とすのは画質であって人ではない。**

use crate::{Error, FRESH_FOR, Quality};

/// 実測した回線を使い切らない割合。
///
/// 測った値をぎりぎりまで使うと、**バーストと他の通信で必ず溢れる。**
/// 溢れたときに落ちるのは、たいてい音声である。
const 安全率: u64 = 3; // 4 分の 3

/// 測った回線。**申告ではなく観測**（`Meter`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Link {
    uplink_bps: u64,
    downlink_bps: u64,
    measured_at: u64,
}

impl Link {
    /// 測定値を組み立てる。
    pub fn new(uplink_bps: u64, downlink_bps: u64, measured_at: u64) -> Self {
        Self {
            uplink_bps,
            downlink_bps,
            measured_at,
        }
    }

    /// 上り（bps）。
    pub fn uplink_bps(&self) -> u64 {
        self.uplink_bps
    }

    /// 下り（bps）。
    pub fn downlink_bps(&self) -> u64 {
        self.downlink_bps
    }

    /// まだ新しいか。
    pub fn is_fresh(&self, now: u64) -> bool {
        self.measured_at <= now && now - self.measured_at <= FRESH_FOR
    }
}

/// 相手へ渡す測定値。
///
/// **絶対時刻ではなく「何秒前に測ったか」を送る。**
///
/// 相手の時計がこちらとずれていると、絶対時刻をそのまま送った場合に
/// **いつまでも古くならない測定値**や、**届いた瞬間に古い測定値**ができる。
/// 経過秒なら、ずれていても「どれくらい前か」は保たれる。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Report {
    uplink_bps: u64,
    downlink_bps: u64,
    age_secs: u32,
}

impl Report {
    /// 受け取った値から組み立てる。
    ///
    /// **中身は検めない。**上りも下りも、相手が幾つと言おうと
    /// [`plan`] は自分の回線を超えて使わない。
    pub fn new(uplink_bps: u64, downlink_bps: u64, age_secs: u32) -> Self {
        Self {
            uplink_bps,
            downlink_bps,
            age_secs,
        }
    }

    /// 測定値を、渡せる形にする。
    ///
    /// `now` より後に測ったことになっている値は、経過 0 秒として扱う
    /// （**自分の時計が戻った場合に、未来の測定値を作らない**）。
    pub fn from_link(link: &Link, now: u64) -> Self {
        Self {
            uplink_bps: link.uplink_bps,
            downlink_bps: link.downlink_bps,
            age_secs: u32::try_from(now.saturating_sub(link.measured_at)).unwrap_or(u32::MAX),
        }
    }

    /// 受け取った測定値を、こちらの時計に載せ直す。
    pub fn to_link(&self, now: u64) -> Link {
        Link::new(
            self.uplink_bps,
            self.downlink_bps,
            now.saturating_sub(u64::from(self.age_secs)),
        )
    }

    /// 上り（bps）。
    pub fn uplink_bps(&self) -> u64 {
        self.uplink_bps
    }

    /// 下り（bps）。
    pub fn downlink_bps(&self) -> u64 {
        self.downlink_bps
    }

    /// 何秒前に測ったか。
    pub fn age_secs(&self) -> u32 {
        self.age_secs
    }
}

/// 相手ごとの割り当て。
///
/// [`Plan::send`] と [`Plan::receive`] は、渡した相手の並びと**同じ順**。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    send: Vec<Quality>,
    receive: Vec<Quality>,
}

impl Plan {
    /// 相手ごとに送る画質。
    pub fn send(&self) -> &[Quality] {
        &self.send
    }

    /// 相手ごとに受け取る画質。
    pub fn receive(&self) -> &[Quality] {
        &self.receive
    }
}

/// 測った回線から、相手ごとの割り当てを決める。
///
/// # 何を見るか
///
/// - **送る**: 自分の上りを人数で割った持ち分と、**相手の下り**の持ち分の、小さいほう
/// - **受け取る**: 自分の下りを人数で割った持ち分
///
/// どちらも [`安全率`] を掛けてから割る。
///
/// # 相手の申告で自分の回線を超えない
///
/// 相手が「下り 1 Gbps」と言っても、出せるのは**自分の上りまで**である。
/// 相手の値は**送る量を下げる方向にしか効かない。**
///
/// # 相手の測定値が古いとき
///
/// **その相手にだけ音声で送る。**会議ぜんぶを止めない。
/// 古い値をそのまま信じると、繋がらない相手へ全力で送り続けることになる。
///
/// # 失敗
///
/// - [`Error::Stale`] — **自分の**測定値が古い。決める材料が無い
/// - [`Error::TooWeak`] — 音声ぶんすら通らない。**黙って 0 人ぶんを返さない**
pub fn plan(me: &Link, peers: &[Link], now: u64) -> Result<Plan, Error> {
    if !me.is_fresh(now) {
        return Err(Error::Stale);
    }
    if peers.is_empty() {
        return Ok(Plan {
            send: Vec::new(),
            receive: Vec::new(),
        });
    }

    let 人数 = peers.len() as u64;
    let 上りの持ち分 = me.uplink_bps / 4 * 安全率 / 人数;
    let 下りの持ち分 = me.downlink_bps / 4 * 安全率 / 人数;

    // 音声すら通らないなら、参加できないと言う
    let 最低 = Quality::AudioOnly.bitrate_bps();
    if 上りの持ち分 < 最低 || 下りの持ち分 < 最低 {
        return Err(Error::TooWeak);
    }

    let receive = vec![収まる段(下りの持ち分); peers.len()];
    let send = peers
        .iter()
        .map(|相手| {
            // 古い値は信じない。**その相手にだけ控えめに送る**
            if !相手.is_fresh(now) {
                return Quality::AudioOnly;
            }
            let 相手の持ち分 = 相手.downlink_bps / 4 * 安全率 / 人数;
            収まる段(上りの持ち分.min(相手の持ち分))
        })
        .collect();

    Ok(Plan { send, receive })
}

/// 与えられた帯域に収まる、いちばん高い段。
///
/// **上から試して、通る所で止める。**どれも通らなければ音声だけ。
fn 収まる段(bps: u64) -> Quality {
    Quality::ALL
        .into_iter()
        .find(|q| q.bitrate_bps() <= bps)
        .unwrap_or(Quality::AudioOnly)
}
