//! **プロフィール。**この端末の人と、この端末の AI が名乗るもの。
//!
//! オーナー指示（2026-09-08）——
//! 「**このPCのAIもまた一人の人としてプロフィールを持てるようにします。
//!   エージェントも人です。その管理はこのPCの持ち主が編集可能とします。**」
//!
//! # 書き換えられるのは、この端末の持ち主だけ
//!
//! **AI が自分の名前を書き換える口は作らない。**
//! 作ると、**同じ机の別のエージェントに化けられる**（`chat_send` は席の名乗りを載せるので、
//! 名乗りを自由に変えられると、誰が言ったのかが崩れる）。
//! MCP の口（`warifu-mcp`）にプロフィールの書き換えは無い。**ここは画面だけ。**
//!
//! # 名乗りは本人確認にしない
//!
//! 相手が付けた呼び名があれば、**そちらが勝つ**（**D46**）。
//! 名乗った名前は誰でも真似できる。**確かめるのは鍵**である。

use tauri::{Emitter as _, State};
use warifu_vault::{Profile, Profiles, Vault, Who};

use crate::{Answer, Bridge, Failure};

/// 画面へ渡す 1 人ぶん。
#[derive(Debug, serde::Serialize)]
pub struct ProfileRow {
    /// 誰のものか（`me` か `desk:<名乗り>`）。**画面はこれで引く。**
    who: String,
    /// 名乗っている名前。**空なら名乗っていない。**
    name: String,
    /// 短い紹介。
    bio: String,
    /// 差し替えた顔（置き場所のファイル名）。**空なら鍵から描く。**
    avatar: Option<String>,
}

fn 読む() -> Result<Profiles, Failure> {
    Vault::default_location()
        .and_then(|v| v.profiles())
        .map_err(|e| Failure {
            message: e.to_string(),
            code: None,
        })
}

/// この端末のプロフィールを並べる。
#[tauri::command]
pub async fn profiles() -> Answer<Vec<ProfileRow>> {
    let 面々 = 読む()?;
    Ok(面々
        .iter()
        .map(|p| ProfileRow {
            who: p.who().to_field(),
            name: p.name().to_owned(),
            bio: p.bio().to_owned(),
            avatar: p.avatar().map(str::to_owned),
        })
        .collect())
}

/// プロフィールを書く。**名前も紹介も空にすると、その 1 人ぶんを消す。**
///
/// `who` は `me` か `desk:<名乗り>`。
#[tauri::command]
pub async fn set_profile(
    bridge: State<'_, Bridge>,
    who: String,
    name: String,
    bio: String,
) -> Answer<()> {
    let Some(誰) = Who::from_field(&who) else {
        return Err(Failure {
            message: "誰のプロフィールか読めません".into(),
            code: None,
        });
    };
    let mut 面々 = 読む()?;

    if name.trim().is_empty() && bio.trim().is_empty() {
        // **空にしたら消す。**空の行を残すと、名乗っていないのに欄だけが残る
        面々.forget(&誰);
    } else {
        let p = Profile::new(誰, &name, &bio).map_err(|e| Failure {
            message: e.to_string(),
            code: Some("profile.bad".into()),
        })?;
        面々.put(p);
    }

    Vault::default_location()
        .and_then(|v| v.save_profiles(&面々))
        .map_err(|e| Failure {
            message: e.to_string(),
            code: None,
        })?;
    記録!("プロフィールを書きました（{who}）");
    // **名乗りが変わったことを、いま繋がっている相手へも伝える。**
    // 伝えないと、相手の画面は前の名前のままになる
    crate::profile::配る(&bridge).await;
    Ok(())
}

/// **机に着いたエージェントが、自分の席のプロフィールを書く。**
///
/// オーナー指示（2026-09-08）——
/// 「**エージェントが MCP で接続できたらエージェント自身に
/// プロフィールをかけるようにしておくと、楽だとおもいます。**」
///
/// **書けるのは自分の席だけ。**どの席かは**繋いできた口で決まる**（`desk.rs`）ので、
/// ここへ「誰の」は渡ってこない。
///
/// **名乗り（どこで動いているか）は変えられない。**あれは立ち上げるときに人が決める。
///
/// # Errors
/// 上限を超えたとき、置き場所へ書けなかったとき。
pub fn 席の名乗りを書く(
    app: &tauri::AppHandle,
    名乗り: &str,
    名前: &str,
    紹介: &str,
) -> Result<(), String> {
    let 誰 = Who::Desk(crate::desk::呼び方(Some(名乗り)));
    let mut 面々 = Vault::default_location()
        .and_then(|v| v.profiles())
        .map_err(|e| e.to_string())?;

    if 名前.trim().is_empty() && 紹介.trim().is_empty() {
        面々.forget(&誰);
    } else {
        面々.put(Profile::new(誰, 名前, 紹介).map_err(|e| e.to_string())?);
    }

    Vault::default_location()
        .and_then(|v| v.save_profiles(&面々))
        .map_err(|e| e.to_string())?;
    記録!("プロフィールを書きました（席: {名乗り}）");
    // **画面にもすぐ出す。**出さないと、書けたのかどうかが人から見えない
    let _ = app.emit(crate::EVENT_PROFILES, ());
    Ok(())
}

/// いまの名乗りを、繋がっている相手へ配る。
///
/// **相手が付けた呼び名は上書きしない**（D46）。相手の画面では
/// 「本人の名乗り」として扱われる。
pub async fn 配る(_bridge: &Bridge) {
    // **まだ線には流していない。**次の段で `Notice` に載せる（`issues/016`）。
    // ここを黙って空のままにしない —— 呼び出し側は「配った」と思うため、
    // 記録に残して、実装が入っていないことを読めるようにしておく。
    記録!("プロフィール: 相手へ配る所はまだありません（手元だけ）");
}
