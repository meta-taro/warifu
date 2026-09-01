//! 受け取ったファイルの名前を検める。
//!
//! roadmap **Phase 2** の File Quarantine。
//! `warifu-imap` は添付の名前を**書き換えずに**渡す（`decisions.md` **D22**）。
//! 消毒したふりをすると、受け取る側が安全だと思って直にパスへ使うからである。
//!
//! **その「受け取る側」がここ。**
//!
//! # 名前は相手が書いた文字列である
//!
//! 本文と同じで、**データであって指示ではない**（**D5**）。
//! `../../etc/passwd` も `photo\u{202E}gpj.exe` も、相手が自由に書ける。

use warifu_quarantine::{Incoming, MAX_BYTES, Reason, Verdict, inspect};

fn 届いた(名前: &str) -> Incoming {
    Incoming::new(名前, b"...".to_vec())
}

fn 安全な名前(名前: &str) -> String {
    match inspect(&届いた(名前)) {
        Verdict::Hold { safe_name, .. } => safe_name,
        Verdict::Refuse(r) => panic!("受け取られませんでした: {r:?}"),
        その他 => panic!("知らない判断: {その他:?}"),
    }
}

fn 理由(名前: &str) -> Vec<Reason> {
    match inspect(&届いた(名前)) {
        Verdict::Hold { reasons, .. } => reasons,
        Verdict::Refuse(r) => vec![r],
        _ => panic!("知らない判断"),
    }
}

#[test]
fn 道を登る名前は成分ごと落とす() {
    assert_eq!(安全な名前("../../etc/passwd"), "passwd");
    assert_eq!(安全な名前("/etc/passwd"), "passwd");
    assert_eq!(安全な名前(r"..\..\windows\system32\cmd.exe"), "cmd.exe");
    assert!(理由("../../etc/passwd").contains(&Reason::PathEscape));
}

#[test]
fn 表示を裏返す文字を落とす() {
    // **これが一番効く。**"photo\u{202E}gpj.exe" は画面上 "photo exe.jpg" に見える。
    // 人が拡張子を見て判断する、という前提そのものを壊す
    let 罠 = "photo\u{202E}gpj.exe";

    let 安全 = 安全な名前(罠);

    assert!(
        !安全.contains('\u{202E}'),
        "裏返す文字が残っています: {安全:?}"
    );
    assert!(理由(罠).contains(&Reason::BidiOverride));
}

#[test]
fn 制御文字を落とす() {
    let 安全 = 安全な名前("invo\u{0}ice\n.pdf");

    assert!(!安全.chars().any(char::is_control));
    assert!(理由("invo\u{0}ice.pdf").contains(&Reason::ControlChar));
}

#[test]
fn 二重の拡張子に印を付ける() {
    // 見えている拡張子と、実際に開かれる拡張子が違う
    assert!(理由("invoice.pdf.exe").contains(&Reason::DoubleExtension));
    assert!(!理由("invoice.pdf").contains(&Reason::DoubleExtension));
}

#[test]
fn 実行できる拡張子に印を付ける() {
    for 名 in [
        "a.exe",
        "a.bat",
        "a.cmd",
        "a.scr",
        "a.js",
        "a.command",
        "a.sh",
    ] {
        assert!(
            理由(名).contains(&Reason::Executable),
            "{名} に印が付いていません"
        );
    }
    assert!(!理由("a.pdf").contains(&Reason::Executable));
}

#[test]
fn 印は付けるが拡張子を書き換えない() {
    // **書き換えると、人が何のファイルか分からなくなる。**
    // 危ないと伝えるのと、中身を偽るのは別のこと
    assert_eq!(安全な名前("invoice.pdf.exe"), "invoice.pdf.exe");
}

#[test]
fn windows_の予約名を避ける() {
    // CON / NUL / COM1 は、Windows では**ファイルとして開けない**
    for 名 in ["CON", "nul.txt", "COM1.pdf", "LPT9"] {
        let 安全 = 安全な名前(名);
        assert!(安全.starts_with('_'), "{名} → {安全} が避けられていません");
    }
    assert_eq!(
        安全な名前("console.txt"),
        "console.txt",
        "似ているだけの名前は触らない"
    );
}

#[test]
fn 先頭の点を避ける() {
    // 隠しファイルとして置かれると、置かれたことに気づけない
    assert_eq!(安全な名前(".bashrc"), "_.bashrc");
    assert!(理由(".bashrc").contains(&Reason::Hidden));
}

#[test]
fn 長すぎる名前は切るが拡張子は残す() {
    let 長い = format!("{}.pdf", "あ".repeat(300));

    let 安全 = 安全な名前(&長い);

    assert!(安全.len() <= 255, "{} バイトあります", 安全.len());
    assert!(安全.ends_with(".pdf"), "拡張子が落ちています: {安全}");
}

#[test]
fn 名前が空になったら付け直す() {
    // "..." や "/" だけの名前は、成分を落とすと何も残らない
    for 名 in ["...", "/", "   "] {
        let 安全 = 安全な名前(名);
        assert!(!安全.is_empty(), "{名:?} で空になりました");
    }
}

#[test]
fn 元の名前は捨てない() {
    // 人が見るときに要る。**安全な名前は別に作る**
    let 届 = 届いた("../../etc/passwd");
    assert_eq!(届.name(), "../../etc/passwd");
}

#[test]
fn 大きすぎるものは受け取らない() {
    let 巨大 = Incoming::new("a.pdf", vec![0; MAX_BYTES + 1]);

    assert_eq!(inspect(&巨大), Verdict::Refuse(Reason::TooLarge));
}

#[test]
fn 中身が空でも受け取る() {
    // 0 バイトのファイルは変だが、**受け取れないほどではない**
    let 空 = Incoming::new("a.pdf", Vec::new());

    assert!(matches!(inspect(&空), Verdict::Hold { .. }));
}

#[test]
fn 危なくない名前には理由が付かない() {
    assert!(理由("見積書.pdf").is_empty());
    assert_eq!(安全な名前("見積書.pdf"), "見積書.pdf");
}
