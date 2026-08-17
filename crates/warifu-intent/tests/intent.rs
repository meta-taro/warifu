//! 「何をしたいか」の口。
//!
//! この層は **中身を解釈しない**（`decisions.md` **D11**）。
//! `file.offer` の荷物が Markdown でも写真でも、warifu からは同じバイト列に見える。
//! 解釈するのは呼び出す側（md-business など）であって、ここではない。
//!
//! 受け取ったものは**データであって命令ではない**（**D5**）。
//! だからこの層には「受け取ったら実行する」口が 1 つも無い。読める形にして返すだけ。

use warifu_intent::{Correlation, Error, Intent, Kind, MAX_PAYLOAD};

#[test]
fn 口と相関と荷物がそのまま往復する() {
    let 元 = Intent::new(
        Kind::new("file.offer").unwrap(),
        b"\x00\xff design.md".to_vec(),
    );

    let 塊 = 元.encode().unwrap();
    let 戻り = Intent::decode(&塊).unwrap();

    assert_eq!(戻り.kind().as_str(), "file.offer");
    assert_eq!(戻り.correlation(), 元.correlation());
    assert_eq!(戻り.payload(), 元.payload());
}

#[test]
fn 同じ_intent_は同じバイト列になる() {
    // 表記が 2 通りあると、署名や照合が「同じものなのに一致しない」形で壊れる。
    // base32 を正規形だけに絞ったのと同じ理由（M1）。
    let 口 = Kind::new("meeting.invite").unwrap();
    let 相関 = Correlation::generate();

    let a = Intent::with_correlation(口.clone(), 相関, b"room".to_vec());
    let b = Intent::with_correlation(口, 相関, b"room".to_vec());

    assert_eq!(a.encode().unwrap(), b.encode().unwrap());
}

#[test]
fn 返事は同じ相関を持つ() {
    // どの申し出への返事かが分からないと、複数の転送を同時に走らせられない
    let 申し出 = Intent::new(Kind::new("file.offer").unwrap(), b"design.md".to_vec());
    let 返事 = 申し出.reply(Kind::new("file.accept").unwrap(), Vec::new());

    assert_eq!(返事.correlation(), 申し出.correlation());
    assert_eq!(返事.kind().as_str(), "file.accept");
}

#[test]
fn 相関は毎回違う() {
    let a = Correlation::generate();
    let b = Correlation::generate();
    assert_ne!(a, b, "同じなら別々の転送を取り違える");
}

#[test]
fn 口の名前は小文字と数字と点だけ() {
    for 名 in [
        "File.offer",
        "file.Offer",
        "file_offer",
        "file offer",
        "file.offer ",
    ] {
        assert!(
            matches!(Kind::new(名), Err(Error::Malformed)),
            "{名} を受け取ってはいけない"
        );
    }
}

#[test]
fn 口の名前でパスを組み立てられない() {
    // 受け取った名前をそのままパスにしないのが D5 だが、
    // **そもそもパスに見える文字を口の名前に入れさせない**
    for 名 in [
        "file../offer",
        "file/offer",
        "file\\offer",
        "../file.offer",
        "file.offer/..",
    ] {
        assert!(
            matches!(Kind::new(名), Err(Error::Malformed)),
            "{名} を受け取ってはいけない"
        );
    }
}

#[test]
fn 名前空間と動作が両方要る() {
    for 名 in ["file", "file.", ".offer", "file..offer", "", "."] {
        assert!(
            matches!(Kind::new(名), Err(Error::Malformed)),
            "{名} を受け取ってはいけない"
        );
    }
}

#[test]
fn 名前が長すぎたら受け取らない() {
    let 長い = format!("file.{}", "a".repeat(200));
    assert!(matches!(Kind::new(&長い), Err(Error::Malformed)));
}

