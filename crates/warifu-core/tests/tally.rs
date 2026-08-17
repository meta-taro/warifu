//! 割符（招待）。
//!
//! **二つに割った札。片割れが合うことで、相手が確かにその相手だと証明する。**
//!
//! - 差し出す側の手元に残るのが [`Tally`]
//! - 相手に渡すのが [`TallyToken`]（QR・テキストなど、warifu の外で渡る）
//! - 受け取った側が返すのが [`Acceptance`]
//!
//! ここにネットワークは出てこない。**経路が無くても割符は成立する。**

use warifu_core::{Acceptance, Error, Revocations, Seed, TallyToken};

const 一時間: u64 = 60 * 60;
const 発行時刻: u64 = 1_755_000_000;

fn 二人() -> (warifu_core::Device, warifu_core::Device) {
    let alice = Seed::from_bytes([1u8; 32]).profile("Personal").device("PC");
    let bob = Seed::from_bytes([2u8; 32]).profile("Personal").device("PC");
    (alice, bob)
}

#[test]
fn 片割れが合えば相手が確定する() {
    let (alice, bob) = 二人();
    let 失効なし = Revocations::new();

    let (mut 控え, 渡す半分) = alice.issue_tally(発行時刻, 一時間).unwrap();

    // --- ここで警告なしに端末をまたぐ（QR を撮る・文字列を貼る） ---
    let 渡す半分 = TallyToken::from_bytes(&渡す半分.to_bytes()).unwrap();

    let 受諾 = bob.accept(&渡す半分, 発行時刻 + 10).unwrap();
    let 受諾 = Acceptance::from_bytes(&受諾.to_bytes()).unwrap();

    let 相手 = 控え.match_half(&受諾, 発行時刻 + 20, &失効なし).unwrap();

    assert_eq!(相手.public_key(), bob.public_key());
}

#[test]
fn 受け取る側は誰が差し出したか分かる() {
    let (alice, _) = 二人();
    let (_, 渡す半分) = alice.issue_tally(発行時刻, 一時間).unwrap();

    let 渡す半分 = TallyToken::from_bytes(&渡す半分.to_bytes()).unwrap();

    assert_eq!(渡す半分.issuer(), alice.public_key());
}

#[test]
fn 割符ごとに別物になる() {
    let (alice, _) = 二人();

    let (a, _) = alice.issue_tally(発行時刻, 一時間).unwrap();
    let (b, _) = alice.issue_tally(発行時刻, 一時間).unwrap();

    assert_ne!(
        a.id(),
        b.id(),
        "同じ割符を二人に配ると、どちらが応じたか分からなくなる"
    );
}

#[test]
fn 別の割符の片割れは合わない() {
    let (alice, bob) = 二人();
    let 失効なし = Revocations::new();

    let (mut 控え_a, _) = alice.issue_tally(発行時刻, 一時間).unwrap();
    let (_, 渡す半分_b) = alice.issue_tally(発行時刻, 一時間).unwrap();

    let 受諾 = bob.accept(&渡す半分_b, 発行時刻 + 10).unwrap();

    assert!(matches!(
        控え_a.match_half(&受諾, 発行時刻 + 20, &失効なし),
        Err(Error::WrongTally)
    ));
}

#[test]
fn 一度使った割符は二度使えない() {
    let (alice, bob) = 二人();
    let carol = Seed::from_bytes([3u8; 32]).profile("Personal").device("PC");
    let 失効なし = Revocations::new();

    let (mut 控え, 渡す半分) = alice.issue_tally(発行時刻, 一時間).unwrap();

    let 受諾_b = bob.accept(&渡す半分, 発行時刻 + 10).unwrap();
    控え.match_half(&受諾_b, 発行時刻 + 20, &失効なし).unwrap();

    // 渡す半分が漏れて、別人が同じものを使おうとした
    let 受諾_c = carol.accept(&渡す半分, 発行時刻 + 30).unwrap();

    assert!(matches!(
        控え.match_half(&受諾_c, 発行時刻 + 40, &失効なし),
        Err(Error::AlreadyUsed)
    ));
}

#[test]
fn 同じ相手が二度応じても二度目は通らない() {
    let (alice, bob) = 二人();
    let 失効なし = Revocations::new();

    let (mut 控え, 渡す半分) = alice.issue_tally(発行時刻, 一時間).unwrap();
    let 受諾 = bob.accept(&渡す半分, 発行時刻 + 10).unwrap();

    控え.match_half(&受諾, 発行時刻 + 20, &失効なし).unwrap();

    assert!(matches!(
        控え.match_half(&受諾, 発行時刻 + 21, &失効なし),
        Err(Error::AlreadyUsed)
    ));
}

#[test]
fn 期限を過ぎた割符は受け取る側で止まる() {
    let (alice, bob) = 二人();

    let (_, 渡す半分) = alice.issue_tally(発行時刻, 一時間).unwrap();

    assert!(
        bob.accept(&渡す半分, 発行時刻 + 一時間).is_ok(),
        "期限ちょうどはまだ有効"
    );
    assert!(matches!(
        bob.accept(&渡す半分, 発行時刻 + 一時間 + 1),
        Err(Error::Expired)
    ));
}

