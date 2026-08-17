//! シードから Profile 鍵・Device 鍵を導く部分。
//!
//! ここが決定的であることが、`decisions.md` D2（鍵の復旧方式が未決）を
//! 実装のブロッカーから外している根拠。**同じシードから同じ鍵が出ること**が
//! 保証される限り、復旧方式が a〜d のどれになっても Identity の形は変わらない。

use warifu_core::Seed;

const SEED_A: [u8; 32] = [1u8; 32];
const SEED_B: [u8; 32] = [2u8; 32];

#[test]
fn 同じシードと同じラベルからは同じ鍵が出る() {
    let a = Seed::from_bytes(SEED_A).profile("Personal").device("PC");
    let b = Seed::from_bytes(SEED_A).profile("Personal").device("PC");

    assert_eq!(a.public_key(), b.public_key());
}

#[test]
fn シードが違えば鍵も違う() {
    let a = Seed::from_bytes(SEED_A).profile("Personal").device("PC");
    let b = Seed::from_bytes(SEED_B).profile("Personal").device("PC");

    assert_ne!(a.public_key(), b.public_key());
}

#[test]
fn profileが違えばdevice鍵も違う() {
    let seed = Seed::from_bytes(SEED_A);
    let personal = seed.profile("Personal").device("PC");
    let work = seed.profile("Work").device("PC");

    assert_ne!(
        personal.public_key(),
        work.public_key(),
        "Personal と Work が同じ鍵になると、相手から同一人物だと分かってしまう"
    );
}

#[test]
fn deviceが違えば鍵も違う() {
    let profile = Seed::from_bytes(SEED_A).profile("Personal");

    assert_ne!(
        profile.device("PC").public_key(),
        profile.device("スマホ").public_key()
    );
}

#[test]
fn profile鍵とdevice鍵は別物() {
    let profile = Seed::from_bytes(SEED_A).profile("Personal");

    assert_ne!(
        profile.public_key(),
        profile.device("PC").public_key(),
        "端末を 1 台失っても Profile ごと失わないために、両者は分かれている必要がある"
    );
}

#[test]
fn ラベルの区切りを跨いで衝突しない() {
    // "a" + "b" と "ab" が同じ鍵に落ちると、ラベルを細工して他人の鍵を作れてしまう
    let seed = Seed::from_bytes(SEED_A);

    assert_ne!(
        seed.profile("a").device("b").public_key(),
        seed.profile("ab").device("").public_key()
    );
    assert_ne!(
        seed.profile("a").device("b").public_key(),
        seed.profile("").device("ab").public_key()
    );
}

#[test]
fn 生成したシードは毎回違う() {
    let a = Seed::generate().expect("乱数が取れない");
    let b = Seed::generate().expect("乱数が取れない");

    assert_ne!(
        a.profile("P").device("D").public_key(),
        b.profile("P").device("D").public_key()
    );
}

#[test]
fn 署名した本人の鍵でだけ検証が通る() {
    let seed = Seed::from_bytes(SEED_A);
    let mine = seed.profile("Personal").device("PC");
    let other = seed.profile("Personal").device("スマホ");

    let sig = mine.sign(b"warifu");

    assert!(mine.public_key().verify(b"warifu", &sig).is_ok());
    assert!(other.public_key().verify(b"warifu", &sig).is_err());
    assert!(
        mine.public_key()
            .verify("warifu ではない".as_bytes(), &sig)
            .is_err()
    );
}

#[test]
fn 公開鍵は文字列にして戻せる() {
    let key = Seed::from_bytes(SEED_A)
        .profile("Personal")
        .device("PC")
        .public_key();

    let text = key.to_string();
    let back = text.parse().expect("自分が出した文字列を読めない");

    assert_eq!(key, back);
}

#[test]
fn 壊れた公開鍵の文字列は読めない() {
    use warifu_core::PublicKey;

    assert!("".parse::<PublicKey>().is_err());
    assert!("ふつうの文字列".parse::<PublicKey>().is_err());
    // 長さは合っているが 16 進として不正
    assert!("zz".repeat(32).parse::<PublicKey>().is_err());
}
