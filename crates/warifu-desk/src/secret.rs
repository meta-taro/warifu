//! **送る前に、秘密らしいものを見つける**（0.1.42）。
//!
//! 割符の会話は、送った時点で各機械へ写しが届く。**編集も削除もできない**（P2P・中央が無い）。
//! だから止められるのは送る前だけである。2026-11-02、中継のトークンが全機の会話に流れた。
//!
//! - エージェントの `chat_send` は、見つけたら**送らずに止める**（押し切る道を持たせない）
//! - 人の画面は、見つけたら**確かめてから送る**（［それでも送る］）
//!
//! 見つけたものの**値は返さない**（返すと、止めた所でまた写る）。種類だけを返す。
//! 画面（`apps/desktop/src/lib/chat/secret.ts`）も同じ決まりで見る。
//! どちらも `tests/secret-cases.tsv` の例文で試すので、決まりがずれれば落ちる。

/// 見つけた秘密らしいものの種類。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum 秘密の種類 {
    /// 長い 16 進の並び（48 字以上。git の指紋 40 字・md5 32 字は通す）。
    十六進,
    /// `TOKEN=…` `API_KEY: …` のような、名前と値の組。
    名前と値,
    /// 名の知れた API キーの形（`sk-` `ghp_` `AKIA` など）。
    キーの形,
    /// 秘密鍵の書き出し（`-----BEGIN … PRIVATE KEY-----`）。
    秘密鍵,
    /// ルームキーの全文（`WARIFU1-…#…`）。持参人式なので、持っていれば入れる。
    ルームキー,
}

impl 秘密の種類 {
    /// 人に見せる名前（例文の表の「種類」と同じ）。
    #[must_use]
    pub const fn 名前(self) -> &'static str {
        match self {
            Self::十六進 => "16進",
            Self::名前と値 => "名前＝値",
            Self::キーの形 => "キーの形",
            Self::秘密鍵 => "秘密鍵",
            Self::ルームキー => "ルームキー",
        }
    }
}

/// 本文に入っている秘密らしいものの種類（重なりなし・並びは決まった順）。無ければ空。
#[must_use]
pub fn 秘密らしいもの(本文: &str) -> Vec<秘密の種類> {
    let mut 見つけた = Vec::new();
    if 本文.contains("-----BEGIN") && 本文.contains("PRIVATE KEY") {
        見つけた.push(秘密の種類::秘密鍵);
    }
    if ルームキーがある(本文) {
        見つけた.push(秘密の種類::ルームキー);
    }
    if 長い十六進がある(本文) {
        見つけた.push(秘密の種類::十六進);
    }
    if キーの形がある(本文) {
        見つけた.push(秘密の種類::キーの形);
    }
    if 名前と値がある(本文) {
        見つけた.push(秘密の種類::名前と値);
    }
    見つけた.sort();
    見つけた
}

/// エージェントの送りを止めるときの言い分。**見つけなければ `None`。**
///
/// 口（`chat_send`）と画面の本体（エージェントの行を流す所）が同じ言い方で断る。値は言い返さない。
#[must_use]
pub fn エージェントへの断り(本文: &str) -> Option<String> {
    let 見つけた = 秘密らしいもの(本文);
    if 見つけた.is_empty() {
        return None;
    }
    let 種類: Vec<&str> = 見つけた.into_iter().map(秘密の種類::名前).collect();
    Some(format!(
        "送りませんでした: 秘密らしいもの（{}）が入っています。割符の会話は送ると取り消せません\
         （届いた各機械に写しが残ります）。その部分を除いて送り直してください。\
         どうしても送るなら、人が画面から送ります（画面は確かめてから送れます）",
        種類.join("・")
    ))
}

/// 16 進の並びを秘密と見る長さ。git の指紋（40）・md5（32）は人がよく貼るので通す
const 十六進の長さ: usize = 48;

/// 名の知れたキーの頭と、その後ろに続く字の最小の数。
const キーの頭: &[(&str, usize)] = &[
    ("sk-ant-", 20),
    ("sk-", 20),
    ("ghp_", 30),
    ("gho_", 30),
    ("ghs_", 30),
    ("ghu_", 30),
    ("ghr_", 30),
    ("github_pat_", 30),
    ("AKIA", 16),
    ("ASIA", 16),
    ("xoxb-", 10),
    ("xoxp-", 10),
    ("xoxa-", 10),
    ("xoxr-", 10),
    ("AIza", 30),
    ("glpat-", 20),
];

/// 名前に入っていれば、その値を秘密と見る語（大文字にして比べる）。
const 秘密の名前: &[&str] = &[
    "TOKEN",
    "SECRET",
    "PASSWORD",
    "PASSWD",
    "API_KEY",
    "APIKEY",
    "ACCESS_KEY",
    "PRIVATE_KEY",
];

/// 鍵の字（英数字と `_` `-`）。
fn 鍵の字か(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '-'
}

