//! 会議の中で画像を送る（チャット）。
//!
//! **画面で縮めてから送る。**ここは運ぶだけで、絵を読まない。
//! ただし**頭のバイトが名乗った型と合うか**だけは見る ——
//! 受け取った画面は、名乗った型で描くからである。

use warifu_meeting::{Error, IMAGE_MAX, MeetingId, Notice};

fn 鍵(seed: u8) -> warifu_core::PublicKey {
    warifu_core::Seed::from_bytes([seed; 32])
        .profile("Personal")
        .device("PC")
        .public_key()
}

/// 本物の JPEG の頭（SOI と APP0）。中身は描けなくてよい —— ここは描かない
fn jpeg(長さ: usize) -> Vec<u8> {
    let mut v = vec![0xFF, 0xD8, 0xFF, 0xE0];
    v.resize(長さ.max(4), 0x11);
    v
}

fn 画像(型: &str, 中身: Vec<u8>) -> Notice {
    Notice::Image {
        meeting: MeetingId::generate(),
        from: 鍵(1),
        話し手: None,
        型: 型.into(),
        中身,
    }
}

#[test]
fn 画像がそのまま往復する() {
    let n = 画像("image/jpeg", jpeg(1000));
    let 戻り = Notice::from_intent(&n.to_intent().unwrap()).unwrap();
    assert_eq!(戻り, n);
}

#[test]
fn 札付きでも往復する() {
    let n = Notice::Image {
        meeting: MeetingId::generate(),
        from: 鍵(2),
        話し手: Some("docs".into()),
        型: "image/png".into(),
        中身: b"\x89PNG\r\n\x1a\n0000".to_vec(),
    };
    assert_eq!(Notice::from_intent(&n.to_intent().unwrap()).unwrap(), n);
}

#[test]
fn 上限ちょうどは通り_超えたら組み立てない() {
    assert!(画像("image/jpeg", jpeg(IMAGE_MAX)).to_intent().is_ok());
    // **受け取る側でも数える**（D15）。相手が上限を守る保証は無い
    assert!(matches!(
        画像("image/jpeg", jpeg(IMAGE_MAX + 1)).to_intent(),
        Err(Error::Malformed)
    ));
}

#[test]
fn 空の画像は送らない() {
    assert!(matches!(
        画像("image/jpeg", Vec::new()).to_intent(),
        Err(Error::Malformed)
    ));
}

#[test]
fn 描ける型だけを通す() {
    // **SVG は通さない。**中に script を持てるので、描く側が危ない
    let svg = b"<svg xmlns='http://www.w3.org/2000/svg'></svg>".to_vec();
    assert!(matches!(
        画像("image/svg+xml", svg).to_intent(),
        Err(Error::Malformed)
    ));
    assert!(matches!(
        画像("text/html", jpeg(10)).to_intent(),
        Err(Error::Malformed)
    ));
}

#[test]
fn 名乗った型と頭が合わないものは通さない() {
    // **JPEG と名乗って中身が別物**なら、受け取った画面は嘘の型で描くことになる
    assert!(matches!(
        画像("image/png", jpeg(100)).to_intent(),
        Err(Error::Malformed)
    ));
}

#[test]
fn 届いた塊でも同じ検めをする() {
    // **組み立てる側を通らない塊**（相手が手で作ったもの）も、読む側で止める
    let n = 画像("image/jpeg", jpeg(100));
    let 正しい = n.to_intent().unwrap();
    let mut 荷物 = 正しい.payload().to_vec();
    // 型の欄の直後の頭を壊す（差出人 32 ＋ 札の長さ 1 ＋ 型の長さ 1 ＋ "image/jpeg" 10）
    荷物[32 + 1 + 1 + 10] = b'<';
    let 壊した =
        warifu_intent::Intent::with_correlation(正しい.kind().clone(), 正しい.correlation(), 荷物);
    assert!(matches!(
        Notice::from_intent(&壊した),
        Err(Error::Malformed)
    ));
}

#[test]
fn 文字の口とは混ざらない() {
    let n = 画像("image/jpeg", jpeg(100));
    assert_eq!(n.to_intent().unwrap().kind().as_str(), "meeting.image");
}
