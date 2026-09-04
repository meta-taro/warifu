//! 招待の文字列（M5-c3）。**宛先と割符を 1 本にする。**
//!
//! 宛先だけを渡す形にすると、**受け取った側は誰でも繋げてしまう**（D31 に反する）。
//! 割符を一緒に渡すことで、戸口が「割符があるから開ける」と言えるようになる（D12 / D31）。

use warifu_app::{format_invite, parse_invite};
use warifu_core::Seed;
use warifu_meeting::MeetingId;

fn 端末() -> warifu_core::Device {
    Seed::from_bytes([7u8; 32]).profile("Personal").device("PC")
}

#[test]
fn 宛先と割符と会議_id_が往復する() {
    let device = 端末();
    let (_tally, token) = device.issue_tally(1000, 3600).unwrap();
    let 宛先 = "WARIFU1-AAAABBBBCCCC";
    let 会議 = MeetingId::generate();

    let 招待 = format_invite(宛先, &token, 会議);
    let (読んだ宛先, 読んだ割符, 読んだ会議) = parse_invite(&招待).unwrap();

    assert_eq!(読んだ宛先, 宛先);
    assert_eq!(読んだ割符.id(), token.id());
    assert_eq!(読んだ割符.issuer(), token.issuer());
    assert_eq!(読んだ割符.not_after(), token.not_after());
    // **会議 id が渡らないと、入る側が別の会議を名乗ることになる**
    // （2026-09-04 に実機で踏んだ。相手が「別の会議あて」として黙って捨てていた）
    assert_eq!(読んだ会議, 会議);
}

#[test]
fn 招待は一続きの文字列で空白を含まない() {
    // 紙・口頭・QR のどれでも運べる必要がある（M1）
    let device = 端末();
    let (_t, token) = device.issue_tally(1000, 3600).unwrap();
    let 招待 = format_invite("WARIFU1-AAAA", &token, MeetingId::generate());

    assert!(!招待.contains(' '));
    assert!(!招待.contains('\n'));
}

#[test]
fn 区切りが無ければ受け取らない() {
    assert!(parse_invite("WARIFU1-AAAABBBB").is_err());
}

#[test]
fn 会議_id_が無ければ受け取らない() {
    // **古い形の会議キーを黙って受け取らない。**受け取ると、また
    // 「別の会議あて」で捨てられて、原因の分からない不通になる
    let device = 端末();
    let (_t, token) = device.issue_tally(1000, 3600).unwrap();
    let 会議なし = format!(
        "WARIFU1-AAAA#{}",
        warifu_core::base32::encode(&token.to_bytes())
    );
    assert!(parse_invite(&会議なし).is_err());
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
    let 招待 = format_invite("", &token, MeetingId::generate());
    assert!(parse_invite(&招待).is_err());
}

#[test]
fn 書き換わった割符は受け取らない() {
    // 署名は TallyToken::from_bytes が見る。1 文字変えれば通らない
    let device = 端末();
    let (_t, token) = device.issue_tally(1000, 3600).unwrap();
    let 招待 = format_invite("WARIFU1-AAAA", &token, MeetingId::generate());

    // **割符の部分**を 1 文字書き換える（署名が守っているのはここ）
    let (頭, 尾) = 招待.split_once('#').unwrap();
    let (割符, 会議) = 尾.split_once('#').unwrap();
    let mut 壊す: Vec<char> = 割符.chars().collect();
    壊す[0] = if 壊す[0] == 'A' { 'B' } else { 'A' };
    let 壊れた = format!("{頭}#{}#{会議}", 壊す.into_iter().collect::<String>());

    assert!(parse_invite(&壊れた).is_err());
}

#[test]
fn 会議_id_の書き換えは_ここでは_見抜けない() {
    // **これは弱点である。**会議 id に署名は無い。
    // 書き換わっていても読めてしまい、**相手が「別の会議あて」として捨てる**
    // ＝ 無言の不通になる（2026-09-04 に踏んだ形）。
    //
    // 見抜けないことを**テストで固定しておく** — 「守られている」と誤解しないため。
    // 実際の防ぎ方は、捨てたことをログに出すこと（`src-tauri` の受信側）。
    let device = 端末();
    let (_t, token) = device.issue_tally(1000, 3600).unwrap();
    let 招待 = format_invite("WARIFU1-AAAA", &token, MeetingId::generate());

    let (頭, 尾) = 招待.split_once('#').unwrap();
    let (割符, 会議) = 尾.split_once('#').unwrap();
    let mut 壊す: Vec<char> = 会議.chars().collect();
    壊す[0] = if 壊す[0] == 'A' { 'B' } else { 'A' };
    let 別の会議 = format!("{頭}#{割符}#{}", 壊す.into_iter().collect::<String>());

    // **読めてしまう。**ここで止められない
    assert!(parse_invite(&別の会議).is_ok());
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
