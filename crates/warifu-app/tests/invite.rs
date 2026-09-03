//! 招待の文字列（M5-c3）。**宛先と割符を 1 本にする。**
//!
//! 宛先だけを渡す形にすると、**受け取った側は誰でも繋げてしまう**（D31 に反する）。
//! 割符を一緒に渡すことで、戸口が「割符があるから開ける」と言えるようになる（D12 / D31）。

use warifu_app::{format_invite, parse_invite};
use warifu_core::Seed;

fn 端末() -> warifu_core::Device {
    Seed::from_bytes([7u8; 32]).profile("Personal").device("PC")
}

#[test]
fn 宛先と割符が往復する() {
    let device = 端末();
    let (_tally, token) = device.issue_tally(1000, 3600).unwrap();
    let 宛先 = "WARIFU1-AAAABBBBCCCC";

    let 招待 = format_invite(宛先, &token);
    let (読んだ宛先, 読んだ割符) = parse_invite(&招待).unwrap();

    assert_eq!(読んだ宛先, 宛先);
    assert_eq!(読んだ割符.id(), token.id());
    assert_eq!(読んだ割符.issuer(), token.issuer());
    assert_eq!(読んだ割符.not_after(), token.not_after());
}

#[test]
fn 招待は一続きの文字列で空白を含まない() {
    // 紙・口頭・QR のどれでも運べる必要がある（M1）
    let device = 端末();
    let (_t, token) = device.issue_tally(1000, 3600).unwrap();
    let 招待 = format_invite("WARIFU1-AAAA", &token);

    assert!(!招待.contains(' '));
    assert!(!招待.contains('\n'));
}

#[test]
fn 区切りが無ければ受け取らない() {
    assert!(parse_invite("WARIFU1-AAAABBBB").is_err());
}

#[test]
fn 割符が読めなければ受け取らない() {
    assert!(parse_invite("WARIFU1-AAAA#ZZZZ").is_err());
    assert!(parse_invite("WARIFU1-AAAA#").is_err());
}

#[test]
fn 宛先が空なら受け取らない() {
    let device = 端末();
    let (_t, token) = device.issue_tally(1000, 3600).unwrap();
    let 招待 = format_invite("", &token);
    assert!(parse_invite(&招待).is_err());
}

#[test]
fn 書き換わった割符は受け取らない() {
    // 署名は TallyToken::from_bytes が見る。1 文字変えれば通らない
    let device = 端末();
    let (_t, token) = device.issue_tally(1000, 3600).unwrap();
    let 招待 = format_invite("WARIFU1-AAAA", &token);

    let mut 壊す: Vec<char> = 招待.chars().collect();
    let 最後 = 壊す.len() - 1;
    壊す[最後] = if 壊す[最後] == 'A' { 'B' } else { 'A' };
    let 壊れた: String = 壊す.into_iter().collect();

    assert!(parse_invite(&壊れた).is_err());
}

#[test]
fn 自分が出した招待だと分かる() {
    // **1 台で 2 窓を開くと必ず踏む。**下の層（iroh）は
    // "Connecting to ourself is not supported" としか言わないので、ここで気づく
    let 私 = 端末();
    let (_t, token) = 私.issue_tally(1000, 3600).unwrap();

    assert!(warifu_app::is_own_invite(私.public_key(), &token));
}

#[test]
fn 相手が出した招待は自分のものではない() {
    let 私 = 端末();
    let 相手 = Seed::from_bytes([9u8; 32])
        .profile("Personal")
        .device("スマホ");
    let (_t, token) = 相手.issue_tally(1000, 3600).unwrap();

    assert!(!warifu_app::is_own_invite(私.public_key(), &token));
}
