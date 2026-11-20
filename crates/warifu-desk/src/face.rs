//! **席の顔を、1 行ずつ運ぶ**（0.1.53）。
//!
//! 顔は 64 KiB まで置ける（`warifu-vault` の `AVATAR_MAX_BYTES`）が、机への 1 行は 4 KiB ほどしか運べない。
//! 口（エージェントの側）が画像を読んで base64 の欠片に分け、机が順に継いで、終わりの欠片で中身を検める。
//!
//! **机にファイルを開かせない。**パスを渡して机に読ませると、エージェントが読めない場所の画像まで
//! 机が代わりに読むことになる。運ぶのは、口が自分で読んだバイト列だけにする。

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;

use crate::{Error, ToDesk};

/// 顔として運べる大きさ（バイト）。**`warifu-vault` の `AVATAR_MAX_BYTES` と同じ 64 KiB。**
///
/// 机はこれより大きいものを継がない。置く前に `顔として読む` でもう一度数える。
pub const 顔の上限: usize = 64 * 1024;

/// 1 行に載せる欠片の大きさ（バイト・base64 にする前）。base64 にすると 4096 字。
pub const 顔の欠片の大きさ: usize = 3 * 1024;

/// 1 行の欠片の字数の上限（base64 の字）。1 行の上限（本文の上限 ＋ 飾り）に収まる。
pub const 顔の欠片の字の上限: usize = 4096;

/// 画像を、机へ送る行に分ける。**空なら「既定へ戻す」の 1 行だけ。**
#[must_use]
pub fn 顔を欠片にする(中身: &[u8]) -> Vec<ToDesk> {
    if 中身.is_empty() {
        return vec![ToDesk::顔 {
            欠片: String::new(),
            終わり: true,
        }];
    }
    let 数 = 中身.chunks(顔の欠片の大きさ).count();
    中身
        .chunks(顔の欠片の大きさ)
        .enumerate()
        .map(|(i, 欠片)| ToDesk::顔 {
            欠片: STANDARD.encode(欠片),
            終わり: i + 1 == 数,
        })
        .collect()
}

/// 欠片の形を見る（書き手の側でも、机の側でも）。**長さと字だけ。**絵かどうかは机が置く前に見る。
pub(crate) fn 欠片を検める(欠片: &str) -> Result<(), Error> {
    if 欠片.len() > 顔の欠片の字の上限 {
        return Err(Error::長すぎる {
            何: "顔の欠片",
            数: 欠片.len(),
            上限: 顔の欠片の字の上限,
            単位: "字",
        });
    }
    if !欠片
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'+' || b == b'/' || b == b'=')
    {
        return Err(Error::Malformed);
    }
    Ok(())
}

/// **机の側で欠片を継ぐ。**口ごとに 1 つ持つ。
#[derive(Debug, Default)]
pub struct 顔の継ぎ {
    溜め: Vec<u8>,
}

impl 顔の継ぎ {
    /// 欠片を 1 つ継ぐ。終わりなら継いだ中身を返す（空なら「既定へ戻す」）。
    ///
    /// **しくじったら、溜めた分を捨てる。**半端な絵を、次の頼みに混ぜない。
    ///
    /// # Errors
    /// 欠片が読めない・継いだ大きさが [`顔の上限`] を超えたとき。理由は人が読む文。
    pub fn 足す(&mut self, 欠片: &str, 終わり: bool) -> Result<Option<Vec<u8>>, String> {
        let 継いだ = 欠片を検める(欠片)
            .map_err(|e| e.to_string())
            .and_then(|()| {
                STANDARD
                    .decode(欠片)
                    .map_err(|_| "顔の欠片を読めません（base64 ではありません）".to_owned())
            });
        let 中身 = match 継いだ {
            Ok(中身) => 中身,
            Err(訳) => {
                self.溜め.clear();
                return Err(訳);
            }
        };
        if self.溜め.len() + 中身.len() > 顔の上限 {
            let 数 = self.溜め.len() + 中身.len();
            self.溜め.clear();
            return Err(format!(
                "画像が大きすぎます（{数} バイト以上 / 上限 {顔の上限}）。512px 角・64 KiB 以下に縮めてから置いてください"
            ));
        }
        self.溜め.extend_from_slice(&中身);
        if 終わり {
            return Ok(Some(std::mem::take(&mut self.溜め)));
        }
        Ok(None)
    }

