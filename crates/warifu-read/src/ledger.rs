//! 会計。**1 通あたり、何にいくら掛かったかを手元に残す。**
//!
//! この層の値打ちは「解釈器を呼ばずに済んだ回数」でしか測れない。
//! 測れなければ、効いているのかどうかを誰も言えない（`issues/007` §39）。
//!
//! **記録の側に本文は入らない。**Level 0 で本文を返さない層が、
//! 会計から本文を漏らしていたら意味が無い。

use core::fmt;
use std::io::Write as _;
use std::path::Path;

use crate::{Error, Kind, Level, Received, SenderId, View};

/// 記録の見出し。**列の順序と数はここが正本。**
const 見出し: &str = "受け取った時刻\t送信元\t種別\t段\t解釈器\tinput\toutput\t規則";

/// 列の数。読むときに数が合わなければ受け取らない。
const 列数: usize = 8;

/// 解釈器を呼んだかどうか。
///
/// **token は呼んだ側が申告する。**この層は解釈器を持たないので、数えようがない。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Interpreter {
    /// 呼ばずに済んだ。**この層の目的が達成された 1 通。**
    NotCalled,
    /// 呼んだ。
    Called {
        /// 入力に使った token。
        input: u64,
        /// 出力に使った token。
        output: u64,
    },
}

/// 1 通をどう読んだかの記録。
///
/// **本文も、送信者の申し送りも入らない。**入るのは、
/// こちらが知っている事実と、こちらが数えた費用だけ。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    received_at: u64,
    sender: SenderId,
    kind: Kind,
    level: Level,
    interpreter: Interpreter,
    rule_approved: bool,
}

impl Entry {
    /// 解釈器を呼ばずに読めた 1 通。
    ///
    /// **読めた結果から作る。**規則が当たって種別が分かっているなら、それが記録に残る。
    pub fn without_interpreter(view: &View) -> Self {
        let m = view.metadata();
        Self {
            received_at: m.received_at(),
            sender: m.sender().clone(),
            kind: m.kind().clone(),
            level: view.level(),
            interpreter: Interpreter::NotCalled,
            rule_approved: false,
        }
    }

    /// 解釈器を呼んだ 1 通。**token は呼んだ側が申告する。**
    ///
    /// 呼ぶことになったのは**読めなかったから**なので、この時点で種別は分かっていない。
    /// 記録も分かっていないままにする。**後から分かった種別を遡って書かない。**
    pub fn with_interpreter(received: &Received, level: Level, input: u64, output: u64) -> Self {
        Self {
            received_at: received.received_at(),
            sender: received.sender().clone(),
            kind: Kind::unknown(),
            level,
            interpreter: Interpreter::Called { input, output },
            rule_approved: false,
        }
    }

    /// この 1 通をきっかけに規則が**承認された**ことを記す。
    ///
    /// 承認は人が通す口（`decisions.md` **D19**）。
    /// **規則ができた通が分からないと、2 通目以降ゼロを誰も確かめられない。**
    pub fn rule_approved(mut self) -> Self {
        self.rule_approved = true;
        self
    }

    /// どの種別として記録したか。
    pub fn kind(&self) -> &Kind {
        &self.kind
    }

    /// 送信元。
    pub fn sender(&self) -> &SenderId {
        &self.sender
    }

    /// 解釈器を呼んだか。
    pub fn interpreter(&self) -> Interpreter {
        self.interpreter
    }

    fn to_tsv(&self) -> String {
        let (呼んだ, input, output) = match self.interpreter {
            Interpreter::NotCalled => ("呼ばず", 0, 0),
            Interpreter::Called { input, output } => ("呼んだ", input, output),
        };
        format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            self.received_at,
            self.sender.as_str(),
            self.kind,
            self.level as u8,
            呼んだ,
            input,
            output,
            if self.rule_approved { "承認" } else { "" }
        )
    }

    fn from_tsv(行: &str) -> Result<Self, Error> {
        let 列: Vec<&str> = 行.split('\t').collect();
        if 列.len() != 列数 {
            return Err(Error::Malformed);
        }
        let 数 = |s: &str| s.parse::<u64>().map_err(|_| Error::Malformed);
        let interpreter = match 列[4] {
            "呼ばず" => Interpreter::NotCalled,
            "呼んだ" => Interpreter::Called {
                input: 数(列[5])?,
                output: 数(列[6])?,
            },
            _ => return Err(Error::Malformed),
        };
        Ok(Self {
            received_at: 数(列[0])?,
            sender: SenderId::new(列[1])?,
            kind: if 列[2] == Kind::UNKNOWN {
                Kind::unknown()
            } else {
                Kind::new(列[2])?
            },
            level: Level::from_number(列[3].parse::<u8>().map_err(|_| Error::Malformed)?)?,
            interpreter,
            rule_approved: match 列[7] {
                "承認" => true,
                "" => false,
                _ => return Err(Error::Malformed),
            },
        })
    }
}

