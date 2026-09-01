//! 置き場所。**Downloads へ直接置かない。**
//!
//! roadmap Phase 2 は「受信 File を Downloads へ直接置かず検査。
//! **Trusted からの File も Zero Trust**」と書いている。
//!
//! 信頼している相手からのファイルを素通ししたら、
//! **信頼を得ることに価値が生まれる**（`warifu-capability` の `Trust` と同じ理屈・**D24**）。

use warifu_quarantine::{Incoming, Quarantine, Verdict};

fn 隔離先() -> std::path::PathBuf {
    std::env::temp_dir().join(format!("warifu-q-{}", std::process::id()))
}

#[test]
fn 隔離先の中にしか置かない() {
    let 箱 = Quarantine::new(隔離先());

    let 置き場 = 箱.path_for("passwd").unwrap();

    assert!(置き場.starts_with(隔離先()), "外へ出ています: {置き場:?}");
}

#[test]
fn 危ない名前を渡しても外へ出ない() {
    // **二重に守る。**inspect を通し忘れても、ここで止まる
    let 箱 = Quarantine::new(隔離先());

    assert!(箱.path_for("../../etc/passwd").is_none());
    assert!(箱.path_for("/etc/passwd").is_none());
    assert!(箱.path_for("").is_none());
}

#[test]
fn 同じ名前でも上書きしない() {
    // 上書きできると、**後から来たファイルで前のものを消せる**
    let 箱 = Quarantine::new(隔離先());
    let 一つ目 = 箱.path_for("a.pdf").unwrap();
    箱.reserve("a.pdf");

    let 二つ目 = 箱.path_for("a.pdf").unwrap();

    assert_ne!(一つ目, 二つ目, "同じ置き場所を 2 回返しています");
}

#[test]
fn 信頼している相手でも隔離する() {
    // **Zero Trust。**信頼を得ることに価値を作らない
    let 箱 = Quarantine::new(隔離先());
    let 届いた = Incoming::new("a.pdf", b"x".to_vec()).from_trusted();

    match 箱.accept(&届いた) {
        Verdict::Hold { safe_name, .. } => {
            let 置き場 = 箱.path_for(&safe_name).unwrap();
            assert!(置き場.starts_with(隔離先()), "素通ししています: {置き場:?}");
        }
        Verdict::Refuse(r) => panic!("{r:?}"),
        その他 => panic!("知らない判断: {その他:?}"),
    }
}

#[test]
fn この層は何も開かないし何も書かない() {
    // 返すのは「どこへ置くべきか」までで、**置くのは呼ぶ側**。
    // 開くかどうかは人が決める（D5 と同じ構え）
    let 箱 = Quarantine::new(隔離先());
    let _ = 箱.path_for("a.pdf");

    assert!(!隔離先().exists(), "勝手に作っています: {:?}", 隔離先());
}
