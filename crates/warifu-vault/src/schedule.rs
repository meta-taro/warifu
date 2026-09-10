//! 予定。**割符の中に持つ。**
//!
//! オーナー判断（2026-09-07）——「**予定は割符の中に持つ**（TSV で保存し、
//! `.ics` で出し入れ）。外のカレンダー API とは繋がない。」
//!
//! # ここで守ること
//!
//! - **人が書いた文字を、そのままファイルへ流さない。**区切り（タブ）と改行が
//!   混ざると TSV が崩れ、**次に読んだとき別の予定に見える**
//! - **終わりが始まりより前の予定は読まない。**描く側で幅が負になる
//! - **読めない行があっても、予定ごと落とさない。**1 行の壊れで全部消えるのは事故
//!
//! **題名を外へ出さない口はここには無い。**空き枠だけを渡すのは
//! `warifu-calendar` の仕事である（あちらは `Span` しか返さない）。

/// 題の長さの上限（文字）。
pub const TITLE_MAX: usize = 120;

/// 覚え書きの長さの上限（文字）。
pub const SCHEDULE_NOTE_MAX: usize = 500;

/// 予定 1 つ。**時刻は Unix 秒**（画面が地元の時計に直す）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Appointment {
    start: u64,
    end: u64,
    title: String,
    note: String,
}

impl Appointment {
    /// 予定を作る。
    ///
    /// **題と覚え書きは、置ける形に直してから持つ** ——
    /// 区切りと改行を空白にし、上限で切る。
    #[must_use]
    pub fn new(start: u64, end: u64, title: &str, note: &str) -> Self {
        Self {
            start,
            end,
            title: 整える(title, TITLE_MAX),
            note: 整える(note, SCHEDULE_NOTE_MAX),
        }
    }

    /// 始まり（Unix 秒）。
    #[must_use]
    pub fn start(&self) -> u64 {
        self.start
    }

    /// 終わり（Unix 秒）。
    #[must_use]
    pub fn end(&self) -> u64 {
        self.end
    }

    /// 題。
    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }

    /// 覚え書き。**自分だけのもの**（相手へは送らない）。
    #[must_use]
    pub fn note(&self) -> &str {
        &self.note
    }

    /// 1 行に書く形（TSV）。
    #[must_use]
    pub fn to_line(&self) -> String {
        format!(
            "{}\t{}\t{}\t{}",
            self.start, self.end, self.title, self.note
        )
    }

    /// 1 行から読む。**読めなければ [`None`]**（その行だけ捨てる）。
    #[must_use]
    pub fn from_line(line: &str) -> Option<Self> {
        let mut 欄 = line.split('\t');
        let start: u64 = 欄.next()?.trim().parse().ok()?;
        let end: u64 = 欄.next()?.trim().parse().ok()?;
        let title = 欄.next()?;
        // 覚え書きは無くてもよい（古い行）
        let note = 欄.next().unwrap_or("");
        // **終わりが始まりより前なら読まない。**描く側で幅が負になる
        if end < start {
            return None;
        }
        Some(Self::new(start, end, title, note))
    }
}

/// 置ける形に直す。**区切りと改行を空白にして、上限で切る。** */
fn 整える(素: &str, 上限: usize) -> String {
    let 直した: String = 素
        .chars()
        .map(|c| {
            if c == '\t' || c == '\n' || c == '\r' {
                ' '
            } else {
                c
            }
        })
        .collect();
    直した
        .chars()
        .take(上限)
        .collect::<String>()
        .trim()
        .to_owned()
}
