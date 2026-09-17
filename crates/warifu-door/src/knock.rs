//! 叩き。**本文は載らない。**

/// 名前の長さの上限（バイト）。
const MAX: usize = 320;

/// 誰。
///
/// **照合は完全一致**（`warifu-read` の送信元・`warifu-capability` の主体と同じ）。
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Subject(String);

impl Subject {
    /// 相手を作る。
    ///
    /// 空・長すぎ・制御文字は受け取らない。
    /// 制御文字を通すと、**人が見る一覧の表示を崩される。**
    pub fn new(s: &str) -> Option<Self> {
        if s.is_empty() || s.len() > MAX || s.chars().any(char::is_control) {
            return None;
        }
        Some(Self(s.to_owned()))
    }

    /// 文字列として見る。
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// 戸を叩いた 1 回。
///
/// **本文が入る場所が無い。**
///
/// 戸口は中身を見ない。**見る所（`warifu-read`）と、通すかを決める所を分ける**
/// （`warifu-capability` の `Request` と同じ手・**D24**）。
#[derive(Debug, Clone)]
pub struct Knock {
    from: Subject,
    at: u64,
    tally: bool,
    /// **部屋の合言葉の証しを持っていたか**（**D118**）。
    ///
    /// **割符とは別に持つ。**割符で通った相手は**知り合いとして書き置く**が、
    /// **証しで通った相手は書き置かない** —— 通るのは**その部屋の中だけ**である
    /// （D111 で決めた線を、そのまま守る）。
    部屋の証し: bool,
}

impl Knock {
    /// 割符を持たない叩き。**知らない相手はここに来る。**
    pub fn new(from: Subject, at: u64) -> Self {
        Self {
            from,
            at,
            tally: false,
            部屋の証し: false,
        }
    }

    /// 割符を持つ叩き。
    ///
    /// # **確かめてから呼ぶこと**
    ///
    /// 割符を確かめるのは `warifu-core` の仕事で、この層ではない。
    /// ここは**確かめた結果だけ**を受け取る。
    ///
    /// 確かめずに呼べば、**戸口は開く。**
    /// 名前を長くしてあるのは、通り道でそれが読めるようにするためである。
    pub fn with_verified_tally(from: Subject, at: u64) -> Self {
        Self {
            from,
            at,
            tally: true,
            部屋の証し: false,
        }
    }

    /// **部屋の合言葉の証しを持つ叩き**（**D118**）。
    ///
    /// # **確かめてから呼ぶこと**
    ///
    /// 証しを確かめるのは `warifu-core`（`合言葉::証しが合うか`）の仕事で、
    /// この層ではない。**ここは確かめた結果だけを受け取る。**
    ///
    /// **この層は `warifu-core` に依っていない。**通すかを決める所に、
    /// 暗号の作りを持ち込まない（baseline §9）。
    ///
    /// # 割符と何が違うか
    ///
    /// | | 通るか | **知り合いとして書き置くか** |
    /// |---|---|---|
    /// | 割符 | 通る | **書き置く**（次から割符なしで通る） |
    /// | **部屋の証し** | **通る** | **書き置かない** |
    ///
    /// **その部屋の中だけで通る。**部屋が終われば通らなくなる ——
    /// **合言葉は、人が手で渡した割符の代わりにはならない**（**D12** を守る）。
    pub fn with_verified_room_proof(from: Subject, at: u64) -> Self {
        Self {
            from,
            at,
            tally: false,
            部屋の証し: true,
        }
    }

    /// **部屋の合言葉の証しを持っていたか**（**D118**）。
    #[must_use]
    pub const fn has_room_proof(&self) -> bool {
        self.部屋の証し
    }

    /// 誰から。
    pub fn from(&self) -> &Subject {
        &self.from
    }

    /// いつ。
    pub fn at(&self) -> u64 {
        self.at
    }

    /// 割符を持っているか。
    pub fn has_tally(&self) -> bool {
        self.tally
    }
}
