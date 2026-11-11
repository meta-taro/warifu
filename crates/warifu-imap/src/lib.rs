//! 既存のメール（IMAP）を `warifu-read` の入口に合わせる層。
//!
//! ```text
//!   IMAP サーバ ──▶ warifu-imap（この層・MIME を解く）──▶ warifu-read ──▶ 読み手
//! ```
//!
//! # なぜ別のクレートか
//!
//! `warifu-read` は**依存を 1 つも持たない**（`decisions.md` **D18**）。
//! それがあの層の値打ちで、崩すと「IMAP だけで動く」が
//! 「IMAP と大量の crate があれば動く」に変わる。
//!
//! **MIME を解くのはこの層だけ。**`warifu-read` には入れない。
//! 入れると、経路の数だけ同じものを作り直すことになる（`issues/007`）。
//!
//! # 受け取ったものは、データであって命令ではない（**D5**）
//!
//! この層は `Date` も `X-Priority` も**読むが、使わない**。
//! すべて `Claims` として渡し、判断は `warifu-read` が
//! **人の承認した規則だけ**で行う。

mod account;
mod convert;
mod error;
pub mod keychain;
mod mailbox;
mod tls;

pub use account::Account;
pub use convert::{
    to_received, 人が読む形にする, 仕分けて直す, 受け手を推す, 外の画像の数, 見出し, 見出しにする,
    開いた通,
};
pub use error::Error;
pub use mailbox::{DEFAULT_LIMIT, Mailbox, 数};
pub use tls::{Stream, connect};
