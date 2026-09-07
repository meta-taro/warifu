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

mod contacts;
mod error;

use std::fs;
use std::path::{Path, PathBuf};

use warifu_core::{PublicKey, Seed, base32};
use zeroize::Zeroize as _;

pub use contacts::{Contact, Contacts};
pub use error::Error;

/// 環境変数でこの場所を差し替えられる。**別の身元で試すときに使う。**
pub const HOME_ENV: &str = "WARIFU_HOME";

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
const KNOWN_HEADER: &str = "warifu-known-v1";
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
        let home = std::env::var_os("HOME").ok_or_else(|| Error::Io {
            path: PathBuf::from("$HOME"),
            doing: "置き場所を決める",
            source: std::io::Error::other("HOME が設定されていません"),
        })?;
        let base = PathBuf::from(home);
        let dir = if cfg!(target_os = "macos") {
            base.join("Library/Application Support/warifu")
        } else {
            base.join(".local/share/warifu")
        };
        Ok(Self::at(dir))
    }

    /// 置き場所そのもの。
    #[must_use]
    pub fn dir(&self) -> &Path {
        &self.dir
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
        let mut out = String::from(CONTACTS_HEADER_V2);
        out.push('\n');
        for c in contacts.iter() {
            out.push_str(&format!(
                "{}\t{}\t{}\t{}\n",
                c.key(),
                c.label(),
                c.added_at(),
                c.address().unwrap_or_default()
            ));
        }
        self.write_private(&self.contacts_path(), &out, "名簿を書く")
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
        _ => {
            return Err(Error::malformed(
                path,
                format!("見出しが違います（{CONTACTS_HEADER_V2} を待っていました）"),
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
            Some((key, label, added_at, address)) => {
                contacts.push_raw(key, label, added_at, address);
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
) -> Option<(PublicKey, String, u64, Option<String>)> {
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
    Some((key, label.to_owned(), added_at, address))
}
