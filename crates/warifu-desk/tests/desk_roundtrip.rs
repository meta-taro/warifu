//! この機械の口を実際に開いて、行が往復することを確かめる。
//!
//! **JSON が読めることと、口が開くことは別。**片方だけ緑でも繋がらない。

use warifu_desk::{FromDesk, ToDesk, 受け口, 口, 繋ぐ, 開いているか};

/// この試験だけの口の場所。**既定のこの機械を踏まない。**
fn 試験の場所(名: &str) -> std::path::PathBuf {
    #[cfg(windows)]
    {
        std::path::PathBuf::from(format!(r"\\.\pipe\warifu-desk-test-{名}"))
    }
    #[cfg(not(windows))]
    {
        std::env::temp_dir().join(format!("warifu-desk-test-{名}.sock"))
    }
}

#[tokio::test]
async fn エージェントが言ったことが_この機械に届き_返事が返る() {
    let 場所 = 試験の場所("roundtrip");
    let mut 待ち = 受け口::開く(&場所).await.expect("口が開くこと");

    let この機械 = tokio::spawn(async move {
        let 一本 = 待ち.受ける().await.expect("1 本受けること");
        let mut 口 = 口::新しく(一本);
        let 行 = 口.受ける().await.unwrap().expect("行が来ること");
        let ToDesk::Say { body } = ToDesk::読む(&行).unwrap() else {
            panic!("say のはず");
        };
        // **この機械が差出人を刻む。**言ってきた側は名乗っていない
        let 返し = FromDesk::Heard {
            id: 1,
            from: "mcp:local-agent".to_owned(),
            body,
            at: "09:05".to_owned(),
        };
        口.送る(&返し.書く()).await.unwrap();
    });

    let mut 客 = 口::新しく(繋ぐ(&場所).await.expect("繋がること"));
    客.送る(&ToDesk::say("直しました").unwrap().書く())
        .await
        .unwrap();
    let 行 = 客.受ける().await.unwrap().expect("返事が来ること");

    let FromDesk::Heard { from, body, at, .. } = FromDesk::読む(&行).unwrap() else {
        panic!("heard のはず");
    };
    assert_eq!(from, "mcp:local-agent");
    assert_eq!(body, "直しました");
    assert_eq!(at, "09:05");
    この機械.await.unwrap();
}

#[tokio::test]
async fn 開いていないこの機械は_開いていないと分かる() {
    // **「繋がらない」を「相手が黙っている」と混ぜない。**
    // 混ざると、この機械を立て忘れているのか会話相手が居ないのかが読めない
    assert!(!開いているか(&試験の場所("nobody")).await);
}

#[cfg(not(windows))]
#[tokio::test]
async fn この機械は所有者だけが読み書きできる() {
    use std::os::unix::fs::PermissionsExt;

    let 場所 = 試験の場所("perm");
    let _待ち = 受け口::開く(&場所).await.expect("口が開くこと");
    let 権限 = std::fs::metadata(&場所).unwrap().permissions();
    // **他人が繋げる口にしない。**同じ機械の別ユーザーも入れてはいけない
    assert_eq!(権限.mode() & 0o777, 0o600, "0600 であること");
}

#[cfg(not(windows))]
#[tokio::test]
async fn 落ちたこの機械の残骸があっても_次は開く() {
    // **残骸を理由に開かないのは、直しようが無い形で止まる**
    let 場所 = 試験の場所("stale");
    std::fs::write(&場所, b"").unwrap();
    let _待ち = 受け口::開く(&場所).await.expect("残骸を片付けて開くこと");
}

#[cfg(not(windows))]
#[tokio::test]
async fn 動いているこの機械は_横取りされない() {
    // **2 つ目のこの機械が同じ口を奪うと、人の画面とエージェントが別の会話を見る**
    let 場所 = 試験の場所("dup");
    let _一つ目 = 受け口::開く(&場所).await.expect("1 つ目は開くこと");
    let 二つ目 = 受け口::開く(&場所).await;
    assert!(二つ目.is_err(), "2 つ目は断られること");
}

