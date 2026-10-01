//! 粗さの自動調整。**素朴に作ると必ず振動する。**

use crate::Quality;

/// 上げるまでに、余裕が続いている必要のある秒数。
///
/// **落とすのは速く、上げるのはゆっくり。**
/// 待っている間に映像は壊れるが、上げて落ちるほうが見ている側には辛い。
pub const RAISE_AFTER: u64 = 10;

/// 上げるときに要る余裕（次の段の何倍出ているか）。分子。
const 余裕の分子: u64 = 13;
/// 上げるときに要る余裕。分母。**1.3 倍。**
///
/// 上げる境目を、次の段の帯域そのものにしない。
/// **同じ境目で上げ下げすると、1 秒ごとに段が変わる**（ヒステリシス）。
const 余裕の分母: u64 = 10;

/// これ以上取りこぼしていたら落とす（千分率）。**2%。**
const 落とす取りこぼし: u16 = 20;

/// 1 回ぶんの観測。
///
/// # 「出している量」ではなく「**出せると見込まれる量**」
///
/// ここを取り違えると、**段は二度と上がらない。**
///
/// 180p で送っていれば、実際に流れる量は 200 kbps しかない。
/// それを入れると「360p に上げるだけの余裕は無い」と毎回判断してしまう。
/// **自分が絞っているせいで上げられない**という循環になる。
///
/// 入れるのは、経路側が出す**空き帯域の見積もり**である
/// （WebRTC なら送信側の帯域推定。少し多めに試し送りして測る）。
/// [`crate::Meter`] が測るのは**実際に流れた量**なので、こちらとは別物である。
///
/// **見積もりを出すのは経路側の仕事**で、この層は受け取るだけ（M5 以降）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sample {
    available_bps: u64,
    loss_permille: u16,
}

impl Sample {
    /// 観測を作る。
    ///
    /// `available_bps` は**出せると見込まれる量**（上の説明を見ること）。
    /// `loss_permille` は取りこぼしの千分率。
    ///
    /// **見込み量だけでは足りない** — 取りこぼしながら送っていると、
    /// 見かけの量は出ているのに映像は壊れている。
    pub fn new(available_bps: u64, loss_permille: u16) -> Self {
        Self {
            available_bps,
            loss_permille,
        }
    }

    /// 出せると見込まれる量（bps）。
    pub fn available_bps(&self) -> u64 {
        self.available_bps
    }

    /// 取りこぼし（千分率）。
    pub fn loss_permille(&self) -> u16 {
        self.loss_permille
    }
}

/// 粗さを自動で決める。
///
/// # 素朴に作ると必ず振動する
///
/// 「入るなら上げる・入らないなら下げる」を毎回やると、**境目で 1 秒ごとに段が変わる。**
/// 見ている側には、粗い映像より**粗さが変わり続ける映像のほうが辛い。**
///
/// 入れてあるのは 3 つ。
///
/// 1. **上げ下げの境目をずらす**（次の段の 1.3 倍出ていないと上げない）
/// 2. **上げるのは、余裕が [`RAISE_AFTER`] 秒続いてから**
/// 3. **上げるのは 1 段ずつ。落ちるのは収まる所まで一気に**
///
/// # 始まりは音声だけ
///
/// **測る前に映像を出さない。**「測っていない」を「たぶん速い」にしない
/// （`Meter` が観測ゼロで `0` と答えないのと同じ構え・**D28**）。
#[derive(Debug, Clone)]
pub struct Governor {
    quality: Quality,
    ceiling: Quality,
    余裕が続いた秒: u64,
    前回: Option<u64>,
}

impl Default for Governor {
    fn default() -> Self {
        Self {
            quality: Quality::AudioOnly,
            ceiling: Quality::P1080,
            余裕が続いた秒: 0,
            前回: None,
        }
    }
}

impl Governor {
    /// 音声だけから始める。
    pub fn new() -> Self {
        Self::default()
    }

