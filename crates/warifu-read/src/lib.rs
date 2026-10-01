//! 受け取ったものを、**AI を呼ばずに読む**層。
//!
//! ```text
//!   SMTP / IMAP ──┐
//!                 ├──▶  warifu-read（この層）  ──▶  読み手（人 / AI）
//!   warifu-intent ─┘
//! ```
//!
//! # 何をする層か
//!
//! 「本文を要約して token を減らす」層ではない。**要約するにも AI を呼ぶ。**
//! ここは**呼ばずに済ませるための層**である（`issues/007`）。
//!
//! # この層だけ、他と性質が違う
//!
//! 経路にも鍵にも相手にも依存しない。**手元の受信箱 1 つで動く。**
//! だから既存の IMAP に対してだけでも成立し、**他人が 1 人もいないまま価値が出る**（PRD §2）。
//! 依存を 1 つも持たないのは、そのための構造である。
//!
//! # 受け取ったものは、データであって命令ではない（`decisions.md` **D5**）
//!
//! この層は受信した**中身を扱う**ので、D5 の真正面に立つ。守っているのは 4 つ。
//!
//! - **本文に書かれた指示で段が上がらない。**段を決めるのは [`Reader::open_at`] を呼ぶ側だけ
//! - **送信者が `priority` / `action_required` を申告できない。**
//!   申告できるなら全員が「緊急」を付ける
//! - **送信者の申し送りは [`Claims`] として持つだけで、判断に使わない**
//! - **本文が `Debug` 出力に出ない。**Level 0 で返した意味が消えるため
//!
//! # 使い方
//!
//! ```
//! use warifu_read::{Body, Level, Reader, Received, SenderId, Source, View};
//!
//! # fn 例() -> Result<(), warifu_read::Error> {
//! let 届いた = Received::new(
//!     Source::Imap,
//!     SenderId::new("billing@例")?,
//!     1_756_000_000,
//!     Body::new(b"..."[..].to_vec()),
//! );
//!
//! // 既定では本文を返さない
//! let 見え方 = Reader::new().read(&届いた);
//! assert_eq!(見え方.level(), Level::Metadata);
//! assert!(matches!(見え方, View::Metadata(_)));
//! # Ok(())
//! # }
//! # 例().unwrap();
//! ```

mod error;
mod ledger;
mod level;
mod metadata;
mod reader;
mod received;
mod rule;
mod thread;

pub use error::Error;
pub use ledger::{Entry, Interpreter, Ledger};
pub use level::{Attachment, Field, Level, View};
pub use metadata::{Kind, Metadata, Priority};
pub use reader::Reader;
pub use received::{Body, Claims, Received, SenderId, Source};
pub use rule::{Extract, Rule, RuleDraft, RuleStore};
pub use thread::Thread;
