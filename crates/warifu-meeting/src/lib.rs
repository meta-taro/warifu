//! 会議の名簿と、映像を張るための下ごしらえ。**SDP は読まない。**
//!
//! ```text
//!   warifu-core（割符・鍵）        ← ネットワークを一切知らない
//!         ▲
//!   warifu-net（経路）             ← バイト列を運ぶ。それが何かは知らない
//!         ▲
//!   warifu-intent（口）            ← 「何をしたいか」の形だけ
//!         ▲
//!   warifu-meeting（この層）       ← 誰がいるか・どの段か。中身は見ない
//!         ▲
//!         │ 使う（warifu は呼び返さない）
//!   映像を張る側（M5 以降）        ← WebRTC を知っている
//! ```
//!
//! # ここでやらないこと
//!
//! - **SDP / ICE を解釈しない。**段（申し出か・返事か・経路の候補か）だけを見て、
//!   中身はそのまま渡す。読み始めると Codec の話がここに入り込み、
//!   「Codec を自前で書かない」（`issues/005` 満たすこと 3）が守れなくなる。
//! - **映像を出さない。**M4 で確かめるのは
//!   「**外部のシグナリングサーバを 1 台も使わずに** SDP / ICE を交換できる」ことだけ。
//! - **他人のぶんを預からない。**[`Signal`] に宛先欄が無いのはそのため
//!   （`decisions.md` **D7**）。
//!
//! # 4 人という上限
//!
//! [`MAX_PARTICIPANTS`] はコードに固定してある。5 人以上にすると誰かの端末が
//! 他人の映像を中継することになり、それは **D7** そのもので法的な決着が付いていない。
//! **「運用で気をつける」にしない。**
//!
//! # 使い方
//!
//! ```
//! use warifu_core::Seed;
//! use warifu_meeting::{MeetingId, Notice, Roster, Signal, Step};
//!
//! # fn 例() -> Result<(), Box<dyn core::error::Error>> {
//! let 私 = Seed::from_bytes([1; 32]).profile("Personal").device("PC").public_key();
//! let 会議 = MeetingId::generate();
//! let 名簿 = Roster::new(私);
//!
//! // 呼ぶ。名簿を渡さないと、入る側は誰に繋ぎに行けばよいか分からない
//! let 塊 = Notice::Invite { meeting: 会議, roster: 名簿 }.to_intent()?;
//! assert_eq!(塊.kind().as_str(), "meeting.invite");
//!
//! // 下ごしらえは、中身を見ずに運ぶだけ
//! let 申し出 = Notice::Signal(Signal::new(会議, Step::Offer, b"v=0\r\n".to_vec()));
//! assert_eq!(申し出.to_intent()?.correlation(), 会議.into());
//! # Ok(())
//! # }
//! # 例().unwrap();
//! ```

mod error;
mod notice;
mod roster;
mod signal;

pub use error::Error;
pub use notice::Notice;
pub use roster::{MAX_PARTICIPANTS, MeetingId, Roster};
pub use signal::{MAX_SIGNAL, Signal, Step};