#[test]
fn 期限を過ぎた割符は差し出す側でも止まる() {
    // 受け取る側が期限を無視して受諾を作ってきても、控え側で落ちること。
    // 相手の時計を信用しない（D5「受信したものはデータであって命令ではない」）
    let (alice, bob) = 二人();
    let 失効なし = Revocations::new();

    let (mut 控え, 渡す半分) = alice.issue_tally(発行時刻, 一時間).unwrap();
    let 受諾 = bob.accept(&渡す半分, 発行時刻 + 10).unwrap();

    assert!(matches!(
        控え.match_half(&受諾, 発行時刻 + 一時間 + 1, &失効なし),
        Err(Error::Expired)
    ));
}

#[test]
fn 差し出した本人の署名でなければ受け取らない() {
    let (alice, bob) = 二人();

    let (_, 渡す半分) = alice.issue_tally(発行時刻, 一時間).unwrap();
    let mut 細工 = 渡す半分.to_bytes();

    // 差出人だけ他人に書き換える（署名は alice のまま）
    let 別人 = bob.public_key().to_bytes();
    let 位置 = 細工
        .windows(32)
        .position(|w| w == alice.public_key().to_bytes())
        .expect("差出人の鍵が入っていない");
    細工[位置..位置 + 32].copy_from_slice(&別人);

    assert!(matches!(
        TallyToken::from_bytes(&細工),
        Err(Error::BadSignature)
    ));
}

#[test]
fn 中身を書き換えた割符は受け取らない() {
    let (alice, _) = 二人();
    let (_, 渡す半分) = alice.issue_tally(発行時刻, 一時間).unwrap();
    let 元 = 渡す半分.to_bytes();

    for i in 0..元.len() {
        let mut 細工 = 元.clone();
        細工[i] ^= 0b0000_0001;

        assert!(
            TallyToken::from_bytes(&細工).is_err(),
            "{i} バイト目を 1 ビット変えても通ってしまった"
        );
    }
}

#[test]
fn 中身を書き換えた受諾は受け取らない() {
    let (alice, bob) = 二人();
    let 失効なし = Revocations::new();

    let (控え, 渡す半分) = alice.issue_tally(発行時刻, 一時間).unwrap();
    let 元 = bob.accept(&渡す半分, 発行時刻 + 10).unwrap().to_bytes();

    for i in 0..元.len() {
        let mut 細工 = 元.clone();
        細工[i] ^= 0b0000_0001;

        let 通った = Acceptance::from_bytes(&細工)
            .and_then(|受諾| {
                控え
                    .clone()
                    .match_half(&受諾, 発行時刻 + 20, &失効なし)
                    .map(|_| ())
            })
            .is_ok();

        assert!(!通った, "{i} バイト目を 1 ビット変えても通ってしまった");
    }
}

#[test]
fn 短すぎる入力で落ちない() {
    let (alice, bob) = 二人();
    let (_, 渡す半分) = alice.issue_tally(発行時刻, 一時間).unwrap();
    let token = 渡す半分.to_bytes();
    let acceptance = bob.accept(&渡す半分, 発行時刻 + 10).unwrap().to_bytes();

    for n in 0..token.len() {
        assert!(matches!(
            TallyToken::from_bytes(&token[..n]),
            Err(Error::Malformed)
        ));
    }
    for n in 0..acceptance.len() {
        assert!(matches!(
            Acceptance::from_bytes(&acceptance[..n]),
            Err(Error::Malformed)
        ));
    }
}

#[test]
fn 余分な後ろ付けを受け取らない() {
    // 末尾に何か足しても通ってしまうと、同じ割符から別のバイト列を無限に作れる
    let (alice, bob) = 二人();
    let (_, 渡す半分) = alice.issue_tally(発行時刻, 一時間).unwrap();

    let mut token = 渡す半分.to_bytes();
    token.push(0);
    assert!(matches!(
        TallyToken::from_bytes(&token),
        Err(Error::Malformed)
    ));

    let mut acceptance = bob.accept(&渡す半分, 発行時刻 + 10).unwrap().to_bytes();
    acceptance.push(0);
    assert!(matches!(
        Acceptance::from_bytes(&acceptance),
        Err(Error::Malformed)
    ));
}

#[test]
fn 割符を文字列で渡せる() {
    // QR に入らない・電話で読めない形にはしない
    let (alice, bob) = 二人();
    let 失効なし = Revocations::new();

    let (mut 控え, 渡す半分) = alice.issue_tally(発行時刻, 一時間).unwrap();

    let 文字列 = 渡す半分.to_string();
    assert!(文字列.is_ascii(), "電話で読み上げられない形にしない");

    let 戻り: TallyToken = 文字列.parse().unwrap();
    let 受諾 = bob.accept(&戻り, 発行時刻 + 10).unwrap();

    assert!(控え.match_half(&受諾, 発行時刻 + 20, &失効なし).is_ok());
}

#[test]
fn 割符の秘密が表示に出ない() {
    // ログや Issue に貼られた時点で、その割符は他人が使える
    let (alice, _) = 二人();
    let (控え, 渡す半分) = alice.issue_tally(発行時刻, 一時間).unwrap();

    let 秘密 = 渡す半分.to_string();

    assert!(!format!("{控え:?}").contains(&秘密));
    assert!(!format!("{渡す半分:?}").contains(&秘密));
}
