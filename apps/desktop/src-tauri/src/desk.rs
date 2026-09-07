//! 机 —— **同じ PC のエージェントが、人と同じ会話に着く口。**
//!
//! ```text
//!   人（この画面）──┐
//!                   ├── 同じ会話 ── iroh P2P ── 相手の PC
//!   AI（warifu mcp）─┘
//!         ↑ ここが机（同じ機械の中だけ。網には出ない）
//! ```
//!
//! **これが「エージェント同士でチャットし、人がそれを見て入れる」の実体。**
//! エージェントに `warifu join` を叩かせる形は、実際に許可プロンプトで
//! 3 回とも止まった（2026-09-06・Mac Air の報告）。MCP の口なら止まらない。
//!
//! # ここで守ること
//!
//! - **繋いできた側は差出人を名乗れない。**誰が言ったかはここで刻む
//! - **机は同じ機械の中だけ。**口の権限は `warifu-desk` が持つ（Unix は 0600）
//! - **届いた文字は相手の言い分。**ここは運ぶだけで、解釈しない

use std::sync::atomic::{AtomicU64, Ordering};

use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::broadcast;
use warifu_desk::{FromDesk, ToDesk, 受け口, 口, 机の場所};

use crate::{Bridge, EVENT_DESK, EVENT_DESK_SEATS, key_to_string};

/// 机に着いている相手へ配る溜め。
///
/// **溜めきれない相手を待たない。**追いつけない客は取りこぼすが、
/// 会話そのものは止めない（止めると人の画面まで止まる）。
pub const 配る溜め: usize = 128;

/// 机に着いた人に振る番号。
///
/// **自分の発言を自分に返さないため**に要る。返すと、
/// エージェントは `chat_read` で自分が言ったことを「誰かの発言」として読む。
static 次の番号: AtomicU64 = AtomicU64::new(1);

/// 机の外から出た発言（相手から届いた文字・人が打った行）に使う番号。
///
/// **机の誰とも一致しない**ので、全員へ配られる。
pub const 机の外: u64 = 0;

/// いまの時刻（`HH:MM`）。**秒は出さない。**
///
/// 無いと、あとから読み返せない（2026-09-06 の「何時に投稿したかわからないです」）。
fn いま時刻() -> String {
    chrono::Local::now().format("%H:%M").to_string()
}

/// 机を開く。**開けなくても画面は止めない。**
///
/// 机が無くても人どうしの会話は成り立つ。
/// **開かなかったことは記録に残す**（黙って落とすと、
/// エージェントが繋がらない理由が誰にも分からない）。
pub fn 開く(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let 場所 = 机の場所();
        let mut 待ち = match 受け口::開く(&場所).await {
            Ok(待ち) => {
                記録!("机を開きました: {}", 場所.display());
                待ち
            }
            Err(e) => {
                記録!("机を開けませんでした（{}）: {e}", 場所.display());
                return;
            }
        };
        loop {
            match 待ち.受ける().await {
                Ok(一本) => {
                    記録!("机に 1 人着きました");
                    座らせる(app.clone(), 一本);
                }
                Err(e) => {
                    記録!("机の口が壊れました: {e}");
                    return;
                }
            }
        }
    });
}

