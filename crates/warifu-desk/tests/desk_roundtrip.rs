//! 机の口を実際に開いて、行が往復することを確かめる。
//!
//! **JSON が読めることと、口が開くことは別。**片方だけ緑でも繋がらない。

use warifu_desk::{FromDesk, ToDesk, 受け口, 口, 繋ぐ, 開いているか};

/// この試験だけの口の場所。**既定の机を踏まない。**
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
async fn エージェントが言ったことが_机に届き_返事が返る() {
    let 場所 = 試験の場所("roundtrip");
    let mut 待ち = 受け口::開く(&場所).await.expect("口が開くこと");

    let 机 = tokio::spawn(async move {
        let 一本 = 待ち.受ける().await.expect("1 本受けること");
        let mut 口 = 口::新しく(一本);
        let 行 = 口.受ける().await.unwrap().expect("行が来ること");
        let ToDesk::Say { body } = ToDesk::読む(&行).unwrap() else {
            panic!("say のはず");
        };
        // **机が差出人を刻む。**言ってきた側は名乗っていない
        let 返し = FromDesk::Heard {
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

    let FromDesk::Heard { from, body, at } = FromDesk::読む(&行).unwrap() else {
        panic!("heard のはず");
    };
    assert_eq!(from, "mcp:local-agent");
    assert_eq!(body, "直しました");
    assert_eq!(at, "09:05");
    机.await.unwrap();
}

#[tokio::test]
async fn 開いていない机は_開いていないと分かる() {
    // **「繋がらない」を「相手が黙っている」と混ぜない。**
    // 混ざると、机を立て忘れているのか会話相手が居ないのかが読めない
    assert!(!開いているか(&試験の場所("nobody")).await);
}

#[cfg(not(windows))]
#[tokio::test]
async fn 机は所有者だけが読み書きできる() {
    use std::os::unix::fs::PermissionsExt;

    let 場所 = 試験の場所("perm");
    let _待ち = 受け口::開く(&場所).await.expect("口が開くこと");
    let 権限 = std::fs::metadata(&場所).unwrap().permissions();
    // **他人が繋げる口にしない。**同じ機械の別ユーザーも入れてはいけない
    assert_eq!(権限.mode() & 0o777, 0o600, "0600 であること");
}

#[cfg(not(windows))]
#[tokio::test]
async fn 落ちた机の残骸があっても_次は開く() {
    // **残骸を理由に開かないのは、直しようが無い形で止まる**
    let 場所 = 試験の場所("stale");
    std::fs::write(&場所, b"").unwrap();
    let _待ち = 受け口::開く(&場所).await.expect("残骸を片付けて開くこと");
}

#[cfg(not(windows))]
#[tokio::test]
async fn 動いている机は_横取りされない() {
    // **2 つ目の机が同じ口を奪うと、人の画面とエージェントが別の会話を見る**
    let 場所 = 試験の場所("dup");
    let _一つ目 = 受け口::開く(&場所).await.expect("1 つ目は開くこと");
    let 二つ目 = 受け口::開く(&場所).await;
    assert!(二つ目.is_err(), "2 つ目は断られること");
}
