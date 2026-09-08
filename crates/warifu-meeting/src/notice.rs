//! 会議の知らせ。[`Intent`] との行き来だけを持つ。

use warifu_core::PublicKey;
use warifu_intent::{Intent, Kind};

use warifu_link::Report;

use crate::{Error, MeetingId, Roster, Signal};

/// 招集。
const INVITE: &str = "meeting.invite";
/// 参加。
const JOIN: &str = "meeting.join";
/// 退出。
const LEAVE: &str = "meeting.leave";
/// 下ごしらえ。
const SIGNAL: &str = "meeting.signal";
/// 測った回線。
const LINK: &str = "meeting.link";
/// 紹介（**D41**）。3 人目が既存の面々の住所を知るための知らせ。
const INTRODUCE: &str = "meeting.introduce";
/// 会議の中で文字を送る（チャット）。
const TEXT: &str = "meeting.text";

/// 測定値の塊の長さ。`[上り 8][下り 8][経過秒 4]`。
const LINK_LEN: usize = 20;
/// 住所の長さの上限。**受け取る側でも数える**（D15）。
/// `WARIFU1-` + base32 の宛先は数百バイトに収まる。1 KiB あれば足りる。
const ADDRESS_MAX: usize = 1024;
/// 文字の長さの上限（バイト）。**受け取る側でも数える**（D15）。
/// 会議の中の一言に 16 KiB は十分で、これを超えるなら別の手段で渡すべきものである。
const TEXT_MAX: usize = 16 * 1024;

/// 話し手の札の長さ（バイト）。
///
/// **画面の 1 行に収まる長さに切る。**`warifu-desk` が 32 文字で切っているのに合わせ、
/// 日本語 1 文字 4 バイトを見込んで 128 バイトにする。
const SPEAKER_MAX: usize = 128;

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
    /// **誰かの住所を紹介する**（**D41**）。
    ///
    /// 3 人目が入ったとき、**既に居る人の住所を知る手段が無い**（名簿は公開鍵しか運ばない）。
    /// 主催者がこれを配る。
    ///
    /// **住所の中身は解釈しない。**この層は経路（`warifu-net`）を知らないので、
    /// そのまま渡す文字列として持つ（SDP を読まないのと同じ構え）。
    ///
    /// **これは中継ではない。**流れるのは住所だけで、
    /// 繋がった後の映像は当人どうしを直接流れる（D7 に触れない・D41）。
    Introduce {
        /// どの会議か。
        meeting: MeetingId,
        /// 誰の住所か。
        who: PublicKey,
        /// 住所そのもの。**読まない。**
        address: String,
    },
    /// **会議の中で文字を送る**（チャット）。
    ///
    /// **中身を検めない。**warifu は文字の意味を知らない（SDP を読まないのと同じ）。
    /// 改行も絵文字もそのまま通る。
    ///
    /// **空は送れない。**中身の無い通知で相手の注意を消費できてはいけない
    /// （D31 が「通知を出せること自体が資源」と書いた筋）。
    Text {
        /// どの会議か。
        meeting: MeetingId,
        /// **誰が言ったか**（**D48**）。
        ///
        /// 三者会議は星形である —— 参加者どうしは繋がっておらず、
        /// **主催だけが全員と繋がっている。**主催が聞いた文字をほかの人へ配るとき、
        /// **これが載っていないと、配られた側には全部が主催の発言に見える。**
        ///
        /// **直接届いた文字では、経路で確定した相手と一致するはず**である。
        /// 一致しないものを通すかは、受け取る側が決める（この層は運ぶだけ）。
        from: PublicKey,
        /// **その席の誰が言ったか**（人なら `None`）。
        ///
        /// 机（**D55**）で同じ PC の AI が喋れるようになったが、
        /// 2026-09-08 まで**その発言は `from` に人の公開鍵を載せて飛んでいた。**
        /// こちらの画面では「zumen の AI」と出るのに、
        /// **相手の画面では人が打ったのと区別が付かなかった。**
        ///
        /// 机の中では「**繋いできた側は差出人を名乗れない。誰が言ったかは机が刻む**」を
        /// 守っている。**線の向こうでも同じにする**ための札である。
        ///
        /// **`from` を置き換えるものではない。**席の持ち主は `from` のまま ——
        /// 混ぜると、AI が別人の席から喋れることになる。
        話し手: Option<String>,
        /// 中身。
        body: String,
    },
    /// 測った回線を渡す。
    ///
    /// **これは申告ではなく観測**（`warifu-link` の `Meter`）。
    /// 受け取った側では、**送る量を下げる方向にしか効かない**（`decisions.md` **D28**）。
    /// 相手が「下り 1 Gbps」と言っても、出せるのは自分の上りまでである。
    Link {
        /// どの会議か。
        meeting: MeetingId,
        /// 測定値。**絶対時刻ではなく経過秒で運ぶ**（時計のずれで壊れないため）。
        report: Report,
    },
}

