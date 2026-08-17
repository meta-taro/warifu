//! 映像を張るための下ごしらえ（SDP / ICE）。**中身は読まない。**

use core::fmt;

use crate::{Error, MeetingId};

/// 一度に運べる下ごしらえの上限（バイト）。
///
/// SDP は普通は数 KB。**16 MiB の「SDP」を送りつけられて確保させられる筋を塞ぐ**ために、
/// 経路の上限（16 MiB）とは別にここで小さく切る。
pub const MAX_SIGNAL: usize = 64 * 1024;

/// 下ごしらえのどの段か。
///
/// **4 つで閉じている。**口の名前（[`warifu_intent::Kind`]）は知らないものも通すが、
/// 段は通さない。口は増やせる場所として開けてあり、段は決まりきった手順で、
/// 知らない段を「たぶん申し出だろう」と扱うと繋ぎ間違えるため。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// こちらから映像を張りたい、という申し出（SDP offer）。
    Offer,
    /// その返事（SDP answer）。
    Answer,
    /// 経路の候補（ICE candidate）。
    Candidate,
    /// もう候補は無い。
    End,
}

impl Step {
    fn to_byte(self) -> u8 {
        match self {
            Self::Offer => 1,
            Self::Answer => 2,
            Self::Candidate => 3,
            Self::End => 4,
        }
    }

    fn from_byte(b: u8) -> Result<Self, Error> {
        match b {
            1 => Ok(Self::Offer),
            2 => Ok(Self::Answer),
            3 => Ok(Self::Candidate),
            4 => Ok(Self::End),
            // 0 もここに落ちる。**知らない段を勝手に読み替えない**
            _ => Err(Error::Malformed),
        }
    }
}

/// 1 通ぶんの下ごしらえ。
///
/// # 相手を指す欄が無い
///
/// 宛先欄を作ると「自分宛でない下ごしらえを預かって渡す」形が書けてしまう。
/// それは `decisions.md` **D7**（他人の通信を中継する）への入口。
/// フルメッシュでは相手ごとに経路が 1 本ずつあり、**誰宛かは経路そのものが決めている。**
///
/// # 中身は解釈しない
///
/// warifu から見れば [`Signal::blob`] はただのバイト列。読み始めると Codec の話が
/// warifu に入り込み、「Codec を自前で書かない」（`issues/005` 満たすこと 3）が守れなくなる。
#[derive(Clone, PartialEq, Eq)]
pub struct Signal {
    meeting: MeetingId,
    step: Step,
    blob: Vec<u8>,
}

impl Signal {
    /// 組み立てる。
    ///
    /// 長すぎるかどうかは、送る形にするとき（[`crate::Notice::to_intent`]）に見る。
    #[must_use]
    pub fn new(meeting: MeetingId, step: Step, blob: Vec<u8>) -> Self {
        Self {
            meeting,
            step,
            blob,
        }
    }

    /// どの会議のものか。
    #[must_use]
    pub fn meeting(&self) -> MeetingId {
        self.meeting
    }

    /// どの段か。
    #[must_use]
    pub fn step(&self) -> Step {
        self.step
    }

    /// 中身。**warifu が作ったものではない。**
    #[must_use]
    pub fn blob(&self) -> &[u8] {
        &self.blob
    }

    /// 荷物にする。`[段 1][中身]`。
    pub(crate) fn encode(&self) -> Result<Vec<u8>, Error> {
        if self.blob.len() > MAX_SIGNAL {
            return Err(Error::TooLarge);
        }
        let mut 荷物 = Vec::with_capacity(1 + self.blob.len());
        荷物.push(self.step.to_byte());
        荷物.extend_from_slice(&self.blob);
        Ok(荷物)
    }

    /// 荷物から読み戻す。
    pub(crate) fn decode(meeting: MeetingId, payload: &[u8]) -> Result<Self, Error> {
        let (&段, 中身) = payload.split_first().ok_or(Error::Malformed)?;
        if 中身.len() > MAX_SIGNAL {
            return Err(Error::TooLarge);
        }
        Ok(Self {
            meeting,
            step: Step::from_byte(段)?,
            blob: 中身.to_vec(),
        })
    }
}

impl fmt::Debug for Signal {
    /// **中身を出さない。**
    ///
    /// SDP と ICE には端末のローカル IP が入っている。
    /// ログに落ちれば、直接つながる相手以外にも住所が漏れる。
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Signal")
            .field("meeting", &self.meeting)
            .field("step", &self.step)
            .field("blob", &format_args!("{} バイト", self.blob.len()))
            .finish()
    }
}
