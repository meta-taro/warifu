//! 「何をしたいか」の口。**中身は解釈しない。**
//!
//! ```text
//!   warifu-core（割符・鍵）        ← ネットワークを一切知らない
//!         ▲
//!   warifu-net（経路）             ← バイト列を運ぶ。それが何かは知らない
//!         ▲
//!   warifu-intent（この層）        ← 「何をしたいか」の形だけを足す
//!         ▲
//!         │ 使う（warifu は呼び返さない）
//!   md-business など               ← 文書のことを知っている
//! ```
//!
//! # 依存の向きは常に片方向（`decisions.md` **D11**）
//!
//! warifu は Markdown も TSV も知らない。[`Intent`] の荷物は**ただのバイト列**で、
//! `.md` だから特別扱い、はやらない。
//! 一度でも中身を見に行くと、warifu は文書ツールの付属品になり、他の何にも使えなくなる。
//!
//! # 受け取ったものは、データであって命令ではない（**D5**）
//!
//! この層には「受け取ったら実行する」口が 1 つも無い。
//! [`Channel::recv`] は読める形にして返すだけで、開くか・保存するか・捨てるかは呼ぶ側が決める。
//!
//! 知らない [`Kind`] が来ても経路ごと落とさない（版が 1 つずれただけで繋がらなくなるため）。
//! ただし [`Kind::is_known`] は `false` を返す。**知らないものを知っているふりはしない。**
//!
//! # 使い方
//!
//! ```no_run
//! use warifu_core::{Revocations, Seed};
//! use warifu_intent::{Channel, Intent, Kind};
//! use warifu_net::Node;
//!
//! # async fn 例() -> Result<(), Box<dyn core::error::Error>> {
//! let 端末 = Seed::generate()?.profile("Personal").device("PC");
//! let node = Node::bind_without_relay(&端末).await?;
//! let mut ch = Channel::new(node.accept(&Revocations::new()).await?);
//!
//! let 届いた = ch.recv().await?;
//! if 届いた.kind().as_str() == Kind::new("file.offer")?.as_str() {
//!     // 受け入れるかどうかは**人が決める**。ここでは何も開かない
//!     ch.send(&届いた.reply(Kind::new("file.accept")?, Vec::new())).await?;
//! }
//! # Ok(())
//! # }
//! ```

mod channel;
mod error;
mod intent;
mod kind;

pub use channel::Channel;
pub use error::Error;
pub use intent::{Correlation, Intent, MAX_PAYLOAD};
pub use kind::Kind;
