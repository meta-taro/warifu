//! 会議の名簿と、その id。

use core::fmt;
use core::str::FromStr;

use warifu_core::PublicKey;
use warifu_intent::Correlation;

use crate::Error;

/// フルメッシュとして成立しうる人数の外枠。
///
/// **これは法的な線ではなく、帯域から来る線である**（`decisions.md` **D27**）。
///
/// フルメッシュは各自が**自分の映像だけ**を全員へ直接送る。
/// 5 人でも 6 人でも**誰も他人の通信を中継しない**ので、
/// **D7（利用者の端末が他人の通信を中継する）は発火しない。**
///
/// 効くのは上りの帯域で、参加者ごとに `K×(N−1)`。
/// 720p を 1.5 Mbps とすると、16 人で上り 22.5 Mbps。
/// **家庭用の光なら届くが、携帯回線では無理**という辺りがこの数字である。
///
/// これを超えたいなら、**誰かが他人の映像を運ぶことになる**（SFU）。
/// そこで初めて D7 の決着が要る。
pub const HARD_LIMIT: usize = 16;

/// 定員を指定しなかったときの人数。
///
/// **上限ではない。**指定しなければこの人数で始まる、というだけ。
/// 会議ごとに変えるなら [`Roster::with_capacity`]。
///
/// 12 人フルメッシュで各自が負う帯域（`K×(N−1)`。上下とも同じだけ掛かる）:
///
/// | 画質 | 上り / 下り |
/// |---|---|
/// | 360p（0.6 Mbps） | 6.6 Mbps |
/// | 540p（1.0 Mbps） | 11.0 Mbps |
/// | 720p（1.5 Mbps） | 16.5 Mbps |
/// | 1080p（3.0 Mbps） | 33.0 Mbps |
///
/// **家庭用の光なら 720p まで届く。携帯回線と混み合った無線では落ちる。**
/// **手元の回線では実測していない**（M6・`issues/005`）。
pub const DEFAULT_CAPACITY: usize = 12;

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
    capacity: usize,
}

impl Roster {
    /// 主催者ひとりから始める。定員は [`DEFAULT_CAPACITY`]。
    #[must_use]
    pub fn new(host: PublicKey) -> Self {
        Self {
            members: vec![host],
            capacity: DEFAULT_CAPACITY,
        }
    }

    /// 定員を決めて始める。
    ///
    /// **上限は [`HARD_LIMIT`]。**これは法的な線ではなく帯域から来る線で、
    /// 超えたいなら誰かが他人の映像を運ぶことになる（`decisions.md` **D27**）。
    ///
    /// # Errors
    /// 定員が 2 未満、または [`HARD_LIMIT`] を超えるとき [`Error::Full`]。
    /// **1 人の会議は会議ではない。**
    pub fn with_capacity(host: PublicKey, capacity: usize) -> Result<Self, Error> {
        if !(2..=HARD_LIMIT).contains(&capacity) {
            return Err(Error::Full);
        }
        Ok(Self {
            members: vec![host],
            capacity,
        })
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

    /// この会議の定員。
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// もう入れないか。
    #[must_use]
    pub fn is_full(&self) -> bool {
        self.members.len() >= self.capacity
    }

    /// 入っているか。**ここに居ない相手は会議に入れない。**
    #[must_use]
    pub fn contains(&self, key: &PublicKey) -> bool {
        self.members.contains(key)
    }

    /// 1 人入れる。
    ///
    /// # Errors
    /// 定員に達していれば [`Error::Full`]。もう載っていれば [`Error::AlreadyIn`]
    /// （**二重に数えると定員が実質 1 人減る**）。
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

    /// 塊にする。`[定員 1][人数 1][鍵 32]*人数`。
    ///
    /// **定員も運ぶ。**運ばないと、受け取った側はその会議が何人までなのかを知らず、
    /// 外枠でしか数えられなくなる。
    pub(crate) fn encode(&self) -> Vec<u8> {
        let mut 塊 = Vec::with_capacity(2 + self.members.len() * 32);
        // 定員も人数も HARD_LIMIT 以下なので u8 に必ず収まる
        #[allow(clippy::cast_possible_truncation)]
        塊.push(self.capacity as u8);
        #[allow(clippy::cast_possible_truncation)]
        塊.push(self.members.len() as u8);
        for 鍵 in &self.members {
            塊.extend_from_slice(&鍵.to_bytes());
        }
        塊
    }

    /// 塊から読み戻す。
    pub(crate) fn decode(bytes: &[u8]) -> Result<Self, Error> {
        let (&定員, 残り) = bytes.split_first().ok_or(Error::Malformed)?;
        let (&人数, 残り) = 残り.split_first().ok_or(Error::Malformed)?;
        let (定員, 人数) = (usize::from(定員), usize::from(人数));

        if 人数 == 0 {
            return Err(Error::Malformed);
        }
        // **受け取る側でも数える。**相手が定員を守る保証は無い。
        // 外枠を超えた定員を名乗る招待も受け取らない
        if !(2..=HARD_LIMIT).contains(&定員) || 人数 > 定員 {
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

        Ok(Self {
            members,
            capacity: 定員,
        })
    }
}
