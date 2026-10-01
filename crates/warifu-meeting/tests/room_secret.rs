//! **部屋の合言葉を渡す**（**D118** / **#28** の本線 D・2026-09-17）。
//!
//! **割符は 2 人の間のもの。**主催とゲスト A、主催とゲスト B ——
//! **A と B の間には何も無い。**だから A が B を呼んでも
//! **B の戸口が黙って落とす**（**D31** の正しい拒否）。3 台で測って出た ——
//!
//! ```text
//! 紹介: 呼べませんでした: 割符が付いていません（宛先だけでは繋げません）
//! ```
//!
//! **合言葉を渡せば、A は B へ「この部屋の者だ」と示せる。**
//!
//! ここで固定するのは 3 つ。
//!
//! 1. **渡して読み戻せる**
//! 2. **記録に出ない**（`Notice` は `#[derive(Debug)]` なので、型で守っている）
//! 3. **長さの違うものを通さない**（相手が上限を守る保証は無い）

use warifu_meeting::{MeetingId, Notice};

#[test]
fn 合言葉は_渡して読み戻せる() {
    let 部屋 = MeetingId::generate();
    let 言 = warifu_core::合言葉::作る().expect("作れる");
    let 元 = 言.バイト列();

    let 塊 = Notice::RoomSecret {
        meeting: 部屋,
        合言葉: 言,
    }
    .to_intent()
    .expect("包める");
    let 戻した = Notice::from_intent(&塊).expect("読める");

    match 戻した {
        Notice::RoomSecret { meeting, 合言葉 } => {
            assert_eq!(meeting, 部屋);
            assert_eq!(合言葉.バイト列(), 元);
        }
        ほか => panic!("別の知らせになった: {ほか:?}"),
    }
}

#[test]
fn 合言葉は_記録に出ない() {
    // **`Notice` は `#[derive(Debug)]`。**
    // 生のバイト列を持たせると **`{:?}` 1 つで合言葉が記録へ落ちる** ——
    // **型で守っている**ことを、ここで固定する
    let 言 = warifu_core::合言葉::作る().expect("作れる");
    let 中身 = 言.バイト列();
    let 知らせ = Notice::RoomSecret {
        meeting: MeetingId::generate(),
        合言葉: 言,
    };

    let 文 = format!("{知らせ:?}");

    assert!(文.contains("出しません"), "中身が出ている: {文}");
    let 十六 = 中身.iter().map(|b| format!("{b:02x}")).collect::<String>();
    assert!(!文.contains(&十六[..8]), "16 進で出ている");
    assert!(
        !文.contains(&warifu_core::base32::encode(&中身)[..8]),
        "base32 で出ている"
    );
}

#[test]
fn 長さの違う合言葉は通さない() {
    // **相手が上限を守る保証は無い。**短いものを通すと、**弱い合言葉を押し込める**
    let 正 = Notice::RoomSecret {
        meeting: MeetingId::generate(),
        合言葉: warifu_core::合言葉::作る().expect("作れる"),
    }
    .to_intent()
    .expect("包める");

    for 長 in [0usize, 1, 16, 31, 33, 64] {
        let 壊れた = warifu_intent::Intent::with_correlation(
            正.kind().clone(),
            正.correlation(),
            vec![7u8; 長],
        );
        assert!(Notice::from_intent(&壊れた).is_err(), "{長} バイトで通った");
    }
}

#[test]
fn どの部屋の合言葉かを言える() {
    // **部屋ごとに別の合言葉。**どの部屋のものか分からないと、しまう先が決まらない
    let 部屋 = MeetingId::generate();
    let 知らせ = Notice::RoomSecret {
        meeting: 部屋,
        合言葉: warifu_core::合言葉::作る().expect("作れる"),
    };

    assert_eq!(知らせ.meeting(), 部屋);
}
