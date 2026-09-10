//! **顔として置ける画像か**を見る。
//!
//! オーナー指示（2026-09-08）——「**ただユーザによって差し替え可能にもします。**」
//!
//! # 受け取ったファイルを、そのまま信じない
//!
//! **拡張子を信じない。**中身の頭を見る（`warifu-quarantine` と同じ構え）。
//!
//! **置けるのは PNG・WebP・JPEG。**どれも、外の道具なしで縦横を読める。
//!
//! **落とした画像は、これより手前で切り取られている**（**D87**）——
//! 画面側が JPG / PNG / WebP を開いて、512px 角にしてから渡す。
//!
//! **出てくる形は 1 つに決まらない**（2026-09-10 に踏んだ）——
//! `canvas.toBlob` は、**出せない形を頼まれると黙って PNG を返す。**
//! WebP を出せない webview では JPEG へ落ちるので、**ここで JPEG も受ける。**
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
    /// PNG でも WebP でも JPEG でもない。
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
            Self::NotPng => write!(f, "PNG でも WebP でも JPEG でもありません"),
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
    let (幅, 高さ) = pngの縦横(中身)
        .or_else(|| webpの縦横(中身))
        .or_else(|| jpegの縦横(中身))
        .ok_or(BadImage::NotPng)?;
    // **0 は絵ではない。**描こうとした側で割り算が壊れる
    if 幅 == 0 || 高さ == 0 {
        return Err(BadImage::NotPng);
    }
    if 幅 > AVATAR_MAX_SIDE || 高さ > AVATAR_MAX_SIDE {
        return Err(BadImage::TooWide(幅, 高さ));
    }
    Ok((幅, 高さ))
}

/// PNG の縦横。**IHDR は必ず先頭にある**（規格でそう決まっている）。
fn pngの縦横(中身: &[u8]) -> Option<(u32, u32)> {
    // 頭 8 バイト ＋ IHDR（長さ 4・型 4・幅 4・高さ 4）＝ 24 バイトは要る
    if 中身.len() < 24 || 中身[..8] != PNG_MAGIC || &中身[12..16] != b"IHDR" {
        return None;
    }
    let 幅 = u32::from_be_bytes(中身[16..20].try_into().ok()?);
    let 高さ = u32::from_be_bytes(中身[20..24].try_into().ok()?);
    Some((幅, 高さ))
}

/// WebP の縦横（**D87**）。
///
/// `RIFF....WEBP` のあと、塊の名前で 3 通りある。**3 つとも読む** ——
/// 画面側がどれを出すかは、その機械の webview 次第である。
///
/// | 塊 | 何 | 縦横の置き場所 |
/// |---|---|---|
/// | `VP8 ` | 非可逆 | フレームの頭。14 bit ずつ（上の 2 bit は倍率） |
/// | `VP8L` | 可逆 | 1 バイト目のあと、14 bit ずつ（**マイナス 1 されている**） |
/// | `VP8X` | 拡張 | 24 bit ずつ（**マイナス 1 されている**） |
fn webpの縦横(中身: &[u8]) -> Option<(u32, u32)> {
    if 中身.len() < 30 || &中身[..4] != b"RIFF" || &中身[8..12] != b"WEBP" {
        return None;
    }
    match &中身[12..16] {
        b"VP8X" => {
            let w = u32::from(中身[24]) | u32::from(中身[25]) << 8 | u32::from(中身[26]) << 16;
            let h = u32::from(中身[27]) | u32::from(中身[28]) << 8 | u32::from(中身[29]) << 16;
            Some((w + 1, h + 1))
        }
        b"VP8L" => {
            let b = &中身[21..25];
            let 詰めた = u32::from(b[0])
                | u32::from(b[1]) << 8
                | u32::from(b[2]) << 16
                | u32::from(b[3]) << 24;
            Some(((詰めた & 0x3FFF) + 1, ((詰めた >> 14) & 0x3FFF) + 1))
        }
        b"VP8 " => {
            // フレームの頭 3 バイト ＋ 合図 3 バイトのあとに縦横が来る
            if 中身.len() < 30 {
                return None;
            }
            let w = u16::from_le_bytes([中身[26], 中身[27]]) & 0x3FFF;
            let h = u16::from_le_bytes([中身[28], 中身[29]]) & 0x3FFF;
            Some((u32::from(w), u32::from(h)))
        }
        _ => None,
    }
}

/// JPEG の縦横。
///
/// **頭に書いていない。**区切り（`0xFF` ＋ 印）を順に飛ばして、
/// フレームの合図（SOF0〜SOF15。ただし `0xC4` / `0xC8` / `0xCC` は別物）を探す。
/// そこに高さ・幅が 16 bit ずつ入っている。
///
/// **見つからなければ `None`。**読めないものを 0×0 で通さない。
fn jpegの縦横(中身: &[u8]) -> Option<(u32, u32)> {
    if 中身.len() < 4 || 中身[0] != 0xFF || 中身[1] != 0xD8 {
        return None;
    }
    let mut i = 2;
    while i + 3 < 中身.len() {
        // 区切りは 0xFF で始まる。詰め物（0xFF の連続）は飛ばす
        if 中身[i] != 0xFF {
            i += 1;
            continue;
        }
        let 印 = 中身[i + 1];
        if 印 == 0xFF {
            i += 1;
            continue;
        }
        // 長さを持たない印（詰め物・SOI・EOI・RSTn）
        if 印 == 0x00 || 印 == 0xD8 || (0xD0..=0xD9).contains(&印) {
            i += 2;
            continue;
        }
        let 長さ = usize::from(u16::from_be_bytes([中身[i + 2], 中身[i + 3]]));
        let フレームの合図 = (0xC0..=0xCF).contains(&印) && 印 != 0xC4 && 印 != 0xC8 && 印 != 0xCC;
        if フレームの合図 {
            // 長さ 2 ＋ 精度 1 のあとに、高さ・幅が 16 bit ずつ
            if i + 9 >= 中身.len() {
                return None;
            }
            let 高さ = u32::from(u16::from_be_bytes([中身[i + 5], 中身[i + 6]]));
            let 幅 = u32::from(u16::from_be_bytes([中身[i + 7], 中身[i + 8]]));
            return Some((幅, 高さ));
        }
        // **前へ進まない長さは、壊れている**（そこで止めないと回り続ける）
        if 長さ < 2 {
            return None;
        }
        i += 2 + 長さ;
    }
    None
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
    // **形は中身で決まる。**名前は 1 つに固定して、置き換えで済ませる ——
    // `.png` と `.webp` が並ぶと、**どちらが今の顔か**が分からなくなる（D87）
    format!("{安全}.img")
}