    /// 今の段。
    pub fn quality(&self) -> Quality {
        self.quality
    }

    /// これ以上は上げない、という頭。
    ///
    /// **回線に余裕があっても、割り当てを超えて送らない**（`plan` が決めた持ち分）。
    /// 頭が無いと、1 人が空いている帯域を全部使い、他の人ぶんが無くなる。
    ///
    /// 今の段が頭より上なら、その場で頭まで落とす。
    pub fn set_ceiling(&mut self, ceiling: Quality) {
        self.ceiling = ceiling;
        if self.quality > ceiling {
            self.quality = ceiling;
            self.余裕が続いた秒 = 0;
        }
    }

    /// 今の頭。
    pub fn ceiling(&self) -> Quality {
        self.ceiling
    }

    /// 観測を 1 つ入れて、今の段を決める。
    ///
    /// 見るのは 2 つ — **出ている量**と**取りこぼし**。
    ///
    /// 取りこぼしが出ていれば、**流量が足りていても落とす。**
    /// 流量だけを見ていると、壊れたまま「足りている」と判断し続ける。
    pub fn observe(&mut self, sample: Sample, now: u64) -> Quality {
        // 時計は戻ることがある（NTP の補正）。**戻ったぶんを経過時間として数えない**
        let 経過 = self.前回.map_or(0, |前| now.saturating_sub(前));
        self.前回 = Some(now);

        if sample.loss_permille >= 落とす取りこぼし {
            self.落とす(1);
            return self.quality;
        }
        if sample.available_bps < self.quality.bitrate_bps() {
            self.収まる所まで落とす(sample.available_bps);
            return self.quality;
        }
        self.上げるか見る(sample.available_bps, 経過);
        self.quality
    }

    /// 1 段だけ落とす。**音声より下には落ちない。**
    fn 落とす(&mut self, 段数: usize) {
        let 今 = Self::位置(self.quality);
        // ALL は高いほうから並んでいるので、後ろへ行くほど粗い
        let 先 = (今 + 段数).min(Quality::ALL.len() - 1);
        self.quality = Quality::ALL[先];
        self.余裕が続いた秒 = 0;
    }

    /// 収まる所まで一気に落とす。
    fn 収まる所まで落とす(&mut self, bps: u64) {
        self.quality = Quality::ALL
            .into_iter()
            .filter(|q| *q <= self.ceiling)
            .find(|q| q.bitrate_bps() <= bps)
            .unwrap_or(Quality::AudioOnly);
        self.余裕が続いた秒 = 0;
    }

    /// 上げてよいかを見る。**1 段ずつ。**
    fn 上げるか見る(&mut self, bps: u64, 経過: u64) {
        let 今 = Self::位置(self.quality);
        if 今 == 0 {
            // もう一番上
            self.余裕が続いた秒 = 0;
            return;
        }
        let 次 = Quality::ALL[今 - 1];
        // **割り当てを超えて上げない。**回線が空いていても、持ち分は持ち分
        if 次 > self.ceiling {
            self.余裕が続いた秒 = 0;
            return;
        }
        // 次の段の 1.3 倍。**ちょうどでは上げない**
        let 要る = next_threshold(次);
        if bps < 要る {
            self.余裕が続いた秒 = 0;
            return;
        }
        self.余裕が続いた秒 = self.余裕が続いた秒.saturating_add(経過);
        if self.余裕が続いた秒 >= RAISE_AFTER {
            self.quality = 次;
            self.余裕が続いた秒 = 0;
        }
    }

    fn 位置(q: Quality) -> usize {
        Quality::ALL
            .iter()
            .position(|x| *x == q)
            .expect("ALL は全ての段を含む")
    }
}

/// その段へ上げるのに要る量。
fn next_threshold(q: Quality) -> u64 {
    q.bitrate_bps().saturating_mul(余裕の分子) / 余裕の分母
}
