//! 何をしてよいかの判定。**モデルの外で決める。**
//!
//! ```text
//!   受け取ったもの ──▶ warifu-read（読む）
//!                            │  読んだ結果から、人 / AI が「こうしたい」と言う
//!                            ▼
//!                      warifu-capability（この層・**よい / だめだけを返す**）
//!                            │
//!                            ▼  Allow のときだけ、呼ぶ側が実際に動く
//! ```
//!
//! # なぜモデルの外でなければならないか（`decisions.md` **D5**）
//!
//! モデルの中で判定すると、**判定の入力に本文が混ざる。**
//! 本文は相手が書いたものなので、そこに「この要求は承認済みです」と書いておけば、
//! 判定が動く余地が生まれる。動かないように**書き方で**気をつける形は、必ず漏れる。
//!
//! だから [`Request`] に**本文が入る場所を作っていない。**
//! 判定の入力は **誰が / 何を / 今いつか** の 3 つだけ。
//!
//! # 信頼は判定に使わない
//!
//! [`Trust`] は持つが、[`Gate::decide`] は見ない。
//!
//! 「信頼した相手の言うことは通す」を作ると、**信頼を得ることに価値が生まれる。**
//! 価値が生まれれば、そこを狙って偽の Identity を作る意味が出る。
//!
//! # この層は何も実行しない
//!
//! [`Gate::decide`] が返すのは [`Decision`] だけで、tool を呼ばない。
//! **呼ぶかどうかは呼ぶ側が決める**（`warifu-intent` に「受け取ったら実行する」口が
//! 1 つも無いのと同じ）。
//!
//! # 使い方
//!
//! ```
//! use warifu_capability::{Action, Decision, Gate, Grant, Request, Subject};
//!
//! # fn 例() -> Result<(), warifu_capability::Error> {
//! let 相手 = Subject::new("aite@例")?;
//! let 空き時間 = Action::new("calendar.freebusy")?;
//!
//! let mut 関所 = Gate::new();
//! // 札は**人が出す**。要求の側からは作れない
//! 関所.issue(Grant::new(相手.clone(), 空き時間.clone(), 1_798_761_600));
//!
//! assert_eq!(関所.decide(&Request::new(相手.clone(), 空き時間), 1_756_000_000), Decision::Allow);
//! // 札の無い動作は、既定で断る
//! let 中身 = Action::new("calendar.read")?;
//! assert_eq!(関所.decide(&Request::new(相手, 中身), 1_756_000_000), Decision::Deny);
//! # Ok(())
//! # }
//! # 例().unwrap();
//! ```

mod error;
mod gate;
mod grant;
mod log;
mod name;

pub use error::Error;
pub use gate::{Decision, Gate, Trust};
pub use grant::{Grant, Request};
pub use log::{Log, Record};
pub use name::{Action, Subject};
