//! スレッド圧縮と重複除去。**過去スレッド全文を毎回渡さない。**
//!
//! `issues/007` の 4 つ目の仕組み。
//! 同じ引用・同じ署名・同じ免責文を毎回モデルに読ませているのが token の無駄で、
//! **落とせる部分は、解釈器を呼ぶ前に落とす。**
//!
//! `issues/007` は「hash で除外する」と書いているが、**hash は使っていない。**
//! 理由は `decisions.md` **D21**（衝突を作れる相手が、読み手から任意の 1 ブロックを消せる）。

use warifu_read::{Body, Received, SenderId, Source, Thread};

fn 一通(本文: &str) -> Received {
    Received::new(
        Source::Imap,
        SenderId::new("aite@例").unwrap(),
        1_756_000_000,
        Body::new(本文.as_bytes().to_vec()),
    )
}

#[test]
fn 二通目は既に読んだ部分を返さない() {
    let mut 話 = Thread::new();

    let 一通目 = 話.add(&一通("見積もりを送ります。\n\n金額は 12,000 円です。"));
    let 二通目 = 話.add(&一通(
        "見積もりを送ります。\n\n金額は 12,000 円です。\n\n了解しました。",
    ));

    assert!(一通目.contains("見積もり"));
    assert_eq!(二通目, "了解しました。", "既に読んだ部分が混ざっています");
}

#[test]
fn 引用記号の行を落とす() {
    // 「> 」で始まる行は、相手の書いた新しい文ではない。
    let mut 話 = Thread::new();
    let 返り = 話.add(&一通(
        "承知しました。\n\n> 金額は 12,000 円です。\n> よろしくお願いします。",
    ));

    assert_eq!(返り, "承知しました。");
}

#[test]
fn 人が登録した定型は落とす() {
    // 署名・免責文。**人が登録する。**
    let mut 話 = Thread::new_with_boilerplate(vec![
        "--\n株式会社れい\n電話 000-0000-0000".to_owned(),
        "本メールは機密情報を含みます。".to_owned(),
    ]);

    let 返り = 話.add(&一通(
        "見積もりを送ります。\n\n--\n株式会社れい\n電話 000-0000-0000\n\n本メールは機密情報を含みます。",
    ));

    assert_eq!(返り, "見積もりを送ります。");
}

#[test]
fn 定型を本文から自動で覚えない() {
    // 覚えると、2 回送るだけで**読み手から任意の 1 ブロックを消せる**（D21）。
    let mut 話 = Thread::new();

    話.add(&一通("重要な連絡です。"));
    let 別の話 = {
        let mut t = Thread::new();
        t.add(&一通("重要な連絡です。"))
    };

    assert_eq!(別の話, "重要な連絡です。", "別の話にまで持ち越されています");
    assert_eq!(話.boilerplate().len(), 0, "本文から定型が増えました");
}

#[test]
fn 空白の違いだけなら同じ塊とみなす() {
    // 引用で字下げが 1 つ増えただけの塊を「新しい」と数えない。
    let mut 話 = Thread::new();
    話.add(&一通("金額は 12,000 円です。"));

    let 二通目 = 話.add(&一通("   金額は 12,000 円です。   \n\n追記します。"));

    assert_eq!(二通目, "追記します。");
}

#[test]
fn 落とした塊の数を数えられる() {
    // 効いているかどうかは、この数でしか測れない（D20 と同じ理屈）。
    let mut 話 = Thread::new();
    話.add(&一通("あ\n\nい\n\nう"));
    話.add(&一通("あ\n\nい\n\nえ"));

    assert_eq!(話.dropped(), 2);
}

#[test]
fn 全部が既出なら空になる() {
    let mut 話 = Thread::new();
    話.add(&一通("あ\n\nい"));

    assert_eq!(話.add(&一通("あ\n\nい")), "");
}

#[test]
fn 順序は変えない() {
    let mut 話 = Thread::new();
    話.add(&一通("い"));

    assert_eq!(話.add(&一通("あ\n\nい\n\nう")), "あ\n\nう");
}

#[test]
fn 話を跨いで持ち越さない() {
    // 別の相手・別の話で同じ文が出たときに、片方が消えると読み違える。
    let mut 一つ目 = Thread::new();
    let mut 二つ目 = Thread::new();

    一つ目.add(&一通("承知しました。"));

    assert_eq!(二つ目.add(&一通("承知しました。")), "承知しました。");
}
