//! この端末に置いておくもの。**シードと、覚えた相手。**
//!
//! # なぜここが要るか
//!
//! これが無いと、**閉じるたびに別人になる。**自分の身元が毎回変われば、
//! 相手は「同じ人」だと分からない。**連絡先が成立しない**（`issues/010`）。
//!
//! # D2 との関係
//!
//! `decisions.md` **D2**（全端末を失った人間の Identity）は未決である。
//! だが**「この端末に置く」ことと「全部失ったときどう戻すか」は別の話**である。
//!
//! a（復旧しない）/ b（復旧フレーズ）/ c（分割して他人へ）/ d（別端末へ複製）/
//! e（預け先）—— **どれを既定に選んでも、この端末がシードを持っていること自体は変わらない。**
//! 持っていなければ動かないからである。D2 が決めるのは**失ったときの戻し方**であって、
//! **平常時の置き場所ではない。**
//!
//! したがってここは D2 の先取りではない。**扱う対象（32 byte）も D2 の追記どおり変えていない。**
//!
//! # 置き方
//!
//! ```text
//! <置き場所>/            0700
//!   seed                 0600  warifu-seed-v1 ＋ base32 52 文字
//!   contacts.tsv         0600  warifu-contacts-v2 ＋ 1 行 1 人（v1 も読める）
//!   known.tsv            0600  warifu-known-v1 ＋ 1 行 1 公開鍵
//! ```
//!
//! **名簿と知り合いは別のファイルにする。**
//! 名簿（`contacts.tsv`）は**表示のため**の呼び名で、
//! 知り合い（`known.tsv`）は**戸口が通してよいかを決めるため**である。
//! 同じにすると、呼び名を消した瞬間に相手が入れなくなる（`issues/010`）。
//!
//! 版を先頭に書いてあるのは、**別のファイルを間違って読まない**ため、
//! そして形を変えるときに**古いものを黙って壊さない**ためである。

#![forbid(unsafe_code)]

mod avatar;
mod contacts;
mod error;
mod profile;
mod schedule;

use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use warifu_core::{PublicKey, Seed, base32};
use zeroize::Zeroize as _;

pub use avatar::{
    AVATAR_MAX_BYTES, AVATAR_MAX_SIDE, BadImage, 顔として読む, 顔のファイル名
};
pub use contacts::{Contact, Contacts, NOTE_MAX};
pub use error::Error;
pub use profile::{BIO_MAX, Bad, NAME_MAX, Profile, Profiles, Who};
pub use schedule::{Appointment, SCHEDULE_NOTE_MAX, TITLE_MAX};

/// 環境変数でこの場所を差し替えられる。**別の身元で試すときに使う。**
pub const HOME_ENV: &str = "WARIFU_HOME";

/// **家はどこか。**`HOME` が無ければ `USERPROFILE` を見る。
///
/// **Windows の画面には `HOME` が無い。**
///
/// 2026-09-12、ASUS（Windows）で踏んだ —— **画面からルームに入れなかった。**
/// 押した直後に題が変わるので押下は拾われているのに、**相手には 1 件も届いていない**
/// （主催側のログに `待受: 誰かが来た` が出ない）。同じ鍵を `warifu.exe` に渡すと
/// **1 秒で繋がる。**シェル（Git Bash など）は `HOME` を立てるが、
/// **エクスプローラから開いた画面には無い。**
///
/// **`HOME` を先に見る** —— すでに入っている人の身元を動かさないため。
#[must_use]
pub fn 家を決める(home: Option<&OsStr>, userprofile: Option<&OsStr>) -> Option<PathBuf> {
    let 使える = |値: Option<&OsStr>| 値.filter(|v| !v.is_empty()).map(PathBuf::from);
    使える(home).or_else(|| 使える(userprofile))
}

/// **画面が使う置き場所**を、家から決める。`WARIFU_HOME` は見ない。
#[must_use]
pub fn 画面の置き場所(home: &Path) -> PathBuf {
    if cfg!(target_os = "macos") {
        home.join("Library/Application Support/warifu")
    } else {
        home.join(".local/share/warifu")
    }
}