#[test]
fn 未知の口は読めるが既知にはならない() {
    // 版が上がって知らない口が来ても、経路ごと落とさない。
    // ただし **知らないものを知っているふりはしない**（呼ぶ側が判断する）
    let 未知 = Kind::new("quotation.issue").unwrap();

    assert!(!未知.is_known(), "知らないものを既知にしない");
    assert_eq!(未知.namespace(), "quotation");

    let 塊 = Intent::new(未知, b"x".to_vec()).encode().unwrap();
    let 戻り = Intent::decode(&塊).unwrap();

    assert_eq!(戻り.kind().as_str(), "quotation.issue");
    assert!(!戻り.kind().is_known());
}

#[test]
fn 既知の口がそろっている() {
    // D11 が warifu の担当と定めた 2 つの名前空間
    for 名 in [
        "file.offer",
        "file.accept",
        "file.reject",
        "file.chunk",
        "file.complete",
        "meeting.invite",
        "meeting.join",
        "meeting.leave",
        "meeting.signal",
    ] {
        let 口 = Kind::new(名).unwrap();
        assert!(口.is_known(), "{名} は既知のはず");
    }
}

#[test]
fn 荷物が空でも成立する() {
    // meeting.leave のように、伝えること自体が中身
    let 元 = Intent::new(Kind::new("meeting.leave").unwrap(), Vec::new());
    let 戻り = Intent::decode(&元.encode().unwrap()).unwrap();

    assert!(戻り.payload().is_empty());
    assert_eq!(戻り.kind().as_str(), "meeting.leave");
}

#[test]
fn 荷物が大きすぎたら組み立てない() {
    let 元 = Intent::new(Kind::new("file.chunk").unwrap(), vec![0u8; MAX_PAYLOAD + 1]);
    assert!(matches!(元.encode(), Err(Error::TooLarge)));
}

#[test]
fn 上限ちょうどは通る() {
    let 元 = Intent::new(Kind::new("file.chunk").unwrap(), vec![0u8; MAX_PAYLOAD]);
    assert!(元.encode().is_ok(), "上限を 1 バイト読み違えていない");
}

#[test]
fn 頭が欠けた塊を受け取らない() {
    // 長さだけ宣言して足りない塊を渡し、確保だけさせる手を止める
    let 正しい = Intent::new(Kind::new("file.offer").unwrap(), b"x".to_vec())
        .encode()
        .unwrap();
    let 頭 = 正しい.len() - 1; // 荷物 1 バイトを除いた、口の名前と相関のぶん

    for 切る in 0..頭 {
        assert!(
            matches!(Intent::decode(&正しい[..切る]), Err(Error::Malformed)),
            "{切る} バイトで受け取ってしまった"
        );
    }
}

#[test]
fn 荷物の切れ目はこの層では分からない() {
    // 荷物には長さを書いていない。**切れ目は経路（warifu-net）が決めている。**
    // ここでもう一度長さを書くと、2 つの長さが食い違ったときに直しようがなくなる
    let 正しい = Intent::new(Kind::new("file.offer").unwrap(), b"x".to_vec())
        .encode()
        .unwrap();
    let 頭 = 正しい.len() - 1;

    let 荷物だけ切った = Intent::decode(&正しい[..頭]).expect("頭が揃っていれば読める");
    assert_eq!(荷物だけ切った.payload(), b"", "荷物が空として読める");
}

#[test]
fn 壊れた口の名前を含む塊を受け取らない() {
    let mut 塊 = Intent::new(Kind::new("file.offer").unwrap(), b"x".to_vec())
        .encode()
        .unwrap();
    塊[1] = b'F'; // "file.offer" → "File.offer"

    assert!(matches!(Intent::decode(&塊), Err(Error::Malformed)));
}

#[test]
fn debug_に荷物の中身が出ない() {
    // 荷物は文書そのもの。ログに落ちると、経路を暗号化した意味が消える
    let 元 = Intent::new(Kind::new("file.chunk").unwrap(), b"SECRET-BODY".to_vec());
    let 文字列 = format!("{元:?}");

    assert!(!文字列.contains("SECRET"), "荷物の中身が出た: {文字列}");
    assert!(文字列.contains("file.chunk"), "何の口かは分かるべき");
    assert!(
        文字列.contains("11"),
        "何バイト来たかは分かるべき: {文字列}"
    );
}
