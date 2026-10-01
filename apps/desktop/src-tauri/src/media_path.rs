//! **映像の道**（**D125**）—— WebRTC の映像を、割符の経路（iroh）で運ぶ。
//!
//! ```text
//! WebRTC ─ 機械の中の TURN（127.0.0.1）─ ここ ─ 包みの口（iroh の datagram）─ 相手のアプリ
//! ```
//!
//! WebRTC は iroh と別の口を使い、その口は WebView が決める。ルーターが開けてくれた
//! iroh の口を映像は通らないので、**外の回線を越えられない**（2026-09-28 に確かめた前提）。
//! 外部の STUN / TURN は使わない（D13 / D124）ので、アプリの中に TURN を置いて、
//! **中身は iroh が越えた道で運ぶ。**
//!
//! 相手ごとに「見せかけの番地 → 包みの口」を控える。見せかけの番地は相手の公開鍵から
//! 決まる（`warifu_turn::見せかけの番地`）ので、宛先から相手が引ける。

use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, LazyLock, Mutex, OnceLock};

use warifu_net::包みの口;
use warifu_turn::{交換所, 運び手};

/// 相手の見せかけの番地 → その相手への包みの口（と、入れ替わりを見分ける番号）。
static 道たち: LazyLock<Mutex<HashMap<IpAddr, (u64, 包みの口)>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// 運ばれてきた包みを、機械の中の TURN の中継口へ配る所。
static 配り場: LazyLock<交換所> = LazyLock::new(交換所::default);

/// 建てた TURN（画面の `iceServers` に渡すもの）。**建てられなかったら空のまま。**
static 建てたTURN: OnceLock<TURNの在り処> = OnceLock::new();

/// 道を足すたびに増やす。**古い道の後片付けが、新しい道を消さない**ため（D83 と同じ構え）。
static 次の番号: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

/// 画面の `iceServers` に渡す 1 件。**合言葉は起動ごとに変わり、この機械の外に出ない。**
#[derive(Debug, Clone, serde::Serialize)]
pub struct TURNの在り処 {
    pub url: String,
    pub username: String,
    pub credential: String,
}

fn 道を引く<T>(f: impl FnOnce(&mut HashMap<IpAddr, (u64, 包みの口)>) -> T) -> T {
    f(&mut 道たち
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner))
}

/// iroh の包みの口で運ぶ運び手。
struct Iroh運び手;

impl 運び手 for Iroh運び手 {
    fn 運ぶ(&self, 宛: SocketAddr, 元: SocketAddr, 中身: &[u8]) {
        let Some(口) = 道を引く(|道| 道.get(&宛.ip()).map(|(_, 口)| 口.clone())) else {
            // **相手の道が無い**（まだ繋がっていない・抜けた）。映像は落としてよい
            return;
        };
        let Some(包み) = warifu_turn::包む(宛, 元, 中身) else {
            return;
        };
        if 口.最大の大きさ().is_some_and(|最大| 包み.len() > 最大) {
            // **大きすぎる包みは送れない。**黙って落とすと映像が出ない理由が分からないので書く
            記録!(
                "映像の道: 包みが大きすぎて送れません（{} バイト・上限 {:?}）",
                包み.len(),
                口.最大の大きさ()
            );
            return;
        }
        // 送れなければ落とす（経路が閉じた・溢れた）。**映像は遅れて届くより落ちたほうがよい**
        let _ = 口.送る(包み);
    }
}

/// **機械の中の TURN を建てる**（起動時に 1 回）。
pub async fn 立てる(自分の鍵: [u8; 32]) {
    match warifu_turn::立てる(&自分の鍵, Arc::new(Iroh運び手), 配り場.clone()).await {
        Ok(turn) => {
            記録!("映像の道: 機械の中の TURN を建てました（{}）", turn.url());
            let _ = 建てたTURN.set(TURNの在り処 {
                url: turn.url(),
                username: turn.名前.clone(),
                credential: turn.合言葉.clone(),
            });
            // **閉じない。**アプリが終わるまで持つ（持ち手を落とすと TURN が止まる）
            std::mem::forget(turn);
        }
        Err(e) => 記録!(
            "映像の道: 機械の中の TURN を建てられません（{e}）。映像は同じ網の中だけになります"
        ),
    }
}

/// 建てた TURN の在り処。**建てられていなければ `None`**（画面は同じ網の中だけで繋ぐ）。
#[must_use]
pub fn 在り処() -> Option<TURNの在り処> {
    建てたTURN.get().cloned()
}

/// **つながった相手への道を足す。**`Channel::new(session)` の直前に呼ぶ。
///
/// 相手から届く包みを受け続け、機械の中の TURN へ配る。経路が閉じたら道を外す。
pub fn 足す(口: 包みの口) {
    let 番地 = warifu_turn::見せかけの番地(&口.相手().to_bytes());
    let 番号 = 次の番号.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    道を引く(|道| 道.insert(番地, (番号, 口.clone())));
    tauri::async_runtime::spawn(async move {
        while let Ok(包み) = 口.受ける().await {
            if let Some((宛, 元, 中身)) = warifu_turn::ほどく(&包み) {
                配り場.届いた(宛, 元, 中身.to_vec());
            }
        }
        // **自分が置いた道だけを外す**（入り直した新しい道を消さない）
        道を引く(|道| {
            if 道.get(&番地).is_some_and(|(n, _)| *n == 番号) {
                道.remove(&番地);
            }
        });
    });
}
