//! `warifu version` が、建てた commit まで名乗るか。
//!
//! 同じ版番号のまま直しを配るので、`0.1.21` だけでは直す前か後かが分からない。
//! MCP の `about` と同じ `0.1.21+<commit>` を出す（git の無い所で建てたら版番号だけ）。

use std::process::Command;

#[test]
fn 版は_mcp_と同じ形で名乗る() {
    let 出 = Command::new(env!("CARGO_BIN_EXE_warifu"))
        .arg("version")
        .output()
        .expect("warifu version が動く");
    assert!(出.status.success());
    let 行 = String::from_utf8(出.stdout).expect("utf-8");
    assert_eq!(行.trim(), format!("warifu {}", warifu_mcp::建てた版));
}
