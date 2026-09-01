//! 予定表。**空いているかどうかだけを外に出す。**
//!
//! 企画書 v2 §17 の会議調整（roadmap **Phase 3** の代表 Demo）。
//!
//! ```text
//!   相手「この窓のどこかで 1 時間ほしい」
//!         │
//!         ▼  warifu-capability の関所を通ったときだけ
//!   Calendar::slots  ──▶  空いている区間だけ（**題名も場所も入らない**）
//!         │
//!         ▼  双方が同じ枠を承認して初めて
//!   Coordination::confirmed  ──▶  確定
//! ```
//!
//! # なぜここが Capability の実地テストになるか
//!
//! **見せてよいものと見せてはいけないものが、同じ予定表の中に混ざっている。**
//!
//! 「空いているか」は渡してよい。「何の予定か」は渡してはいけない。
//! 型を分けずに 1 つの予定表から両方を出そうとすると、
//! **どこかで題名が付いてくる経路ができる。**
//!
//! だから [`Calendar::slots`] は [`Span`] しか返さない。**題名が入る場所が無い。**
//!
//! # 空き枠を返すことは、埋まっている時間を教えることでもある
//!
//! これは避けられない。だから**一度に見える範囲を絞る。**
//!
//! - 窓は [`MAX_WINDOW`]（31 日）まで。**広く取れるなら予定表を丸ごと写し取れる**
//! - 返す件数に上限を置く。**細かく刻んで尋ねられても、一度に出る量を絞る**
//!
//! # 片方だけでは確定しない
//!
//! 片方の Agent が勝手に予定を入れられるなら、
//! **予定表は相手の Agent に開放されているのと同じ**である。
//!
//! # 使い方
//!
//! ```
//! use warifu_calendar::{Calendar, Coordination, Event, Side, Span};
//!
//! # fn 例() -> Result<(), warifu_calendar::Error> {
//! let 朝 = 1_756_803_600;
//! let mut 予定表 = Calendar::new();
//! 予定表.add(Event::new(Span::new(朝 + 3_600, 朝 + 7_200)?, "歯医者"));
//!
//! // 相手へ渡すのはここまで。**題名は入らない**
//! let 候補 = 予定表.slots(&Span::new(朝, 朝 + 14_400)?, 3_600, 3)?;
//! assert_eq!(候補.len(), 2);
//!
//! let mut 調整 = Coordination::new(候補.clone());
//! 調整.accept(Side::Organizer, &候補[0])?;
//! assert_eq!(調整.confirmed(), None, "片方だけでは確定しない");
//!
//! 調整.accept(Side::Invitee, &候補[0])?;
//! assert_eq!(調整.confirmed(), Some(候補[0]));
//! # Ok(())
//! # }
//! # 例().unwrap();
//! ```

mod calendar;
mod coordination;
mod error;
mod span;

pub use calendar::{Calendar, Event, MAX_WINDOW};
pub use coordination::{Coordination, Side};
pub use error::Error;
pub use span::Span;
