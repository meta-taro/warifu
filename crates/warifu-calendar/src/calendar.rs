//! 予定表。**空いているかどうかだけを外に出す。**

use core::fmt;

use crate::{Error, Span};

/// 尋ねてよい窓の上限（秒）。**31 日。**
///
/// 窓を広く取れるなら、空き枠を尋ねるだけで**相手の予定表を丸ごと写し取れる。**
/// 空き枠を返すことは、裏返せば**埋まっている時間を教えること**でもある。
/// そこは避けられないので、**一度に見える範囲を絞る。**
pub const MAX_WINDOW: u64 = 31 * 24 * 60 * 60;

/// 予定 1 件。
///
/// 題名を持つが、**外へ出す口が無い。**
/// 参照するのは持ち主だけで、[`Calendar::slots`] は区間しか見ない。
#[derive(Clone)]
pub struct Event {
    span: Span,
    title: String,
}

impl Event {
    /// 予定を作る。
    pub fn new(span: Span, title: &str) -> Self {
        Self {
            span,
            title: title.to_owned(),
        }
    }

    /// いつか。
    pub fn span(&self) -> Span {
        self.span
    }

    /// 題名。**持ち主が自分の予定表を見るときにだけ使う。**
    ///
    /// この値が相手へ渡る経路は、この層のどこにも無い。
    pub fn title(&self) -> &str {
        &self.title
    }
}

impl fmt::Debug for Event {
    /// **題名を出さない。**`{:?}` で予定表を出したときに漏れると、
    /// 空き時間だけを返している意味が消える。
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Event")
            .field("span", &self.span)
            .field("title", &"（伏せる）")
            .finish()
    }
}

/// 予定表。
#[derive(Debug, Clone, Default)]
pub struct Calendar {
    events: Vec<Event>,
}

impl Calendar {
    /// 空の予定表。
    pub fn new() -> Self {
        Self::default()
    }

    /// 予定を足す。
    pub fn add(&mut self, event: Event) {
        self.events.push(event);
    }

    /// 自分の予定。**持ち主だけが見る。**
    pub fn events(&self) -> &[Event] {
        &self.events
    }

    /// 空いている枠を返す。**返るのは区間だけで、予定の中身は入らない。**
    ///
    /// - `within` の中だけを見る。**`MAX_WINDOW` より広ければ断る**
    /// - `duration` に満たない隙間は返さない（使えない枠を数えても仕方がない）
    /// - `max` 件で打ち切る。**細かく刻んで尋ねられても、一度に出る量を絞る**
    ///
    /// # 失敗
    ///
    /// [`Error::WindowTooWide`] / [`Error::Malformed`]（`duration` が 0）。
    pub fn slots(&self, within: &Span, duration: u64, max: usize) -> Result<Vec<Span>, Error> {
        if within.duration() > MAX_WINDOW {
            return Err(Error::WindowTooWide);
        }
        if duration == 0 {
            return Err(Error::Malformed);
        }

        // 窓に掛かる予定だけを取り、始まり順に並べる
        let mut 埋まり: Vec<Span> = self
            .events
            .iter()
            .map(Event::span)
            .filter(|s| s.overlaps(within))
            .collect();
        埋まり.sort_unstable();

        let mut 空き = Vec::new();
        let mut 手前 = within.start();
        for s in 埋まり {
            // 隙間が求めた長さに届いていれば残す。
            // 接した予定の間に 0 秒の空きを作らないよう、届かない隙間はここで落ちる
            if s.start() > 手前 && s.start() - 手前 >= duration {
                空き.push(Span::new(手前, s.start())?);
                if 空き.len() == max {
                    return Ok(空き);
                }
            }
            手前 = 手前.max(s.end());
        }
        if within.end() > 手前 && within.end() - 手前 >= duration {
            空き.push(Span::new(手前, within.end())?);
        }
        空き.truncate(max);
        Ok(空き)
    }
}
