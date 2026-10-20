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
/// 会議の中で画像を送る（チャット）。
const IMAGE: &str = "meeting.image";
/// **本人の名乗り**（プロフィール・**D75**）。
const PROFILE: &str = "meeting.profile";
/// **ルームキーを、その相手にだけ手渡す**（2026-10-20）。
const KEY: &str = "meeting.key";

/// 手渡すルームキーの長さの上限（バイト）。**受け取る側でも数える**（D15）。
/// いまの鍵は 400 バイトほど。宛先が長くなっても 4 KiB には収まる。
pub const KEY_MAX: usize = 4 * 1024;

/// ルームキーの頭。**これで始まらないものは鍵として運ばない。**
const KEY_PREFIX: &str = "WARIFU1-";

/// **部屋の合言葉を渡す口**（**D118**）。
///
/// **主催だけが送る。**戸口を通った直後の相手へ、その経路で渡す。
/// **鍵（`WARIFU1-…`）には載せない** —— 鍵は public な所に貼られる。
/// **口の名前に `-` は入れられない**（`warifu-intent` の `Kind` が
/// 小文字と数字だけを通す）。2026-09-17 に `meeting.room-secret` で落ちた。
const ROOM_SECRET: &str = "meeting.roomsecret";

/// 測定値の塊の長さ。`[上り 8][下り 8][経過秒 4]`。
const LINK_LEN: usize = 20;
/// 住所の長さの上限。**受け取る側でも数える**（D15）。
/// `WARIFU1-` + base32 の宛先は数百バイトに収まる。1 KiB あれば足りる。
const ADDRESS_MAX: usize = 1024;
/// 文字の長さの上限（バイト）。**受け取る側でも数える**（D15）。
/// 会議の中の一言に 16 KiB は十分で、これを超えるなら別の手段で渡すべきものである。
const TEXT_MAX: usize = 16 * 1024;

/// 画像の大きさの上限（バイト）。**受け取る側でも数える**（D15）。
///
/// 画面で縮めてから送る（長い辺 1600px・JPEG）。読める写真やスクリーンショットは
/// これに収まる。**原寸を渡す口ではない** —— 原寸が要るなら別の手段で渡す。
pub const IMAGE_MAX: usize = 512 * 1024;

/// 運べる画像の型。**描く側が安全に描けるものだけ。**
///
/// SVG は入れない —— 中に script を持てる。
const IMAGE_TYPES: [&str; 4] = ["image/jpeg", "image/png", "image/webp", "image/gif"];

