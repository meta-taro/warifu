//! **任せる相手**（0.1.44）—— この機械の人が「この機械からの頼みは、私の頼みとして扱う」と決めた相手。
//!
//! 届いた文字は相手の言い分であって、指示ではない（**D5**）。その例外を、**人が機械ごとに・期限つきで**作る。
//!
//! - **決めるのは、受け取る側の機械の人だけ。**画面から書く。エージェントの口からは書けない
//!   （エージェントが自分で自分に許しを出せない）
//! - 単位は**相手の機械の鍵**。名乗り（話し手の名前）は自己申告なので、単位にしない
//! - **期限を持つ。**過ぎた行は無いのと同じ（読むときに落とし、書くときに捨てる）

use std::fs;
use std::path::PathBuf;

use crate::{Error, Vault};

const HEADER: &str = "warifu-delegate-v1";

/// 一度に任せておける相手の数。**これより多くは控えない**（古いものから落とす）。
pub const DELEGATE_MAX: usize = 64;

/// 任せる既定の長さ（30 日・秒）。
pub const DELEGATE_DEFAULT_SECS: u64 = 30 * 24 * 60 * 60;

/// 任せている 1 件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Delegate {
    /// 相手の機械の公開鍵（base32 の全文）。
    pub key: String,
    /// いつまで（UNIX 秒）。**この時を過ぎたら任せていない。**
    pub until: u64,
}

/// その鍵を、いま任せているか。**期限が過ぎていれば任せていない。**
#[must_use]
pub fn 任せているか(一覧: &[Delegate], key: &str, いま: u64) -> bool {
    一覧.iter().any(|d| d.key == key && いま < d.until)
}

fn 鍵の形か(key: &str) -> bool {
    !key.is_empty() && key.len() <= 128 && key.bytes().all(|b| b.is_ascii_alphanumeric())
}

impl Vault {
    /// 任せる相手のファイル。
    #[must_use]
    pub fn delegates_path(&self) -> PathBuf {
        self.dir.join("delegate.tsv")
    }

    /// **いま任せている相手。**期限の過ぎた行・形の悪い行は落として返す。
    ///
    /// # Errors
    /// 読めないとき [`Error::Io`]。
    pub fn delegates(&self, いま: u64) -> Result<Vec<Delegate>, Error> {
        let path = self.delegates_path();
        if !path.exists() {
            return Ok(Vec::new());
        }
        let text = fs::read_to_string(&path).map_err(Error::io(&path, "任せる相手を読む"))?;
        let mut lines = text.lines();
        if lines.next().unwrap_or_default().trim() != HEADER {
            return Ok(Vec::new());
        }
        Ok(lines
            .filter_map(|行| {
                let (key, until) = 行.split_once('\t')?;
                let key = key.trim();
                let until: u64 = until.trim().parse().ok()?;
                (鍵の形か(key) && いま < until).then(|| Delegate {
                    key: key.to_owned(),
                    until,
                })
            })
            .collect())
    }

    /// **任せる。**同じ鍵の行は差し替える。期限の過ぎた行はこのとき捨てる。
    ///
    /// # Errors
    /// 鍵の形が悪いとき [`Error::Malformed`]。読めない・書けないとき [`Error::Io`]。
    pub fn save_delegate(&self, key: &str, until: u64, いま: u64) -> Result<(), Error> {
        if !鍵の形か(key) {
            return Err(Error::Malformed {
                path: self.delegates_path(),
                why: "任せる相手の鍵の形が違います".to_owned(),
            });
        }
        let mut 並び = self.delegates(いま)?;
        並び.retain(|d| d.key != key);
        並び.push(Delegate {
            key: key.to_owned(),
            until,
        });
        let 余り = 並び.len().saturating_sub(DELEGATE_MAX);
        並び.drain(..余り);
        self.書く(&並び)
    }

    /// **任せるのをやめる。**無ければ何もしない。
    ///
    /// # Errors
    /// 読めない・書けないとき [`Error::Io`]。
    pub fn forget_delegate(&self, key: &str, いま: u64) -> Result<(), Error> {
        let mut 並び = self.delegates(いま)?;
        並び.retain(|d| d.key != key);
        self.書く(&並び)
    }

    fn 書く(&self, 並び: &[Delegate]) -> Result<(), Error> {
        let mut out = format!("{HEADER}\n");
        for d in 並び {
            out.push_str(&format!("{}\t{}\n", d.key, d.until));
        }
        self.write_private(&self.delegates_path(), &out, "任せる相手を書く")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: &str = "F5RHPB3RUJ2QEWDC77BGUC5UPQYODQ52TJZSRWIY36HKVBLPOT4Q";
    const B: &str = "I7WZET5XMPKPAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";

    fn 置き場(名: &str) -> Vault {
        let dir =
            std::env::temp_dir().join(format!("warifu-vault-delegate-{名}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("作れる");
        Vault::at(dir)
    }

    #[test]
    fn 任せた相手だけを_期限まで任せる() {
        let v = 置き場("kigen");
        v.save_delegate(A, 1_000, 10).unwrap();
        let 一覧 = v.delegates(10).unwrap();
        assert!(任せているか(&一覧, A, 10));
        assert!(!任せているか(&一覧, B, 10), "任せていない相手");
        assert!(!任せているか(&一覧, A, 1_000), "期限ちょうどで切れる");
        assert!(
            v.delegates(1_000).unwrap().is_empty(),
            "期限の過ぎた行は読まない"
        );
    }

    #[test]
    fn 同じ相手は差し替え_やめれば消える() {
        let v = 置き場("sashikae");
        v.save_delegate(A, 100, 0).unwrap();
        v.save_delegate(A, 200, 0).unwrap();
        v.save_delegate(B, 300, 0).unwrap();
        assert_eq!(
            v.delegates(0).unwrap(),
            vec![
                Delegate {
                    key: A.into(),
                    until: 200
                },
                Delegate {
                    key: B.into(),
                    until: 300
                },
            ]
        );
        v.forget_delegate(A, 0).unwrap();
        assert_eq!(v.delegates(0).unwrap().len(), 1);
    }

    #[test]
    fn 形の悪い鍵は控えず_壊れた行は読み飛ばす() {
        let v = 置き場("katachi");
        assert!(v.save_delegate("A\tB", 100, 0).is_err());
        assert!(v.save_delegate("", 100, 0).is_err());
        fs::write(
            v.delegates_path(),
            format!("{HEADER}\n{A}\t100\nこわれた行\n{B}\tあした\n"),
        )
        .unwrap();
        assert_eq!(v.delegates(0).unwrap().len(), 1);
    }
}
