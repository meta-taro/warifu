//! 外から届く口が、その OS で塞がれていないかを**調べる**。
//!
//! ```text
//!   warifu-cli（doctor）      画面（Tauri）
//!         │                       │
//!         └───────┬───────────────┘
//!                 ▼
//!        warifu-guard（この層）
//!                 │  「その実行ファイルに規則があるか」だけを返す
//!                 ▼
//!            OS のファイアウォール
//! ```
//!
//! # 開けはしない
//!
//! **調べるだけ。**規則は作らない —— **外から届く口を開けるのは、人が決めること**である
//! （baseline §13）。ここが返すのは事実だけで、直し方を実行はしない。
//!
//! # なぜ実行ファイルごとに分けるのか
//!
//! **`warifu` という名前でまとめて数えると、塞がっている方を隠す。**
//!
//! 2026-09-12 に Windows で実際に起きた形がこれである。
//!
//! ```text
//!   warifu.exe          （CLI）  規則あり 2 本  ← コマンドは繋がる
//!   warifu-desktop.exe  （画面）  規則なし      ← 画面だけ繋がらない
//! ```
//!
//! この状態で「`*warifu*` の規則を数える」と **2 件**になり、
//! 「規則はある」と読める。**実際には画面が塞がっている。**
//! 利用者からは「コマンドは動くのに画面だけ繋がらない」という、
//! **いちばん切り分けにくい形**で出る。
//!
//! だから**聞く単位は実行ファイル 1 つ**にしてある。
//!
//! # 「分からない」を「無い」に倒さない
//!
//! 調べる手段そのものが失敗することがある（権限・実行ポリシー・OS 違い）。
//! それを「規則が無い」と読むと、**足りている人に「足してください」と言う**。
//! だから [`遮り`] は 3 つある（線 7）。

#![forbid(unsafe_code)]

use std::path::Path;

/// その実行ファイルに、外から届く口が開いているか。
///
/// **「分からない」を「無い」に倒さない**ための 3 値である。
/// **`#[non_exhaustive]` は付けない。**この 3 つで閉じており、
/// 4 つ目を足すときは**呼ぶ側も直すべき**である（`_` で黙って落とされるより、
/// 建たなくなって気づくほうがよい）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum 遮り {
    /// 規則がある（件数）。**開いている見込み**。
    ///
    /// 件数まで返すのは、**人が「そんなに要らないはず」と気づける**ようにするため。
    開いている(usize),
    /// 規則が 1 つも無い。**塞がっている見込み**。
    塞がっている,
    /// **調べられなかった。**塞がっているとは限らない。
    ///
    /// `理由` はそのまま人に見せてよい（コマンドの失敗や、その OS では見ないこと）。
    分からない(String),
}

impl 遮り {
    /// 人に見せる 1 行。**推測を混ぜない。**
    #[must_use]
    pub fn 一行(&self) -> String {
        match self {
            Self::開いている(n) => format!("規則が {n} 件あります"),
            Self::塞がっている => "規則が **ありません**".to_owned(),
            Self::分からない(理由) => format!("調べられませんでした（{理由}）"),
        }
    }
}

/// その実行ファイルに当たる規則を調べる。
///
/// **パスを 1 つずつ渡す。**まとめて聞くと、塞がっている方が隠れる（この層の冒頭を見よ）。
///
/// Windows 以外では [`遮り::分からない`] を返す。**「無い」ではない。**
#[must_use]
pub fn 調べる(実行ファイル: &Path) -> 遮り {
    #[cfg(target_os = "windows")]
    {
        self::windows::調べる(実行ファイル)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = 実行ファイル;
        遮り::分からない("この OS では見ていません".to_owned())
    }
}

/// この OS で、**画面**（デスクトップアプリ）が入っていそうな場所。
///
/// `doctor` は CLI の中から動く。**自分の規則だけを見ても、画面が塞がっているかは分からない。**
/// だから画面の在り処をここで持つ。
///
/// **在ることを確かめてから返す。**入れていない人に
/// 「画面の規則がありません」と言っても、直しようがない。
#[must_use]
pub fn 画面の在り処() -> Option<std::path::PathBuf> {
    #[cfg(target_os = "windows")]
    let 候補 = std::env::var_os("LOCALAPPDATA")
        .map(|p| Path::new(&p).join("warifu").join("warifu-desktop.exe"));
    #[cfg(target_os = "macos")]
    let 候補 = Some(std::path::PathBuf::from(
        "/Applications/warifu.app/Contents/MacOS/warifu",
    ));
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let 候補: Option<std::path::PathBuf> = None;

    候補.filter(|p| p.exists())
}

#[cfg(target_os = "windows")]
mod windows {
    use super::遮り;
    use std::path::Path;