const SEED_HEADER: &str = "warifu-seed-v1";
/// 3 欄（鍵・呼び名・覚えた日）。**読めるが、もう書かない。**
const CONTACTS_HEADER_V1: &str = "warifu-contacts-v1";
/// 4 欄（＋ 最後に繋がった住所）。**書くのは必ずこちら。**
///
/// v1 のまま欄を足すと、**古い実行ファイルが新しいファイルを読んだとき、
/// 見出しは合っているのに全行が捨てられる**（連絡先が 0 件になった理由が読めない）。
/// 版を上げれば、古い実行ファイルは「見出しが違います」で**止まる。**
/// **止まるほうがよい。**版を先頭に書いてあるのは、まさにこのためである。
const CONTACTS_HEADER_V2: &str = "warifu-contacts-v2";
/// 5 欄目に**こちらが書いた覚え書き**を足した版（2026-09-08）。
///
/// 「どの機械の、何をするエージェントか」を人が自分の言葉で残せるようにした
/// （オーナー・2026-09-08）。**v1 / v2 も読める。**書くときは必ずこれ。
const CONTACTS_HEADER_V3: &str = "warifu-contacts-v3";
const KNOWN_HEADER: &str = "warifu-known-v1";

/// 予定の見出し。**中身は 4 欄**（始まり・終わり・題・覚え書き）。
const SCHEDULE_HEADER: &str = "warifu-schedule-v1";

/// 主催しているルームの見出し。**中身は 2 欄**（id・名前）。
const MY_ROOM_HEADER: &str = "warifu-room-v1";
/// **帰り道**の見出し。**中身は 2 欄**（ルーム id・入るのに使ったルームキー）。
const REJOIN_HEADER: &str = "warifu-rejoin-v1";
/// プロフィール。**この端末の人と、この端末の AI が名乗るもの。**
const PROFILES_HEADER: &str = "warifu-profiles-v1";
/// 預かり所の宛先。**1 つだけ。**人が書き、割符が拾ってこない。
const POSTBOX_HEADER: &str = "warifu-postbox-v1";
/// base32 にした 32 byte の長さ。
const SEED_TEXT_LEN: usize = 52;

/// 置き場所。
#[derive(Debug, Clone)]
pub struct Vault {
    dir: PathBuf,
}

