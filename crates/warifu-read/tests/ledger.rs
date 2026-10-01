//! 会計。**1 通あたり、何にいくら掛かったかが手元に残る。**
//!
//! `issues/007` の完了条件の 1 つ目と 3 つ目
//! 「2 通目で **LLM 呼び出しが 0 回**であることを**記録で**確認できる」
//! 「1 通あたりの input / output token・LLM を呼んだか・parser が作られたかが**ローカルに残る**」
//! を、ここで固定する。
//!
//! **記録の側に本文は入らない。**Level 0 で本文を返さない層が、
//! 会計の側から本文を漏らしていたら意味が無い。

use warifu_read::{
    Body, Entry, Extract, Kind, Ledger, Level, Priority, Reader, Received, RuleDraft, RuleStore,
    SenderId, Source,
};

/// 記録のどこにも出てはいけない文字列。
const 目印: &str = "SHIRUSHI-会計-7b2e";

fn 一通() -> Received {
    Received::new(
        Source::Imap,
        SenderId::new("billing@例").unwrap(),
        1_756_000_000,
        Body::new(format!("請求書 {目印} 合計 12,000 円").into_bytes()),
    )
}

/// 人が承認済みの規則を持った読む人。**2 通目以降はこれで読める。**
fn 読む人() -> Reader {
    let mut 棚 = RuleStore::new();
    棚.approve(
        RuleDraft::new(
            SenderId::new("billing@例").unwrap(),
            Kind::new("invoice").unwrap(),
        )
        .marker("請求書")
        .priority(Priority::High)
        .extract(Extract::new("金額", "合計 ")),
    )
    .unwrap();
    Reader::with_rules(棚)
}

/// 規則で読めた 1 通の見え方。
fn 読めた(level: Level) -> warifu_read::View {
    読む人().open_at(&一通(), level).unwrap()
}

#[test]
fn 解釈器を呼ばずに読めた一通が記録される() {
    let mut 帳簿 = Ledger::new();
    帳簿.record(Entry::without_interpreter(&読めた(Level::Structured)));

    assert_eq!(帳簿.entries().len(), 1);
    assert_eq!(帳簿.interpreter_calls(), 0);
    assert_eq!(帳簿.tokens(), (0, 0));
}

#[test]
fn 呼んだときだけ_token_が入る() {
    let mut 帳簿 = Ledger::new();
    帳簿.record(Entry::with_interpreter(
        &一通(),
        Level::Structured,
        1_200,
        80,
    ));
    帳簿.record(Entry::without_interpreter(&読めた(Level::Structured)));

    assert_eq!(帳簿.interpreter_calls(), 1);
    assert_eq!(帳簿.tokens(), (1_200, 80));
}

#[test]
fn 二通目が呼ばれていないことを記録で確認できる() {
    // ここが issues/007 の完了条件そのもの。**振る舞いではなく記録で示す。**
    let mut 帳簿 = Ledger::new();
    let 差出人 = SenderId::new("billing@例").unwrap();

    帳簿.record(Entry::with_interpreter(&一通(), Level::Structured, 1_200, 80).rule_approved());
    帳簿.record(Entry::without_interpreter(&読めた(Level::Structured)));
    帳簿.record(Entry::without_interpreter(&読めた(Level::Structured)));

    assert_eq!(
        帳簿.interpreter_calls_for(&差出人),
        1,
        "1 通目だけがコストのはず"
    );
    assert_eq!(帳簿.entries_for(&差出人), 3);
}

#[test]
fn 規則が承認されたことが記録に残る() {
    let mut 帳簿 = Ledger::new();
    帳簿.record(Entry::with_interpreter(&一通(), Level::Structured, 900, 40).rule_approved());
    帳簿.record(Entry::without_interpreter(&読めた(Level::Metadata)));

    assert_eq!(帳簿.rules_approved(), 1);
}

