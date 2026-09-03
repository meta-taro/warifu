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
