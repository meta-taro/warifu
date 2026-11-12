//! **建てた commit を版に足す**（11-12・`crates/warifu-mcp/build.rs` と同じ形）。
//!
//! 同じ版番号のまま候補を配るので、画面の `0.1.48` だけでは、どの候補か分からなかった
//! （席の口は `0.1.48+a9645af` と言うのに、`app_view` の版にはハッシュが無かった）。

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
    for 見る in [
        "../../../.git/HEAD",
        "../../../.git/refs/heads",
        "../../../.git/packed-refs",
    ] {
        println!("cargo:rerun-if-changed={見る}");
    }
    tauri_build::build()
}
