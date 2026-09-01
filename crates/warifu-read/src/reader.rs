//! 読む人。**AI を呼ばない。**

use crate::{Error, Kind, Level, Metadata, Priority, Received, RuleStore, View};

/// 受け取ったものを読む。
///
/// **既定は Level 0。**段を上げるのは [`Reader::open_at`] を呼んだ側であって、
/// 受け取った中身ではない。本文がこの判断に触れる経路は 1 本も無い。
///
/// 承認済みの規則を持たせると、**解釈器を呼ばずに構造化まで出せる**。
/// 規則を増やせるのは [`RuleStore::approve`] だけで、
/// この層が受信内容から規則を作る経路は無い（`decisions.md` **D5**）。
#[derive(Debug, Default)]
pub struct Reader {
    rules: RuleStore,
}

impl Reader {
    /// 規則を 1 つも持たない読む人。
    pub fn new() -> Self {
        Self::default()
    }

    /// 承認済みの規則を持った読む人。
    pub fn with_rules(rules: RuleStore) -> Self {
        Self { rules }
    }

    /// 持っている規則。**人が見るためにそのまま出す。**
    pub fn rules(&self) -> &RuleStore {
        &self.rules
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
    /// - [`Error::NeedsInterpreter`] — 要約は常に、構造化は**当たる規則が無いとき**、
    ///   解釈器が要る。ここで黙って解釈器を呼ばないのがこの層の目的（`issues/007`）
    ///
    /// 添付（Level 4）は経路側が組み立てたものをそのまま返す。**無ければ空。**
    pub fn open_at(&self, received: &Received, level: Level) -> Result<View, Error> {
        let metadata = self.metadata(received);
        match level {
            Level::Metadata => Ok(View::Metadata(metadata)),
            // 要約は規則では作れない。**規則は抽出であって要約ではない。**
            Level::Summary => Err(Error::NeedsInterpreter(level)),
            Level::Structured => match self.matching(received) {
                // ここが「2 通目以降はゼロ」になる場所
                Some(規則) => Ok(View::Structured {
                    metadata,
                    fields: 規則.extract_from(&self.text(received)),
                }),
                // 「1 通目だけがコスト」の 1 通目。**呼ぶかどうかは呼ぶ側が決める**
                None => Err(Error::NeedsInterpreter(level)),
            },
            Level::Raw => Ok(View::Raw {
                metadata,
                body: received.body().clone(),
            }),
            // 組み立てたのは経路側。**この層は渡すだけで、解釈しない。**
            // 無ければ空で返す — 「無い」と「まだ作っていない」を混ぜない
            Level::Attachments => Ok(View::Attachments {
                metadata,
                attachments: received.attachments().to_vec(),
            }),
        }
    }

    /// 既定で返すものを組み立てる。
    ///
    /// **見るのは、こちら側が知っている事実と、人が承認した規則だけ。**
    /// [`Received::claims`] には触れない。
    ///
    /// 当たる規則が無ければ、知らないままにしておく。
    /// 知らないものを知っているふりはしない。
    fn metadata(&self, received: &Received) -> Metadata {
        let 規則 = self.matching(received);
        Metadata::new(
            received.sender().clone(),
            received.source(),
            received.received_at(),
            規則.map_or_else(Kind::unknown, |r| r.kind().clone()),
            規則.map_or(Priority::Normal, |r| r.priority()),
            規則.is_some_and(|r| r.action_required()),
        )
    }

    fn matching(&self, received: &Received) -> Option<&crate::Rule> {
        self.rules.matching(received.sender(), &self.text(received))
    }

    /// 当たり判定と抽出に使う文字列。
    ///
    /// 壊れたバイト列は置き換えて進む。**文字符号化の解決は IMAP 側（R5）の仕事**で、
    /// ここで止めると「読めない 1 通で受信箱全体が止まる」ことになる。
    fn text(&self, received: &Received) -> String {
        String::from_utf8_lossy(received.body().as_bytes()).into_owned()
    }
}