/// `位置` の前が鍵の字でないか（語の頭か）。
fn 語の頭か(本文: &str, 位置: usize) -> bool {
    本文[..位置]
        .chars()
        .next_back()
        .is_none_or(|c| !c.is_ascii_alphanumeric())
}

fn ルームキーがある(本文: &str) -> bool {
    本文.match_indices("WARIFU1-").any(|(i, 頭)| {
        let 体: String = 本文[i + 頭.len()..]
            .chars()
            .take_while(|c| c.is_ascii_uppercase() || ('2'..='7').contains(c) || *c == '#')
            .collect();
        体.len() >= 40 && 体.contains('#')
    })
}

fn 長い十六進がある(本文: &str) -> bool {
    // 英数字の切れ目で区切った語ごとに見る。語ぜんぶが 16 進で、長いものだけ
    // （base32 の公開鍵の途中に 16 進の字が続いても、語ぜんぶではないので数えない）
    本文
        .split(|c: char| !c.is_ascii_alphanumeric())
        .any(|語| 語.len() >= 十六進の長さ && 語.chars().all(|c| c.is_ascii_hexdigit()))
}

fn キーの形がある(本文: &str) -> bool {
    キーの頭.iter().any(|(頭, 最小)| {
        本文.match_indices(頭).any(|(i, _)| {
            語の頭か(本文, i)
                && 本文[i + 頭.len()..]
                    .chars()
                    .take_while(|c| 鍵の字か(*c))
                    .count()
                    >= *最小
        })
    })
}

fn 名前と値がある(本文: &str) -> bool {
    本文
        .char_indices()
        .filter(|(_, c)| *c == '=' || *c == ':')
        .any(|(i, _)| {
            let 名前: String = 本文[..i]
                .trim_end()
                .chars()
                .rev()
                .take_while(|c| 鍵の字か(*c) || *c == '.')
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect();
            let 大 = 名前.to_ascii_uppercase();
            if !秘密の名前.iter().any(|語| 大.contains(語)) {
                return false;
            }
            値らしい(本文[i + 1..].trim_start())
        })
}

/// `=` `:` の後ろが、置き場所の印（`<…>` `${…}` `***`）でない、本物らしい値か。
fn 値らしい(後ろ: &str) -> bool {
    let 後ろ = 後ろ.trim_start_matches(['"', '\'']);
    if 後ろ.starts_with(['<', '$', '{', '*']) {
        return false;
    }
    let 値: String = 後ろ
        .chars()
        .take_while(|c| !c.is_whitespace() && !matches!(c, '"' | '\'' | ',' | ';'))
        .collect();
    let 数字あり = 値.chars().any(|c| c.is_ascii_digit());
    値.is_ascii() && 値.len() >= 8 && (数字あり || 値.len() >= 16)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `{hex64}` などを展開する。本物らしい鍵をリポジトリに置かないため
    fn 展開(本文: &str) -> String {
        let mut 出 = String::new();
        let mut 残り = 本文;
        while let Some(始) = 残り.find('{') {
            let Some(終) = 残り[始..].find('}') else {
                break;
            };
            let 印 = &残り[始 + 1..始 + 終];
            // 長さは末尾の数字（`b32_60` の 60）
            let 数字の始め = 印.trim_end_matches(|c: char| c.is_ascii_digit()).len();
            let 長さ: usize = 印[数字の始め..].parse().unwrap_or(0);
            let 字たち: &str = if 印.starts_with("hex") {
                "0f1e2d3c4b5a6978"
            } else if 印.starts_with("alnum") {
                "aZ3kQ9mX7pL2"
            } else if 印.starts_with("upper") {
                "QZ7XK2MW9RT4"
            } else if 印.starts_with("b32") {
                "KQ2ZT7MXR4WP"
            } else {
                出.push_str(&残り[..=始]);
                残り = &残り[始 + 1..];
                continue;
            };
            出.push_str(&残り[..始]);
            出.extend(字たち.chars().cycle().take(長さ));
            残り = &残り[始 + 終 + 1..];
        }
        出.push_str(残り);
        出
    }

    #[test]
    fn 例文の表どおりに止めて_通す() {
        let 表 = include_str!("../tests/secret-cases.tsv");
        let mut 外れ = Vec::new();
        for 行 in 表
            .lines()
            .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        {
            let 欄: Vec<&str> = 行.splitn(3, '\t').collect();
            let (判定, 種類, 本文) = (欄[0], 欄[1], 展開(欄[2]));
            let 見つけた: Vec<&str> = 秘密らしいもの(&本文)
                .into_iter()
                .map(秘密の種類::名前)
                .collect();
            let 合う = match 判定 {
                "止める" => 見つけた.contains(&種類),
                "通す" => 見つけた.is_empty(),
                _ => false,
            };
            if !合う {
                外れ.push(format!("{判定} {種類}: {本文} → {見つけた:?}"));
            }
        }
        assert!(外れ.is_empty(), "{}", 外れ.join("\n"));
    }
}