impl Notice {
    /// どの会議のものか。
    #[must_use]
    pub fn meeting(&self) -> MeetingId {
        match self {
            Self::Invite { meeting, .. }
            | Self::Join { meeting }
            | Self::Leave { meeting }
            | Self::Link { meeting, .. }
            | Self::Introduce { meeting, .. }
            | Self::Text { meeting, .. } => *meeting,
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
            Self::Text {
                from, 話し手, body,
            ..
            } => {
                if body.is_empty() || body.len() > TEXT_MAX {
                    return Err(Error::Malformed);
                }
                let 札 = 話し手.as_deref().unwrap_or_default();
                // **行と欄を壊すものを通さない。**通すと、受け取った側の画面が崩れる
                if 札.len() > SPEAKER_MAX
                    || 札.chars().any(|c| c.is_control() || c == '\t')
                    || (話し手.is_some() && 札.trim().is_empty())
                {
                    return Err(Error::Malformed);
                }
                // **差出人を先頭に固定長で置く。**後ろに置くと、中身との境目が要る。
                // 札は長さを 1 byte 前置きする（0 なら札なし＝人が言った）
                let mut 荷物 = Vec::with_capacity(32 + 1 + 札.len() + body.len());
                荷物.extend_from_slice(&from.to_bytes());
                荷物.push(u8::try_from(札.len()).map_err(|_| Error::Malformed)?);
                荷物.extend_from_slice(札.as_bytes());
                荷物.extend_from_slice(body.as_bytes());
                (TEXT, 荷物)
            }
            Self::Introduce { who, address, .. } => {
                if address.len() > ADDRESS_MAX {
                    return Err(Error::Malformed);
                }
                let mut 塊 = Vec::with_capacity(32 + address.len());
                塊.extend_from_slice(&who.to_bytes());
                塊.extend_from_slice(address.as_bytes());
                (INTRODUCE, 塊)
            }
            Self::Link { report, .. } => {
                let mut 塊 = Vec::with_capacity(LINK_LEN);
                塊.extend_from_slice(&report.uplink_bps().to_be_bytes());
                塊.extend_from_slice(&report.downlink_bps().to_be_bytes());
                塊.extend_from_slice(&report.age_secs().to_be_bytes());
                (LINK, 塊)
            }
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
            TEXT => {
                // 差出人 32 byte ＋ 札の長さ 1 byte ＋ 札 ＋ 中身。
                // **中身が空のものは通さない**
                if 荷物.len() < 34 || 荷物.len() > 32 + 1 + SPEAKER_MAX + TEXT_MAX {
                    return Err(Error::Malformed);
                }
                let from =
                    PublicKey::from_bytes(荷物[..32].try_into().map_err(|_| Error::Malformed)?)
                        .map_err(|_| Error::Malformed)?;
                let 札の長さ = usize::from(荷物[32]);
                if 札の長さ > SPEAKER_MAX || 荷物.len() <= 33 + 札の長さ {
                    return Err(Error::Malformed);
                }
                let 話し手 = if 札の長さ == 0 {
                    None
                } else {
                    Some(String::from_utf8_lossy(&荷物[33..33 + 札の長さ]).into_owned())
                };
                // **中身を検めない。**読めないバイト列でも、そのまま文字にして渡す
                Ok(Self::Text {
                    meeting,
                    from,
                    話し手,
                    body: String::from_utf8_lossy(&荷物[33 + 札の長さ..]).into_owned(),
                })
            }
            INTRODUCE => {
                if 荷物.len() < 32 || 荷物.len() > 32 + ADDRESS_MAX {
                    return Err(Error::Malformed);
                }
                let 鍵: [u8; 32] = 荷物[..32].try_into().expect("長さは確認済み");
                let who = PublicKey::from_bytes(鍵).map_err(|_| Error::Malformed)?;
                // **住所は解釈しない。**読めないバイト列でも、そのまま文字列にして渡す
                let address = String::from_utf8_lossy(&荷物[32..]).into_owned();
                Ok(Self::Introduce {
                    meeting,
                    who,
                    address,
                })
            }
            LINK => {
                if 荷物.len() != LINK_LEN {
                    return Err(Error::Malformed);
                }
                // 長さを確かめてあるので、切り出しは必ず成功する
                let 数 = |a: usize, b: usize| -> u64 {
                    u64::from_be_bytes(荷物[a..b].try_into().expect("長さは確認済み"))
                };
                Ok(Self::Link {
                    meeting,
                    report: Report::new(
                        数(0, 8),
                        数(8, 16),
                        u32::from_be_bytes(荷物[16..20].try_into().expect("長さは確認済み")),
                    ),
                })
            }
            // `file.offer` はもちろん、知らない `meeting.*` もここに落ちる。
            // **知らないものを知っているふりはしない**（warifu-intent と同じ構え）
            _ => Err(Error::NotMeeting),
        }
    }
}
