//! **割符を持たずに叩く**（`issues/010` 段 3 / D59 の提案）。
//!
//! 戸口は前から「一度開けた相手は、次から割符なしで開ける」と決めていた（**D31**）。
//! だが握手の手順（**D39**）は「生のバイト列で `Acceptance` を 1 通だけ」しか
//! 定めておらず、**割符を持たない相手が何を送るかが決まっていなかった。**
//!
//! 何も送らないと、迎える側は 10 秒待ってから諦める。**待たせない。**

use warifu_app::{KNOCK_WITHOUT_TALLY, is_knock_without_tally};
use warifu_core::{Acceptance, Device, Seed};

fn 端末(seed: [u8; 32]) -> Device {
    Seed::from_bytes(seed).profile("Personal").device("PC")
}

#[test]
fn 割符なしの目印は_割符として読めない() {
    // **迎える側はこれを読もうとして失敗し、すぐ戸口へ回す。**
    // 読めてしまうと、割符を持っていることになる
    assert!(Acceptance::from_bytes(KNOCK_WITHOUT_TALLY).is_err());
}

#[test]
fn 本物の片割れを_割符なしの目印と間違えない() {
    let 主催 = 端末([1u8; 32]);
    let 客 = 端末([2u8; 32]);
    let (_tally, token) = 主催.issue_tally(0, 600).unwrap();
    let 片割れ = 客.accept(&token, 1).unwrap();

    assert!(!is_knock_without_tally(&片割れ.to_bytes()));
}

#[test]
fn 目印は_割符とも宛先とも別の綴りである() {
    // **読んで区別が付く形にする。**同じ 4 文字だと、
    // 目印なのか壊れた割符なのかがログから読めない
    assert!(is_knock_without_tally(KNOCK_WITHOUT_TALLY));
    assert_ne!(&KNOCK_WITHOUT_TALLY[..4], b"WRF1", "割符の綴り");
    assert_ne!(&KNOCK_WITHOUT_TALLY[..4], b"WRFA", "宛先の綴り");
}

#[test]
fn 短すぎる塊を_目印と間違えない() {
    // **切れた通信の残りを目印として読まない**
    assert!(!is_knock_without_tally(b""));
    assert!(!is_knock_without_tally(b"WRF"));
    assert!(!is_knock_without_tally(b"WRFK"));
}

#[test]
fn 版が違う目印は_受け取らない() {
    // **黙って新しい版として扱わない**（D39 の「旧版の鍵は長さで落ちる」と同じ構え）
    let mut 別の版 = KNOCK_WITHOUT_TALLY.to_vec();
    *別の版.last_mut().unwrap() = 0xff;
    assert!(!is_knock_without_tally(&別の版));
}
