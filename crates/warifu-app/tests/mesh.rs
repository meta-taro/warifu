//! 3 人以上の網の張り方（M6）。**経路もカメラも登場しない。**
//!
//! フルメッシュでは、**組ごとに 1 本だけ**張らないといけない。
//! 両側から呼びに行くと同じ組に 2 本張られ、どちらを使うかで揉める。
//! **中央の調停者は居ない**（D38 と同じ問題）。

use warifu_app::{Conference, peers_to_dial, should_dial};
use warifu_core::{PublicKey, Seed};

fn 鍵(seed: u8) -> PublicKey {
    Seed::from_bytes([seed; 32])
        .profile("Personal")
        .device("端末")
        .public_key()
}

#[test]
fn 組ごとに一方だけが呼びに行く() {
    for i in 1..14u8 {
        for j in (i + 1)..14u8 {
            let (a, b) = (鍵(i), 鍵(j));
            assert_ne!(
                should_dial(a, b),
                should_dial(b, a),
                "同じ組の両側が同じ答えを出している"
            );
        }
    }
}

#[test]
fn 自分には呼びに行かない() {
    let 私 = 鍵(1);
    assert!(!should_dial(私, 私));
}

#[test]
fn 名簿から呼ぶ相手だけを取り出す() {
    let 私 = 鍵(5);
    let mut c = Conference::host(私, 12).unwrap();
    for s in [2u8, 8, 11] {
        c.admit(鍵(s)).unwrap();
    }

    let 呼ぶ = peers_to_dial(私, c.members());

    // 自分は入らない
    assert!(!呼ぶ.contains(&私));
    // 呼ばない相手は、向こうから呼んでくる
    for p in [鍵(2), 鍵(8), 鍵(11)] {
        assert_eq!(
            呼ぶ.contains(&p),
            should_dial(私, p),
            "取り出しと判定が食い違う"
        );
    }
}

#[test]
fn 全員が同じ名簿を見れば網は一度だけ張られる() {
    // 4 人。**組は 6 つ。**各組でちょうど 1 本になることを、全員分の足し合わせで確かめる
    let 面々 = [鍵(3), 鍵(7), 鍵(9), 鍵(12)];
    let mut 本数 = 0;
    for 私 in 面々 {
        本数 += peers_to_dial(私, &面々).len();
    }
    assert_eq!(本数, 6, "4 人なら 6 本。重複も欠落もない");
}

#[test]
fn 名簿が一人なら呼ぶ相手は居ない() {
    let 私 = 鍵(1);
    assert!(peers_to_dial(私, &[私]).is_empty());
}

#[test]
fn 呼ぶ側とofferを出す側を混ぜない() {
    // **呼びに行く側が offer も出す。**規則を 1 本にしておく。
    // 別々にすると「呼んだのに offer が来ない」を追うことになる
    let (a, b) = (鍵(1), 鍵(2));
    let ca = Conference::host(a, 12).unwrap();
    assert_eq!(should_dial(a, b), ca.should_offer_to(&b));
}

// ── 紹介（D41） ────────────────────────────────────────────

#[test]
fn 主催者は入った人を既存の面々へ紹介する() {
    let 主催 = 鍵(5);
    let mut c = Conference::host(主催, 12).unwrap();
    c.admit(鍵(2)).unwrap();
    c.admit(鍵(8)).unwrap();

    // 3 人目が入った
    let 新入り = 鍵(11);
    let 配り先 = warifu_app::introductions_for(&c, 新入り, 主催).unwrap();

    // 既存の 2 人へ「新入りの住所」を配る
    assert!(配り先.tell_existing.contains(&鍵(2)));
    assert!(配り先.tell_existing.contains(&鍵(8)));
    // **主催者自身と新入りには配らない**
    assert!(!配り先.tell_existing.contains(&主催));
    assert!(!配り先.tell_existing.contains(&新入り));

    // 新入りへは「既存の面々」を教える。**主催者は既に知っている**（会議キーで繋いだ）
    assert!(配り先.tell_newcomer.contains(&鍵(2)));
    assert!(配り先.tell_newcomer.contains(&鍵(8)));
    assert!(!配り先.tell_newcomer.contains(&主催));
    assert!(!配り先.tell_newcomer.contains(&新入り));
}

#[test]
fn 二人目のときは紹介する相手が居ない() {
    let 主催 = 鍵(5);
    let mut c = Conference::host(主催, 12).unwrap();
    let 新入り = 鍵(2);
    c.admit(新入り).unwrap();

    let 配り先 = warifu_app::introductions_for(&c, 新入り, 主催).unwrap();

    assert!(配り先.tell_existing.is_empty());
    assert!(配り先.tell_newcomer.is_empty());
}

#[test]
fn 主催者でなければ紹介しない() {
    // **紹介役は主催者だけ**（D41）。誰でも配ると、同じ紹介が何度も飛ぶ
    let 主催 = 鍵(5);
    let mut c = Conference::host(主催, 12).unwrap();
    c.admit(鍵(2)).unwrap();

    assert!(warifu_app::introductions_for(&c, 鍵(11), 鍵(2)).is_none());
}

#[test]
fn 名簿に居ない人は紹介しない() {
    let 主催 = 鍵(5);
    let c = Conference::host(主催, 12).unwrap();
    // 入っていない相手を紹介しようとしても、配り先は空
    let 配り先 = warifu_app::introductions_for(&c, 鍵(9), 主催).unwrap();
    assert!(配り先.tell_existing.is_empty());
    assert!(配り先.tell_newcomer.is_empty());
}