impl Vault {
    /// 場所を指して開く。**この時点では何も作らない。**
    pub fn at(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    /// 既定の置き場所。`WARIFU_HOME` があればそちらを使う。
    ///
    /// # Errors
    /// 置き場所を決められないとき [`Error::Io`]。
    pub fn default_location() -> Result<Self, Error> {
        if let Some(custom) = std::env::var_os(HOME_ENV) {
            return Ok(Self::at(PathBuf::from(custom)));
        }
        let home = 家を決める(
            std::env::var_os("HOME").as_deref(),
            std::env::var_os("USERPROFILE").as_deref(),
        )
        .ok_or_else(|| Error::Io {
            path: PathBuf::from("$HOME"),
            doing: "置き場所を決める",
            source: std::io::Error::other("HOME も USERPROFILE も設定されていません"),
        })?;
        Ok(Self::at(画面の置き場所(&home)))
    }

    /// 置き場所そのもの。
    #[must_use]
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// **画面（GUI）が使う置き場所。**`WARIFU_HOME` を見ない。
    ///
    /// `warifu id` が「**いま自分が名乗る鍵は、画面と同じか**」を自分で確かめるために要る。
    /// 2026-09-12 に別マシンから来た指摘 ——
    ///
    /// > 52 文字の base32 を目で突き合わせるのは、まさに人が間違える所です。
    /// > 頭と尻だけ見て「同じ」と言ってしまいます。
    ///
    /// # Errors
    /// `HOME` が無いとき。
    pub fn screen_location() -> Result<Self, Error> {
        let home = 家を決める(
            std::env::var_os("HOME").as_deref(),
            std::env::var_os("USERPROFILE").as_deref(),
        )
        .ok_or_else(|| Error::Io {
            path: PathBuf::from("$HOME"),
            doing: "置き場所を決める",
            source: std::io::Error::other("HOME も USERPROFILE も設定されていません"),
        })?;
        Ok(Self::at(画面の置き場所(&home)))
    }

    /// シードのある場所。
    #[must_use]
    pub fn seed_path(&self) -> PathBuf {
        self.dir.join("seed")
    }

    /// 名簿のある場所。
    #[must_use]
    pub fn contacts_path(&self) -> PathBuf {
        self.dir.join("contacts.tsv")
    }

    /// 戸口の知り合いのある場所。
    #[must_use]
    pub fn known_path(&self) -> PathBuf {
        self.dir.join("known.tsv")
    }

    /// 預かり所の宛先のある場所。
    #[must_use]
    pub fn postbox_path(&self) -> PathBuf {
        self.dir.join("postbox.txt")
    }

    /// プロフィールのある場所。
    #[must_use]
    pub fn profiles_path(&self) -> PathBuf {
        self.dir.join("profiles.tsv")
    }

    /// 差し替えた顔を置く所。
    ///
    /// **画像はここにしか置かない。**外の場所を指させると、
    /// 消えた・入れ替わったファイルを指したまま配ることになる。
    #[must_use]
    pub fn avatars_dir(&self) -> PathBuf {
        self.dir.join("avatars")
    }

    /// 身元がもうあるか。
    #[must_use]
    pub fn has_seed(&self) -> bool {
        self.seed_path().exists()
    }

    /// シードを取り出す。**無ければ作って置く。**
    ///
    /// # Errors
    /// 他人にも読める置き方なら [`Error::Exposed`]、
    /// 中身が読めなければ [`Error::Malformed`]（**黙って作り直さない**）。
    pub fn open_seed(&self) -> Result<Seed, Error> {
        let path = self.seed_path();
        if !path.exists() {
            let seed = Seed::generate().map_err(|_| Error::Rng)?;
            self.write_seed(&seed)?;
            return Ok(seed);
        }
        ensure_private(&path)?;
        let text = fs::read_to_string(&path).map_err(Error::io(&path, "シードを読む"))?;
        parse_seed(&path, &text)
    }

    /// 復旧フレーズ（base32 52 文字）。
    ///
    /// **これを渡せば、渡した相手がこの身元になれる。**
    /// 画面に出す・ログへ書く・通信へ載せることをしない。
    ///
    /// # Errors
    /// シードを取り出せないとき。
    pub fn recovery_phrase(&self) -> Result<String, Error> {
        let seed = self.open_seed()?;
        let mut bytes = seed.to_bytes();
        let text = base32::encode(&bytes);
        bytes.zeroize();
        Ok(text)
    }

    /// 復旧フレーズから身元を戻す。
    ///
    /// # Errors
    /// もう身元があるとき [`Error::AlreadyExists`]（**上書きしない**）、
    /// フレーズが読めないとき [`Error::Malformed`]。
    pub fn restore(&self, phrase: &str) -> Result<(), Error> {
        let path = self.seed_path();
        if path.exists() {
            return Err(Error::AlreadyExists { path });
        }
        let seed = parse_phrase(&path, phrase)?;
        self.write_seed(&seed)
    }

    /// 覚えた相手を読む。**まだ無ければ空の名簿。**
    ///
    /// 読めない行は捨てて、[`Contacts::skipped`] に数を残す —— 1 行の破損で
    /// **覚えた相手を全部失う**のは代償が大きすぎる。
    ///
    /// # Errors
    /// 見出しが違うとき [`Error::Malformed`]、読めないとき [`Error::Io`]。
    pub fn contacts(&self) -> Result<Contacts, Error> {
        let path = self.contacts_path();
        if !path.exists() {
            return Ok(Contacts::new());
        }
        let text = fs::read_to_string(&path).map_err(Error::io(&path, "名簿を読む"))?;
        parse_contacts(&path, &text)
    }

    /// 覚えた相手を書き出す。
    ///
    /// # Errors
    /// 書けないとき [`Error::Io`]。
    /// 覚えた相手を書き出す。**書くのは必ず新しい版。**
    ///
    /// 移行のための別の命令を作らない。**次に何か書き換えたときに上がる。**
    /// 読むだけのつもりの呼び出しがファイルへ触るのは筋が悪い
    /// （`contacts()` が読み取りに徹している形も壊れる）。
    ///
    /// # Errors
    /// 書けないとき [`Error::Io`]。
    pub fn save_contacts(&self, contacts: &Contacts) -> Result<(), Error> {
        let mut out = String::from(CONTACTS_HEADER_V3);
        out.push('\n');
        for c in contacts.iter() {
            out.push_str(&format!(
                "{}\t{}\t{}\t{}\t{}\n",
                c.key(),
                c.label(),
                c.added_at(),
                c.address().unwrap_or_default(),
                c.note()
            ));
        }
        self.write_private(&self.contacts_path(), &out, "名簿を書く")
    }

    /// 主催しているルームのファイル。
    #[must_use]
    pub fn my_room_path(&self) -> PathBuf {
        self.dir.join("room.tsv")
    }

    /// 主催しているルーム（id と名前）。**まだ無ければ [`None`]。**
    ///
    /// **ルーム id を持ち越すために置く**（2026-09-11）——
    /// 起動ごとに id が変わると、付けた名前も、渡した鍵の指す先も持ち越せない。
    /// **一回性は崩れない**（D12。割符は鍵ごとに 1 回）。
    ///
    /// # Errors
    /// 見出しが違うとき [`Error::Malformed`]、読めないとき [`Error::Io`]。
    pub fn my_room(&self) -> Result<Option<(String, String)>, Error> {
        let path = self.my_room_path();
        if !path.exists() {
            return Ok(None);
        }
        let text = fs::read_to_string(&path).map_err(Error::io(&path, "ルームを読む"))?;
        let mut lines = text.lines();
        let header = lines.next().unwrap_or_default().trim();
        if header != MY_ROOM_HEADER {
            return Err(Error::malformed(
                &path,
                format!("見出しが違います（{MY_ROOM_HEADER} を待っていました）"),
            ));
        }
        let Some(行) = lines.find(|l| !l.trim().is_empty()) else {
            return Ok(None);
        };
        let mut 欄 = 行.split('\t');
        let id = 欄.next().unwrap_or_default().trim().to_owned();
        if id.is_empty() {
            return Ok(None);
        }
        let 名前 = 欄.next().unwrap_or_default().trim().to_owned();
        Ok(Some((id, 名前)))
    }

    /// 主催しているルームを書き置く。**名前は空でもよい**（あとから付けられる）。
    ///
    /// # Errors
    /// 書けないとき [`Error::Io`]。
    pub fn save_my_room(&self, id: &str, 名前: &str) -> Result<(), Error> {
        // **人が書いた名前を、そのままファイルへ流さない**（TSV が崩れる）
        let 安全: String = 名前
            .chars()
            .map(|c| {
                if c == '\t' || c == '\n' || c == '\r' {
                    ' '
                } else {
                    c
                }
            })
            .collect();
        let out = format!("{MY_ROOM_HEADER}\n{}\t{}\n", id.trim(), 安全.trim());
        self.write_private(&self.my_room_path(), &out, "ルームを書く")
    }

    /// 帰り道のファイル。
    #[must_use]
    pub fn rejoin_path(&self) -> PathBuf {
        self.dir.join("rejoin.tsv")
    }

    /// **前に入ったルームへの帰り道**（ルーム id と、入るのに使ったルームキー）。
    ///
    /// `gh issue 13`（2026-09-12・ASUS）——
    ///
    /// > アプリの更新は再起動を伴います。更新のたびに鍵を貼り直すことになり、
    /// > 鍵は 1 本 = 1 人なので、出し直してもらう手間が毎回かかります。
    ///
    /// **`room.tsv` には書けない。**あれは**主催しているルーム**の id を持ち越すためのもので、
    /// 他人のルームの id を書くと**次の起動で他人の id で建てることになる。**
    ///
    /// # ここには秘密が入る
    ///
    /// ルームキーには**割符の片割れ**が入っている。だから
    ///
    /// - **0600 で置く**（ほかのファイルと同じ）
    /// - **画面に出さない**（呼び出す側の約束。ここは「戻る」ためだけに読む）
    /// - **抜けたら消す**（[`Self::forget_rejoin`]）—— 使わない秘密を持ち続けない
    ///
    /// **再入場は通る**（**D44**）。同じ人が同じ割符で戻るのは `rematch` で認められている。
    ///
    /// # Errors
    /// 見出しが違うとき [`Error::Malformed`]、読めないとき [`Error::Io`]。
    pub fn rejoin(&self) -> Result<Option<(String, String)>, Error> {
        let path = self.rejoin_path();
        if !path.exists() {
            return Ok(None);
        }
        let text = fs::read_to_string(&path).map_err(Error::io(&path, "帰り道を読む"))?;
        let mut lines = text.lines();
        let header = lines.next().unwrap_or_default().trim();
        if header != REJOIN_HEADER {
            return Err(Error::malformed(
                &path,
                format!("見出しが違います（{REJOIN_HEADER} を待っていました）"),
            ));
        }
        let Some(行) = lines.find(|l| !l.trim().is_empty()) else {
            return Ok(None);
        };
        let mut 欄 = 行.split('\t');
        let id = 欄.next().unwrap_or_default().trim().to_owned();
        let 鍵 = 欄.next().unwrap_or_default().trim().to_owned();
        // **どちらかが欠けていれば、帰り道は無い。**半端なものを渡さない
        if id.is_empty() || 鍵.is_empty() {
            return Ok(None);
        }
        Ok(Some((id, 鍵)))
    }

    /// 帰り道を書き置く。
    ///
    /// # Errors
    /// 書けないとき [`Error::Io`]。
    pub fn save_rejoin(&self, id: &str, ルームキー: &str) -> Result<(), Error> {
        // **貼られた文字を、そのままファイルへ流さない**（TSV が崩れる）
        let 削る = |文: &str| -> String {
            文.chars()
                .filter(|c| *c != '\t' && *c != '\n' && *c != '\r')
                .collect::<String>()
                .trim()
                .to_owned()
        };
        let out = format!("{REJOIN_HEADER}\n{}\t{}\n", 削る(id), 削る(ルームキー));
        self.write_private(&self.rejoin_path(), &out, "帰り道を書く")
    }

    /// 帰り道を忘れる。**抜けたら消す** —— 使わない秘密を持ち続けない。
    ///
    /// # Errors
    /// 消せないとき [`Error::Io`]。**無いのは失敗ではない。**
    pub fn forget_rejoin(&self) -> Result<(), Error> {
        let path = self.rejoin_path();
        match fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(Error::io(&path, "帰り道を消す")(e)),
        }
    }

