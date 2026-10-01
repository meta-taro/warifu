//! 受け取ったもの。**まだどこにも置いていない。**

/// 受け取ってよい大きさの上限（バイト）。**100 MiB。**
///
/// 上限を置くのは、**長さだけ大きく宣言して確保させる攻撃**を止めるため
/// （`warifu-intent` の荷物と同じ理屈・**D14**）。
pub const MAX_BYTES: usize = 100 * 1024 * 1024;

/// 届いたファイル 1 つ。
///
/// **名前は相手が書いた文字列である。**本文と同じで、
/// データであって指示ではない（`decisions.md` **D5**）。
#[derive(Clone)]
pub struct Incoming {
    name: String,
    bytes: Vec<u8>,
    trusted: bool,
}

impl Incoming {
    /// 届いたものを組み立てる。
    pub fn new(name: &str, bytes: Vec<u8>) -> Self {
        Self {
            name: name.to_owned(),
            bytes,
            trusted: false,
        }
    }

    /// 信頼している相手から来た、と記す。
    ///
    /// **記しても扱いは変わらない。**Zero Trust（roadmap Phase 2）。
    /// 変えると、**信頼を得ることに価値が生まれる**
    /// （`warifu-capability` の `Trust` と同じ理屈・**D24**）。
    ///
    /// 持っておくのは、**人が見るときの材料**としてだけ。
    pub fn from_trusted(mut self) -> Self {
        self.trusted = true;
        self
    }

    /// 相手が書いてきた名前。**書き換えずに残す。**
    ///
    /// 人が「何という名前で届いたか」を見るために要る。
    /// 置くときに使うのは、[`crate::inspect`] が作る安全な名前のほう。
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 中身。
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// 信頼している相手からか。**判定には使わない。**
    pub fn is_trusted(&self) -> bool {
        self.trusted
    }
}

impl core::fmt::Debug for Incoming {
    /// **中身も名前も出さない。**
    ///
    /// 名前は相手が書いた文字列で、裏返す文字や制御文字が入りうる。
    /// ログへ素通しすると、**ログの表示そのものを崩される。**
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Incoming({} バイト)", self.bytes.len())
    }
}
