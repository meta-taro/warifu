//! 名前。**正規形しか受け取らない。**

use core::fmt;

use crate::Error;

/// 名前の長さの上限（バイト）。
const MAX: usize = 320;

/// 誰。
///
/// **照合は完全一致**（`warifu-read` の送信元と同じ。当たらない側に倒す）。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Subject(String);

impl Subject {
    /// 相手を作る。空・長すぎ・制御文字は受け取らない。
    pub fn new(s: &str) -> Result<Self, Error> {
        if s.is_empty() || s.len() > MAX || s.chars().any(char::is_control) {
            return Err(Error::Malformed);
        }
        Ok(Self(s.to_owned()))
    }

    /// 文字列として見る。
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Subject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// 何を。`名前空間 . 動作`。
///
/// **正規形しか受け取らない。**表記が 2 通りあると、同じ動作に札が二重にできる
/// （`warifu-intent::Kind` と同じ姿勢）。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Action(String);

impl Action {
    /// 動作を作る。小文字・数字・点だけ。点は区切りにしか置けない。
    pub fn new(s: &str) -> Result<Self, Error> {
        let 形が正しい = !s.is_empty()
            && s.len() <= 64
            && !s.starts_with('.')
            && !s.ends_with('.')
            && !s.contains("..")
            && s.bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'.');
        if !形が正しい {
            return Err(Error::Malformed);
        }
        Ok(Self(s.to_owned()))
    }

    /// 文字列として見る。
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
