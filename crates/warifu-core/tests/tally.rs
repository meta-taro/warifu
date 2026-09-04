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

// --- 開始のある割符（予定に紐づく会議キー・D43） -----------------------------

/// **予定の前には入れない。**
///
/// 週次 MTG の鍵を前もって配ると、いままでは**渡した瞬間から使えた**。
/// 「10 時から 11 時」の鍵は、9 時に使えてはいけない。
#[test]
fn 始まる前は入れない() {
    let (主催, 客) = 二人();

    let (_tally, token) = 主催.issue_tally_between(1_000, 2_000).expect("割符");
    assert_eq!(token.not_before(), 1_000);
    assert_eq!(token.not_after(), 2_000);

    assert!(
        matches!(客.accept(&token, 999), Err(Error::TooEarly)),
        "始まる前なのに応じられてしまった"
    );
    assert!(
        客.accept(&token, 1_000).is_ok(),
        "始まった時刻ちょうどで入れない"
    );
    assert!(
        客.accept(&token, 2_000).is_ok(),
        "終わる時刻ちょうどで入れない"
    );
    assert!(
        matches!(客.accept(&token, 2_001), Err(Error::Expired)),
        "終わった後なのに入れてしまった"
    );
}

/// 差し出した側も、始まる前の片割れは受け取らない。
///
/// **相手の時計を信じない。**受ける側が自分の時計で見直す。
#[test]
fn 差し出した側も始まる前は合わせない() {
    let (主催, 客) = 二人();

    let (mut tally, token) = 主催.issue_tally_between(1_000, 2_000).expect("割符");
    let acceptance = 客.accept(&token, 1_500).expect("応じる");

    assert!(
        matches!(
            tally.match_half(&acceptance, 999, &Revocations::new()),
            Err(Error::TooEarly)
        ),
        "主催の時計ではまだ始まっていないのに合わせた"
    );
    assert!(
        tally
            .match_half(&acceptance, 1_500, &Revocations::new())
            .is_ok()
    );
}

/// 終わりが始まりより前の窓は作らせない。
#[test]
fn 逆さまの窓は作れない() {
    let (主催, _) = 二人();
    assert!(matches!(
        主催.issue_tally_between(2_000, 1_999),
        Err(Error::BadWindow)
    ));
    // 一瞬だけの窓は認める（始まりと終わりが同じ）
    assert!(主催.issue_tally_between(2_000, 2_000).is_ok());
}

/// `issue_tally` は「いまから ttl 秒」。**今までどおり動く。**
#[test]
fn 期間指定は今から始まる() {
    let (主催, _) = 二人();
    let (tally, token) = 主催.issue_tally(1_000, 600).expect("割符");
    assert_eq!(token.not_before(), 1_000);
    assert_eq!(token.not_after(), 1_600);
    assert_eq!(tally.not_before(), 1_000);
}

/// **開始は署名に入る。**書き換えたら受け取らない。
#[test]
fn 開始を書き換えたら受け取らない() {
    let (主催, _) = 二人();
    let (_tally, token) = 主催.issue_tally_between(1_000, 2_000).expect("割符");

    let mut bytes = token.to_bytes();
    // 開始（69..77）を 0 にすると「いつでも入れる」鍵になる
    for b in &mut bytes[69..77] {
        *b = 0;
    }
    assert!(
        matches!(TallyToken::from_bytes(&bytes), Err(Error::BadSignature)),
        "開始を書き換えた鍵を受け取ってしまった"
    );
}

/// 開始が入った分、鍵は長くなる。**古い長さのものは受け取らない。**
#[test]
fn 開始の無い古い鍵は受け取らない() {
    let (主催, _) = 二人();
    let (_tally, token) = 主催.issue_tally_between(1_000, 2_000).expect("割符");
    let bytes = token.to_bytes();

    // 開始の 8 byte を抜いたもの＝旧版の並び
    let mut 旧版 = Vec::new();
    旧版.extend_from_slice(&bytes[..69]);
    旧版.extend_from_slice(&bytes[77..]);

    assert!(
        matches!(TallyToken::from_bytes(&旧版), Err(Error::Malformed)),
        "開始の無い鍵を読めてしまった"
    );
}
