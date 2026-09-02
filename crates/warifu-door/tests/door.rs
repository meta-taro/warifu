//! 戸口。**知らない相手は、人に取り次がずに断る。**
//!
//! roadmap **Phase 2**（Connection Request / Rate Limit / Spam Defense）。
//!
//! # なぜ「人に聞く」を既定にしないか
//!
//! 「知らない相手が来ました、通しますか」を出す形にすると、
//! **知らない相手が、こちらの注意を消費できる。**
//! **通知を出せること自体が資源**であり、そこを開けたら spam の入口になる。
//!
//! SimpleX が「利用者 ID を持たず、招待リンク経由でしか繋がれない」形で
//! spam を止めたのと同じ筋である（`docs/research/existing-specs.md`）。
//! こちらは割符（**D12**）がその招待にあたる。

use warifu_door::{Answer, Door, KNOWN_QUOTA, Knock, STRANGER_QUOTA, Subject, WINDOW};

const 今: u64 = 1_756_800_000;

fn 誰か(名: &str) -> Subject {
    Subject::new(名).unwrap()
}

#[test]
fn 割符があれば開ける() {
    // 割符は**人が渡したもの**。渡した時点で、人はもう判断している
    let mut 戸 = Door::new();

    let 答え = 戸.answer(&Knock::with_verified_tally(誰か("a@例"), 今));

    assert_eq!(答え, Answer::Open);
}

#[test]
fn 割符が無ければ断り人に聞かない() {
    let mut 戸 = Door::new();

    let 答え = 戸.answer(&Knock::new(誰か("stranger@例"), 今));

    assert_eq!(答え, Answer::Refuse, "知らない相手を通しています");
}

#[test]
fn 一度開けた相手は次から開ける() {
    let mut 戸 = Door::new();
    戸.answer(&Knock::with_verified_tally(誰か("a@例"), 今));

    // 二度目は割符が無い
    let 答え = 戸.answer(&Knock::new(誰か("a@例"), 今 + 1));

    assert_eq!(答え, Answer::Open);
    assert!(戸.knows(&誰か("a@例")));
}

#[test]
fn 知らない相手の連打を絞る() {
    // 断るのにも計算が要る。**断る相手にも上限を掛ける**
    let mut 戸 = Door::new();

    for i in 0..STRANGER_QUOTA {
        戸.answer(&Knock::new(誰か("stranger@例"), 今 + i as u64));
    }

    // 窓のあいだの叩きは、上限で止まる
    assert_eq!(戸.knocks_from(&誰か("stranger@例")), STRANGER_QUOTA);
}

#[test]
fn 別々の相手からの洪水も止める() {
    // 1 人ずつ絞っても、相手を変えられたら意味が無い。**全体にも上限が要る**
    let mut 戸 = Door::new();

    for i in 0..(STRANGER_QUOTA * 10) {
        戸.answer(&Knock::new(誰か(&format!("s{i}@例")), 今 + i as u64));
    }

    assert!(
        戸.knocks_from(&誰か("s0@例")) <= STRANGER_QUOTA,
        "1 人ぶんの記録が上限を超えています"
    );
}

#[test]
fn 洪水が来ても知っている相手は締め出さない() {
    // **知らない相手の量で、知っている相手が入れなくなってはいけない。**
    // ここが分かれていないと、洪水を送るだけで会話を止められる
    let mut 戸 = Door::new();
    戸.answer(&Knock::with_verified_tally(誰か("tomodachi@例"), 今));

    for i in 0..(STRANGER_QUOTA * 20) {
        戸.answer(&Knock::new(誰か(&format!("s{i}@例")), 今 + i as u64));
    }

    let 答え = 戸.answer(&Knock::new(誰か("tomodachi@例"), 今 + 1_000));

    assert_eq!(答え, Answer::Open, "洪水で知り合いが締め出されました");
}

#[test]
fn 知っている相手にも上限はある() {
    // 知り合いの端末が乗っ取られる場合がある。**無制限にはしない**
    let mut 戸 = Door::new();
    戸.answer(&Knock::with_verified_tally(誰か("a@例"), 今));

    let mut 最後 = Answer::Open;
    for i in 0..(KNOWN_QUOTA + 5) {
        最後 = 戸.answer(&Knock::new(誰か("a@例"), 今 + i as u64));
    }

    assert_eq!(
        最後,
        Answer::Refuse,
        "知っている相手なら無制限に通しています"
    );
}

