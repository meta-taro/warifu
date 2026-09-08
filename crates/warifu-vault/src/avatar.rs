//! **顔として置ける画像か**を見る。
//!
//! オーナー指示（2026-09-08）——「**ただユーザによって差し替え可能にもします。**」
//!
//! # 受け取ったファイルを、そのまま信じない
//!
//! **拡張子を信じない。**中身の頭を見る（`warifu-quarantine` と同じ構え）。
//! **PNG だけを受ける** —— 読める形かどうかを、外の道具なしで確かめられるためである。
//! JPEG を受けるなら、**その形を確かめる手立てを先に持つ。**
//!
//! # ここで守ること
//!
//! - **大きさに上限を置く。**画面に出すだけのものに、機械を埋めさせない
//! - **縦横にも上限を置く。**小さいファイルでも、桁の大きい画像は描くときに膨らむ
//! - **断る理由を分ける。**「大きすぎる」と「形が違う」は、人の次の手が違う

/// 置ける大きさ（バイト）。**64 KiB。**
///
/// 画面に出すのは 40px 角である。**それに 64 KiB は十分すぎる。**
pub const AVATAR_MAX_BYTES: usize = 64 * 1024;

/// 置ける縦横（px）。
///
/// **小さいファイルでも、桁の大きい画像は描くときに膨らむ**
/// （圧縮が効く絵ほど、展開したときの差が大きい）。
pub const AVATAR_MAX_SIDE: u32 = 512;

/// 顔として置けない理由。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BadImage {
    /// ファイルが大きすぎる。
    TooBig(usize),
    /// PNG ではない。
    NotPng,
    /// 縦か横が大きすぎる。
    TooWide(u32, u32),
}

impl core::fmt::Display for BadImage {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::TooBig(n) => write!(
                f,
                "画像が大きすぎます（{n} バイト / 上限 {AVATAR_MAX_BYTES}）"
            ),
            Self::NotPng => write!(f, "PNG ではありません"),
            Self::TooWide(w, h) => write!(
                f,
                "画像の縦横が大きすぎます（{w}×{h} / 上限 {AVATAR_MAX_SIDE}）"
            ),
        }
    }
}

impl core::error::Error for BadImage {}

/// PNG の頭にある決まった 8 バイト。
const PNG_MAGIC: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];

/// **顔として置ける形か。**置けるなら縦横を返す。
///
/// # Errors
/// 大きすぎるとき、PNG でないとき、縦横が上限を超えるとき。
pub fn 顔として読む(中身: &[u8]) -> Result<(u32, u32), BadImage> {
    if 中身.len() > AVATAR_MAX_BYTES {
        return Err(BadImage::TooBig(中身.len()));
    }
    // 頭 8 バイト ＋ IHDR（長さ 4・型 4・幅 4・高さ 4）＝ 24 バイトは要る
    if 中身.len() < 24 || 中身[..8] != PNG_MAGIC || &中身[12..16] != b"IHDR" {
        return Err(BadImage::NotPng);
    }
    let 幅 = u32::from_be_bytes(中身[16..20].try_into().map_err(|_| BadImage::NotPng)?);
    let 高さ = u32::from_be_bytes(中身[20..24].try_into().map_err(|_| BadImage::NotPng)?);
    // **0 は絵ではない。**描こうとした側で割り算が壊れる
    if 幅 == 0 || 高さ == 0 {
        return Err(BadImage::NotPng);
    }
    if 幅 > AVATAR_MAX_SIDE || 高さ > AVATAR_MAX_SIDE {
        return Err(BadImage::TooWide(幅, 高さ));
    }
    Ok((幅, 高さ))
}

/// 顔の置き場所に使うファイル名。
///
/// **人が書いた名前をそのままファイル名にしない**（`warifu-quarantine` と同じ構え）——
/// `desk:../../..` のような名乗りが来ても、置き場所の外へ出られないようにする。
#[must_use]
pub fn 顔のファイル名(誰: &crate::Who) -> String {
    let 素 = 誰.to_field();
    let 安全: String = 素
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    format!("{安全}.png")
}