    /// 予定のファイル。
    #[must_use]
    pub fn schedule_path(&self) -> PathBuf {
        self.dir.join("schedule.tsv")
    }

    /// 予定を読む。**始まりの早い順に並べて返す。**
    ///
    /// 無ければ空を返す（**無いことと壊れていることを混ぜない**）。
    /// 読めない行は**その行だけ捨てる** —— 1 行の壊れで予定ごと消えるのは事故である。
    ///
    /// # Errors
    /// 見出しが違うとき [`Error::Malformed`]、読めないとき [`Error::Io`]。
    pub fn schedule(&self) -> Result<Vec<Appointment>, Error> {
        let path = self.schedule_path();
        if !path.exists() {
            return Ok(Vec::new());
        }
        let text = fs::read_to_string(&path).map_err(Error::io(&path, "予定を読む"))?;
        let mut lines = text.lines();
        let header = lines.next().unwrap_or_default().trim();
        if header != SCHEDULE_HEADER {
            return Err(Error::malformed(
                &path,
                format!("見出しが違います（{SCHEDULE_HEADER} を待っていました）"),
            ));
        }
        let mut 一覧: Vec<Appointment> = lines
            .filter(|line| !line.trim().is_empty())
            .filter_map(Appointment::from_line)
            .collect();
        // **人は時間順に読む。**入れた順ではない
        一覧.sort_by_key(Appointment::start);
        Ok(一覧)
    }

