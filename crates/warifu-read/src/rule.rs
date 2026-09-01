//! 規則。**一度学習した形式は、二度と解釈器を呼ばない。**
//!
//! `decisions.md` **D5** の真正面に立つ場所なので、
//! **規則を増やせる口は [`RuleStore::approve`] 1 本だけ**にしてある。
//! 受信した本文からこの口が呼ばれる経路は無い。

use core::fmt;

use crate::{Field, Kind, Priority, SenderId};

/// 抽出の 1 項目。**何を抜くかが人に読める形**になっている。
///
/// 表現できるのは「ある目印の後ろを取る」だけ。
/// 抜くこと以外を書けないので、**規則が命令になりようがない。**
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Extract {
    name: String,
    after: String,
    until: Option<String>,
}

impl Extract {
    /// `after` の直後から**行末まで**を取る。
    pub fn new(name: &str, after: &str) -> Self {
        Self {
            name: name.to_owned(),
            after: after.to_owned(),
            until: None,
        }
    }

    /// 行末ではなく、指定の文字列の手前までを取る。
    pub fn until(mut self, until: &str) -> Self {
        self.until = Some(until.to_owned());
        self
    }

    /// 項目名。
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 本文から 1 項目を抜く。
    ///
    /// **見つからなければ空のまま返す。**`—` や `N/A` で埋めない（baseline §19）。
    /// 埋めると、抜けなかったことが誰にも見えなくなる。
    ///
    /// 当たるのは**最初の 1 箇所だけ**。2 箇所目を拾う規則にすると、
    /// 本文を足すだけで抽出結果を動かせてしまう。
    fn apply(&self, text: &str) -> Field {
        let Some(始まり) = text.find(&self.after).map(|i| i + self.after.len()) else {
            return Field::new(&self.name, "");
        };
        let 残り = &text[始まり..];
        let 終わり = match &self.until {
            Some(u) => 残り.find(u.as_str()).unwrap_or(残り.len()),
            None => 残り.find('\n').unwrap_or(残り.len()),
        };
        Field::new(&self.name, 残り[..終わり].trim())
    }
}

impl fmt::Display for Extract {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.until {
            Some(u) => write!(
                f,
                "{} ← 「{}」の後ろ「{}」の手前まで",
                self.name, self.after, u
            ),
            None => write!(f, "{} ← 「{}」の後ろ、行末まで", self.name, self.after),
        }
    }
}

/// 承認前の規則の候補。
///
/// 解釈器が出すのはここまで。**これ自体は読み取りに使われない。**
/// 生成と適用を分けているのが D5 の要で、
/// 分けないと「この形式はこう読め」と本文に書くだけで読み手を乗っ取れる。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleDraft {
    sender: SenderId,
    kind: Kind,
    priority: Priority,
    action_required: bool,
    markers: Vec<String>,
    extracts: Vec<Extract>,
}

impl RuleDraft {
    /// 送信元と種別を決めて候補を起こす。
    pub fn new(sender: SenderId, kind: Kind) -> Self {
        Self {
            sender,
            kind,
            priority: Priority::Normal,
            action_required: false,
            markers: Vec::new(),
            extracts: Vec::new(),
        }
    }

    /// 当たり判定の目印を足す。**すべて含まれたときだけ当たる。**
    pub fn marker(mut self, marker: &str) -> Self {
        self.markers.push(marker.to_owned());
        self
    }

    /// 優先度を決める。**決めるのは承認する人であって、送信者ではない。**
    pub fn priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    /// 人の判断が要るかを決める。
    pub fn action_required(mut self, required: bool) -> Self {
        self.action_required = required;
        self
    }

    /// 抽出項目を足す。
    pub fn extract(mut self, extract: Extract) -> Self {
        self.extracts.push(extract);
        self
    }
}

/// 承認された規則。
///
/// [`RuleStore::approve`] を通ったものだけが存在する。**候補から直接は作れない。**
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    draft: RuleDraft,
}

impl Rule {
    /// この規則が当たる送信元。
    pub fn sender(&self) -> &SenderId {
        &self.draft.sender
    }

    /// 当たったときに付ける種別。
    pub fn kind(&self) -> &Kind {
        &self.draft.kind
    }

    /// 当たったときに付ける優先度。
    pub fn priority(&self) -> Priority {
        self.draft.priority
    }

    /// 当たったときに人の判断が要るか。
    pub fn action_required(&self) -> bool {
        self.draft.action_required
    }

    /// この本文に当たるか。
    ///
    /// 送信元は**完全一致**、目印は**すべて含まれること**（D18）。
    fn matches(&self, sender: &SenderId, text: &str) -> bool {
        sender == &self.draft.sender && self.draft.markers.iter().all(|m| text.contains(m.as_str()))
    }

    /// 本文から項目を抜く。**規則に書いてある順に、書いてある数だけ返す。**
    pub(crate) fn extract_from(&self, text: &str) -> Vec<Field> {
        self.draft.extracts.iter().map(|e| e.apply(text)).collect()
    }
}

impl fmt::Display for Rule {
    /// **人が読める形。**承認する人が、何を抽出しているか読めなければ承認できない。
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let d = &self.draft;
        writeln!(f, "規則 {}", d.kind)?;
        writeln!(f, "  送信元 {}", d.sender.as_str())?;
        writeln!(
            f,
            "  目印   {}",
            if d.markers.is_empty() {
                "（無し・送信元だけで当たる）".to_owned()
            } else {
                d.markers
                    .iter()
                    .map(|m| format!("「{m}」"))
                    .collect::<Vec<_>>()
                    .join(" ")
            }
        )?;
        writeln!(f, "  優先度 {:?}", d.priority)?;
        writeln!(
            f,
            "  要判断 {}",
            if d.action_required {
                "はい"
            } else {
                "いいえ"
            }
        )?;
        if d.extracts.is_empty() {
            writeln!(f, "  抽出   （無し）")?;
        } else {
            for (i, e) in d.extracts.iter().enumerate() {
                writeln!(f, "  {}{e}", if i == 0 { "抽出   " } else { "       " })?;
            }
        }
        Ok(())
    }
}

/// 承認された規則の棚。
///
/// **増やせる口は [`RuleStore::approve`] 1 本だけ。**
/// 受信した本文からこの口が呼ばれる経路は、この層のどこにも無い。
#[derive(Debug, Clone, Default)]
pub struct RuleStore {
    rules: Vec<Rule>,
}

impl RuleStore {
    /// 空の棚。
    pub fn new() -> Self {
        Self::default()
    }

    /// 候補を承認して棚へ入れる。**人が通す口。**
    ///
    /// ここを自動で呼ぶ実装を書かないこと。書いた時点で D5 が崩れる。
    pub fn approve(&mut self, draft: RuleDraft) {
        self.rules.push(Rule { draft });
    }

    /// 承認済みの規則。**人が読むためにそのまま出す。**
    pub fn rules(&self) -> &[Rule] {
        &self.rules
    }

    /// 何件承認されているか。
    pub fn len(&self) -> usize {
        self.rules.len()
    }

    /// 1 件も承認されていないか。
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// 当たる規則を探す。**先に承認されたものが勝つ。**
    ///
    /// 後から入れた規則が既存の読み方を黙って変えないようにするため、
    /// 後勝ちにはしない。
    pub(crate) fn matching(&self, sender: &SenderId, text: &str) -> Option<&Rule> {
        self.rules.iter().find(|r| r.matches(sender, text))
    }
}
