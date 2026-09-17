//! **ゲスト同士が、部屋の合言葉で互いを通す**（**D118** / **#28** の本線 D）。
//!
//! # 何を通し試験にするのか
//!
//! 段ごとの試験は在る（`warifu-core` の証し、`warifu-door` の受け）。
//! **繋ぎ目が無い。**
//!
//! - 証しを**バイト列にして渡し、読み戻して**検めるまで
//! - 検めた結果を**戸口へ渡し、通って、しかも書き置かれない**まで
//!
//! **2026-09-17 に 3 台で測って落ちたのは、まさにこの繋ぎ目**である ——
//! 段ごとには正しくても、**繋がっていなければ「割符が付いていません」で止まる。**

use warifu_core::{Device, PublicKey, Seed, 合言葉, 部屋の叩き};
use warifu_door::{Answer, Door, Knock, Subject};

const 部屋: &[u8] = b"4QNXPYQIHAYIOV6OE2ZYEG4MXQ";
const 今: u64 = 1_758_000_000;

fn 端末(seed: u8) -> Device {
    Seed::from_bytes([seed; 32])
        .profile("Personal")
        .device("PC")
}

fn 印(鍵: PublicKey) -> Subject {
    Subject::new(&鍵.to_string()).expect("主体にできる")
}

/// 呼ぶ側がやること（証しを作って、経路に載せる形にする）。
fn 呼ぶ側が差し出す(
    言: &合言葉, 呼ぶ側: PublicKey, 受ける側: PublicKey
) -> Vec<u8> {
    部屋の叩き::new(部屋, 言.証しを作る(部屋, 呼ぶ側, 受ける側)).to_bytes()
}

/// 受ける側がやること（読み戻して検め、戸口へ渡す）。
fn 受ける側が答える(
    door: &mut Door,
    言: Option<&合言葉>,
    バイト: &[u8],
    呼ぶ側: PublicKey,
    me: PublicKey,
) -> Answer {
    let 叩き = 部屋の叩き::from_bytes(バイト).expect("読める");
    let 合った = 言.is_some_and(|言| 言.証しが合うか(&叩き.証し(), 叩き.部屋(), 呼ぶ側, me));
    let knock = if 合った {
        Knock::with_verified_room_proof(印(呼ぶ側), 今)
    } else {
        Knock::new(印(呼ぶ側), 今)
    };
    door.answer(&knock)
}

#[test]
fn 同じ合言葉を持つゲスト同士は_通る() {
    // **これが #28 の狙いそのもの。**主催を通さずに、ゲスト A が ゲスト B へ通る
    let (a, b) = (端末(1), 端末(2));
    let 言 = 合言葉::作る().expect("作れる");
    // 主催が同じものを 2 人へ渡した、という形
    let bの言 = 合言葉::から(言.バイト列());
    let mut bの戸口 = Door::new();

    let 差し出した = 呼ぶ側が差し出す(&言, a.public_key(), b.public_key());
    let 答え = 受ける側が答える(
        &mut bの戸口,
        Some(&bの言),
        &差し出した,
        a.public_key(),
        b.public_key(),
    );

    assert_eq!(答え, Answer::Open);
}

#[test]
fn 通しても_連絡帳には載らない() {
    // **D12 の迂回を作らない。**通るのはその部屋の中だけである
    let (a, b) = (端末(1), 端末(2));
    let 言 = 合言葉::作る().expect("作れる");
    let bの言 = 合言葉::から(言.バイト列());
    let mut bの戸口 = Door::new();

    let 差し出した = 呼ぶ側が差し出す(&言, a.public_key(), b.public_key());
    受ける側が答える(
        &mut bの戸口,
        Some(&bの言),
        &差し出した,
        a.public_key(),
        b.public_key(),
    );

    assert!(!bの戸口.knows(&印(a.public_key())), "知り合いになっている");
    assert_eq!(bの戸口.known().count(), 0, "書き置く側に出ている");
}

#[test]
fn 合言葉を持っていない相手は_通らない() {
    // **部屋に居ない人は通らない。**これが無いと、誰でも入れる部屋になる
    let (a, b) = (端末(1), 端末(2));
    let 言 = 合言葉::作る().expect("作れる");
    let mut bの戸口 = Door::new();

    let 差し出した = 呼ぶ側が差し出す(&言, a.public_key(), b.public_key());
    // **B は合言葉を持っていない**（まだ主催から貰っていない、など）
    let 答え = 受ける側が答える(
        &mut bの戸口,
        None,
        &差し出した,
        a.public_key(),
        b.public_key(),
    );

    assert_eq!(答え, Answer::Refuse);
}

#[test]
fn 別の合言葉を持つ相手は_通らない() {
    let (a, b) = (端末(1), 端末(2));
    let 言 = 合言葉::作る().expect("作れる");
    let 別の言 = 合言葉::作る().expect("作れる");
    let mut bの戸口 = Door::new();

    let 差し出した = 呼ぶ側が差し出す(&言, a.public_key(), b.public_key());
    let 答え = 受ける側が答える(
        &mut bの戸口,
        Some(&別の言),
        &差し出した,
        a.public_key(),
        b.public_key(),
    );

    assert_eq!(答え, Answer::Refuse);
}

#[test]
fn 横取りした証しは_別の人には使えない() {
    // **A が B へ出した証しを、B が C へ使い回せない。**
    // 使えてしまうと、**1 人通した相手が、その証しで別の人を通せる**
    let (a, b, c) = (端末(1), 端末(2), 端末(3));
    let 言 = 合言葉::作る().expect("作れる");
    let cの言 = 合言葉::から(言.バイト列());
    let mut cの戸口 = Door::new();

    // A → B あての証しを、そのまま C へ差し出す
    let 横取り = 呼ぶ側が差し出す(&言, a.public_key(), b.public_key());
    let 答え = 受ける側が答える(
        &mut cの戸口,
        Some(&cの言),
        &横取り,
        a.public_key(),
        c.public_key(),
    );

    assert_eq!(答え, Answer::Refuse, "横取りした証しで通った");
}

#[test]
fn 名乗りを差し替えても_通らない() {
    // **経路が確定させた鍵を使う**という前提そのもの。
    // 「A の証しを、自分は A だと名乗って出す」が通らないこと ——
    // **受ける側は名乗りを見ず、経路の鍵を使う**ので、B が A の証しを出しても
    // 呼ぶ側の鍵は B として検められ、合わない
    let (a, b, c) = (端末(1), 端末(2), 端末(3));
    let 言 = 合言葉::作る().expect("作れる");
    let cの言 = 合言葉::から(言.バイト列());
    let mut cの戸口 = Door::new();

    // A → C あての正しい証しを、B が横取りして差し出す
    let 盗んだ = 呼ぶ側が差し出す(&言, a.public_key(), c.public_key());
    // **受ける側は「経路の相手は B」として検める**
    let 答え = 受ける側が答える(
        &mut cの戸口,
        Some(&cの言),
        &盗んだ,
        b.public_key(),
        c.public_key(),
    );

    assert_eq!(答え, Answer::Refuse, "名乗りを差し替えて通った");
}