/// 机に着いた 1 人の面倒を見る。
fn 座らせる(app: AppHandle, 一本: impl warifu_desk::一本) {
    tauri::async_runtime::spawn(async move {
        let 自分 = 次の番号.fetch_add(1, Ordering::Relaxed);
        let mut 口 = 口::新しく(一本);
        let mut 聞く = app.state::<Bridge>().desk.subscribe();
        // **人数が変わったことを画面へ伝える。**伝えないと、
        // AI が居るのに「入ってきたら送れます」と出たままになる
        席の数を伝える(&app);
        loop {
            tokio::select! {
                来た = 口.受ける() => {
                    let Ok(Some(行)) = 来た else { break };
                    if 応じる(&app, &行, &mut 口, 自分).await.is_err() {
                        break;
                    }
                }
                配られた = 聞く.recv() => {
                    match 配られた {
                        // **自分が言ったことは自分に返さない。**返すと、
                        // エージェントは自分の発言を「誰かの発言」として読む
                        Ok((出所, _)) if 出所 == 自分 => continue,
                        Ok((_, 一つ)) => {
                            if 口.送る(&一つ.書く()).await.is_err() {
                                break;
                            }
                        }
                        // 追いつけずに落ちた分がある。**会話は止めない**
                        Err(broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(broadcast::error::RecvError::Closed) => break,
                    }
                }
            }
        }
        記録!("机から 1 人抜けました");
        // subscribe を落としてから数える。**落とす前だと自分を数えてしまう**
        drop(聞く);
        席の数を伝える(&app);
    });
}

/// 机に何人着いているかを、画面へ伝える。
///
/// **「相手が居ない」と「話し相手が 1 人も居ない」は違う。**
/// 会議に人が居なくても、同じ席の AI が居るなら人は話しかけられる。
pub fn 席の数を伝える(app: &AppHandle) {
    let 数 = app.state::<Bridge>().desk.receiver_count();
    let _ = app.emit(EVENT_DESK_SEATS, 数);
}

/// いま机に何人着いているか。
#[must_use]
pub fn 席の数(bridge: &Bridge) -> usize {
    bridge.desk.receiver_count()
}

/// 机から来た 1 行に応じる。
async fn 応じる(
    app: &AppHandle,
    行: &str,
    口: &mut 口<impl warifu_desk::一本>,
    自分: u64,
) -> Result<(), ()> {
    let 中身 = match ToDesk::読む(行) {
        Ok(中身) => 中身,
        Err(e) => {
            // **読めなかったことを、断ったことと混ぜない**
            let _ = 口
                .送る(&FromDesk::Denied { why: e.to_string() }.書く())
                .await;
            return Ok(());
        }
    };
    match 中身 {
        // 「聞く」は名乗りだけ。**過去は残していない**ので、返すものが無い（`issues/010`）
        ToDesk::Listen => Ok(()),
        ToDesk::Say { body } => {
            // **必ず返事をする。**返さないと、言った側は待ち続ける（**D49**）
            let 返事 = match 流す(app, &body, 自分).await {
                Ok(人数) => FromDesk::Sent { to: 人数 },
                Err(訳) => {
                    記録!("机からの発言を流せませんでした: {訳}");
                    // 相手が居ないのと、会議そのものが無いのは違う。**混ぜない**
                    if 訳 == 誰も居ない {
                        FromDesk::Nobody
                    } else {
                        FromDesk::Denied { why: 訳 }
                    }
                }
            };
            let _ = 口.送る(&返事.書く()).await;
            Ok(())
        }
    }
}

/// 相手が 1 人も居ないときの言い分。**判定に使うので、1 か所に置く。**
const 誰も居ない: &str = "まだ誰も居ません";

/// 机から来た発言を、会話へ流す。**流した人数を返す。**
///
/// **人の画面にも、繋がっている相手にも、同じ 1 行が出る。**
/// 片方にしか出ないと、見ている人と話している人がずれる。
async fn 流す(app: &AppHandle, body: &str, 出所: u64) -> Result<usize, String> {
    let bridge = app.state::<Bridge>();
    let meeting = {
        let slot = bridge.conference.lock().await;
        slot.as_ref().map(warifu_app::Conference::id)
    };
    let Some(meeting) = meeting else {
        return Err("まだ会議がありません".to_owned());
    };
    let 自分 = bridge.device.public_key();

    // 先に人の画面へ出す。**相手が居なくても、この PC の人には見える**
    let _ = app.emit(
        EVENT_DESK,
        (key_to_string(自分), body.to_owned(), いま時刻()),
    );
    // **同じ席の AI が喋ったことにも気づけるようにする。**
    // 窓が前に居るときは鳴らないので、うるさくならない
    crate::notify::届いたと知らせる(app, "この PC の AI");
    // 机に着いている**他の**エージェントにも同じ行を見せる（言った本人には返さない）
    let _ = bridge.desk.send((
        出所,
        FromDesk::Heard {
            from: key_to_string(自分),
            body: body.to_owned(),
            at: いま時刻(),
        },
    ));

    let out = bridge.outbound.lock().await;
    if out.is_empty() {
        return Err(誰も居ない.to_owned());
    }
    let 人数 = out.len();
    for tx in out.values() {
        // 届かない相手が居ても止めない。**送る側を待たせない**
        let _ = tx
            .send(warifu_meeting::Notice::Text {
                meeting,
                // **自分が言ったと載せる**（D48）
                from: 自分,
                body: body.to_owned(),
            })
            .await;
    }
    Ok(人数)
}

/// 会話に出たことを、机に着いている全員へ配る。
///
/// **人が打った行も、相手から届いた行も、ここを通す。**
/// 通さないと、エージェントは人の発言が見えないまま返事をすることになる。
pub fn 配る(bridge: &Bridge, 中身: FromDesk) {
    // 誰も着いていなければ落ちる。**それは失敗ではない**
    let _ = bridge.desk.send((机の外, 中身));
}

/// 会話に出た文字を、机へ配る形にする。
#[must_use]
pub fn 聞いた(from: &str, body: &str) -> FromDesk {
    FromDesk::Heard {
        from: from.to_owned(),
        body: body.to_owned(),
        at: いま時刻(),
    }
}
