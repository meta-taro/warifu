//! **建てた commit を版に足す**（2026-10-07）。
//!
//! 同じ版番号のまま直しを配るので、`0.1.20` だけでは直す前か後かが返りから分からない。
//! 版を `0.1.20+22104db` の形にする（git が無い所で建てたら、版番号だけ）。

use std::process::Command;

fn main() {
    let 版 = env!("CARGO_PKG_VERSION");
    let 印 = Command::new("git")
        .args(["rev-parse", "--short=7", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty());
    let 建てた版 = match 印 {
        Some(印) => format!("{版}+{印}"),
        None => 版.to_owned(),
    };
    println!("cargo:rustc-env=WARIFU_BUILD_VERSION={建てた版}");
    // commit が進んだら建て直す（HEAD は枝の名前、枝の先は refs と packed-refs に在る）
    for 見る in [
        "../../.git/HEAD",
        "../../.git/refs/heads",
        "../../.git/packed-refs",
    ] {
        println!("cargo:rerun-if-changed={見る}");
    }
}
