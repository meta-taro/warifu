//! warifu Desktop の器（M5-a）。
//!
//! **まだ映像も鍵も扱わない。**この段で確かめたのは、
//! 「OS のタイトルバーを使わずに窓が出せる」ことだけである（DESIGN.md §8 / D34）。

pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("warifu の窓を開けませんでした");
}