/// **同じ機械で 2 つの身元を動かせるか。**
///
/// オーナー（2026-09-11）——「A さんエージェントと B さんエージェントが
/// 必要に応じてお互いにやりとりできるか」を**同じ PC で見たい**。
///
/// ところが、この機械の口は `HOME` から決まる 1 か所だけだった ——
/// つまり**2 つ目の身元は口を開けない**（同じ場所を取り合う）。
/// `WARIFU_HOME` は金庫（鍵・名簿）を分ける仕組みとして既にあるので、
/// **口の場所もそれに従わせる。**
///
/// **環境変数を試験の中で書き換えない**（この crate は `unsafe` を禁じているし、
/// 並んで走る試験に漏れる）。**決め方を純粋な関数にして、そこを見る。**
///
/// **Windows は形が違う。**名前付きパイプは「場所」ではなく**名前**なので、
/// `\\.\pipe\warifu-desk-<印>` になる（`place.rs`）。
/// **2026-09-13 に CI で Windows の試験を回して、ここが落ちて分かった** ——
/// それまで**この試験は Unix の形しか見ていなかった。**
/// 見たいのは「**家を分ければ口も分かれる**」であって、パスの形ではない。
#[test]
#[cfg(not(windows))]
fn 家を指定すれば_この機械の口も分かれる() {
    use std::ffi::OsString;
    use std::path::Path;
    use warifu_desk::場所を決める;

    let 家 = Some(OsString::from("/tmp/warifu-b"));
    assert_eq!(
        場所を決める(家.as_deref(), None, None),
        Path::new("/tmp/warifu-b/desk.sock"),
        "WARIFU_HOME があれば、そこに開く"
    );

    // **既定は変えない。**`WARIFU_HOME` が無ければ、これまでと同じ決め方
    let 実行時 = Some(OsString::from("/run/user/501"));
    assert_eq!(
        場所を決める(None, 実行時.as_deref(), None),
        Path::new("/run/user/501/warifu/desk.sock"),
        "実行時ディレクトリがあれば、そちら"
    );

    let ホーム = Some(OsString::from("/Users/someone"));
    let 出た = 場所を決める(None, None, ホーム.as_deref());
    assert!(
        出た.starts_with("/Users/someone"),
        "家の下に開く: {}",
        出た.display()
    );

    // **家も実行時も分からないときでも、場所は返す**（呼ぶ側を困らせない）
    assert!(場所を決める(None, None, None).ends_with("desk.sock"));
}

/// **Windows でも、家を分ければ口が分かれる。**
///
/// 名前付きパイプは**名前の取り合い**なので、分かれていなければ
/// **同じ機械の 2 つ目の身元が口を開けない**（そこが D55 の眼目である）。
#[test]
#[cfg(windows)]
fn 家を指定すれば_この機械の口も分かれる() {
    use std::ffi::OsString;
    use warifu_desk::場所を決める;

    let 既定 = 場所を決める(None, None, None);
    let 家 = Some(OsString::from(r"C:\tmp\warifu-b"));
    let 別 = 場所を決める(家.as_deref(), None, None);

    // **パイプの名前であること**（場所ではない）
    assert!(
        既定.to_string_lossy().starts_with(r"\\.\pipe\warifu-desk"),
        "既定はパイプの名前: {}",
        既定.display()
    );
    // **家を分ければ、名前も分かれる**
    assert_ne!(既定, 別, "家を分けたら口も分かれる");
    assert!(
        別.to_string_lossy().starts_with(r"\\.\pipe\warifu-desk-"),
        "分けた口も同じ形: {}",
        別.display()
    );
    // **実行時ディレクトリには引っ張られない**（Windows では見ない）
    let 実行時 = Some(OsString::from(r"C:\run"));
    assert_eq!(場所を決める(家.as_deref(), 実行時.as_deref(), None), 別);
}

#[test]
#[cfg(not(windows))]
fn 家を指定すると_実行時ディレクトリより優先する() {
    // **試験のために家を分けたのに、実行時ディレクトリに引っ張られては意味がない**
    use std::ffi::OsString;
    use std::path::Path;
    use warifu_desk::場所を決める;

    let 家 = Some(OsString::from("/tmp/warifu-b"));
    let 実行時 = Some(OsString::from("/run/user/501"));
    assert_eq!(
        場所を決める(家.as_deref(), 実行時.as_deref(), None),
        Path::new("/tmp/warifu-b/desk.sock")
    );
}

// --- 机の口と控えの置き場所（gh issue 15・2026-09-14） --------------------

