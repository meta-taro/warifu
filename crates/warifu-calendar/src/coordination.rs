//! 会議の調整。**双方の承認が揃うまで確定しない。**

use crate::{Error, Span};

/// どちら側か。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// 招く側。
    Organizer,
    /// 招かれる側。
    Invitee,
}

/// 会議の調整。
///
/// **片方だけでは確定しない。**片方の Agent が勝手に予定を入れられるなら、
/// 予定表は相手の Agent に開放されているのと同じである。
///
/// 承認は**候補の中からしか選べない。**候補の外を通せるなら、
/// 候補を出した意味が無くなる。
#[derive(Debug, Clone)]
pub struct Coordination {
    offered: Vec<Span>,
    organizer: Option<Span>,
    invitee: Option<Span>,
}

impl Coordination {
    /// 候補を出して調整を始める。
    ///
    /// # パニック
    ///
    /// 候補が空のとき。空の候補は「選べるものが無い」という状態で、
    /// 調整として成立しない。空を渡しうる場所からは [`Coordination::try_new`] を使うこと。
    pub fn new(offered: Vec<Span>) -> Self {
        Self::try_new(offered).expect("候補が空です")
    }

    /// 候補を出して調整を始める。**空の候補は受け取らない。**
    pub fn try_new(offered: Vec<Span>) -> Result<Self, Error> {
        if offered.is_empty() {
            return Err(Error::Malformed);
        }
        Ok(Self {
            offered,
            organizer: None,
            invitee: None,
        })
    }

    /// 出してある候補。
    pub fn offered(&self) -> &[Span] {
        &self.offered
    }

    /// この枠でよい、と示す。**候補の中からしか選べない。**
    ///
    /// 何度でも変えてよい。**間違えて押したときに直せないと、人は押すのを怖がる。**
    ///
    /// # 失敗
    ///
    /// 候補に無い枠なら [`Error::NotOffered`]。
    pub fn accept(&mut self, side: Side, span: &Span) -> Result<(), Error> {
        if !self.offered.contains(span) {
            return Err(Error::NotOffered);
        }
        match side {
            Side::Organizer => self.organizer = Some(*span),
            Side::Invitee => self.invitee = Some(*span),
        }
        Ok(())
    }

    /// 断る。**確定していても取り消せる。**
    pub fn decline(&mut self, side: Side) {
        match side {
            Side::Organizer => self.organizer = None,
            Side::Invitee => self.invitee = None,
        }
    }

    /// 確定した枠。
    ///
    /// **双方が同じ枠を承認したときだけ返る。**
    /// 別々の枠を選んでいる間は、確定していない。
    pub fn confirmed(&self) -> Option<Span> {
        match (self.organizer, self.invitee) {
            (Some(a), Some(b)) if a == b => Some(a),
            _ => None,
        }
    }
}
