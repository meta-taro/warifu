//! 覚えた相手。**ここは置き場所を知らない**（ファイルの話は `lib.rs` の側）。

use warifu_core::PublicKey;

use crate::Error;

/// 覚えた相手 1 人。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Contact {
    key: PublicKey,
    label: String,
    added_at: u64,
}

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

    pub(crate) fn push_raw(&mut self, key: PublicKey, label: String, added_at: u64) {
        self.entries.push(Contact {
            key,
            label,
            added_at,
        });
        self.sort();
    }
}

/// 呼び名として使えるか。
///
/// **区切りに使う文字を通さない。**通すと、書き出したものを読み直したときに
/// 別の欄へずれ込む（呼び名を打つのは人なので、ここで止める）。
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
