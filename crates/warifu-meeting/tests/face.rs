//! **顔の知らせ**（0.1.53）。その機械の人か席の顔（画像）を、同じルームの相手へ渡す。
//!
//! 名乗り（`Profile`）は名前と紹介だけを運ぶので、置いた顔は置いた機械の画面にしか出なかった。
//! 名乗りの形は変えず、別の知らせとして足す。古い版は知らない知らせとして読み飛ばす。

use warifu_meeting::{Error, FACE_MAX, MeetingId, Notice};

fn 鍵(seed: u8) -> warifu_core::PublicKey {
    warifu_core::Seed::from_bytes([seed; 32])
        .profile("Personal")
        .device("PC")
        .public_key()
}

fn 顔(席: &str, 中身: Vec<u8>) -> Notice {
    Notice::顔 {
        meeting: MeetingId::generate(),
        from: 鍵(1),
        席: 席.to_owned(),
        中身,
    }
}

#[test]
fn 顔は往復し_会議を取り違えない() {
    for n in [
        顔("", vec![1, 2, 3]),
        顔("docs", vec![9; FACE_MAX]),
        // 中身が空は「既定の顔へ戻した」
        顔("docs", Vec::new()),
    ] {
        let 戻った = Notice::from_intent(&n.to_intent().unwrap()).unwrap();
        assert_eq!(戻った, n);
        assert_eq!(戻った.meeting(), n.meeting());
    }
}

#[test]
fn 大きすぎる顔は送らない() {
    assert!(matches!(
        顔("", vec![0; FACE_MAX + 1]).to_intent(),
        Err(Error::Malformed)
    ));
}

#[test]
fn 行を壊す席の名は送らない() {
    for 席 in ["a\nb", "a\tb", "  ", &"x".repeat(200)] {
        assert!(
            matches!(顔(席, vec![1]).to_intent(), Err(Error::Malformed)),
            "{席:?}"
        );
    }
}

#[test]
fn 受ける側でも_大きすぎる顔と壊れた荷物は通さない() {
    let 正しい = 顔("docs", vec![1]).to_intent().unwrap();
    let 作る = |荷物: Vec<u8>| {
        warifu_intent::Intent::with_correlation(正しい.kind().clone(), 正しい.correlation(), 荷物)
    };
    // 送る側の上限を守らない相手が居ても、受ける側で数える
    let mut 大きい = 正しい.payload()[..33 + 4].to_vec();
    大きい.extend(vec![0; FACE_MAX + 1]);
    // 席の長さが荷物より長い
    let mut 足りない = 正しい.payload()[..32].to_vec();
    足りない.push(200);
    // 席に改行
    let mut 改行 = 正しい.payload()[..32].to_vec();
    改行.extend_from_slice(&[2, b'a', b'\n']);
    for 壊れ in [vec![], vec![1; 31], 大きい, 足りない, 改行] {
        assert!(
            matches!(Notice::from_intent(&作る(壊れ)), Err(Error::Malformed)),
            "壊れた荷物を通した"
        );
    }
}
