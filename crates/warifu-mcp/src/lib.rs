//! 受信箱を MCP の口として出す。**すべての呼び出しが関所を通る。**
//!
//! ```text
//!   AI（MCP クライアント）
//!         │  tools/call
//!         ▼
//!   warifu-mcp（この層）
//!         │  **必ず** warifu-capability::Gate に尋ねる
//!         ▼
//!   Allow のときだけ warifu-read を読む ──▶ 返す
//! ```
//!
//! # 「人のように操作できる」ことと、「人の代わりに決められる」ことは別
//!
//! AI が受信箱を人のように扱えることが、この口の目的である。
//! **だが、何をしてよいかを AI が決めてよいわけではない。**
//!
//! だから **tool から関所を迂回する経路を 1 本も置いていない。**
//! 呼べば必ず [`warifu_capability::Gate::decide`] を通り、通らなければ何も返らない。
//! これが PRD §12-2 の「**モデルから直接 tool を呼ばせない / policy engine 経由**」にあたる。
//!
//! # 承認の口を出さない
//!
//! 規則の承認（`RuleStore::approve`）も札の発行（`Gate::issue`）も、**tool にしていない。**
//!
//! 出した時点で、**AI が自分に許可を出せる。**
//! 生成と適用を分けた意味（`decisions.md` **D19** / **D24**）が、そこで消える。
//!
//! # 段ごとに札が要る
//!
//! 本文を読む（Level 3）札と、metadata を見る（Level 0）札は**別**である。
//! 関所の照合は完全一致なので、`inbox.open.raw` の札で `inbox.list` は通らない。

mod server;
mod tools;

pub use server::{Warifu, subject};
pub use tools::{OpenArgs, ToolError};