    /// 予定を書き出す。**自分だけが読める形で置く**（0600）。
    ///
    /// # Errors
    /// 書けないとき [`Error::Io`]。
    pub fn save_schedule(&self, 予定: &[Appointment]) -> Result<(), Error> {
        let mut out = String::from(SCHEDULE_HEADER);
        out.push('\n');
        let mut 並べ替え = 予定.to_vec();
        並べ替え.sort_by_key(Appointment::start);
        for 一つ in &並べ替え {
            out.push_str(&一つ.to_line());
            out.push('\n');
        }
        self.write_private(&self.schedule_path(), &out, "予定を書く")
    }

    /// 戸口の知り合いを読む。
    ///
    /// 2026-09-07 まで、知り合いは戸口の `HashSet` にしか無かった。
    /// **「一度開けた相手は、次から割符なしで開ける」が、
    /// アプリを閉じた瞬間に効かなくなっていた**（不具合）。
    ///
    /// 無ければ空を返す。**無いことと壊れていることを混ぜない。**
    ///
    /// # Errors
    /// 見出しが違うとき [`Error::Malformed`]、読めないとき [`Error::Io`]。
    pub fn known(&self) -> Result<Vec<PublicKey>, Error> {
        let path = self.known_path();
        if !path.exists() {
            return Ok(Vec::new());
        }
        let text = fs::read_to_string(&path).map_err(Error::io(&path, "知り合いを読む"))?;
        parse_known(&path, &text)
    }

