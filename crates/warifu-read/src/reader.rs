//! 読む人。**AI を呼ばない。**

use crate::{Error, Kind, Level, Metadata, Priority, Received, View};

/// 受け取ったものを読む。
///
/// **既定は Level 0。**段を上げるのは [`Reader::open_at`] を呼んだ側であって、
/// 受け取った中身ではない。本文がこの判断に触れる経路は 1 本も無い。
#[derive(Debug, Default)]
pub struct Reader {}

impl Reader {
    /// 読む人を作る。
    pub fn new() -> Self {
        Self::default()
    }

    /// 既定の読み方。**Level 0 しか返さない。**
    ///
    /// 返り値の型に本文が入る場所が無いので、ここから本文が漏れることはない。
    pub fn read(&self, received: &Received) -> View {
        View::Metadata(self.metadata(received))
    }

    /// 段を上げて読む。**上げると決めるのは呼ぶ側。**
    ///
    /// # 失敗
    ///
    /// - [`Error::NeedsInterpreter`] — 要約と構造化は、**規則が無ければ解釈器が要る**。
    ///   ここで黙って解釈器を呼ばないのがこの層の目的（`issues/007`）
    /// - [`Error::NotBuiltYet`] — 添付の組み立てはまだ作っていない
    pub fn open_at(&self, received: &Received, level: Level) -> Result<View, Error> {
        let metadata = self.metadata(received);
        match level {
            Level::Metadata => Ok(View::Metadata(metadata)),
            // 規則を持つのは R2。**それまでは「作れない」と言う。**
            // ここで要約を捏造すると、呼ばずに済ませるという目的が最初に消える
            Level::Summary | Level::Structured => Err(Error::NeedsInterpreter(level)),
            Level::Raw => Ok(View::Raw {
                metadata,
                body: received.body().clone(),
            }),
            Level::Attachments => Err(Error::NotBuiltYet(level)),
        }
    }

    /// 既定で返すものを組み立てる。
    ///
    /// **見るのは、こちら側が知っている事実だけ。**
    /// 送信元・経路・こちらの時計で受け取った時刻の 3 つで、
    /// [`Received::claims`] にも本文にも触れない。
    ///
    /// 種別・優先度・人の判断が要るかは、**規則が決める**（R2）。
    /// 規則がまだ無いので、知らないままにしておく。
    /// 知らないものを知っているふりはしない。
    fn metadata(&self, received: &Received) -> Metadata {
        Metadata::new(
            received.sender().clone(),
            received.source(),
            received.received_at(),
            Kind::unknown(),
            Priority::default(),
            false,
        )
    }
}