    /// ここまでに継いだバイト数。
    #[must_use]
    pub fn 受けた(&self) -> usize {
        self.溜め.len()
    }
}

#[cfg(test)]
mod 試験 {
    use super::*;

    fn 継ぐ(行たち: &[ToDesk]) -> Result<Option<Vec<u8>>, String> {
        let mut 継ぎ = 顔の継ぎ::default();
        let mut 最後 = Ok(None);
        for 行 in 行たち {
            // **線を通した形で読む**（書いた行が、机の読む形で通ること）
            let ToDesk::顔 { 欠片, 終わり } = ToDesk::読む(&行.書く()).expect("読める")
            else {
                panic!("顔の行ではない");
            };
            最後 = 継ぎ.足す(&欠片, 終わり);
            最後.as_ref().map_err(Clone::clone)?;
        }
        最後
    }

    #[test]
    fn 分けた欠片は_継ぐと元に戻る() {
        let 中身: Vec<u8> = (0..顔の上限).map(|i| (i % 251) as u8).collect();
        let 行たち = 顔を欠片にする(&中身);
        assert_eq!(行たち.len(), 顔の上限.div_ceil(顔の欠片の大きさ));
        assert_eq!(継ぐ(&行たち).unwrap(), Some(中身));
    }

    #[test]
    fn 欠片の_1_行は_机の行の上限に収まる() {
        let 行たち = 顔を欠片にする(&vec![0xFF; 顔の欠片の大きさ * 2]);
        for 行 in &行たち {
            assert!(
                行.書く().len() <= crate::本文の上限 + 512,
                "{}",
                行.書く().len()
            );
        }
    }

    #[test]
    fn 空なら_既定へ戻す_1_行だけ() {
        let 行たち = 顔を欠片にする(&[]);
        assert_eq!(行たち.len(), 1);
        assert_eq!(継ぐ(&行たち).unwrap(), Some(Vec::new()));
    }

    #[test]
    fn 上限を超えたら継がず_溜めた分を捨てる() {
        let mut 継ぎ = 顔の継ぎ::default();
        let 欠片 = STANDARD.encode(vec![0u8; 顔の欠片の大きさ]);
        let mut 断り = None;
        for _ in 0..=(顔の上限 / 顔の欠片の大きさ) {
            if let Err(訳) = 継ぎ.足す(&欠片, false) {
                断り = Some(訳);
                break;
            }
        }
        let 訳 = 断り.expect("上限を超えたら断る");
        assert!(訳.contains("大きすぎます"), "{訳}");
        assert_eq!(継ぎ.受けた(), 0, "半端な絵を残さない");
    }

    #[test]
    fn 読めない欠片は断る() {
        let mut 継ぎ = 顔の継ぎ::default();
        assert!(継ぎ.足す("あいう", true).is_err());
        assert!(
            継ぎ.足す("abc", true).is_err(),
            "base64 として長さが合わない"
        );
        assert!(ToDesk::読む(r#"{"型":"顔","欠片":"a b","終わり":true}"#).is_err());
    }

    #[test]
    fn 誰の顔かは書けない() {
        // **置けるのは口で名乗った席の顔だけ。**「誰の」を書けると、別の席や人の顔に化けられる
        assert!(ToDesk::読む(r#"{"型":"顔","欠片":"","終わり":true,"who":"me"}"#).is_err());
        assert!(ToDesk::読む(r#"{"型":"顔","欠片":"","終わり":true,"席":"docs"}"#).is_err());
    }

    #[test]
    fn 長すぎる欠片は_書き手の側でも断る() {
        let 行 = ToDesk::顔 {
            欠片: "A".repeat(顔の欠片の字の上限 + 4),
            終わり: true,
        };
        assert!(ToDesk::読む(&行.書く()).is_err());
    }
}