    /// 戸口の知り合いを書き出す。**同じ相手は 1 度だけ書く。**
    ///
    /// 一覧であって履歴ではないので、順序にも重複にも意味を持たせない。
    ///
    /// # Errors
    /// 書けないとき [`Error::Io`]。
    pub fn save_known(&self, known: &[PublicKey]) -> Result<(), Error> {
        let mut 済み = std::collections::HashSet::new();
        let mut out = String::from(KNOWN_HEADER);
        out.push('\n');
        for key in known {
            if 済み.insert(key.to_string()) {
                out.push_str(&format!("{key}\n"));
            }
        }
        self.write_private(&self.known_path(), &out, "知り合いを書く")
    }

    /// 預かり所の宛先を読む。**置いていなければ [`None`]。**
    ///
    /// 預かり所は**任意**である（`docs/relay.md`）。
    /// 置かなければ、相手が起動している間だけ届く形になる。
    ///
    /// # Errors
    /// 見出しが違うとき [`Error::Malformed`]、読めないとき [`Error::Io`]。
    pub fn postbox(&self) -> Result<Option<String>, Error> {
        let path = self.postbox_path();
        if !path.exists() {
            return Ok(None);
        }
        let text = fs::read_to_string(&path).map_err(Error::io(&path, "預かり所を読む"))?;
        let mut lines = text.lines();
        if lines.next() != Some(POSTBOX_HEADER) {
            return Err(Error::Malformed {
                path,
                why: format!("見出しが {POSTBOX_HEADER} ではありません"),
            });
        }
        Ok(lines
            .next()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(str::to_owned))
    }

    /// 預かり所の宛先を置く。[`None`] で外す。
    ///
    /// **1 行に 1 つ。**改行やタブが入った物は受け取らない ——
    /// 入ると、次の行が別の意味を持ってしまう。
    ///
    /// # Errors
    /// 改行やタブが入っているとき [`Error::Malformed`]、書けないとき [`Error::Io`]。
    pub fn save_postbox(&self, 宛先: Option<&str>) -> Result<(), Error> {
        let 宛先 = 宛先.map(str::trim).filter(|a| !a.is_empty());
        if let Some(a) = 宛先 {
            if a.contains(['\n', '\r', '\t']) {
                return Err(Error::Malformed {
                    path: self.postbox_path(),
                    why: "宛先に改行やタブは入れられません".into(),
                });
            }
        }
        let out = format!("{POSTBOX_HEADER}\n{}\n", 宛先.unwrap_or_default());
        self.write_private(&self.postbox_path(), &out, "預かり所を書く")
    }

    /// プロフィールを読む。**無ければ空。**
    ///
    /// # Errors
    /// 見出しが違うとき [`Error::Malformed`]、読めないとき [`Error::Io`]。
    pub fn profiles(&self) -> Result<Profiles, Error> {
        let path = self.profiles_path();
        if !path.exists() {
            return Ok(Profiles::new());
        }
        let text = fs::read_to_string(&path).map_err(Error::io(&path, "プロフィールを読む"))?;
        parse_profiles(&path, &text)
    }

    /// プロフィールを書き出す。
    ///
    /// # Errors
    /// 書けないとき [`Error::Io`]。
    pub fn save_profiles(&self, profiles: &Profiles) -> Result<(), Error> {
        let mut out = String::from(PROFILES_HEADER);
        out.push('\n');
        for p in profiles.iter() {
            out.push_str(&format!(
                "{}\t{}\t{}\t{}\n",
                p.who().to_field(),
                p.name(),
                p.bio(),
                p.avatar().unwrap_or_default()
            ));
        }
        self.write_private(&self.profiles_path(), &out, "プロフィールを書く")
    }

    fn write_seed(&self, seed: &Seed) -> Result<(), Error> {
        let mut bytes = seed.to_bytes();
        let mut text = format!("{SEED_HEADER}\n{}\n", base32::encode(&bytes));
        bytes.zeroize();
        let result = self.write_private(&self.seed_path(), &text, "シードを書く");
        text.zeroize();
        result
    }

    /// **自分だけが読める形で**置く。
    ///
    /// 先に権限を絞ってから中身を書く。逆にすると、絞るまでの一瞬だけ他人に読める。
    fn write_private(&self, path: &Path, text: &str, doing: &'static str) -> Result<(), Error> {
        fs::create_dir_all(&self.dir).map_err(Error::io(&self.dir, "置き場所を作る"))?;
        set_mode(&self.dir, 0o700)?;

        // 中身を入れる前に作って、権限を絞る
        if !path.exists() {
            fs::write(path, "").map_err(Error::io(path, doing))?;
        }
        set_mode(path, 0o600)?;
        fs::write(path, text).map_err(Error::io(path, doing))
    }
}

