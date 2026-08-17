//! 会議の知らせ。[`Intent`] との行き来だけを持つ。

use warifu_intent::{Intent, Kind};

use crate::{Error, MeetingId, Roster, Signal};

/// 招集。
const INVITE: &str = "meeting.invite";
/// 参加。
const JOIN: &str = "meeting.join";
/// 退出。
const LEAVE: &str = "meeting.leave";
/// 下ごしらえ。
const SIGNAL: &str = "meeting.signal";

/// 会議まわりで相手に渡すもの。
///
/// **これを受け取っても、何も起きない。**読める形にして返すだけで、
/// 入るか・断るか・映像を張るかは呼ぶ側が決める（`decisions.md` **D5**）。
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Notice {
    /// 会議に呼ぶ。名簿も一緒に渡す。
    ///
    /// フルメッシュでは、**入る側は他の参加者が誰かを知らないと繋ぎに行けない。**
    Invite {
        /// どの会議か。
        meeting: MeetingId,
        /// 今いる人。
        roster: Roster,
    },
    /// 入る。
    Join {
        /// どの会議か。
        meeting: MeetingId,
    },
    /// 出る。
    Leave {
        /// どの会議か。
        meeting: MeetingId,
    },
    /// 映像を張るための下ごしらえ。
    Signal(Signal),
}

impl Notice {
    /// どの会議のものか。
    #[must_use]
    pub fn meeting(&self) -> MeetingId {
        match self {
            Self::Invite { meeting, .. } | Self::Join { meeting } | Self::Leave { meeting } => {
                *meeting
            }
            Self::Signal(s) => s.meeting(),
        }
    }

    /// 経路に載せられる形にする。
    ///
    /// 相関は**会議 id そのもの**。別に会議 id を荷物へ書くと、
    /// 相関と食い違ったときに直しようがなくなる。
    ///
    /// # Errors
    /// 下ごしらえが [`crate::MAX_SIGNAL`] を超えたら [`Error::TooLarge`]。
    pub fn to_intent(&self) -> Result<Intent, Error> {
        let (名前, 荷物) = match self {
            Self::Invite { roster, .. } => (INVITE, roster.encode()),
            // 入る・出るのに荷物は要らない。**どの会議かは相関が持っている**
            Self::Join { .. } => (JOIN, Vec::new()),
            Self::Leave { .. } => (LEAVE, Vec::new()),
            Self::Signal(s) => (SIGNAL, s.encode()?),
        };

        Ok(Intent::with_correlation(
            Kind::new(名前)?,
            self.meeting().into(),
            荷物,
        ))
    }

    /// 届いた塊を、会議の知らせとして読む。
    ///
    /// # Errors
    /// 会議の口でなければ [`Error::NotMeeting`]。形が壊れていれば [`Error::Malformed`]。
    /// 名簿が 5 人以上なら [`Error::Full`]（**相手が上限を守る保証は無い**）。
    pub fn from_intent(intent: &Intent) -> Result<Self, Error> {
        let meeting = MeetingId::from(intent.correlation());
        let 荷物 = intent.payload();

        match intent.kind().as_str() {
            INVITE => Ok(Self::Invite {
                meeting,
                roster: Roster::decode(荷物)?,
            }),
            JOIN | LEAVE if !荷物.is_empty() => Err(Error::Malformed),
            JOIN => Ok(Self::Join { meeting }),
            LEAVE => Ok(Self::Leave { meeting }),
            SIGNAL => Ok(Self::Signal(Signal::decode(meeting, 荷物)?)),
            // `file.offer` はもちろん、知らない `meeting.*` もここに落ちる。
            // **知らないものを知っているふりはしない**（warifu-intent と同じ構え）
            _ => Err(Error::NotMeeting),
        }
    }
}
