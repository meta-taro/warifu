//! この層が返す誤り。**握り潰さず、何が起きたかを名前で言う。**

use core::fmt;
use std::path::PathBuf;

/// 置き場所で起きること。
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// 読み書きそのものが失敗した。
    Io {
        /// どこで。
        path: PathBuf,
        /// 何をしていて。
        doing: &'static str,
        /// OS の言い分。
        source: std::io::Error,
    },
    /// 中身が読めない形になっている。**黙って作り直さない。**
    Malformed {
        /// どこが。
        path: PathBuf,
        /// どう読めなかったか。
        why: String,
    },
    /// 他人にも読める置き方になっている。
    Exposed {
        /// どこが。
        path: PathBuf,
        /// いまの権限。
        mode: u32,
    },
    /// もう身元がある所へ、別の身元を入れようとした。
    AlreadyExists {
        /// どこに。
        path: PathBuf,
    },
    /// 同じ呼び名が既にある。
    DuplicateLabel {
        /// 重なった呼び名。
        label: String,
    },
    /// 呼び名として使えない。
    BadLabel {
        /// なぜ使えないか。
        why: &'static str,
    },
    /// 乱数が取れなかった。
    Rng,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io {
                path,
                doing,
                source,
            } => {
                write!(f, "{doing}のに失敗しました（{}）: {source}", path.display())
            }
            Self::Malformed { path, why } => {
                write!(f, "中身を読めませんでした（{}）: {why}", path.display())
            }
            Self::Exposed { path, mode } => write!(
                f,
                "他人にも読める置き方になっています（{} は {mode:o}）。\
                 `chmod 600` で自分だけにしてください",
                path.display()
            ),
            Self::AlreadyExists { path } => write!(
                f,
                "もう身元があります（{}）。\
                 上書きすると、いまの身元とつながりが消えます。消してよいなら先に退避してください",
                path.display()
            ),
            Self::DuplicateLabel { label } => {
                write!(f, "その呼び名はもう使われています: {label}")
            }
            Self::BadLabel { why } => write!(f, "呼び名として使えません: {why}"),
            Self::Rng => f.write_str("乱数を取れませんでした"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl Error {
    pub(crate) fn io<'a>(
        path: &'a std::path::Path,
        doing: &'static str,
    ) -> impl FnOnce(std::io::Error) -> Self + 'a {
        move |source| Self::Io {
            path: path.to_path_buf(),
            doing,
            source,
        }
    }

    pub(crate) fn malformed(path: &std::path::Path, why: impl Into<String>) -> Self {
        Self::Malformed {
            path: path.to_path_buf(),
            why: why.into(),
        }
    }
}