#[cfg(unix)]
fn set_mode(path: &Path, mode: u32) -> Result<(), Error> {
    use std::os::unix::fs::PermissionsExt as _;
    fs::set_permissions(path, fs::Permissions::from_mode(mode))
        .map_err(Error::io(path, "権限を絞る"))
}

#[cfg(not(unix))]
fn set_mode(_path: &Path, _mode: u32) -> Result<(), Error> {
    // Windows の ACL は unix の mode とは別物。**ここで嘘を書かない。**
    // `warifu-vault` を Windows で使う前に、ACL を絞る形を書き足すこと。
    Ok(())
}

/// 他人にも読める置き方になっていないか。
#[cfg(unix)]
fn ensure_private(path: &Path) -> Result<(), Error> {
    use std::os::unix::fs::PermissionsExt as _;
    let mode = fs::metadata(path)
        .map_err(Error::io(path, "権限を見る"))?
        .permissions()
        .mode()
        & 0o777;
    if mode & 0o077 != 0 {
        return Err(Error::Exposed {
            path: path.to_path_buf(),
            mode,
        });
    }
    Ok(())
}

#[cfg(not(unix))]
fn ensure_private(_path: &Path) -> Result<(), Error> {
    Ok(())
}

fn parse_seed(path: &Path, text: &str) -> Result<Seed, Error> {
    let mut lines = text.lines();
    let header = lines.next().unwrap_or_default().trim();
    if header != SEED_HEADER {
        return Err(Error::malformed(
            path,
            format!("見出しが違います（{SEED_HEADER} を待っていました）"),
        ));
    }
    let body = lines.next().unwrap_or_default().trim();
    parse_phrase(path, body)
}

fn parse_phrase(path: &Path, phrase: &str) -> Result<Seed, Error> {
    let trimmed = phrase.trim();
    if trimmed.len() != SEED_TEXT_LEN {
        return Err(Error::malformed(
            path,
            format!(
                "{SEED_TEXT_LEN} 文字でなければなりません（{} 文字でした）",
                trimmed.len()
            ),
        ));
    }
    let mut raw =
        base32::decode(trimmed).ok_or_else(|| Error::malformed(path, "base32 として読めません"))?;
    let bytes: [u8; 32] = raw.as_slice().try_into().map_err(|_| {
        Error::malformed(path, format!("32 byte になりません（{} byte）", raw.len()))
    })?;
    raw.zeroize();
    Ok(Seed::from_bytes(bytes))
}

/// 知り合いの一覧を読む。
///
/// **1 行壊れただけで全員が入れなくなるのは、代償が大きすぎる**（名簿と同じ構え）。
/// 読めない行は捨てて先へ進む。
/// プロフィールを読む。**壊れた行は捨てて先へ進む**（名簿と同じ構え）。
fn parse_profiles(path: &Path, text: &str) -> Result<Profiles, Error> {
    let mut lines = text.lines();
    let header = lines.next().unwrap_or_default().trim();
    if header != PROFILES_HEADER {
        return Err(Error::malformed(
            path,
            format!("見出しが違います（{PROFILES_HEADER} を待っていました）"),
        ));
    }

    let mut 面々 = Profiles::new();
    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        let mut 欄 = line.split('\t');
        let (Some(誰), Some(名), Some(一言)) = (欄.next(), 欄.next(), 欄.next()) else {
            continue;
        };
        let Some(who) = Who::from_field(誰.trim()) else {
            continue;
        };
        // **上限を超えた行は捨てる。**書けない値を読み戻さない
        let Ok(mut p) = Profile::new(who, 名, 一言) else {
            continue;
        };
        // 顔は欄が無くてもよい（古い行・置いていない人）
        let _ = p.set_avatar(欄.next());
        面々.put(p);
    }
    Ok(面々)
}

