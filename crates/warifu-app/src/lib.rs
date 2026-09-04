//! 会議の進行（M5-c）。
//!
//! **この層に経路もカメラも入らない。**
//! 「知らせを受けて名簿がどう動くか」「誰が offer を出すか」だけを持つ。
//!
//! 実際に繋ぐのは `warifu-net`、SDP を運ぶ封筒は `warifu-meeting`、
//! 映像そのものは WebView の WebRTC（**Codec は書かない**・`issues/005`）。
//! ここを純粋に保つ理由は 2 つある。
//!
//! - **試験に機材が要らない。**2 台の実機もカメラも無しで、進行の規則を固定できる
//! - 画面（Tauri）と入れ替えても、規則が動かない（baseline §9）

#![forbid(unsafe_code)]

mod invite;

pub use invite::{InviteError, format_invite, is_own_invite, parse_invite};

use warifu_core::PublicKey;
use warifu_meeting::{Error as MeetingError, MeetingId, Notice, Roster, Signal, Step};

pub use warifu_meeting::{DEFAULT_CAPACITY, HARD_LIMIT};

/// 会議の進行で起きたこと。**画面へ渡すのはこれだけ。**
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// 名簿に加わった。
    Joined(PublicKey),
    /// 名簿から抜けた。
    Left(PublicKey),
    /// 映像を張るための下ごしらえが届いた。**中身は解釈しない。**
    Signal {
        /// 誰から。
        from: PublicKey,
        /// どの段か。
        step: Step,
        /// SDP / ICE そのもの。**この層は読まない。**
        blob: Vec<u8>,
    },
}

/// 進行で断った理由。
///
/// `warifu_meeting::Error` は `Clone` も `Eq` も持たない（下の層の理由を捨てないため、
/// 中に原因を抱えている）。**こちらで足さない** — 比較したくなったら、
/// それは「理由の種類」を見たいということなので、そのための口を別に作る。
#[derive(Debug)]
pub enum Error {
    /// 名簿の側の言い分（定員・重複など）。
    Roster(MeetingError),
    /// 別の会議あての知らせだった。**黙って取り込まない。**
    OtherMeeting,
    /// 名簿に居ない相手からの下ごしらえ。
    NotAMember,
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Roster(e) => write!(f, "名簿が受け取らなかった: {e}"),
            Self::OtherMeeting => write!(f, "別の会議あての知らせ"),
            Self::NotAMember => write!(f, "名簿に居ない相手からの下ごしらえ"),
        }
    }
}

impl core::error::Error for Error {}

/// 1 つの会議。
#[derive(Debug, Clone)]
pub struct Conference {
    me: PublicKey,
    id: MeetingId,
    roster: Roster,
}

impl Conference {
    /// 主催者ひとりから始める。
    ///
    /// # Errors
    /// 定員が `2..=HARD_LIMIT` の外なら受け取らない（**D27**）。
    pub fn host(me: PublicKey, capacity: usize) -> Result<Self, Error> {
        let roster = Roster::with_capacity(me, capacity).map_err(Error::Roster)?;
        Ok(Self {
            me,
            id: MeetingId::generate(),
            roster,
        })
    }

    /// 招かれた側として始める。名簿は招待が運んでくる（`Notice::Invite`）。
    #[must_use]
    pub fn joined(me: PublicKey, id: MeetingId, roster: Roster) -> Self {
        Self { me, id, roster }
    }

    #[must_use]
    pub fn id(&self) -> MeetingId {
        self.id
    }

    #[must_use]
    pub fn members(&self) -> &[PublicKey] {
        self.roster.members()
    }

    #[must_use]
    pub fn capacity(&self) -> usize {
        self.roster.capacity()
    }

    /// 名簿へ直接入れる（招待を受けた側が、運ばれてきた名簿を組み直すときに使う）。
    ///
    /// # Errors
    /// 定員を超えるとき。
    pub fn admit(&mut self, who: PublicKey) -> Result<(), Error> {
        self.roster.add(who).map_err(Error::Roster)
    }

    /// 知らせを 1 つ受けて、名簿を動かす。
    ///
    /// # Errors
    /// 別の会議あて・定員超過・名簿に居ない相手からの下ごしらえ。
    pub fn on_notice(&mut self, from: PublicKey, notice: &Notice) -> Result<Vec<Event>, Error> {
        if notice.meeting() != self.id {
            return Err(Error::OtherMeeting);
        }
        match notice {
            Notice::Join { .. } => self.join(from),
            Notice::Leave { .. } => Ok(self.leave(&from)),
            Notice::Signal(signal) => self.signal(from, signal),
            // 招待と回線の報せは、この層では名簿を動かさない。
            // 招待は `joined()` で始める側の入口、回線は warifu-link の担当（D28）。
            //
            // `Notice` は non_exhaustive なので、こちらが知らない知らせも来うる。
            // **名簿を動かさない**という答えは、それらにも当てはまる。
            // 握り潰しではない — この層が持っているのは名簿だけで、
            // 知らない知らせに対して名簿がすべきことは何も無い（warifu-intent が
            // 「知らない口も経路は通すが、既知のふりはしない」としているのと同じ構え）。
            _ => Ok(Vec::new()),
        }
    }