#[test]
fn 会計に本文が入らない() {
    let mut 帳簿 = Ledger::new();
    帳簿.record(Entry::with_interpreter(&一通(), Level::Raw, 10, 10));

    let 記録 = 帳簿.to_tsv();
    assert!(!記録.contains(目印), "会計に本文が漏れています:\n{記録}");
    assert!(!format!("{帳簿:?}").contains(目印));
}

#[test]
fn 記録が往復する() {
    // 書いて読んで同じにならないなら、残した意味が無い。
    let mut 帳簿 = Ledger::new();
    帳簿.record(Entry::with_interpreter(&一通(), Level::Structured, 1_200, 80).rule_approved());
    帳簿.record(Entry::without_interpreter(&読めた(Level::Metadata)));

    let 戻り = Ledger::from_tsv(&帳簿.to_tsv()).unwrap();

    assert_eq!(戻り.entries(), 帳簿.entries());
    assert_eq!(戻り.to_tsv(), 帳簿.to_tsv());
}

#[test]
fn 見出しの列が_tsv_の一行目に入る() {
    let 帳簿 = Ledger::new();
    let 一行目 = 帳簿.to_tsv();
    let 一行目 = 一行目.lines().next().unwrap();

    for 列 in [
        "受け取った時刻",
        "送信元",
        "種別",
        "段",
        "解釈器",
        "input",
        "output",
        "規則",
    ] {
        assert!(一行目.contains(列), "見出しに {列} がありません: {一行目}");
    }
}

#[test]
fn 壊れた記録は受け取らない() {
    // 読めない行を黙って捨てると、「呼んだ回数」が後から減る。
    assert!(Ledger::from_tsv("見出しだけ").is_err());
    assert!(Ledger::from_tsv(&format!("{}\n1\t2\n", Ledger::new().to_tsv().trim())).is_err());
}

#[test]
fn 送信元にタブと改行を入れられない() {
    // 入れられると 1 行を 2 行に割れる＝**記録を偽造できる。**
    assert!(SenderId::new("a\tb@例").is_err());
    assert!(SenderId::new("a\nb@例").is_err());
    assert!(SenderId::new("a\rb@例").is_err());
}

#[test]
fn ファイルへは追記される() {
    // 上書きにすると、前回までの記録が消える＝**掛かった費用を後から減らせる。**
    let 置き場 = std::env::temp_dir().join(format!("warifu-read-{}.tsv", std::process::id()));
    let _ = std::fs::remove_file(&置き場);

    let mut 一回目 = Ledger::new();
    一回目.record(Entry::without_interpreter(&読めた(Level::Metadata)));
    一回目.append_to(&置き場).unwrap();

    let mut 二回目 = Ledger::new();
    二回目.record(Entry::with_interpreter(&一通(), Level::Structured, 5, 5));
    二回目.append_to(&置き場).unwrap();

    let 読み戻し = Ledger::load(&置き場).unwrap();
    assert_eq!(読み戻し.entries().len(), 2, "追記されていません");
    assert_eq!(読み戻し.interpreter_calls(), 1);

    let _ = std::fs::remove_file(&置き場);
}

#[test]
fn 規則で読めたときは種別も記録に残る() {
    // 「未読 43 件」ではなく「承認 3 / 支払 2」を出すには、記録の側にも種別が要る。
    let mut 帳簿 = Ledger::new();
    帳簿.record(Entry::without_interpreter(&読めた(Level::Structured)));

    assert_eq!(帳簿.entries()[0].kind().as_str(), "invoice");
    assert!(帳簿.to_tsv().contains("invoice"));
}

#[test]
fn 呼んだ時点では種別が分かっていないと記録する() {
    // 呼ぶことになったのは読めなかったから。**後から分かった種別を遡って書かない。**
    let mut 帳簿 = Ledger::new();
    帳簿.record(Entry::with_interpreter(&一通(), Level::Structured, 900, 40));

    assert_eq!(帳簿.entries()[0].kind().as_str(), Kind::UNKNOWN);
}
