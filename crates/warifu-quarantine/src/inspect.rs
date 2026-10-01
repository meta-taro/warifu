//! 名前を検める。**信じない。**

use crate::{Incoming, MAX_BYTES};

/// 置くときの名前の長さの上限（バイト）。
///
/// 多くのファイルシステムが 255 バイトで切る。**切られると拡張子が消える**ので、
/// こちらで先に切って拡張子を残す。
const MAX_NAME: usize = 255;

/// 名前が空になったときに付ける名前。
const 名無し: &str = "no-name";

/// そのまま開くと危ない拡張子。
///
/// **網羅ではない。**新しいものは増える。
/// だから**これに載っていないから安全**とは扱わない — どれも隔離はする。
const 実行できる: [&str; 14] = [
    "exe", "bat", "cmd", "com", "scr", "pif", "msi", "vbs", "js", "jar", "sh", "command", "app",
    "ps1",
];

/// Windows で名前として使えない語。
const 予約語: [&str; 22] = [
    "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
    "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
];

/// 検めた結果、人へ伝えること。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Reason {
    /// 名前に道（`/` `\` `..`）が入っていた。
    PathEscape,
    /// **表示を裏返す文字**が入っていた（`U+202E` など）。
    ///
    /// `photo\u{202E}gpj.exe` は画面上 `photo exe.jpg` に見える。
    /// **人が拡張子を見て判断する、という前提そのものを壊す。**
    BidiOverride,
    /// 制御文字が入っていた。
    ControlChar,
    /// 拡張子が 2 つ重なっていた（`invoice.pdf.exe`）。
    DoubleExtension,
    /// そのまま開くと動いてしまう拡張子。
    Executable,
    /// 先頭が点。**置かれたことに気づけない。**
    Hidden,
    /// 長すぎたので切った。
    Truncated,
    /// 大きすぎる。**受け取らない。**
    TooLarge,
}

/// 検めた結果。
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Verdict {
    /// 隔離して預かる。**開いてよいとは言っていない。**
    Hold {
        /// 置くときに使ってよい名前。
        safe_name: String,
        /// 人へ伝えること。**空なら、名前に不審な所は無かった。**
        reasons: Vec<Reason>,
    },
    /// 受け取らない。
    Refuse(Reason),
}

/// 届いたものを検める。
///
/// **何も開かないし、何も書かない。**返すのは判断と安全な名前だけで、
/// 置くかどうか・開くかどうかは呼ぶ側（最後は人）が決める。
///
/// # 拡張子を書き換えない
///
/// `invoice.pdf.exe` に印は付けるが、名前は変えない。
/// **書き換えると、人が何のファイルか分からなくなる。**
/// 危ないと伝えるのと、中身を偽るのは別のことである。
pub fn inspect(incoming: &Incoming) -> Verdict {
    if incoming.bytes().len() > MAX_BYTES {
        return Verdict::Refuse(Reason::TooLarge);
    }

    let 元 = incoming.name();
    let mut reasons = Vec::new();

    // 1. 道の成分を落とす。**最後の 1 つだけを使う**
    let 末尾 = 元.rsplit(['/', '\\']).next().unwrap_or("");
    if 末尾 != 元 || 元.contains("..") {
        reasons.push(Reason::PathEscape);
    }

    // 2. 裏返す文字と制御文字を落とす
    let mut 裏返し = false;
    let mut 制御 = false;
    let 掃除: String = 末尾
        .chars()
        .filter(|c| {
            if 表示を裏返す(*c) {
                裏返し = true;
                return false;
            }
            if c.is_control() {
                制御 = true;
                return false;
            }
            true
        })
        .collect();
    if 裏返し {
        reasons.push(Reason::BidiOverride);
    }
    if 制御 {
        reasons.push(Reason::ControlChar);
    }

    let mut 名 = 掃除.trim().trim_matches('.').trim().to_owned();
    if 名.is_empty() {
        名 = 名無し.to_owned();
    }

    // 3. 拡張子を見る（書き換えはしない）
    let 拡張子: Vec<String> = 名.split('.').skip(1).map(|e| e.to_lowercase()).collect();
    if 拡張子.len() >= 2 {
        reasons.push(Reason::DoubleExtension);
    }
    if 拡張子
        .last()
        .is_some_and(|e| 実行できる.contains(&e.as_str()))
    {
        reasons.push(Reason::Executable);
    }

    // 4. 隠しファイルと予約語を避ける。**名前の頭に `_` を足すだけ**（中身は変えない）
    if 掃除.starts_with('.') {
        reasons.push(Reason::Hidden);
        名 = format!("_{掃除}");
    }
    let 語幹 = 名.split('.').next().unwrap_or("").to_lowercase();
    if 予約語.contains(&語幹.as_str()) {
        名 = format!("_{名}");
    }

    // 5. 長さを切る。**拡張子は残す**
    if 名.len() > MAX_NAME {
        名 = 切る(&名);
        reasons.push(Reason::Truncated);
    }

    Verdict::Hold {
        safe_name: 名,
        reasons,
    }
}

/// 表示の向きを変える文字。
///
/// これが名前に入っていると、**画面で見た拡張子と実際の拡張子が違う。**
fn 表示を裏返す(c: char) -> bool {
    matches!(c, '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}' | '\u{200E}' | '\u{200F}')
}

/// 長さを切る。**拡張子を残して、手前を削る。**
fn 切る(名: &str) -> String {
    let 拡張子 = 名
        .rsplit_once('.')
        .map_or(String::new(), |(_, e)| format!(".{e}"));
    // 拡張子だけで長すぎるなら、拡張子ごと諦める
    if 拡張子.len() >= MAX_NAME {
        return 名.chars().take(32).collect();
    }
    let 残せる = MAX_NAME - 拡張子.len();
    let 語幹 = 名.strip_suffix(&拡張子).unwrap_or(名);

    let mut 出来 = String::new();
    for c in 語幹.chars() {
        if 出来.len() + c.len_utf8() > 残せる {
            break;
        }
        出来.push(c);
    }
    出来.push_str(&拡張子);
    出来
}