/// 会計の帳簿。
///
/// **消す口を置いていない。**消せると、掛かった費用を後から減らせる。
/// 失効の名簿を取り消せなくしたのと同じ理由（`warifu-core` の `Revocations`）。
#[derive(Debug, Clone, Default)]
pub struct Ledger {
    entries: Vec<Entry>,
}

impl Ledger {
    /// 空の帳簿。
    pub fn new() -> Self {
        Self::default()
    }

    /// 1 通ぶんを記す。
    pub fn record(&mut self, entry: Entry) {
        self.entries.push(entry);
    }

    /// 記した内容。
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    /// 解釈器を呼んだ回数。**この層が効いているかは、この数でしか測れない。**
    pub fn interpreter_calls(&self) -> usize {
        self.entries
            .iter()
            .filter(|e| e.interpreter != Interpreter::NotCalled)
            .count()
    }

    /// ある送信元について、解釈器を呼んだ回数。
    ///
    /// **形式が固定されている相手なら、ここが 1 で止まる**のが期待する形。
    pub fn interpreter_calls_for(&self, sender: &SenderId) -> usize {
        self.entries
            .iter()
            .filter(|e| &e.sender == sender && e.interpreter != Interpreter::NotCalled)
            .count()
    }

    /// ある送信元から受け取った通数。
    pub fn entries_for(&self, sender: &SenderId) -> usize {
        self.entries.iter().filter(|e| &e.sender == sender).count()
    }

    /// 承認された規則の数。
    pub fn rules_approved(&self) -> usize {
        self.entries.iter().filter(|e| e.rule_approved).count()
    }

    /// 使った token の合計（input, output）。
    pub fn tokens(&self) -> (u64, u64) {
        self.entries
            .iter()
            .fold((0, 0), |(i, o), e| match e.interpreter {
                Interpreter::NotCalled => (i, o),
                Interpreter::Called { input, output } => (i + input, o + output),
            })
    }

    /// TSV にする。**1 行目は見出し。**
    ///
    /// 設計書類は Markdown / TSV で書く（baseline §19）。
    /// 表計算でそのまま開けるので、**人が実測値を書き足せる。**
    pub fn to_tsv(&self) -> String {
        let mut s = String::from(見出し);
        for e in &self.entries {
            s.push('\n');
            s.push_str(&e.to_tsv());
        }
        s.push('\n');
        s
    }

    /// TSV から読む。
    ///
    /// **読めない行があれば、そこで失敗する。**黙って捨てると、
    /// 「呼んだ回数」が後から減る。
    pub fn from_tsv(text: &str) -> Result<Self, Error> {
        let mut 行 = text.lines();
        if 行.next().map(str::trim_end) != Some(見出し) {
            return Err(Error::Malformed);
        }
        let entries = 行
            .filter(|l| !l.trim().is_empty())
            .map(Entry::from_tsv)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self { entries })
    }

    /// ファイルへ**追記**する。
    ///
    /// 上書きにしない。上書きにすると、前回までの記録が消える＝
    /// **掛かった費用を後から減らせる。**
    ///
    /// # 失敗
    ///
    /// ファイルを開けない・書けないときに [`Error::Storage`]。
    /// **理由を捨てない**（下の層の失敗を握り潰すと、直しようがなくなる）。
    pub fn append_to(&self, path: &Path) -> Result<(), Error> {
        let 既にある = path.exists();
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|e| Error::Storage(e.to_string()))?;
        let 書くもの = if 既にある {
            self.to_tsv()
                .lines()
                .skip(1)
                .map(|l| format!("{l}\n"))
                .collect::<String>()
        } else {
            self.to_tsv()
        };
        f.write_all(書くもの.as_bytes())
            .map_err(|e| Error::Storage(e.to_string()))
    }

    /// ファイルから読む。
    ///
    /// # 失敗
    ///
    /// 読めないときに [`Error::Storage`]、中身が壊れているときに [`Error::Malformed`]。
    pub fn load(path: &Path) -> Result<Self, Error> {
        let text = std::fs::read_to_string(path).map_err(|e| Error::Storage(e.to_string()))?;
        Self::from_tsv(&text)
    }
}

impl fmt::Display for Ledger {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_tsv())
    }
}
