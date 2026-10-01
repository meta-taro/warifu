//! 段。**上げるのは呼ぶ側だけ。**

use core::fmt;

use crate::{Body, Metadata};

/// どこまで開くか（`issues/007` の Progressive Disclosure）。
///
/// **既定は [`Level::Metadata`]。**「とりあえず [`Level::Raw`] で取る」ができてしまうと、
/// この層は無いのと同じになる。
///
/// 段には順序がある。`Level::Metadata < Level::Raw`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum Level {
    /// 0 — metadata だけ。**本文は 1 バイトも出ない。**
    #[default]
    Metadata,
    /// 1 — 要約。**解釈器が要る。**
    Summary,
    /// 2 — 構造化した本文。**規則が要る**（規則が無ければ解釈器が要る）。
    Structured,
    /// 3 — 原文。**解釈は要らない。**渡すだけ。
    Raw,
    /// 4 — 添付。
    Attachments,
}

impl Level {
    /// 数から段に戻す。**知らない数は受け取らない。**
    ///
    /// 記録を読み戻すときに使う。知らない段を「たぶん 0」に丸めると、
    /// 会計の集計が静かにずれる。
    pub(crate) fn from_number(n: u8) -> Result<Self, crate::Error> {
        match n {
            0 => Ok(Self::Metadata),
            1 => Ok(Self::Summary),
            2 => Ok(Self::Structured),
            3 => Ok(Self::Raw),
            4 => Ok(Self::Attachments),
            _ => Err(crate::Error::Malformed),
        }
    }
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let n = *self as u8;
        let 名 = match self {
            Self::Metadata => "metadata",
            Self::Summary => "summary",
            Self::Structured => "structured",
            Self::Raw => "raw",
            Self::Attachments => "attachments",
        };
        write!(f, "Level {n}（{名}）")
    }
}

/// 構造化した本文の 1 項目。
///
/// **何を抽出したかが人に読める形**になっている（`issues/007`「rule は人が見られる形で保存する」）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    name: String,
    value: String,
}

impl Field {
    /// 項目を作る。
    pub fn new(name: &str, value: &str) -> Self {
        Self {
            name: name.to_owned(),
            value: value.to_owned(),
        }
    }

    /// 項目名。
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 値。
    pub fn value(&self) -> &str {
        &self.value
    }
}

/// 添付 1 つ。
#[derive(Clone)]
pub struct Attachment {
    name: String,
    bytes: Vec<u8>,
}

impl Attachment {
    /// 添付を作る。
    pub fn new(name: &str, bytes: Vec<u8>) -> Self {
        Self {
            name: name.to_owned(),
            bytes,
        }
    }

    /// 添付の名前。**そのままパスとして使わない**のは呼ぶ側の責任。
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 添付の中身。
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

impl fmt::Debug for Attachment {
    /// 中身を出さない。
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Attachment({} バイト)", self.bytes.len())
    }
}

/// 段ごとの返り。
///
/// [`View::Metadata`] には**本文が入る場所が無い**。
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum View {
    /// Level 0。
    Metadata(Metadata),
    /// Level 1。
    Summary {
        /// 既定で返るもの。
        metadata: Metadata,
        /// 要約。
        summary: String,
    },
    /// Level 2。
    Structured {
        /// 既定で返るもの。
        metadata: Metadata,
        /// 抽出した項目。
        fields: Vec<Field>,
    },
    /// Level 3。
    Raw {
        /// 既定で返るもの。
        metadata: Metadata,
        /// 原文。
        body: Body,
    },
    /// Level 4。
    Attachments {
        /// 既定で返るもの。
        metadata: Metadata,
        /// 添付。
        attachments: Vec<Attachment>,
    },
}

impl View {
    /// どの段か。
    pub fn level(&self) -> Level {
        match self {
            Self::Metadata(_) => Level::Metadata,
            Self::Summary { .. } => Level::Summary,
            Self::Structured { .. } => Level::Structured,
            Self::Raw { .. } => Level::Raw,
            Self::Attachments { .. } => Level::Attachments,
        }
    }

    /// どの段でも metadata は付いてくる。
    pub fn metadata(&self) -> &Metadata {
        match self {
            Self::Metadata(m)
            | Self::Summary { metadata: m, .. }
            | Self::Structured { metadata: m, .. }
            | Self::Raw { metadata: m, .. }
            | Self::Attachments { metadata: m, .. } => m,
        }
    }
}
