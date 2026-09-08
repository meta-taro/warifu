//! **預かり所。**相手が起動していない間、封を預ける／留守中の分を受け取る。
//!
//! ```text
//!   相手が居る    画面 ─────────────▶ 相手      その場で届く
//!   相手が居ない  画面 ──封──▶ 預かり所        相手が起動したときに届く
//! ```
//!
//! # ここで守ること
//!
//! - **預かり所は任意である**（`docs/relay.md`）。置いていなければ、
//!   これまでどおり「相手が起動している間だけ届く」。**黙って中央へ繋ぎに行かない**（D68）
//! - **宛先は人が書く。**割符が拾ってこない
//! - **留守中に届いた分は、そう分かる形で出す。**届いた時刻は
//!   **出した側の時計**であって、こちらの時計ではない（D71 / `letter.rs`）

use std::str::FromStr as _;

use tauri::State;
use warifu_core::PublicKey;
use warifu_net::Address;
use warifu_vault::Vault;

use crate::{Answer, Bridge, Failure, key_to_string};

/// 置いてある宛先を読む。**置いていなければ `None`。**
///
/// 読めなくても止めない。**預かり所が無くても、割符は動く。**
pub fn 宛先を読む() -> Option<String> {
    match Vault::default_location().and_then(|v| v.postbox()) {
        Ok(宛先) => 宛先,
        Err(e) => {
            記録!("預かり所の宛先を読めませんでした（置いていないものとして進みます）: {e}");
            None
        }
    }
}

/// 宛先を、繋げる形に直す。
fn 宛先() -> Option<Address> {
    Address::from_str(&宛先を読む()?).ok()
}

/// **預かり所に預ける。**相手が起動していなくても、次に起動したときに届く。
///
/// # Errors
/// 預かり所を置いていないとき、繋がらないとき、断られたとき。
pub async fn 預ける(bridge: &Bridge, 相手: PublicKey, 本文: &str) -> Answer<()> {
    let Some(所) = 宛先() else {
        return Err(Failure {
            message: "いま居ません".into(),
            code: Some("contact.unreachable".into()),
        });
    };
    let node = bridge.node().await?;
    warifu_postbox::預ける(
        &node,
        &所,
        &bridge.device,
        相手,
        いま時刻(),
        本文.as_bytes(),
    )
    .await
    .map_err(|e| {
        記録!("預かり所へ預けられませんでした: {e}");
        Failure {
            // **預かり所の言い分をそのまま出さない。**押した人に読めない
            message: "預かり所へ預けられませんでした".into(),
            code: Some("postbox.failed".into()),
        }
    })?;
    記録!("預けました: {} バイトを 1 通", 本文.len());
    Ok(())
}

/// **留守中の分を受け取る。**開けて、誰が言ったかを確かめたものだけを返す。
///
/// 署名の合わない手紙は `warifu-postbox` が捨てる。**名乗りだけでは通らない**（D72）。
///
/// **返す形にしてある。**知らせ（`emit`）で渡していたときは、
/// **画面が聞き始める前に渡してしまう**ことがあった ——
/// 預かり所は渡したら手放すので、**そのまま消える**（2026-09-08 に実物で踏んだ）。
async fn 取りに行く(bridge: &Bridge) -> Vec<(String, String, u64)> {
    let Some(所) = 宛先() else { return Vec::new() };
    let Ok(node) = bridge.node().await else {
        return Vec::new();
    };
    let 手紙たち = match warifu_postbox::受け取る(&node, &bridge.device, &所).await {
        Ok(手紙たち) => 手紙たち,
        Err(e) => {
            // **握り潰さない。**預かり所が落ちていることに気づけなくなる
            記録!("預かり所から受け取れませんでした: {e}");
            return Vec::new();
        }
    };
    if !手紙たち.is_empty() {
        記録!("留守中に届いていた分: {} 通", 手紙たち.len());
    }
    手紙たち
        .into_iter()
        .map(|手紙| {
            (
                key_to_string(手紙.差出人),
                String::from_utf8_lossy(&手紙.本文).to_string(),
                手紙.時刻,
            )
        })
        .collect()
}

/// いまの時刻（Unix 秒）。
fn いま時刻() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

// --- 画面から呼ぶ口 ---------------------------------------------------------

/// いま置いてある預かり所の宛先。**置いていなければ空。**
#[tauri::command]
pub async fn postbox() -> Answer<Option<String>> {
    Ok(宛先を読む())
}

/// 預かり所の宛先を置く。空にすると外れる。
///
/// **人が書く。**割符が拾ってこない（D71）。
#[tauri::command]
pub async fn set_postbox(address: Option<String>) -> Answer<()> {
    let 入力 = address.as_deref().map(str::trim).filter(|a| !a.is_empty());
    // **繋がる形かだけは、置く前に見る。**置いてから毎回失敗するより早く分かる
    if let Some(a) = 入力 {
        if Address::from_str(a).is_err() {
            return Err(Failure {
                message: "預かり所の宛先として読めません".into(),
                code: Some("postbox.malformed".into()),
            });
        }
    }
    Vault::default_location()
        .and_then(|v| v.save_postbox(入力))
        .map_err(|e| Failure {
            message: e.to_string(),
            code: None,
        })?;
    記録!(
        "預かり所の宛先を{}",
        if 入力.is_some() {
            "置きました"
        } else {
            "外しました"
        }
    );
    Ok(())
}

/// **留守中の分を取りに行く。**開けた 1 通ずつを `[公開鍵, 中身, 出した側の時刻]` で返す。
///
/// **画面が呼ぶ。**呼ばれた分だけ取りに行く ——
/// 預かり所は渡したら手放すので、**受け取る側が構えてから**取りに行く。
#[tauri::command]
pub async fn fetch_postbox(bridge: State<'_, Bridge>) -> Answer<Vec<(String, String, u64)>> {
    Ok(取りに行く(&bridge).await)
}
