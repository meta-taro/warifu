//! 画質の段。

/// 送る／受け取る画質。
///
/// **段を増やすときは、間を埋めるのではなく端を伸ばす。**
/// 細かく刻んでも、切り替わりが増えて見づらくなるだけで、通る量は変わらない。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Quality {
    /// 音声だけ。**最後まで残す段。**
    ///
    /// 映像から先に落とす。**声が切れるくらいなら映像を捨てる。**
    AudioOnly,
    /// 180p。
    P180,
    /// 360p。
    P360,
    /// 540p。
    P540,
    /// 720p。
    P720,
    /// 1080p。
    P1080,
}

impl Quality {
    /// 高いほうから順に。**割り当ては上から試して、通る所で止める。**
    pub const ALL: [Self; 6] = [
        Self::P1080,
        Self::P720,
        Self::P540,
        Self::P360,
        Self::P180,
        Self::AudioOnly,
    ];

    /// この段に要る帯域（bps）。
    ///
    /// 音声（32 kbps・Opus）を含む。**映像だけの数字にしない** —
    /// 足し忘れると、ぎりぎりの回線で必ず音が落ちる。
    pub fn bitrate_bps(&self) -> u64 {
        match self {
            Self::AudioOnly => 32_000,
            Self::P180 => 200_000,
            Self::P360 => 600_000,
            Self::P540 => 1_000_000,
            Self::P720 => 1_500_000,
            Self::P1080 => 3_000_000,
        }
    }
}
