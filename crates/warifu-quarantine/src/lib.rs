//! 受け取ったファイルを、そのまま置かない。**名前を信じない。**
//!
//! roadmap **Phase 2** の File Quarantine。
//!
//! ```text
//!   warifu-imap（添付を名前ごとそのまま渡す・D22）
//!         │
//!         ▼
//!   warifu-quarantine（この層・**名前を検める**）
//!         │  Hold { safe_name, reasons }
//!         ▼
//!   呼ぶ側が隔離先へ置く ──▶ **開くかどうかは人が決める**
//! ```
//!
//! # なぜ `warifu-imap` は消毒しないのか
//!
//! **消毒したふりをすると、受け取る側が安全だと思って直にパスへ使う**（**D22**）。
//! だからあちらは書き換えずに渡し、**検めるのはここ 1 箇所**にしてある。
//! 両方が中途半端にやると、**両方が「相手がやっている」と思う形**になる。
//!
//! # 名前は相手が書いた文字列である
//!
//! 本文と同じで、**データであって指示ではない**（**D5**）。
//! `../../etc/passwd` も `photo\u{202E}gpj.exe` も、相手が自由に書ける。
//!
//! **一番効くのは表示を裏返す文字**（`U+202E`）で、
//! `photo\u{202E}gpj.exe` は画面上 `photo exe.jpg` に見える。
//! **人が拡張子を見て判断する、という前提そのものを壊す。**
//!
//! # 信頼している相手でも隔離する
//!
//! roadmap Phase 2 は「**Trusted からの File も Zero Trust**」と書いている。
//! 素通しすると、**信頼を得ることに価値が生まれる**
//! （`warifu-capability` の `Trust` と同じ理屈・**D24**）。
//!
//! # この層は何も開かないし、何も書かない
//!
//! 返すのは判断と安全な名前と置き場所だけ。
//! **置くのも開くのも呼ぶ側**で、最後は人が決める。
//!
//! # 使い方
//!
//! ```
//! use warifu_quarantine::{Incoming, Quarantine, Reason, Verdict};
//!
//! let 箱 = Quarantine::new("/tmp/warifu-quarantine");
//! let 届いた = Incoming::new("photo\u{202E}gpj.exe", b"MZ".to_vec());
//!
//! match 箱.accept(&届いた) {
//!     Verdict::Hold { safe_name, reasons } => {
//!         assert!(reasons.contains(&Reason::BidiOverride));
//!         assert!(reasons.contains(&Reason::Executable));
//!         // **開いてよいとは言っていない。**置き場所を返すだけ
//!         let 置き場 = 箱.path_for(&safe_name).unwrap();
//!         assert!(置き場.starts_with("/tmp/warifu-quarantine"));
//!     }
//!     Verdict::Refuse(理由) => eprintln!("受け取りません: {理由:?}"),
//!     その他 => eprintln!("知らない判断: {その他:?}"),
//! }
//! ```

mod incoming;
mod inspect;
mod quarantine;

pub use incoming::{Incoming, MAX_BYTES};
pub use inspect::{Reason, Verdict, inspect};
pub use quarantine::Quarantine;
