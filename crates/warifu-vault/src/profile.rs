//! **プロフィール。**この端末の人と、この端末の AI が名乗るもの。
//!
//! オーナー指示（2026-09-08）——
//! 「**名前と、github のアバターアイコンみたいなものも設置したい。
//!   すると、先方にもそれがみれます。**」
//! 「**このPCのAIもまた一人の人としてプロフィールを持てるようにします。
//!   エージェントも人です。その管理はこのPCの持ち主が編集可能とします。**」
//!
//! # ここで守ること
//!
//! - **名乗りは本人確認にしない。**確かめるのは鍵である（**D46** は残る）。
//!   相手が付けた呼び名があれば、**そちらが勝つ**
//! - **書き換えられるのは、この端末の持ち主だけ。**
//!   AI が自分の名前を書き換える口は作らない（作ると、
//!   **同じ机の別のエージェントに化けられる**）
//! - **顔は既定で鍵から描く。**差し替えたときだけ、置いた画像を指す

/// 名前の上限（文字）。**画面の 1 行に収まる長さ。**
pub const NAME_MAX: usize = 32;
/// 一言の上限（文字）。**プロフィールであって、文書ではない。**
pub const BIO_MAX: usize = 140;

/// 誰のプロフィールか。
///
/// **鍵で分けない。**この端末の AI は、席の持ち主（＝この端末）の鍵で喋る（**D48**）ので、
/// 鍵で分けると人と AI が同じ 1 つになってしまう。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Who {
    /// この端末の人。
    Me,
    /// この端末の AI。**どこで動いているかの名乗り**で分ける（**D62**）。
    Desk(String),
}

impl Who {
    /// ファイルに書く形。
    #[must_use]
    pub fn to_field(&self) -> String {
        match self {
            Self::Me => "me".to_owned(),
            Self::Desk(名) => format!("desk:{名}"),
        }
    }

    /// ファイルの 1 欄から読む。**読めなければ `None`。**
    #[must_use]
    pub fn from_field(欄: &str) -> Option<Self> {
        if 欄 == "me" {
            return Some(Self::Me);
        }
        let 名 = 欄.strip_prefix("desk:")?;
        if 名.is_empty() {
            return None;
        }
        Some(Self::Desk(名.to_owned()))
    }
}

/// 1 人ぶんのプロフィール。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profile {
    who: Who,
    name: String,
    bio: String,
    /// 差し替えた顔の置き場所（ファイル名）。**空なら鍵から描く。**
    avatar: Option<String>,
}

/// 書けない値だったとき。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Bad {
    /// 名前が長すぎる。
    NameTooLong,
    /// 一言が長すぎる。
    BioTooLong,
    /// 行を壊す文字（タブ・改行）が入っている。
    Malformed,
}

impl core::fmt::Display for Bad {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NameTooLong => write!(f, "名前が長すぎます（{NAME_MAX} 文字まで）"),
            Self::BioTooLong => write!(f, "紹介が長すぎます（{BIO_MAX} 文字まで）"),
            Self::Malformed => write!(f, "改行やタブは入れられません"),
        }
    }
}

impl core::error::Error for Bad {}

/// 行を壊す文字が無いか。
fn 一行か(値: &str) -> bool {
    !値.contains(['\t', '\n', '\r'])
}

impl Profile {
    /// 作る。**上限を超えたら断る**（切り詰めない —— 黙って削ると、
    /// 書いた人は削られたことに気づかない）。
    ///
    /// # Errors
    /// 長すぎるとき、行を壊す文字が入っているとき。
    pub fn new(who: Who, name: &str, bio: &str) -> Result<Self, Bad> {
        let name = name.trim();
        let bio = bio.trim();
        if !一行か(name) || !一行か(bio) {
            return Err(Bad::Malformed);
        }
        if name.chars().count() > NAME_MAX {
            return Err(Bad::NameTooLong);
        }
        if bio.chars().count() > BIO_MAX {
            return Err(Bad::BioTooLong);
        }
        Ok(Self {
            who,
            name: name.to_owned(),
            bio: bio.to_owned(),
            avatar: None,
        })
    }

    /// 誰のものか。
    #[must_use]
    pub fn who(&self) -> &Who {
        &self.who
    }

    /// 名乗っている名前。**空なら名乗っていない。**
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 短い紹介。**空なら書いていない。**
    #[must_use]
    pub fn bio(&self) -> &str {
        &self.bio
    }

    /// 差し替えた顔。**`None` なら鍵から描く。**
    #[must_use]
    pub fn avatar(&self) -> Option<&str> {
        self.avatar.as_deref()
    }

    /// 顔を差し替える。`None` で既定（鍵から描く）へ戻す。
    ///
    /// # Errors
    /// 行を壊す文字が入っているとき。
    pub fn set_avatar(&mut self, 場所: Option<&str>) -> Result<(), Bad> {
        let 場所 = 場所.map(str::trim).filter(|a| !a.is_empty());
        if let Some(a) = 場所 {
            if !一行か(a) {
                return Err(Bad::Malformed);
            }
        }
        self.avatar = 場所.map(str::to_owned);
        Ok(())
    }
}

/// この端末のプロフィールたち。**人が 1 つと、机の席のぶん。**
#[derive(Debug, Clone, Default)]
pub struct Profiles {
    面々: Vec<Profile>,
}

impl Profiles {
    /// 空で作る。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 何人ぶんあるか。
    #[must_use]
    pub fn len(&self) -> usize {
        self.面々.len()
    }

    /// 1 つも無いか。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.面々.is_empty()
    }

    /// 順に見る。
    pub fn iter(&self) -> impl Iterator<Item = &Profile> {
        self.面々.iter()
    }

    /// その人のものを引く。
    #[must_use]
    pub fn find(&self, who: &Who) -> Option<&Profile> {
        self.面々.iter().find(|p| &p.who == who)
    }

    /// 置く。**同じ人のものは 1 つだけ**（上書きする）。
    pub fn put(&mut self, profile: Profile) {
        if let Some(場所) = self.面々.iter().position(|p| p.who == profile.who) {
            self.面々[場所] = profile;
            return;
        }
        self.面々.push(profile);
    }

    /// 消す。**無ければ何もしない。**
    pub fn forget(&mut self, who: &Who) -> bool {
        let 前 = self.面々.len();
        self.面々.retain(|p| &p.who != who);
        self.面々.len() != 前
    }
}
