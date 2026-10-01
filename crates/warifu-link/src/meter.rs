//! 実際に流れた量から回線を測る。**申告ではなく観測。**

/// 測定値を新しいとみなす長さ（秒）。
///
/// **回線は変わる。**30 分前の実測で今の割り当てを決めない。
pub const FRESH_FOR: u64 = 60;

/// 1 回ぶんの観測。
#[derive(Debug, Clone, Copy)]
struct 観測 {
    bps: u64,
    at: u64,
}

/// 回線の太さを測る。
///
/// 相手に「あなたの回線は何 Mbps ですか」と尋ねる形にしない。
/// 尋ねれば、**多く送ってほしい側は多めに答える**
/// （`warifu-read` で送信者に優先度を申告させなかったのと同じ理屈）。
///
/// 見るのは**実際に流れた量**だけである。
#[derive(Debug, Clone, Default)]
pub struct Meter {
    観測: Vec<観測>,
}

impl Meter {
    /// 空の計器。
    pub fn new() -> Self {
        Self::default()
    }

    /// 「`from` から `to` までの間に `bytes` バイト流れた」を記す。
    ///
    /// **長さ 0 と未来の観測は捨てる。**
    /// 未来を受け取ると、**いつまでも古くならない観測**ができてしまう
    /// （時計がずれた相手の値をそのまま入れた場合に起きる）。
    pub fn observe(&mut self, bytes: u64, from: u64, to: u64) {
        if to <= from {
            return;
        }
        self.観測.push(観測 {
            bps: bytes.saturating_mul(8) / (to - from),
            at: to,
        });
    }

    /// 今の回線（bps）。
    ///
    /// **測っていなければ `None`。**`0` と答えない —
    /// 「測っていない」と「0 だった」は別のことで、混ぜると
    /// 起動直後の会議が必ず音声だけになる。
    ///
    /// 新しい観測のうち**一番速かったもの**を採る。
    /// 平均にすると、送るものが無かった区間で実力より低く出る。
    pub fn measured(&self, now: u64) -> Option<u64> {
        self.観測
            .iter()
            .filter(|o| o.at <= now && now - o.at <= FRESH_FOR)
            .map(|o| o.bps)
            .max()
    }

    /// 古い観測を捨てる。
    ///
    /// 捨てなくても [`Meter::measured`] は見ないが、
    /// **持ち続ければ増え続ける。**
    pub fn forget_old(&mut self, now: u64) {
        self.観測.retain(|o| o.at <= now && now - o.at <= FRESH_FOR);
    }
}
