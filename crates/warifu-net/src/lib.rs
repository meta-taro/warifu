//! 経路。**割符の形はここに依存しない。**
//!
//! ```text
//!   warifu-core（割符・鍵）        ← ネットワークを一切知らない
//!         ▲
//!         │ 公開鍵で相手を指すだけ
//!   warifu-net（この層）           ← iroh の上に載る薄い層
//!         ▲
//!         │ QUIC / NAT 越え / 暗号は全部下に任せる
//!       iroh
//! ```
//!
//! この層が引き受けるのは 2 つだけ。
//!
//! 1. **相手が本当にその公開鍵の持ち主か**を、繋がった時点で確かめる
//! 2. **失効している相手を通さない**（呼ぶ側・受ける側の双方で見る）
//!
//! [`Node::bind_without_relay`] は中継を使わない。
//! 中継を挟むと「誰がいつ誰に繋いだか」が中継の運用者に出る（`decisions.md` **D10**）。
//!
//! # 使い方
//!
//! ```no_run
//! use warifu_core::{Revocations, Seed};
//! use warifu_net::Node;
//!
//! # async fn 例() -> Result<(), Box<dyn core::error::Error>> {
//! let 端末 = Seed::generate()?.profile("Personal").device("PC");
//! let node = Node::bind_without_relay(&端末).await?;
//!
//! // この文字列を相手に渡す（割符と同じく warifu の外を通る）
//! let 宛先 = node.address().await?;
//! println!("{宛先}");
//!
//! let mut session = node.accept(&Revocations::new()).await?;
//! let 届いた = session.recv().await?;
//! session.send(&届いた).await?;
//! # Ok(())
//! # }
//! ```

mod address;
mod error;
mod node;

pub use address::Address;
pub use error::Error;
pub use node::{MAX_MESSAGE, Node, Session};
