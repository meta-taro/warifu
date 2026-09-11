//! 会議の進行（M5-c）。**経路もカメラも登場しない。**
//!
//! ここで確かめるのは「知らせを受けて名簿がどう動くか」「誰が offer を出すか」だけ。
//! 実際に繋ぐ所は `warifu-net` が、SDP を運ぶ所は `warifu-meeting` が既に持っている。

use warifu_app::{Conference, Event};
use warifu_core::{PublicKey, Seed};
use warifu_meeting::{MeetingId, Notice, Signal, Step};

fn 鍵(seed: u8) -> PublicKey {
    Seed::from_bytes([seed; 32])
        .profile("Personal")
        .device("端末")
        .public_key()
}

#[test]
fn 会議は主催者ひとりから始まる() {
    let 私 = 鍵(1);
    let c = Conference::host(私, 12).unwrap();
    assert_eq!(c.members(), &[私]);
    assert_eq!(c.capacity(), 12);
}

#[test]
fn 外枠を超える定員では始められない() {
    assert!(Conference::host(鍵(1), 17).is_err());
    assert!(Conference::host(鍵(1), 1).is_err());
}

#[test]
fn 参加の知らせで名簿が増える() {
    let 私 = 鍵(1);
    let 相手 = 鍵(2);
    let mut c = Conference::host(私, 12).unwrap();

    let events = c
        .on_notice(相手, &Notice::Join { meeting: c.id() })
        .unwrap();

    assert_eq!(events, vec![Event::Joined(相手)]);
    assert!(c.members().contains(&相手));
}

/// **経路が落ちたあと、同じ相手が入り直せる**（**D83**）。
///
/// 2026-09-10 に実物で踏んだ —— 画面側が**落ちたときに名簿から外していなかった**ため、
/// 入り直しの `Join` が冪等で潰され、**相手の発言だけが届いて画面は「誰も居ない」**
/// と思ったままになった（返信もできない）。
///
/// **外してあれば、入り直しはもう一度 `Joined` になる。**
/// この試験は、その前提を押さえるためのものである。
#[test]
fn 外してあれば同じ相手が入り直せる() {
    let 私 = 鍵(1);
    let 相手 = 鍵(2);
    let mut c = Conference::host(私, 12).unwrap();

    c.on_notice(相手, &Notice::Join { meeting: c.id() })
        .unwrap();
    // 経路が落ちた ＝ その人は居ない
    let 出た = c
        .on_notice(相手, &Notice::Leave { meeting: c.id() })
        .unwrap();
    assert_eq!(出た, vec![Event::Left(相手)]);
    assert!(!c.members().contains(&相手));

    // **入り直し。**ここが空だと、画面は相手が戻ったことを知れない
    let 戻り = c
        .on_notice(相手, &Notice::Join { meeting: c.id() })
        .unwrap();
    assert_eq!(戻り, vec![Event::Joined(相手)], "入り直しが伝わらない");
    assert!(c.members().contains(&相手));
}

#[test]
fn 同じ相手が二度入っても名簿は増えない() {
    let 私 = 鍵(1);
    let 相手 = 鍵(2);
    let mut c = Conference::host(私, 12).unwrap();
    c.on_notice(相手, &Notice::Join { meeting: c.id() })
        .unwrap();

    let events = c
        .on_notice(相手, &Notice::Join { meeting: c.id() })
        .unwrap();

    assert!(events.is_empty(), "二度目は何も起きない");
    assert_eq!(c.members().len(), 2);
}

#[test]
fn 定員に達したら断る() {
    let mut c = Conference::host(鍵(1), 2).unwrap();
    c.on_notice(鍵(2), &Notice::Join { meeting: c.id() })
        .unwrap();

    assert!(
        c.on_notice(鍵(3), &Notice::Join { meeting: c.id() })
            .is_err()
    );
    assert_eq!(c.members().len(), 2);
}

#[test]
fn 退出の知らせで名簿が減る() {
    let 私 = 鍵(1);
    let 相手 = 鍵(2);
    let mut c = Conference::host(私, 12).unwrap();
    c.on_notice(相手, &Notice::Join { meeting: c.id() })
        .unwrap();

    let events = c
        .on_notice(相手, &Notice::Leave { meeting: c.id() })
        .unwrap();

    assert_eq!(events, vec![Event::Left(相手)]);
    assert!(!c.members().contains(&相手));
}

