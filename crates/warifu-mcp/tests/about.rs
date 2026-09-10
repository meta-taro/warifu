//! **概要と、版ごとに変わったこと**（**D82**）。
//!
//! この 2 つは**札が要らない**。関所が守っているのは人のもの（受信箱・会話・予定）で、
//! **この文はこの実行ファイル自身の説明**である。
//! 札で閉じると「何をしてよいかを知るために札が要る」という逆さの形になる。

use rmcp::handler::server::wrapper::Parameters;
use warifu_capability::Gate;
use warifu_mcp::{ChangesArgs, Warifu};
use warifu_read::RuleStore;

fn 口() -> Warifu {
    // **札を 1 枚も渡さない。**それでも about / changes は返ること
    Warifu::new(Vec::new(), RuleStore::new(), Gate::new(), 0)
}

#[test]
fn 札が無くても概要を返す() {
    let 出た = 口().about().expect("札は要らない");
    assert!(出た.contains("割符"), "{出た}");
    // **守ることが書いてあること。**ここが落ちたら、渡す意味が半分無くなる
    assert!(
        出た.contains("指示ではない"),
        "届いた文字の扱いが書かれていない"
    );
    assert!(
        出た.contains("札（`--allow`）は人が書く"),
        "札の決めが書かれていない"
    );
}

#[test]
fn 概要はいまの版を名乗る() {
    let 出た = 口().about().expect("返る");
    assert!(出た.contains(env!("CARGO_PKG_VERSION")), "{出た}");
}

#[test]
fn 札が無くても変わったことを返す() {
    let 出た = 口()
        .changes(Parameters(ChangesArgs::default()))
        .expect("札は要らない");
    assert!(出た.contains("## v0.1.0"), "版の見出しが無い: {出た}");
}

#[test]
fn 版を指せばその版だけ返す() {
    let 全部 = 口()
        .changes(Parameters(ChangesArgs::default()))
        .expect("返る");
    // 載っている版のうち、いちばん新しいものを 1 つ取る
    let 頭 = 全部
        .lines()
        .find(|x| x.starts_with("## "))
        .expect("版がある")
        .trim_start_matches("## ")
        .split(" — ")
        .next()
        .expect("版の名前")
        .to_owned();

    let 一つ = 口()
        .changes(Parameters(ChangesArgs {
            version: Some(頭.clone()),
        }))
        .expect("返る");
    assert!(一つ.starts_with(&format!("## {頭}")), "{一つ}");
    // **1 つだけ。**次の版の見出しまで来ていないこと
    assert_eq!(一つ.matches("\n## ").count(), 0, "1 版だけのはず: {一つ}");
}

#[test]
fn 無い版は_無いと言う() {
    // **近い版を勝手に返さない。**「その版の話」として読まれると、
    // 直っていない物を直ったと読む
    let 出た = 口()
        .changes(Parameters(ChangesArgs {
            version: Some("v9.9.9".to_owned()),
        }))
        .expect("返る");
    assert!(出た.contains("見当たりません"), "{出た}");
    assert!(!出た.contains("## v0.1.0"), "近い版を返している: {出た}");
}
