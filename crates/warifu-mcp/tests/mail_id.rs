//! **送ったメールの印**（`mail_send` の返りの Message-ID・0.1.59）。
//!
//! 送ったあと、送信済み（`sent_list`）と突き合わせて「送れたか」を確かめられるように、
//! 口が印を決めて机へ渡し、返りに書く。古い画面は印の欄を読めないので、外して頼み直す。

use rmcp::handler::server::wrapper::Parameters;
use warifu_capability::{Action, Gate, Grant};
use warifu_desk::{FromDesk, ToDesk, 受け口, 口 as 行の口};
use warifu_mcp::{MailSendArgs, Warifu, subject};
use warifu_read::RuleStore;

fn 用意() -> Warifu {
    let mut 関所 = Gate::new();
    関所.issue(Grant::new(
        subject(),
        Action::new("mail.send").unwrap(),
        1_798_761_600,
    ));
    Warifu::new(Vec::new(), RuleStore::new(), 関所, 1_756_000_000)
}

fn 試験の机(名: &str) -> std::path::PathBuf {
    let 走り = std::process::id();
    #[cfg(windows)]
    {
        std::path::PathBuf::from(format!(r"\\.\pipe\warifu-mcp-{名}-{走り}"))
    }
    #[cfg(not(windows))]
    {
        std::env::temp_dir().join(format!("warifu-mcp-{名}-{走り}.sock"))
    }
}

fn 試験の控え(名: &str) -> std::path::PathBuf {
    let 控え = std::env::temp_dir().join(format!("warifu-mcp-控え-{名}-{}", std::process::id()));
    std::fs::create_dir_all(&控え).expect("フォルダが作れること");
    控え
}

/// 机の真似。`古い` なら、印の欄の付いた行を「行の形」で断る（0.1.58 までの机）。届いた生の行を返す。
async fn 机を立てる(
    名: &str,
    古い: bool,
) -> (std::path::PathBuf, tokio::task::JoinHandle<Vec<String>>) {
    let 場所 = 試験の机(名);
    let mut 待ち = 受け口::開く(&場所).await.expect("この机が開くこと");
    let 机 = tokio::spawn(async move {
        let mut 口 = 行の口::新しく(待ち.受ける().await.unwrap());
        let _挨拶 = 口.受ける().await.unwrap().unwrap();
        口.送る(
            &FromDesk::Seated {
                at: "12:00".to_owned(),
                who: "試験の席".to_owned(),
            }
            .書く(),
        )
        .await
        .unwrap();
        let mut 届いた = Vec::new();
        while let Ok(Some(行)) = 口.受ける().await {
            let 返事 = if 古い && 行.contains("通の印") {
                FromDesk::Denied {
                    why: "行の形が壊れています".to_owned(),
                }
            } else {
                let ToDesk::メールを送る { 宛先, .. } = ToDesk::読む(&行).expect("読める")
                else {
                    panic!("メールの行ではない: {行}");
                };
                FromDesk::メール {
                    結果: warifu_desk::メールの返り::送った {
                        宛先の数: 宛先.len(),
                    },
                }
            };
            let 終わり = matches!(返事, FromDesk::メール { .. });
            届いた.push(行);
            口.送る(&返事.書く()).await.unwrap();
            if 終わり {
                break;
            }
        }
        届いた
    });
    (場所, 机)
}

fn 頼み() -> MailSendArgs {
    MailSendArgs {
        body: "確かめました".to_owned(),
        reply_to: None,
        to: Some(vec!["you@example.com".to_owned()]),
        subject: Some("見積".to_owned()),
        from: None,
        seconds: Some(1),
        send_again: false,
    }
}

#[tokio::test]
async fn 送った返りには_机へ渡したのと同じ印が付く() {
    let (場所, 机) = 机を立てる("mail-id-new", false).await;
    let 口 = 用意()
        .この機械につながる_控えは(&場所, &試験の控え("mail-id-new"))
        .await
        .expect("つながれること");
    let 返り = 口.mail_send(Parameters(頼み())).await.expect("返ること");
    let 届いた = 机.await.unwrap();
    assert_eq!(届いた.len(), 1);
    let ToDesk::メールを送る {
        通の印: Some(印),
    ..
    } = ToDesk::読む(&届いた[0]).unwrap()
    else {
        panic!("印が机へ渡っていない: {}", 届いた[0]);
    };
    assert!(warifu_desk::通の印として読めるか(&印), "{印}");
    assert!(返り.contains(&format!("Message-ID {印}")), "{返り}");
    // 送信済みを読む手の無い口なので、二重送信を確かめられなかったことを黙らずに添える（0.1.60）
    assert!(返り.contains("送信済みを確かめられませんでした"), "{返り}");
}

#[tokio::test]
async fn 古い画面は印を外して頼み直し_印は分からないと言う() {
    let (場所, 机) = 机を立てる("mail-id-old", true).await;
    let 口 = 用意()
        .この機械につながる_控えは(&場所, &試験の控え("mail-id-old"))
        .await
        .expect("つながれること");
    let 返り = 口.mail_send(Parameters(頼み())).await.expect("返ること");
    let 届いた = 机.await.unwrap();
    assert_eq!(届いた.len(), 2, "{届いた:?}");
    assert!(!届いた[1].contains("通の印"), "{}", 届いた[1]);
    assert!(返り.contains("送りました"), "{返り}");
    assert!(返り.contains("分かりません"), "{返り}");
}