    /// **どちらが offer を出すかを、中央の調停者なしで決める。**
    ///
    /// 双方が offer を出すと衝突する（glare）。かといって「先に押したほう」にすると、
    /// 同時に押した場合が残る。**両側が同じ答えへ辿り着く規則**が要る。
    ///
    /// 公開鍵の並びで決める。鍵は 32 バイトの決まった長さで、
    /// **2 人が同じ鍵を持つことはない**（同じなら同一人物である）。
    /// だから必ず一方だけが true になる。
    #[must_use]
    pub fn should_offer_to(&self, peer: &PublicKey) -> bool {
        self.me.to_bytes() < peer.to_bytes()
    }

    fn join(&mut self, who: PublicKey) -> Result<Vec<Event>, Error> {
        if self.roster.contains(&who) {
            // 二度目は何も起きない。**握り潰しではなく、冪等である**
            return Ok(Vec::new());
        }
        self.roster.add(who).map_err(Error::Roster)?;
        Ok(vec![Event::Joined(who)])
    }

    fn leave(&mut self, who: &PublicKey) -> Vec<Event> {
        if self.roster.remove(who) {
            vec![Event::Left(*who)]
        } else {
            Vec::new()
        }
    }

    fn signal(&self, from: PublicKey, signal: &Signal) -> Result<Vec<Event>, Error> {
        if !self.roster.contains(&from) {
            return Err(Error::NotAMember);
        }
        Ok(vec![Event::Signal {
            from,
            step: signal.step(),
            blob: signal.blob().to_vec(),
        }])
    }
}

/// **どちらが呼びに行くか**（M6）。
///
/// フルメッシュでは**組ごとに 1 本だけ**張らないといけない。
/// 両側から呼びに行くと同じ組に 2 本張られ、どちらを使うかで揉める。
/// **中央の調停者は居ない**ので、D38 と同じ手で解く — **公開鍵の並び。**
///
/// **呼びに行く側が offer も出す**（`should_offer_to` と同じ向き）。
/// 別々にすると「呼んだのに offer が来ない」を追うことになる。**規則は 1 本にしておく。**
#[must_use]
pub fn should_dial(me: PublicKey, peer: PublicKey) -> bool {
    me.to_bytes() < peer.to_bytes()
}

/// 名簿のうち、**自分が呼びに行く相手**だけを取り出す。
///
/// 全員が同じ名簿を見れば、`n` 人の網は `n(n-1)/2` 本になる — **重複も欠落もない。**
#[must_use]
pub fn peers_to_dial(me: PublicKey, roster: &[PublicKey]) -> Vec<PublicKey> {
    roster
        .iter()
        .copied()
        .filter(|p| should_dial(me, *p))
        .collect()
}

/// 紹介の配り先（**D41**）。
///
/// **主催者が入った人を紹介する。**名簿は公開鍵しか運ばないので、
/// 3 人目は既に居る人の住所を知る手段が無い。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Introductions {
    /// 既に居る人たち。**「新しく入った人の住所」を伝える先。**
    pub tell_existing: Vec<PublicKey>,
    /// 新しく入った人へ伝える、**既に居る人たち。**
    pub tell_newcomer: Vec<PublicKey>,
}

/// 誰へ何を紹介するかを決める。
///
/// **紹介役は主催者だけ**（D41）。誰でも配ると同じ紹介が何度も飛び、
/// 受け取った側は「もう繋がっている相手」に何度も呼びに行くことになる。
///
/// 主催者と新入り自身は、どちらの側からも外す —
/// **主催者は会議キーで既に繋がっており、自分を自分に紹介する意味は無い。**
///
/// 主催者でなければ `None`。
#[must_use]
pub fn introductions_for(
    conference: &Conference,
    newcomer: PublicKey,
    me: PublicKey,
) -> Option<Introductions> {
    if conference.members().first() != Some(&me) {
        // 名簿の先頭が主催者（`Roster::new` がそう作る）
        return None;
    }
    let 他 = |exclude: PublicKey| -> Vec<PublicKey> {
        conference
            .members()
            .iter()
            .copied()
            .filter(|p| *p != me && *p != exclude)
            .collect()
    };
    Some(Introductions {
        tell_existing: 他(newcomer),
        tell_newcomer: 他(newcomer),
    })
}
