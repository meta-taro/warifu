//! 失効。
//!
//! 端末を失くしたとき・配った割符を取り消したいときに、**中央に問い合わせずに**止める。
//! 中央 Directory を作らない以上、失効は各自が持つ手元の名簿でしかありえない。

use warifu_core::{Error, Revocations, Seed};

const 一時間: u64 = 60 * 60;
const 発行時刻: u64 = 1_755_000_000;

#[test]
fn 失効させた端末は片割れが合っても通さない() {
    let alice = Seed::from_bytes([1u8; 32]).profile("Personal").device("PC");
    let bob = Seed::from_bytes([2u8; 32]).profile("Personal").device("PC");

    let (mut 控え, 渡す半分) = alice.issue_tally(発行時刻, 一時間).unwrap();
    let 受諾 = bob.accept(&渡す半分, 発行時刻 + 10).unwrap();

    let mut 名簿 = Revocations::new();
    名簿.revoke_device(bob.public_key(), 発行時刻 + 15);

    assert!(matches!(
        控え.match_half(&受諾, 発行時刻 + 20, &名簿),
        Err(Error::Revoked)
    ));
}

#[test]
fn 失効させた割符は使えない() {
    let alice = Seed::from_bytes([1u8; 32]).profile("Personal").device("PC");
    let bob = Seed::from_bytes([2u8; 32]).profile("Personal").device("PC");

    let (mut 控え, 渡す半分) = alice.issue_tally(発行時刻, 一時間).unwrap();
    let 受諾 = bob.accept(&渡す半分, 発行時刻 + 10).unwrap();

    let mut 名簿 = Revocations::new();
    名簿.revoke_tally(控え.id(), 発行時刻 + 15);

    assert!(matches!(
        控え.match_half(&受諾, 発行時刻 + 20, &名簿),
        Err(Error::Revoked)
    ));
}

#[test]
fn 関係ない相手を巻き込まない() {
    let alice = Seed::from_bytes([1u8; 32]).profile("Personal").device("PC");
    let bob = Seed::from_bytes([2u8; 32]).profile("Personal").device("PC");
    let carol = Seed::from_bytes([3u8; 32]).profile("Personal").device("PC");

    let mut 名簿 = Revocations::new();
    名簿.revoke_device(carol.public_key(), 発行時刻);

    let (mut 控え, 渡す半分) = alice.issue_tally(発行時刻, 一時間).unwrap();
    let 受諾 = bob.accept(&渡す半分, 発行時刻 + 10).unwrap();

    assert!(控え.match_half(&受諾, 発行時刻 + 20, &名簿).is_ok());
}

#[test]
fn 失効は取り消せない() {
    // 「やっぱり戻す」を作ると、鍵を盗った側が戻せてしまう。
    // 戻したいときは新しい鍵を作る（＝新しい割符を配り直す）
    let bob = Seed::from_bytes([2u8; 32]).profile("Personal").device("PC");

    let mut 名簿 = Revocations::new();
    名簿.revoke_device(bob.public_key(), 発行時刻);

    assert!(名簿.is_revoked_device(&bob.public_key()));

    // 同じものを二度失効させても壊れない
    名簿.revoke_device(bob.public_key(), 発行時刻 + 100);
    assert!(名簿.is_revoked_device(&bob.public_key()));
    assert_eq!(名簿.devices().count(), 1);
}

#[test]
fn 失効した時刻が残る() {
    // いつ失くしたかが分からないと、その前後どちらの通信を疑うか決められない
    let bob = Seed::from_bytes([2u8; 32]).profile("Personal").device("PC");

    let mut 名簿 = Revocations::new();
    名簿.revoke_device(bob.public_key(), 発行時刻);
    名簿.revoke_device(bob.public_key(), 発行時刻 + 100);

    let (_, 時刻) = 名簿.devices().next().unwrap();

    assert_eq!(
        時刻, 発行時刻,
        "後から上書きすると、最初に失くした時刻が消える"
    );
}

#[test]
fn 空の名簿は誰も止めない() {
    let bob = Seed::from_bytes([2u8; 32]).profile("Personal").device("PC");
    let 名簿 = Revocations::new();

    assert!(!名簿.is_revoked_device(&bob.public_key()));
    assert_eq!(名簿.devices().count(), 0);
    assert_eq!(名簿.tallies().count(), 0);
}
