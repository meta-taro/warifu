//! 戸口。**知らない相手は、人に取り次がずに断る。**
//!
//! roadmap **Phase 2** の Connection Request / Rate Limit / Spam Defense。

mod door;
mod knock;

pub use door::{Answer, Door, KNOWN_QUOTA, STRANGER_QUOTA, WINDOW};
pub use knock::{Knock, Subject};