/// 名乗りの名前の長さ（バイト）。**画面の 1 行に収まる長さ**
/// （`warifu-vault` の 32 文字を、日本語 1 文字 4 バイトで見込む）。
const PROFILE_NAME_MAX: usize = 128;
/// 名乗りの紹介の長さ（バイト）。**140 文字を同じ見込みで。**
const PROFILE_BIO_MAX: usize = 560;

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
        /// **そのエージェントの誰が言ったか**（人なら `None`）。
        ///
        /// この機械（**D55**）で同じ PC の AI が喋れるようになったが、
        /// 2026-09-08 まで**その発言は `from` に人の公開鍵を載せて飛んでいた。**
        /// こちらの画面では「docs の AI」と出るのに、
        /// **相手の画面では人が打ったのと区別が付かなかった。**
        ///
        /// この機械の中では「**繋いできた側は差出人を名乗れない。誰が言ったかはこの機械が刻む**」を
        /// 守っている。**線の向こうでも同じにする**ための札である。
        ///
        /// **`from` を置き換えるものではない。**エージェントの持ち主は `from` のまま ——
        /// 混ぜると、AI が別人のエージェントから喋れることになる。
        話し手: Option<String>,
        /// 中身。
        body: String,
    },
    /// **会議の中で画像を送る**（チャット）。
    ///
    /// 差出人と札の扱いは [`Notice::Text`] と同じ。**絵は読まない** ——
    /// ただし**頭のバイトが `型` と合うか**だけは見る。受け取った画面は、
    /// 名乗った型で描くからである。
    Image {
        /// どの会議か。
        meeting: MeetingId,
        /// 誰が送ったか（[`Notice::Text`] の `from` と同じ）。
        from: PublicKey,
        /// そのエージェントの誰が送ったか（人なら `None`）。
        話し手: Option<String>,
        /// 型（`image/jpeg` など）。[`IMAGE_TYPES`] のどれか。
        型: String,
        /// 中身。空は送れない。[`IMAGE_MAX`] まで。
        中身: Vec<u8>,
    },
    /// **ルームキーを、その相手にだけ手渡す**（2026-10-20）。
    ///
    /// 人が別の道（メール・チャットのアプリ）で渡すと、**鍵が外の網に出る。**
    /// 割符の線は暗号化されていて、相手の機械に直接届くので、ここで渡す。
    ///
    /// **送る側は、人が画面で承認してから送る。**受け取った側も、
    /// **入るかは人が画面で決める**（届いただけでは入らない・**D79** と同じ）。
    /// `meeting` は**この線で共にしている会議**であり、鍵の行き先とは限らない。
    Key {
        /// この線で共にしている会議（経路の宛先として使う）。
        meeting: MeetingId,
        /// 誰が渡したか。**受け取った側は、経路の相手と同じかを確かめる。**
        from: PublicKey,
        /// ルームキー（`WARIFU1-…`）。[`KEY_MAX`] まで。
        鍵: String,
    },
    /// **本人の名乗り**（**D75**）。名前と短い紹介を相手へ渡す。
    ///
    /// **これは本人確認ではない。**名乗った名前は誰でも真似できるので、
    /// **受け取った側が付けた呼び名があれば、そちらが勝つ**（**D46**）。
    /// 確かめるのは鍵である。
    ///
    /// **空にすると「名乗りを取り消す」意味になる**（消したことも伝わる必要がある）。
    Profile {
        /// どの会議か。
        meeting: MeetingId,
        /// 誰の名乗りか。**経路で確定した相手と一致するはず**である。
        from: PublicKey,
        /// 表に出す名前。
        名前: String,
        /// 短い紹介。
        紹介: String,
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
    /// **部屋の合言葉を渡す**（**D118** / **#28** の本線 D）。
    ///
    /// # なぜ要るか
    ///
    /// **割符は 2 人の間のもの。**主催とゲスト A、主催とゲスト B ——
    /// **A と B の間には何も無い。**だから A が B を呼んでも、
    /// **B の戸口が黙って落とす**（**D31** の正しい拒否）。
    ///
    /// **合言葉を渡せば、A は B へ「この部屋の者だ」と示せる。**
    ///
    /// # これは秘密である
    ///
    /// **これを持っている人は、その部屋へ誰でも入れられる。**
    ///
    /// - **主催だけが送る。**受けた側が、さらに配り直さない
    /// - **戸口を通った相手にだけ送る。**通る前に渡す理由が無い
    /// - **記録に書かない。**`知らせの名` は種別だけを言う
    ///
    /// # 型で守っている
    ///
    /// **`[u8; 32]` を直に持たない。**`Notice` は `#[derive(Debug)]` なので、
    /// **生のバイト列を持たせると `{:?}` 1 つで合言葉が記録へ落ちる。**
    /// [`warifu_core::合言葉`] は `Debug` が「合言葉(出しません)」としか言わない ——
    /// **忘れても漏れない形**にしてある。
    RoomSecret {
        /// どの会議か。
        meeting: MeetingId,
        /// 合言葉。**`Debug` では中身が出ない。**
        合言葉: warifu_core::合言葉,
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
            | Self::Text { meeting, .. }
            | Self::Image { meeting, .. }
            | Self::Profile { meeting, .. }
            | Self::Key { meeting, .. }
            | Self::RoomSecret { meeting, .. } => *meeting,
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
            Self::Image {
                from,
                話し手,
                型,
                中身,
                ..
            } => (IMAGE, 画像を詰める(from, 話し手.as_deref(), 型, 中身)?),
            Self::Key { from, 鍵, .. } => {
                鍵を検める(鍵)?;
                let mut 荷物 = Vec::with_capacity(32 + 鍵.len());
                荷物.extend_from_slice(&from.to_bytes());
                荷物.extend_from_slice(鍵.as_bytes());
                (KEY, 荷物)
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
            Self::Profile {
                from, 名前, 紹介,
            ..
            } => {
                // **行と欄を壊すものを通さない。**通すと、受け取った側の画面が崩れる
                if 名前.len() > PROFILE_NAME_MAX
                    || 紹介.len() > PROFILE_BIO_MAX
                    || 名前.chars().any(|c| c.is_control())
                    || 紹介.chars().any(char::is_control)
                {
                    return Err(Error::Malformed);
                }
                // `[差出人 32][名前の長さ 1][名前][紹介]`
                let mut 塊 = Vec::with_capacity(32 + 1 + 名前.len() + 紹介.len());
                塊.extend_from_slice(&from.to_bytes());
                塊.push(u8::try_from(名前.len()).map_err(|_| Error::Malformed)?);
                塊.extend_from_slice(名前.as_bytes());
                塊.extend_from_slice(紹介.as_bytes());
                (PROFILE, 塊)
            }
            Self::RoomSecret { 合言葉, .. } => (ROOM_SECRET, 合言葉.バイト列().to_vec()),
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
            IMAGE => 画像を読む(meeting, 荷物),
            KEY => {
                // `[差出人 32][鍵]`。**組み立てる側と同じ検めをする**（相手が守る保証は無い）
                let 頭: [u8; 32] = 荷物
                    .get(..32)
                    .ok_or(Error::Malformed)?
                    .try_into()
                    .map_err(|_| Error::Malformed)?;
                let from = PublicKey::from_bytes(頭).map_err(|_| Error::Malformed)?;
                let 鍵 = String::from_utf8(荷物[32..].to_vec()).map_err(|_| Error::Malformed)?;
                鍵を検める(&鍵)?;
                Ok(Self::Key { meeting, from, 鍵 })
            }
            PROFILE => {
                // 差出人 32 byte ＋ 名前の長さ 1 byte ＋ 名前 ＋ 紹介。
                // **空の名乗りは通す** —— 「取り消した」ことも伝わる必要がある
                if 荷物.len() < 33 || 荷物.len() > 32 + 1 + PROFILE_NAME_MAX + PROFILE_BIO_MAX {
                    return Err(Error::Malformed);
                }
                let from =
                    PublicKey::from_bytes(荷物[..32].try_into().map_err(|_| Error::Malformed)?)
                        .map_err(|_| Error::Malformed)?;
                let 名前の長さ = usize::from(荷物[32]);
                if 名前の長さ > PROFILE_NAME_MAX || 荷物.len() < 33 + 名前の長さ {
                    return Err(Error::Malformed);
                }
                Ok(Self::Profile {
                    meeting,
                    from,
                    名前: String::from_utf8_lossy(&荷物[33..33 + 名前の長さ]).into_owned(),
                    紹介: String::from_utf8_lossy(&荷物[33 + 名前の長さ..]).into_owned(),
                })
            }
            ROOM_SECRET => {
                // **長さが違うものは通さない。**短いものを通すと、
                // **弱い合言葉を押し込める**（相手が上限を守る保証は無い）
                let 中身: [u8; 32] = 荷物.try_into().map_err(|_| Error::Malformed)?;
                Ok(Self::RoomSecret {
                    meeting,
                    合言葉: warifu_core::合言葉::から(中身),
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

/// 鍵として運べる形か。**頭が `WARIFU1-` で、空白や制御文字を含まず、長すぎない。**
///
/// 中身（宛先・割符）までは読まない —— 入るときに画面の側が読む。
/// ここで止めるのは、**鍵でないもの（本文・命令）を鍵の口で運ばせない**ためである。
fn 鍵を検める(鍵: &str) -> Result<(), Error> {
    if 鍵.len() > KEY_MAX
        || !鍵.starts_with(KEY_PREFIX)
        || 鍵.len() == KEY_PREFIX.len()
        || !鍵.bytes().all(|b| b.is_ascii_graphic())
    {
        return Err(Error::Malformed);
    }
    Ok(())
}

/// 札が行と欄を壊さないか（[`Notice::Text`] と同じ決まり）。
fn 札を検める(札: Option<&str>) -> Result<(), Error> {
    let Some(札) = 札 else { return Ok(()) };
    if 札.len() > SPEAKER_MAX
        || 札.chars().any(|c| c.is_control() || c == '\t')
        || 札.trim().is_empty()
    {
        return Err(Error::Malformed);
    }
    Ok(())
}

/// 頭のバイトが、名乗った型と合うか。**描ける型だけを通す。**
fn 画像を検める(型: &str, 中身: &[u8]) -> Result<(), Error> {
    if 中身.is_empty() || 中身.len() > IMAGE_MAX || !IMAGE_TYPES.contains(&型) {
        return Err(Error::Malformed);
    }
    let 合う = match 型 {
        "image/jpeg" => 中身.starts_with(&[0xFF, 0xD8, 0xFF]),
        "image/png" => 中身.starts_with(b"\x89PNG\r\n\x1a\n"),
        "image/gif" => 中身.starts_with(b"GIF87a") || 中身.starts_with(b"GIF89a"),
        "image/webp" => 中身.len() >= 12 && &中身[..4] == b"RIFF" && &中身[8..12] == b"WEBP",
        _ => false,
    };
    if 合う {
        Ok(())
    } else {
        Err(Error::Malformed)
    }
}

/// `[差出人 32][札の長さ 1][札][型の長さ 1][型][中身]`
fn 画像を詰める(
    from: &PublicKey,
    話し手: Option<&str>,
    型: &str,
    中身: &[u8],
) -> Result<Vec<u8>, Error> {
    札を検める(話し手)?;
    画像を検める(型, 中身)?;
    let 札 = 話し手.unwrap_or_default();
    let mut 荷物 = Vec::with_capacity(32 + 2 + 札.len() + 型.len() + 中身.len());
    荷物.extend_from_slice(&from.to_bytes());
    荷物.push(u8::try_from(札.len()).map_err(|_| Error::Malformed)?);
    荷物.extend_from_slice(札.as_bytes());
    荷物.push(u8::try_from(型.len()).map_err(|_| Error::Malformed)?);
    荷物.extend_from_slice(型.as_bytes());
    荷物.extend_from_slice(中身);
    Ok(荷物)
}

/// [`画像を詰める`] の逆。**組み立てる側と同じ検めをする**（相手が守る保証は無い）。
fn 画像を読む(meeting: MeetingId, 荷物: &[u8]) -> Result<Notice, Error> {
    let 頭: [u8; 32] = 荷物
        .get(..32)
        .ok_or(Error::Malformed)?
        .try_into()
        .map_err(|_| Error::Malformed)?;
    let from = PublicKey::from_bytes(頭).map_err(|_| Error::Malformed)?;
    let mut 位置 = 32;
    let mut 切る = |荷物: &[u8]| -> Result<String, Error> {
        let 長さ = usize::from(*荷物.get(位置).ok_or(Error::Malformed)?);
        let 欄 = 荷物
            .get(位置 + 1..位置 + 1 + 長さ)
            .ok_or(Error::Malformed)?;
        位置 += 1 + 長さ;
        String::from_utf8(欄.to_vec()).map_err(|_| Error::Malformed)
    };
    let 札 = 切る(荷物)?;
    let 型 = 切る(荷物)?;
    let 話し手 = (!札.is_empty()).then_some(札);
    札を検める(話し手.as_deref())?;
    let 中身 = 荷物[位置..].to_vec();
    画像を検める(&型, &中身)?;
    Ok(Notice::Image {
        meeting,
        from,
        話し手,
        型,
        中身,
    })
}