#[test]
fn 絞ったときと知らないときで断り方が同じ() {
    // **区別できると、絞りの境界を探れる。**
    // 「今は絞られているだけ」と分かれば、時間を空けて叩き直せばよいと分かる
    let mut 戸 = Door::new();
    let 知らない = 戸.answer(&Knock::new(誰か("x@例"), 今));

    戸.answer(&Knock::with_verified_tally(誰か("a@例"), 今));
    let mut 絞られた = Answer::Open;
    for i in 0..(KNOWN_QUOTA + 5) {
        絞られた = 戸.answer(&Knock::new(誰か("a@例"), 今 + i as u64));
    }

    assert_eq!(知らない, 絞られた, "断り方が違うと、境界を探れます");
}

#[test]
fn 最後の叩きから窓が過ぎれば絞りが戻る() {
    // **窓は「最初の叩き」からではなく「今から遡って」数える。**
    // 最初から数える形にすると、窓の頭で叩き続けるだけで
    // いつまでも上限に掛からない相手が作れる
    let mut 戸 = Door::new();
    戸.answer(&Knock::with_verified_tally(誰か("a@例"), 今));
    let mut 最後の叩き = 今;
    for i in 0..(KNOWN_QUOTA + 5) {
        最後の叩き = 今 + i as u64;
        戸.answer(&Knock::new(誰か("a@例"), 最後の叩き));
    }
    // 直後はまだ絞られたまま
    assert_eq!(
        戸.answer(&Knock::new(誰か("a@例"), 最後の叩き + 1)),
        Answer::Refuse,
        "絞った直後に戻っています"
    );
    // **窓は滑るので、戻り方は一度にではなく少しずつ。**
    // 「全部だめ」から「全部よい」へ跳ぶ形にすると、
    // その跳ぶ瞬間を狙って叩ける

    let 答え = 戸.answer(&Knock::new(誰か("a@例"), 最後の叩き + WINDOW + 1));

    assert_eq!(答え, Answer::Open, "窓が過ぎても絞られたままです");
}

#[test]
fn 記録は増え続けない() {
    // 断った相手を永久に覚えていると、**断るほど重くなる**
    let mut 戸 = Door::new();
    for i in 0..500 {
        戸.answer(&Knock::new(誰か(&format!("s{i}@例")), 今 + i as u64));
    }

    戸.forget_old(今 + WINDOW * 2);

    assert_eq!(戸.knocks_from(&誰か("s0@例")), 0, "古い記録が残っています");
}

#[test]
fn 知っている相手は忘れない() {
    // 記録を落としても、**開けたという事実は残す**
    let mut 戸 = Door::new();
    戸.answer(&Knock::with_verified_tally(誰か("a@例"), 今));

    戸.forget_old(今 + WINDOW * 100);

    assert!(戸.knows(&誰か("a@例")), "知り合いを忘れました");
}

#[test]
fn 時計が戻っても壊れない() {
    let mut 戸 = Door::new();
    戸.answer(&Knock::new(誰か("x@例"), 今 + 100));

    let 答え = 戸.answer(&Knock::new(誰か("x@例"), 今));

    assert_eq!(答え, Answer::Refuse);
}

#[test]
fn 名前の形が壊れていれば受け取らない() {
    assert!(Subject::new("").is_none());
    assert!(Subject::new("a\tb@例").is_none(), "タブ");
    assert!(Subject::new("a\nb@例").is_none(), "改行");
    assert!(Subject::new(&"a".repeat(321)).is_none());
}

#[test]
fn 叩きに本文が入らない() {
    // **戸口は中身を見ない。**見る所と、通すかを決める所を分ける
    // （warifu-capability の Request と同じ手・D24）
    let 叩き = Knock::new(誰か("x@例"), 今);

    let 出力 = format!("{叩き:?}");
    assert!(!出力.contains("body"));
    assert_eq!(叩き.from().as_str(), "x@例");
    assert_eq!(叩き.at(), 今);
}