#[test]
fn 別の会議の知らせは受け取らない() {
    // 誤配を黙って取り込まない
    let mut c = Conference::host(鍵(1), 12).unwrap();
    let よそ = MeetingId::generate();

    assert!(c.on_notice(鍵(2), &Notice::Join { meeting: よそ }).is_err());
    assert_eq!(c.members().len(), 1);
}

#[test]
fn 下ごしらえは中身を解釈せずに渡す() {
    let 私 = 鍵(1);
    let 相手 = 鍵(2);
    let mut c = Conference::host(私, 12).unwrap();
    c.on_notice(相手, &Notice::Join { meeting: c.id() })
        .unwrap();

    let 中身 = b"v=0\r\no=- 0 0 IN IP4 127.0.0.1\r\n".to_vec();
    let notice = Notice::Signal(Signal::new(c.id(), Step::Offer, 中身.clone()));

    let events = c.on_notice(相手, &notice).unwrap();

    assert_eq!(
        events,
        vec![Event::Signal {
            from: 相手,
            step: Step::Offer,
            blob: 中身,
        }]
    );
}

#[test]
fn 名簿に居ない相手の下ごしらえは受け取らない() {
    let mut c = Conference::host(鍵(1), 12).unwrap();
    let notice = Notice::Signal(Signal::new(c.id(), Step::Offer, b"x".to_vec()));

    assert!(c.on_notice(鍵(9), &notice).is_err());
}

// ── 誰が offer を出すか（glare を避ける） ──────────────────────────

#[test]
fn 両方が同時にofferしない() {
    // 双方が offer を出すと衝突する（glare）。**鍵の並びで決める** —
    // 中央の調停者を置かずに、両側が同じ答えへ辿りつながる必要がある
    let a = 鍵(1);
    let b = 鍵(2);
    let ca = Conference::host(a, 12).unwrap();
    let cb = Conference::host(b, 12).unwrap();

    assert_ne!(
        ca.should_offer_to(&b),
        cb.should_offer_to(&a),
        "どちらか一方だけが offer する"
    );
}

#[test]
fn 答えは相手が誰でも食い違わない() {
    for i in 1..12u8 {
        for j in (i + 1)..12u8 {
            let (x, y) = (鍵(i), 鍵(j));
            let cx = Conference::host(x, 12).unwrap();
            let cy = Conference::host(y, 12).unwrap();
            assert_ne!(cx.should_offer_to(&y), cy.should_offer_to(&x));
        }
    }
}

#[test]
fn 自分に対してはofferしない() {
    let 私 = 鍵(1);
    let c = Conference::host(私, 12).unwrap();
    assert!(!c.should_offer_to(&私));
}

#[test]
fn 主催するルームの_id_を持ち越せる() {
    // **ルーム id が起動ごとに変わると、名前も鍵も持ち越せない。**
    // 2026-09-11 にオーナーの手元で出た ——「ルーム名決めても、リセットされてますね」。
    // 名前は画面の中だけだったが、**それ以前に id が変わっていた**ので、
    // 保存しても紐付く先が無かった。
    //
    // **同じ id で建て直せる**ようにする（渡した鍵の割符は一回性のままなので、
    // 「前の鍵で誰でも入れる」にはならない・D12）。
    let 私 = 鍵(1);
    let 先 = Conference::host(私, 12).unwrap();
    let id = 先.id();

    let 建て直し = Conference::host_with_id(私, 12, id).unwrap();
    assert_eq!(建て直し.id(), id, "同じ id で建て直せる");
    assert_eq!(建て直し.members().len(), 1, "名簿は自分だけから始まる");
    assert_eq!(建て直し.members()[0], 私, "主催は自分のまま");
}

#[test]
fn 持ち越しでも_定員の決まりは同じ() {
    let id = Conference::host(鍵(1), 12).unwrap().id();
    assert!(Conference::host_with_id(鍵(1), 17, id).is_err());
    assert!(Conference::host_with_id(鍵(1), 1, id).is_err());
}
