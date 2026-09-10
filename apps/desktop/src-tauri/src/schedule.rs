//! **予定。**割符の中に持つ（オーナー判断 2026-09-07）。
//!
//! 「予定は割符の中に持つ（TSV で保存し、`.ics` で出し入れ）。
//! **外のカレンダー API とは繋がない。**」
//!
//! # ここは「自分の予定表」である
//!
//! 相手へ渡すのは**空いている枠だけ**で、それは `warifu-calendar` の仕事
//! （あちらは題名の入る場所を持たない）。**ここから外へ出す口は作らない。**

use warifu_vault::{Appointment, Vault};

use crate::{Answer, Failure};

/// 画面へ渡す予定 1 つ。
#[derive(Debug, serde::Serialize)]
pub struct AppointmentRow {
    /// 始まり（Unix 秒）。**画面が地元の時計に直す。** */
    start: u64,
    /// 終わり（Unix 秒）。
    end: u64,
    /// 題。
    title: String,
    /// 覚え書き。**自分だけのもの。** */
    note: String,
}

fn 金庫() -> Result<Vault, Failure> {
    Vault::default_location().map_err(|e| Failure {
        message: e.to_string(),
        code: None,
    })
}

fn 読む() -> Result<Vec<Appointment>, Failure> {
    金庫()?.schedule().map_err(|e| Failure {
        message: e.to_string(),
        code: None,
    })
}

fn 書く(予定: &[Appointment]) -> Result<(), Failure> {
    金庫()?.save_schedule(予定).map_err(|e| Failure {
        message: e.to_string(),
        code: None,
    })
}

/// 予定を並べる。**始まりの早い順。**
#[tauri::command]
pub async fn schedule_list() -> Answer<Vec<AppointmentRow>> {
    Ok(読む()?
        .iter()
        .map(|a| AppointmentRow {
            start: a.start(),
            end: a.end(),
            title: a.title().to_owned(),
            note: a.note().to_owned(),
        })
        .collect())
}

/// 予定を足す。
///
/// **終わりが始まりより後であることを、ここで確かめる** ——
/// 画面の作りに任せない（画面は差し替えられる）。
#[tauri::command]
pub async fn schedule_add(start: u64, end: u64, title: String, note: String) -> Answer<()> {
    if end <= start {
        return Err(Failure {
            message: "終わりは始まりより後にしてください".to_owned(),
            code: None,
        });
    }
    if title.trim().is_empty() {
        return Err(Failure {
            message: "題を書いてください".to_owned(),
            code: None,
        });
    }
    let mut 予定 = 読む()?;
    予定.push(Appointment::new(start, end, &title, &note));
    書く(&予定)
}

/// 予定を消す。**始まりと題で引く**（id を持たせていないので）。
///
/// **同じ始まりに同じ題が 2 つあるなら、1 つだけ消す** ——
/// 消しすぎない。
#[tauri::command]
pub async fn schedule_remove(start: u64, title: String) -> Answer<()> {
    let 予定 = 読む()?;
    let mut 消した = false;
    let 残り: Vec<Appointment> = 予定
        .into_iter()
        .filter(|a| {
            if !消した && a.start() == start && a.title() == title {
                消した = true;
                return false;
            }
            true
        })
        .collect();
    書く(&残り)
}
