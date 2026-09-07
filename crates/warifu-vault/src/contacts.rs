//! 覚えた相手。**ここは置き場所を知らない**（ファイルの話は `lib.rs` の側）。

use warifu_core::PublicKey;

use crate::Error;

/// 覚えた相手 1 人。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Contact {
    key: PublicKey,
    label: String,
    added_at: u64,
    address: Option<String>,
}

/// 住所の長さの上限（バイト）。
///
/// `warifu-meeting` が知らせで運ぶ住所と同じ値にする。
/// **書けても読み戻せない行を作らない。**
const ADDRESS_MAX: usize = 1024;

impl Contact {
    /// 相手の公開鍵。**warifu ではこれが相手の名前そのもの。**
    #[must_use]
    pub fn key(&self) -> PublicKey {
        self.key
    }

    /// こちらが付けた呼び名。**相手が名乗ったものではない。**
    ///
    /// 名乗りを信じると、同じ名前を名乗る別人が入り込める。
    /// 呼び名は**こちらの手元にしかない**ラベルである。
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }

    /// 覚えた時刻（Unix 秒）。
    #[must_use]
    pub fn added_at(&self) -> u64 {
        self.added_at
    }

    /// **最後に繋がったときの住所。**まだ知らなければ `None`。
    ///
    /// **1 つしか持たない。**居場所の履歴にしない（`issues/010` の止めるべき条件）。
    /// 中身は解釈しない —— 住所を読むのは経路の層の仕事であり、
    /// ここは置き場所である（`Notice::Introduce` が文字列で持つのと同じ構え）。
    ///
    /// **当たる保証は無い。**当たらなければ会議キーを渡してもらう。
    #[must_use]
    pub fn address(&self) -> Option<&str> {
        self.address.as_deref()
    }
}

/// 覚えた相手の一覧。
#[derive(Debug, Clone, Default)]
pub struct Contacts {
    entries: Vec<Contact>,
    skipped: usize,
}

impl Contacts {
    /// 空の名簿。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 覚えている人数。
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 誰も覚えていないか。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 読めずに捨てた行の数。**0 でないことを、呼ぶ側が人へ伝えること。**
    #[must_use]
    pub fn skipped(&self) -> usize {
        self.skipped
    }

    pub(crate) fn note_skipped(&mut self, count: usize) {
        self.skipped = count;
    }

    /// 呼び名の順に見る。
    pub fn iter(&self) -> impl Iterator<Item = &Contact> {
        self.entries.iter()
    }

    /// 鍵で引く。
    #[must_use]
    pub fn find(&self, key: PublicKey) -> Option<&Contact> {
        self.entries.iter().find(|c| c.key == key)
    }

    /// 呼び名で引く。
    #[must_use]
    pub fn find_by_label(&self, label: &str) -> Option<&Contact> {
        self.entries.iter().find(|c| c.label == label)
    }

    /// 覚える。**同じ鍵なら呼び名を付け直すだけ**（覚えた日は動かさない）。
    ///
    /// # Errors
    /// 呼び名が使えないとき [`Error::BadLabel`]、
    /// 別人が同じ呼び名を使っているとき [`Error::DuplicateLabel`]。
    pub fn add(&mut self, key: PublicKey, label: &str, now: u64) -> Result<(), Error> {
        let label = check_label(label)?;

        if self
            .entries
            .iter()
            .any(|c| c.label == label && c.key != key)
        {
            return Err(Error::DuplicateLabel { label });
        }

        if let Some(existing) = self.entries.iter_mut().find(|c| c.key == key) {
            existing.label = label; // 呼び名は付け直せる。覚えた日はそのまま
        } else {
            self.entries.push(Contact {
                key,
                label,
                added_at: now,
                address: None,
            });
        }
        self.sort();
        Ok(())
    }

    /// 忘れる。覚えていなければ `false`。
    pub fn remove(&mut self, key: PublicKey) -> bool {
        let before = self.entries.len();
        self.entries.retain(|c| c.key != key);
        self.entries.len() != before
    }

    fn sort(&mut self) {
        self.entries.sort_by(|a, b| a.label.cmp(&b.label));
    }

    pub(crate) fn push_raw(
        &mut self,
        key: PublicKey,
        label: String,
        added_at: u64,
        address: Option<String>,
    ) {
        self.entries.push(Contact {
            key,
            label,
            added_at,
            address,
        });
        self.sort();
    }

    /// **最後に繋がった住所**を書き留める。覚えていない相手なら `false`。
    ///
    /// **行を作らない。**呼び名の無い相手を連絡帳に出さないため
    /// （住所だけの行は、人が見ても誰か分からない）。
    ///
    /// 二度書けば**上書きする。**足していかない ——
    /// 溜めると居場所の履歴になる（`issues/010` の止めるべき条件）。
    ///
    /// # Errors
    /// 区切りを壊す住所・空・長すぎるとき [`Error::BadAddress`]。
    pub fn note_address(&mut self, key: PublicKey, address: &str) -> Result<bool, Error> {
        let address = check_address(address)?;
        let Some(existing) = self.entries.iter_mut().find(|c| c.key == key) else {
            return Ok(false);
        };
        existing.address = Some(address);
        Ok(true)
    }
}

/// 呼び名として使えるか。
///
/// **区切りに使う文字を通さない。**通すと、書き出したものを読み直したときに
/// 別の欄へずれ込む（呼び名を打つのは人なので、ここで止める）。
/// 住所が**置ける形か**だけを見る。
///
/// **中身が正しい住所かは見ない。**それは経路の層の仕事である。
/// ここが見るのは「この行を書いて、読み戻せるか」だけ。
fn check_address(address: &str) -> Result<String, Error> {
    let trimmed = address.trim();
    if trimmed.is_empty() {
        return Err(Error::BadAddress { why: "空です" });
    }
    if trimmed.contains('\t') {
        return Err(Error::BadAddress {
            why: "タブは使えません（欄の区切りに使っています）",
        });
    }
    if trimmed.contains(['\n', '\r']) {
        return Err(Error::BadAddress {
            why: "改行は使えません（行の区切りに使っています）",
        });
    }
    if trimmed.len() > ADDRESS_MAX {
        return Err(Error::BadAddress {
            why: "長すぎます（1024 バイトまで）",
        });
    }
    Ok(trimmed.to_owned())
}

fn check_label(label: &str) -> Result<String, Error> {
    let trimmed = label.trim();
    if trimmed.is_empty() {
        return Err(Error::BadLabel { why: "空です" });
    }
    if trimmed.contains('\t') {
        return Err(Error::BadLabel {
            why: "タブは使えません（欄の区切りに使っています）",
        });
    }
    if trimmed.contains(['\n', '\r']) {
        return Err(Error::BadLabel {
            why: "改行は使えません（行の区切りに使っています）",
        });
    }
    if trimmed.chars().count() > 64 {
        return Err(Error::BadLabel {
            why: "長すぎます（64 文字まで）",
        });
    }
    Ok(trimmed.to_owned())
}
