use core::fmt;

/// 経路で起きる失敗。
///
/// 下の層（iroh / QUIC）の理由は捨てずに [`Error::Network`] の中に持つ。
/// **握り潰すと、繋がらない理由が「繋がらない」しか残らない。**
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// 失効している相手だった。**名簿は呼ぶ側・受ける側の双方で見る。**
    Revoked,
    /// 宛先の形が壊れている。
    Malformed,
    /// 一度に送るには大きすぎる。
    TooLarge,
    /// **相手が正しく閉じた。**落ちたのではない。
    ///
    /// 上の層は、この 2 つで振る舞いを変える必要がある。
    /// 主催は「挨拶して帰った相手」では終わってよいが、
    /// **「回線が切れて消えた相手」では待ち直さないと会議が終わってしまう**
    /// （予定に紐づく会議キー・D43）。
    Closed,
    /// 下の層で落ちた。`doing` は何をしている最中だったか。
    Network {
        /// 何をしている最中だったか（`"結ぶ"` `"呼ぶ"` `"受ける"` …）。
        doing: &'static str,
        /// 下の層が返した理由。
        cause: Box<dyn core::error::Error + Send + Sync>,
    },
}

impl Error {
    /// 下の層の失敗を、何をしていたかと一緒に包む。
    pub(crate) fn network<E>(doing: &'static str) -> impl FnOnce(E) -> Self
    where
        E: core::error::Error + Send + Sync + 'static,
    {
        move |cause| Self::Network {
            doing,
            cause: Box::new(cause),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Revoked => f.write_str("失効しています"),
            Self::Malformed => f.write_str("宛先の形が壊れています"),
            Self::TooLarge => f.write_str("一度に送るには大きすぎます"),
            Self::Closed => f.write_str("相手が閉じました"),
            Self::Network { doing, cause } => write!(f, "{doing}途中で落ちました: {cause}"),
        }
    }
}

impl core::error::Error for Error {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Network { cause, .. } => Some(cause.as_ref()),
            _ => None,
        }
    }
}

impl From<warifu_core::Error> for Error {
    fn from(e: warifu_core::Error) -> Self {
        match e {
            warifu_core::Error::Revoked => Self::Revoked,
            _ => Self::Malformed,
        }
    }
}