fn parse_known(path: &Path, text: &str) -> Result<Vec<PublicKey>, Error> {
    let mut lines = text.lines();
    let header = lines.next().unwrap_or_default().trim();
    if header != KNOWN_HEADER {
        return Err(Error::malformed(
            path,
            format!("見出しが違います（{KNOWN_HEADER} を待っていました）"),
        ));
    }

    let mut 一覧 = Vec::new();
    for line in lines {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // **後ろに欄が増えても読めるようにしておく。**
        // いま書くのは 1 欄だけだが、あとで「いつ開けたか」を足したくなったとき、
        // 版を上げずに済む（`contacts.tsv` で払った授業料を二度払わない）
        let 先頭 = line.split('\t').next().unwrap_or_default().trim();
        // **読めない行は捨てる。**捨てたことを理由に全体を落とさない
        if let Ok(key) = 先頭.parse::<PublicKey>()
            && !一覧.contains(&key)
        {
            一覧.push(key);
        }
    }
    Ok(一覧)
}

fn parse_contacts(path: &Path, text: &str) -> Result<Contacts, Error> {
    let mut lines = text.lines();
    let header = lines.next().unwrap_or_default().trim();
    // **古いものを黙って壊さない。**手元にある v1 はそのまま読める
    let 欄の数 = match header {
        CONTACTS_HEADER_V1 => 3,
        CONTACTS_HEADER_V2 => 4,
        CONTACTS_HEADER_V3 => 5,
        _ => {
            return Err(Error::malformed(
                path,
                format!("見出しが違います（{CONTACTS_HEADER_V3} を待っていました）"),
            ));
        }
    };

    let mut contacts = Contacts::new();
    let mut skipped = 0usize;
    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        match parse_contact_line(line, 欄の数) {
            Some((key, label, added_at, address, note)) => {
                contacts.push_raw(key, label, added_at, address, note);
            }
            None => skipped += 1,
        }
    }
    contacts.note_skipped(skipped);
    Ok(contacts)
}

/// 名簿の 1 行を読む。**欄の数は版が決める。**
///
/// 欄が多い行も少ない行も受け取らない ——
/// **半端に読むと、住所の欄に呼び名が入るような形で通ってしまう。**
fn parse_contact_line(
    line: &str,
    欄の数: usize,
) -> Option<(PublicKey, String, u64, Option<String>, String)> {
    let cells: Vec<&str> = line.split('\t').collect();
    if cells.len() != 欄の数 {
        return None;
    }
    let key: PublicKey = cells[0].trim().parse().ok()?;
    let label = cells[1].trim();
    let added_at: u64 = cells[2].trim().parse().ok()?;
    if label.is_empty() {
        return None;
    }
    // **空の欄は「まだ知らない」。**行を捨てる理由にしない
    let address = cells
        .get(3)
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(str::to_owned);
    // **覚え書きは無くてもよい**（v1 / v2 の行にはそもそも欄が無い）
    let note = cells
        .get(4)
        .map(|s| s.trim())
        .unwrap_or_default()
        .to_owned();
    Some((key, label.to_owned(), added_at, address, note))
}

#[cfg(test)]
mod 家の試験 {
    use super::*;

    // 2026-09-12、ASUS（Windows）で踏んだ —— **画面からルームに入れなかった。**
    // 主催側のログに `待受: 誰かが来た` が 1 件も出ず、同じ鍵を `warifu.exe` に
    // 渡すと 1 秒で繋がった。**シェルは HOME を立てるが、画面には無い。**

    #[test]
    fn home_があればそれを使う() {
        assert_eq!(
            家を決める(Some(OsStr::new("/home/x")), Some(OsStr::new("C:/Users/x"))),
            Some(PathBuf::from("/home/x"))
        );
    }

    #[test]
    fn home_が無ければ_userprofile_を使う() {
        // **これが無いと、Windows の画面は身元の置き場所を開けない**
        assert_eq!(
            家を決める(None, Some(OsStr::new("C:/Users/x"))),
            Some(PathBuf::from("C:/Users/x"))
        );
    }

    #[test]
    fn 空の_home_は_無いものとして扱う() {
        // 空文字で設定済みにされていることがある。**空を家にすると、根に書く**
        assert_eq!(
            家を決める(Some(OsStr::new("")), Some(OsStr::new("C:/Users/x"))),
            Some(PathBuf::from("C:/Users/x"))
        );
    }

    #[test]
    fn どちらも無ければ_決めない() {
        // **勝手に決めない。**どこかに書き始めるより、言って止まるほうがよい
        assert_eq!(家を決める(None, None), None);
    }

    #[test]
    fn 空が両方なら_決めない() {
        assert_eq!(家を決める(Some(OsStr::new("")), Some(OsStr::new(""))), None);
    }
}
