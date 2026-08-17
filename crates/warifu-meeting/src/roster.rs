//! 会議の名簿と、その id。

use core::fmt;
use core::str::FromStr;

use warifu_core::PublicKey;
use warifu_intent::Correlation;

use crate::Error;

/// 1 つの会議に入れる人数の上限。
///
/// **これを 5 以上にしてはならない。**
///
/// フルメッシュは各自が自分の映像だけを送る形で、誰も他人の映像を運ばない。
/// 5 人以上にすると誰かの端末が他人の映像を中継することになり、
/// それは `decisions.md` **D7**（利用者の端末が他人の通信を中継する）そのもので、
/// 法的な決着が付いていない。
///
/// **4 人という数字自体は文献値**で、手元の回線で成立するかは M6 で実測する
/// （`issues/005`）。成立しなければ 3 に下げる。**上げる側には D7 の決着が要る。**
pub const MAX_PARTICIPANTS: usize = 4;

/// 会議 1 つを指す印。
///
/// 中身は [`Correlation`] そのもの。**会議 id がそのまま「どの話の続きか」になる**ので、
/// 同じ会議のやり取りが 1 本の話として辿れる。
/// 別に会議 id を荷物へ書くと、相関と食い違ったときに直しようがなくなる。
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MeetingId(Correlation);

impl MeetingId {
    /// 新しい会議を起こす。
    #[must_use]
    pub fn generate() -> Self {
        Self(Correlation::generate())
    }
}

impl From<Correlation> for MeetingId {
    fn from(c: Correlation) -> Self {
        Self(c)
    }
}

impl From<MeetingId> for Correlation {
    fn from(m: MeetingId) -> Self {
        m.0
    }
}

impl fmt::Display for MeetingId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // warifu が外に出す文字列は base32 の 1 種類に揃える（M1・M2・M3 と同じ）
        write!(f, "{}", self.0)
    }
}

impl fmt::Debug for MeetingId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self}")
    }
}

impl FromStr for MeetingId {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Error> {
        Ok(Self(s.parse::<Correlation>()?))
    }
}

/// 誰が入っている会議か。
///
/// 先頭が主催者。**上限は [`MAX_PARTICIPANTS`] で、送る側でも受け取る側でも数える。**
/// 相手が上限を守る保証は無い。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Roster {
    members: Vec<PublicKey>,
}

impl Roster {
    /// 主催者ひとりから始める。
    #[must_use]
    pub fn new(host: PublicKey) -> Self {
        Self {
            members: vec![host],
        }
    }

    /// 主催者。
    ///
    /// # Panics
    /// 起こりえない（主催者は [`Roster::new`] で必ず入り、[`Roster::remove`] で抜けない）。
    #[must_use]
    pub fn host(&self) -> PublicKey {
        self.members[0]
    }

    /// 入っている人。**先頭が主催者。**
    #[must_use]
    pub fn members(&self) -> &[PublicKey] {
        &self.members
    }

    /// 何人か。
    #[must_use]
    pub fn len(&self) -> usize {
        self.members.len()
    }

    /// 空にはならない（主催者が必ずいる）。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        false
    }

    /// もう入れないか。
    #[must_use]
    pub fn is_full(&self) -> bool {
        self.members.len() >= MAX_PARTICIPANTS
    }

    /// 入っているか。**ここに居ない相手は会議に入れない。**
    #[must_use]
    pub fn contains(&self, key: &PublicKey) -> bool {
        self.members.contains(key)
    }

    /// 1 人入れる。
    ///
    /// # Errors
    /// もう 4 人なら [`Error::Full`]。もう載っていれば [`Error::AlreadyIn`]
    /// （**二重に数えると 4 人の上限が実質 3 人になる**）。
    pub fn add(&mut self, key: PublicKey) -> Result<(), Error> {
        if self.contains(&key) {
            return Err(Error::AlreadyIn);
        }
        if self.is_full() {
            return Err(Error::Full);
        }
        self.members.push(key);
        Ok(())
    }

    /// 1 人抜く。抜けたら `true`。
    ///
    /// **主催者は抜けない。**主催者が消えると誰の会議か分からなくなる。
    /// 会議を終えるのは退出ではなく、会議そのものを捨てること。
    pub fn remove(&mut self, key: &PublicKey) -> bool {
        if *key == self.host() {
            return false;
        }
        let 前 = self.members.len();
        self.members.retain(|m| m != key);
        self.members.len() != 前
    }

    /// 塊にする。`[人数 1][鍵 32]*人数`。
    pub(crate) fn encode(&self) -> Vec<u8> {
        let mut 塊 = Vec::with_capacity(1 + self.members.len() * 32);
        // 上限が 4 なので u8 に必ず収まる
        #[allow(clippy::cast_possible_truncation)]
        塊.push(self.members.len() as u8);
        for 鍵 in &self.members {
            塊.extend_from_slice(&鍵.to_bytes());
        }
        塊
    }

    /// 塊から読み戻す。
    pub(crate) fn decode(bytes: &[u8]) -> Result<Self, Error> {
        let (&人数, 残り) = bytes.split_first().ok_or(Error::Malformed)?;
        let 人数 = usize::from(人数);

        if 人数 == 0 {
            return Err(Error::Malformed);
        }
        // **受け取る側でも数える。**相手が上限を守る保証は無い
        if 人数 > MAX_PARTICIPANTS {
            return Err(Error::Full);
        }
        if 残り.len() != 人数 * 32 {
            return Err(Error::Malformed);
        }

        let mut members = Vec::with_capacity(人数);
        for 塊 in 残り.chunks_exact(32) {
            let 鍵: [u8; 32] = 塊.try_into().map_err(|_| Error::Malformed)?;
            let 鍵 = PublicKey::from_bytes(鍵).map_err(|_| Error::Malformed)?;
            // 同じ鍵が 2 回書かれていれば、数えた人数が実際と合わない
            if members.contains(&鍵) {
                return Err(Error::Malformed);
            }
            members.push(鍵);
        }

        Ok(Self { members })
    }
}