    /// `Get-NetFirewallApplicationFilter` の出力から件数を読む。
    ///
    /// **空白と改行だけを落として、数として読めるかを見る。**
    /// 読めないものを 0 として扱わない —— 0 は「規則が無い」という**主張**であり、
    /// 「読めなかった」とは別の事実である。
    ///
    /// **この中に置いてある理由。**呼ぶのは Windows の [`調べる`] だけなので、
    /// 外に出すと他の OS で「使っていない」と言われる。
    /// 試験も一緒にここへ置いてあり、**CI の windows 段で回る。**
    #[must_use]
    fn 件数を読む(出力: &str) -> Option<usize> {
        出力.trim().parse::<usize>().ok()
    }

    /// PowerShell へ渡す文字列の中で、シングルクォートを閉じさせない。
    ///
    /// **パスは人が置いた場所であって、こちらが決めた文字列ではない。**
    /// `C:\it's here\warifu.exe` のような名前は作れてしまうので、
    /// **そのまま挟むと命令が途中で終わる。**
    pub(super) fn 引用符を潰す(s: &str) -> String {
        s.replace('\'', "''")
    }

    pub(super) fn 調べる(実行ファイル: &Path) -> 遮り {
        let パス = 引用符を潰す(&実行ファイル.display().to_string());
        // **`-eq` で当てる。**`-like '*warifu*'` だと別の実行ファイルまで数え、
        // 塞がっている方を隠す
        let 命令 = format!(
            "Get-NetFirewallApplicationFilter | \
             Where-Object {{ $_.Program -eq '{パス}' }} | \
             Measure-Object | Select-Object -ExpandProperty Count"
        );
        match std::process::Command::new("powershell")
            .args(["-NoProfile", "-Command", &命令])
            .output()
        {
            Ok(o) if !o.status.success() => {
                let 理由 = String::from_utf8_lossy(&o.stderr).trim().to_owned();
                遮り::分からない(if 理由.is_empty() {
                    "PowerShell が失敗しました".to_owned()
                } else {
                    理由
                })
            }
            Ok(o) => match 件数を読む(&String::from_utf8_lossy(&o.stdout)) {
                Some(0) => 遮り::塞がっている,
                Some(n) => 遮り::開いている(n),
                // **読めなかったものを 0 にしない**（線 7）
                None => 遮り::分からない("件数として読めませんでした".to_owned()),
            },
            Err(e) => 遮り::分からない(e.to_string()),
        }
    }

    #[cfg(test)]
    mod 試験 {
        use super::*;

        /// **パスに `'` が入っていても命令が壊れない。**
        #[test]
        fn 引用符は潰す() {
            assert_eq!(引用符を潰す(r"C:\it's\warifu.exe"), r"C:\it''s\warifu.exe");
        }

        #[test]
        fn 数はそのまま読む() {
            assert_eq!(件数を読む("0"), Some(0));
            assert_eq!(件数を読む("2"), Some(2));
        }

        /// PowerShell の出力は `\r\n` で終わる。**空白ごと落とす。**
        #[test]
        fn 前後の空白と改行は落とす() {
            assert_eq!(件数を読む(" 3 \r\n"), Some(3));
            assert_eq!(件数を読む("\n1\n"), Some(1));
        }

        /// **読めないものを 0 にしない。**0 は「規則が無い」という主張である。
        #[test]
        fn 読めないものは数にしない() {
            assert_eq!(件数を読む(""), None);
            assert_eq!(件数を読む("エラー"), None);
            assert_eq!(件数を読む("-1"), None);
            assert_eq!(件数を読む("1 2"), None);
        }

        /// **無い実行ファイルには規則も無い。**
        ///
        /// PowerShell が動く環境でのみ意味を持つ。動かなければ
        /// [`遮り::分からない`] になり、**それでも「塞がっている」とは言わない。**
        #[test]
        fn 無いパスは塞がっているか分からない() {
            let 結果 = 調べる(Path::new(r"C:\warifu\これは存在しない.exe"));
            assert!(
                matches!(結果, 遮り::塞がっている | 遮り::分からない(_)),
                "無いパスに規則があってはならない: {結果:?}"
            );
        }
    }
}

#[cfg(test)]
mod 試験 {
    use super::*;

    #[test]
    fn 一行は状態ごとに違う() {
        assert_eq!(遮り::開いている(2).一行(), "規則が 2 件あります");
        assert_eq!(遮り::塞がっている.一行(), "規則が **ありません**");
        assert_eq!(
            遮り::分からない("権限がありません".to_owned()).一行(),
            "調べられませんでした（権限がありません）"
        );
    }

    /// **Windows 以外では「無い」と言わない。**
    #[cfg(not(target_os = "windows"))]
    #[test]
    fn 見ていない_os_では分からないを返す() {
        assert!(matches!(
            調べる(Path::new("/usr/bin/warifu")),
            遮り::分からない(_)
        ));
    }

    /// **在るものだけを返す。**入れていない人に「画面の規則がありません」と言わない。
    #[test]
    fn 画面の在り処は実在するものだけ() {
        if let Some(道) = 画面の在り処() {
            assert!(道.exists(), "在らないものを返した: {}", 道.display());
        }
    }
}