/// **控えを「口の親フォルダ」から作るのをやめた。**
///
/// `.claude/issues/019` で控えを足したとき、**Unix のパスの形を前提にしていた** ——
/// **Windows の名前付きパイプに、フォルダは無い。**
/// Windows の人が実測して `gh issue 15` に上げてくれた（`cargo test` が 7 件落ちた）。
#[test]
fn 口と控えは別に決まる() {
    use std::ffi::OsString;
    use warifu_desk::在り処を決める;

    let 家 = Some(OsString::from(if cfg!(windows) {
        r"C:\tmp\warifu-b"
    } else {
        "/tmp/warifu-b"
    }));
    let 出た = 在り処を決める(家.as_deref(), None, None);

    // **控えはフォルダである**（口がパイプの名前になる OS でも）
    assert!(
        !出た.控え.as_os_str().is_empty(),
        "控えの置き場所が空: {:?}",
        出た
    );
    #[cfg(windows)]
    {
        // Windows の口はパイプの名前。**親フォルダを控えに使えない**
        assert!(出た.口.to_string_lossy().starts_with(r"\\.\pipe\"));
        assert!(!出た.控え.to_string_lossy().starts_with(r"\\.\pipe\"));
    }
    #[cfg(not(windows))]
    {
        assert_eq!(出た.口, std::path::Path::new("/tmp/warifu-b/desk.sock"));
        assert_eq!(出た.控え, std::path::Path::new("/tmp/warifu-b"));
    }
}

#[test]
fn 家も実行時もホームも無くても_控えは返る() {
    use warifu_desk::在り処を決める;
    let 出た = 在り処を決める(None, None, None);
    assert!(!出た.控え.as_os_str().is_empty());
}

// ── 札を頼む（**D119** / **#26**・2026-09-18） ────────────────────────
//
// **頼みは部屋へ流さない。**この機械（同じ PC の中）を通る。
// 2026-09-15、ASUS の画面に「許可が必要です」が届いた —— **聞く口と流す口が同じだった。**

#[test]
fn 頼みは_渡して読み戻せる() {
    let 元 = ToDesk::Ask {
        動作: "chat.send".into(),
        訳: "部屋へ 1 行流したい".into(),
    };
    assert_eq!(ToDesk::読む(&元.書く()).expect("読める"), 元);
}

#[test]
fn 答えは_四つとも渡して読み戻せる() {
    use warifu_desk::頼みの返り;
    for 答え in [
        頼みの返り::まだ,
        頼みの返り::許した,
        頼みの返り::断った,
        頼みの返り::受け付けない {
            訳: "訳がありません".into(),
        },
    ] {
        let 元 = FromDesk::Asked {
            動作: "chat.send".into(),
            答え: 答え.clone(),
        };
        assert_eq!(FromDesk::読む(&元.書く()).expect("読める"), 元);
    }
}

#[test]
fn 訳が長すぎる頼みは_この機械まで運ばない() {
    // **上限は書き手の側でも見る**（`本文の上限` と同じ構え）
    let 長い = "あ".repeat(warifu_desk::訳の上限);
    let 行 = ToDesk::Ask {
        動作: "chat.send".into(),
        訳: 長い,
    }
    .書く();
    assert!(matches!(
        ToDesk::読む(&行),
        Err(warifu_desk::Error::TooLong(_))
    ));
}

#[test]
fn 訳に行や欄を壊すものは通さない() {
    // **通すと、受け取った側の画面が崩れる**（`名乗りを検める` と同じ扱い）
    for 悪い in ["改\n行", "欄\tを壊す", "制御\u{0007}文字"] {
        let 行 = ToDesk::Ask {
            動作: "chat.send".into(),
            訳: 悪い.into(),
        }
        .書く();
        assert!(ToDesk::読む(&行).is_err(), "{悪い:?} を通した");
    }
}

#[test]
fn 訳が空でも_行としては通る() {
    // **空は弾かない。**空は「受け付けない」として**答えを返す**必要がある ——
    // 黙って落とすと、エージェントは
    // **「届いていない」と「形が悪い」を見分けられない**（D102 で踏んだのと同じ形）
    let 行 = ToDesk::Ask {
        動作: "chat.send".into(),
        訳: String::new(),
    }
    .書く();
    assert!(ToDesk::読む(&行).is_ok());
}

#[test]
fn 知らない鍵が付いた頼みは受けない() {
    // **受けると、そこが差出人を騙る入口になる**（この機械の約束）
    let 行 = r#"{"型":"ask","動作":"chat.send","訳":"要る","誰":"別人"}"#;
    assert!(ToDesk::読む(行).is_err());
}
