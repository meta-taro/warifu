//! warifu の核。**ネットワークには触れない。**
//!
//! 割符 — 二つに割った札。片割れが合うことで、相手が確かにその相手だと証明する。
//!
//! ```text
//!   シード ──▶ Profile ──▶ Device ──▶ 割符 ──▶ 相手（Peer）
//!                                       └─ 失効の名簿で止める
//! ```
//!
//! ここには**経路が一切出てこない**。割符は QR で撮っても、紙に書いて渡しても成立する。
//! 経路（[`iroh`] を使う `warifu-net`）はこの上に載る別の層で、
//! **入れ替えても割符の形は変わらない。**
//!
//! [`iroh`]: https://github.com/n0-computer/iroh
//!
//! # 使い方
//!
//! ```
//! use warifu_core::{Revocations, Seed, TallyToken};
//!
//! let alice = Seed::generate()?.profile("Personal").device("PC");
//! let bob = Seed::generate()?.profile("Personal").device("スマホ");
//!
//! // alice が割符を作り、片方を bob に渡す（QR・紙・口頭 — warifu の外を通る）
//! let (mut 控え, 渡す半分) = alice.issue_tally(1_755_000_000, 60 * 60)?;
//! let 文字列 = 渡す半分.to_string();
//!
//! // bob が受け取って応じる
//! let 受け取った: TallyToken = 文字列.parse()?;
//! let 受諾 = bob.accept(&受け取った, 1_755_000_010)?;
//!
//! // alice の手元で片割れが合う
//! let 相手 = 控え.match_half(&受諾, 1_755_000_020, &Revocations::new())?;
//! assert_eq!(相手.public_key(), bob.public_key());
//! # Ok::<(), warifu_core::Error>(())
//! ```

pub mod base32;
mod error;
mod key;
mod revocation;
mod tally;

/// **この機械の呼び名。**鍵はこれを混ぜて導かれる（`Profile::device`）。
///
/// # 1 台は 1 つの身元である
///
/// 2026-09-11 に別マシンで見つかった —— 本番のコードに呼び名が **3 つ**あった。
///
/// | どこ | 呼び名 | 結果 |
/// |---|---|---|
/// | 画面 | `この端末` | 画面の鍵 |
/// | `warifu id` | `cli` | **別の鍵** |
/// | `warifu host` / `join` | `PC` | **さらに別の鍵** |
///
/// つまり **1 台が最大 3 人に見えていた。**
/// `warifu id` が出した鍵で相手が待っているのに、`warifu join` は別の鍵で入る ——
/// **同じ人だと分からない。**
///
/// # **変えてはいけない**
///
/// 変えると**全員の鍵が変わる。**連絡先も戸口の知り合いも、
/// 渡してあるルームキーも、指す先を失う。**変えるなら決定から。**
pub const 端末の呼び名: &str = "この端末";

pub use error::Error;
pub use key::{Device, Profile, PublicKey, Seed, Signature};
pub use revocation::Revocations;
pub use tally::{Acceptance, Peer, Tally, TallyId, TallyToken};
