//! 受け取った 1 通。**まだ何も読んでいない。**

use core::fmt;

use crate::Error;

/// 送信元の長さの上限（バイト）。メールアドレスの上限（RFC 5321）に合わせてある。
const MAX_SENDER: usize = 320;

/// どの経路から受け取ったか。
///
/// **経路が違っても、この層は同じ判断をする。**
/// 読み取りを Adapter の内側に書くと、経路の数だけ同じものを作り直すことになる
/// （`issues/007`「置き場所を間違えない」）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Source {
    /// 既存のメール。**warifu の通信層を一切使わない**ので、相手が誰でも成立する。
    Imap,
    /// warifu の口（`warifu-intent`）。
    Intent,
}

/// 送信元。
///
/// **照合は完全一致で行う。**大文字小文字を潰すと、`Billing@例` と `billing@例` が
/// 同じ規則に当たる。取り違えて損をするのは読む側なので、**当たらない側に倒す**
/// （当たらなければ解釈器を 1 回余計に呼ぶだけで済む）。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SenderId(String);

impl SenderId {
    /// 送信元を作る。
    ///
    /// 空と長すぎるものに加えて、**制御文字を受け取らない**。
    /// タブや改行が通ると、TSV の会計で 1 行を 2 行に割れる＝**記録を偽造できる**。
    pub fn new(s: &str) -> Result<Self, Error> {
        if s.is_empty() || s.len() > MAX_SENDER || s.chars().any(char::is_control) {
            return Err(Error::Malformed);
        }
        Ok(Self(s.to_owned()))
    }

    /// 文字列として見る。
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// 送信者が付けてきた申し送り（メールのヘッダ等）。
///
/// **こちらの判断には使わない。**`X-Priority` も `Importance` も送る側が自由に書ける。
/// 申告できるようにすると、全員が「緊急」を付ける（`issues/007`）。
///
/// 持っておくのは、**後から人が見たときに「何を無視したか」が分かるようにするため**。
#[derive(Clone, Default)]
pub struct Claims(Vec<(String, String)>);

impl Claims {
    /// 空の申し送り。
    pub fn new() -> Self {
        Self::default()
    }

    /// 申し送りを 1 つ足す。**足しても metadata は変わらない。**
    pub fn with(mut self, name: &str, value: &str) -> Self {
        self.0.push((name.to_owned(), value.to_owned()));
        self
    }

    /// 何件付いてきたか。
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// 1 件も付いていないか。
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl fmt::Debug for Claims {
    /// **中身を出さない。**申し送りは相手が書いた文字列で、
    /// ログへ素通しすると「読む前に読む」ことになる。
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Claims({} 件)", self.0.len())
    }
}

/// 本文。**この層は既定で開かない。**
///
/// バイト列のまま持つ。文字符号化も MIME も、ここでは解釈しない。
#[derive(Clone)]
pub struct Body(Vec<u8>);

impl Body {
    /// 本文を作る。
    pub fn new(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    /// 本文のバイト列。**呼ぶ側が段を上げたときにだけ渡る。**
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// 何バイトあるか。
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// 空か。
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl fmt::Debug for Body {
    /// **中身を出さない。**`{:?}` に本文が出ると、Level 0 で返した意味が消える。
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Body({} バイト)", self.0.len())
    }
}

/// 受け取った 1 通。
///
/// `received_at` は **こちらの時計**で入れる。
/// 相手が書いてきた日時は [`Claims`] であって、事実ではない。
#[derive(Debug, Clone)]
pub struct Received {
    source: Source,
    sender: SenderId,
    received_at: u64,
    claims: Claims,
    body: Body,
}

impl Received {
    /// 1 通を組み立てる。
    pub fn new(source: Source, sender: SenderId, received_at: u64, body: Body) -> Self {
        Self {
            source,
            sender,
            received_at,
            claims: Claims::new(),
            body,
        }
    }

    /// 送信者の申し送りを添える。**添えても metadata は変わらない。**
    pub fn with_claims(mut self, claims: Claims) -> Self {
        self.claims = claims;
        self
    }

    /// どの経路から来たか。
    pub fn source(&self) -> Source {
        self.source
    }

    /// 送信元。
    pub fn sender(&self) -> &SenderId {
        &self.sender
    }

    /// こちらの時計で受け取った時刻。
    pub fn received_at(&self) -> u64 {
        self.received_at
    }

    /// 送信者の申し送り。**判断には使わない。**
    pub fn claims(&self) -> &Claims {
        &self.claims
    }

    /// 本文。**この層の外へ出るのは、呼ぶ側が段を上げたときだけ。**
    pub(crate) fn body(&self) -> &Body {
        &self.body
    }
}
