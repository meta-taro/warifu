//! **預かり所とのやりとり。**預ける・受け取る・応じる。
//!
//! ```text
//!   A ──封──▶ 預かり所（読めない）
//!                  │
//!   B が起動 ──────┘ 受け取って、預かり所からは消える
//! ```
//!
//! 預かり所そのもの（[`warifu_post::Box`]）は経路を知らない。
//! **経路を知っているのはこの層だけ**である（`product-baseline.md` §9）。
//!
//! # ここで守ること
//!
//! - **封をしてから預ける**（**D69**）。預かり所は開ける鍵を持たない
//! - **受け取れるのは、経路で確定した自分あてだけ**（[`Ask::Take`] は宛先を持たない）
//! - **断る理由を言わない**（総当たりの手がかりになる）

#![forbid(unsafe_code)]

mod letter;

pub use letter::{手紙, 手紙にする, 手紙を読む};

use warifu_core::{Device, PublicKey};
use warifu_net::{Address, Node};
use warifu_post::{Ask, Box as 預かり所, Reply};

/// うまくいかなかったとき。
#[derive(Debug)]
pub enum Error {
    /// 経路で落ちた。
    Net(warifu_net::Error),
    /// 封ができなかった／開かなかった。
    Seal(warifu_seal::Error),
    /// 預かり所が断った。**理由は返らない。**
    Refused,
    /// 返事が読めなかった（版が違う相手かもしれない）。
    Unreadable,
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Net(e) => write!(f, "預かり所へ繋がりませんでした（{e}）"),
            Self::Seal(e) => write!(f, "封を扱えませんでした（{e}）"),
            Self::Refused => write!(f, "預かり所が預かりませんでした"),
            Self::Unreadable => write!(f, "預かり所の返事が読めませんでした"),
        }
    }
}

impl core::error::Error for Error {}

impl From<warifu_net::Error> for Error {
    fn from(e: warifu_net::Error) -> Self {
        Self::Net(e)
    }
}

impl From<warifu_seal::Error> for Error {
    fn from(e: warifu_seal::Error) -> Self {
        Self::Seal(e)
    }
}

/// **1 通に組み、封をして、預かり所へ預ける。**
///
/// 預かり所は中身を読めない。**誰から誰へ、も分からない**（使い捨ての鍵で封をする）。
/// **署名のない言葉は預けられない** —— 受け取った側が、誰が言ったかを確かめられなくなる。
///
/// # Errors
///
/// 繋がらなかったとき、封ができなかったとき、預かり所が断ったとき。
pub async fn 預ける(
    node: &Node,
    預かり所: &Address,
    私: &Device,
    宛先: PublicKey,
    今: u64,
    本文: &[u8],
) -> Result<(), Error> {
    let 封 = warifu_seal::seal(宛先, &手紙にする(私, 宛先, 今, 本文))?;
    let 返 = 一度だけ話す(
        node,
        預かり所,
        &Ask::Put {
            to: 宛先,
            sealed: 封.to_bytes(),
        },
    )
    .await?;
    match 返 {
        Reply::Kept => Ok(()),
        Reply::Refused => Err(Error::Refused),
        Reply::Handed(_) => Err(Error::Unreadable),
    }
}

/// **預かり所から受け取って、開けて、誰が言ったかを確かめる。**
///
/// 開けられなかった封と、**署名が合わなかった手紙は黙って捨てる。**
/// 1 通のために全部が止まらないようにする ——
/// **ただし「捨てた」ことは呼ぶ側が数えられる**（返る数と、預かり所が渡した数の差）。
///
/// # Errors
///
/// 繋がらなかったとき、返事が読めなかったとき。
pub async fn 受け取る(
    node: &Node,
    私: &Device,
    預かり所: &Address,
) -> Result<Vec<手紙>, Error> {
    let 返 = 一度だけ話す(node, 預かり所, &Ask::Take).await?;
    let Reply::Handed(封たち) = 返 else {
        return Err(Error::Unreadable);
    };
    Ok(封たち
        .iter()
        .filter_map(|b| warifu_seal::Sealed::from_bytes(b))
        .filter_map(|封| warifu_seal::open(私, &封).ok())
        // **名乗りだけの言葉は通さない**（`letter.rs`）
        .filter_map(|中身| 手紙を読む(私.public_key(), &中身))
        .collect())
}

/// 1 往復だけして閉じる。**繋ぎっぱなしにしない。**
async fn 一度だけ話す(
    node: &Node, 預かり所: &Address, 頼み: &Ask
) -> Result<Reply, Error> {
    let mut session = node
        .connect(預かり所, &warifu_core::Revocations::new())
        .await?;
    session.send(&頼み.to_bytes()).await?;
    let 塊 = session.recv().await?;
    let 返 = Reply::from_bytes(&塊).ok_or(Error::Unreadable)?;
    let _ = session.finish().await;
    Ok(返)
}

/// 届いた塊を、頼みとして読めるか。**読めなければ何もしない。**
#[must_use]
pub fn 頼みを聞ける形か(塊: &[u8]) -> Option<Ask> {
    Ask::from_bytes(塊)
}

/// **預かり所として応える。**ここに経路は出てこない（だから試験できる）。
///
/// `相手` は**経路で確定した公開鍵**である。尋ねる側の申告ではない。
#[must_use]
pub fn 応える(箱: &mut 預かり所, 相手: PublicKey, 頼み: Ask, 今: u64) -> Reply {
    箱.forget_old(今);
    match 頼み {
        // **差出人を見ない。**宛先と封だけで足りる
        Ask::Put { to, sealed } => match 箱.put(to, sealed, 今) {
            Ok(()) => Reply::Kept,
            // **理由を言わない。**総当たりの手がかりになる
            Err(_) => Reply::Refused,
        },
        // **経路で確定した相手あてだけを渡す。**
        // 尋ねる側に宛先を名乗らせると、他人あてを引き取れる
        Ask::Take => Reply::Handed(
            箱.take(相手, 今)
                .into_iter()
                .map(|i| i.sealed().to_vec())
                .collect(),
        ),
    }
}
