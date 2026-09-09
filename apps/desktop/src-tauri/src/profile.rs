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

/// **顔を差し替える。**人が選んだ画像を、置き場所へ写して指す。
///
/// オーナー指示（2026-09-08）——「**ただユーザによって差し替え可能にもします。**」
///
/// **受け取ったファイルを、そのまま信じない。**
/// 拡張子ではなく中身の頭を見て、大きさと縦横に上限を置く（`warifu-vault` の `顔として読む`）。
///
/// **外の場所を指させない。**置き場所（`avatars/`）へ写してから指す ——
/// 指したままにすると、**消えた・入れ替わったファイル**を指すことになる。
#[tauri::command]
pub async fn set_avatar(app: tauri::AppHandle, who: String, path: String) -> Answer<()> {
    let Some(誰) = Who::from_field(&who) else {
        return Err(Failure {
            message: "誰のプロフィールか読めません".into(),
            code: None,
        });
    };
    let 中身 = std::fs::read(&path).map_err(|e| Failure {
        message: format!("画像を読めませんでした（{e}）"),
        code: Some("avatar.unreadable".into()),
    })?;
    // **人に読める形で断る。**なぜ置けないかが分からないと、次の手が打てない
    let (幅, 高さ) = warifu_vault::顔として読む(&中身).map_err(|e| Failure {
        message: e.to_string(),
        code: Some("avatar.bad".into()),
    })?;

    let vault = Vault::default_location().map_err(|e| Failure {
        message: e.to_string(),
        code: None,
    })?;
    let 置き場 = vault.avatars_dir();
    std::fs::create_dir_all(&置き場).map_err(|e| Failure {
        message: format!("置き場所を作れませんでした（{e}）"),
        code: None,
    })?;
    let 名 = warifu_vault::顔のファイル名(&誰);
    std::fs::write(置き場.join(&名), &中身).map_err(|e| Failure {
        message: format!("画像を置けませんでした（{e}）"),
        code: None,
    })?;

    let mut 面々 = 読む()?;
    // **名乗りがまだ無くても顔は置ける。**空のプロフィールを作って指す
    let mut p = 面々
        .find(&誰)
        .cloned()
        .unwrap_or(Profile::new(誰.clone(), "", "").map_err(|e| Failure {
            message: e.to_string(),
            code: None,
        })?);
    p.set_avatar(Some(&名)).map_err(|e| Failure {
        message: e.to_string(),
        code: None,
    })?;
    面々.put(p);
    vault.save_profiles(&面々).map_err(|e| Failure {
        message: e.to_string(),
        code: None,
    })?;

    記録!(
        "顔を差し替えました（{who} / {幅}×{高さ} / {} バイト）",
        中身.len()
    );
    let _ = app.emit(crate::EVENT_PROFILES, ());
    Ok(())
}

/// **顔を既定へ戻す。**置いた画像も消す。
#[tauri::command]
pub async fn clear_avatar(app: tauri::AppHandle, who: String) -> Answer<()> {
    let Some(誰) = Who::from_field(&who) else {
        return Err(Failure {
            message: "誰のプロフィールか読めません".into(),
            code: None,
        });
    };
    let vault = Vault::default_location().map_err(|e| Failure {
        message: e.to_string(),
        code: None,
    })?;
    // **置いた物も消す。**指すのをやめただけだと、消したつもりの画像が残る
    let _ = std::fs::remove_file(vault.avatars_dir().join(warifu_vault::顔のファイル名(&誰)));

    let mut 面々 = 読む()?;
    if let Some(p) = 面々.find(&誰).cloned() {
        let mut p = p;
        let _ = p.set_avatar(None);
        面々.put(p);
        vault.save_profiles(&面々).map_err(|e| Failure {
            message: e.to_string(),
            code: None,
        })?;
    }
    記録!("顔を既定へ戻しました（{who}）");
    let _ = app.emit(crate::EVENT_PROFILES, ());
    Ok(())
}

/// 置いてある顔の中身。**無ければ `None`。**
///
/// **画面へバイト列で渡す。**外の場所を画面に触らせない
/// （置き場所は 0700 で、画面から辿らせるものではない）。
#[tauri::command]
pub async fn avatar_bytes(who: String) -> Answer<Option<Vec<u8>>> {
    let Some(誰) = Who::from_field(&who) else {
        return Ok(None);
    };
    let Ok(vault) = Vault::default_location() else {
        return Ok(None);
    };
    let 面々 = 読む()?;
    let Some(名) = 面々.find(&誰).and_then(|p| p.avatar().map(str::to_owned)) else {
        return Ok(None);
    };
    // **指している名前を信じない。**置き場所の中の、決まった名前だけを読む
    if 名 != warifu_vault::顔のファイル名(&誰) {
        記録!("顔の指し先が置き場所の名前と違います（読みません）");
        return Ok(None);
    }
    Ok(std::fs::read(vault.avatars_dir().join(&名)).ok())
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

/// いまの名乗り（この端末の人のぶん）を、繋がっている相手へ配る。
///
/// **相手が付けた呼び名は上書きしない**（**D46**）。相手の画面では
/// 「本人の名乗り」として扱われる。
///
/// **空も配る。**消したことが伝わらないと、相手の画面に前の名前が残る。
pub async fn 配る(bridge: &Bridge) {
    let (名前, 紹介) = match 読む() {
        Ok(面々) => match 面々.find(&Who::Me) {
            Some(p) => (p.name().to_owned(), p.bio().to_owned()),
            // **名乗りを消した。**空を配って、相手の画面からも消す
            None => (String::new(), String::new()),
        },
        Err(e) => {
            記録!("名乗りを読めませんでした（配りません）: {}", e.message);
            return;
        }
    };

    let Some(meeting) = crate::いま見ている部屋(&bridge.いまの部屋).await else {
        return;
    };
    let 送り先 = crate::その部屋の相手(
        &bridge.conferences,
        &bridge.outbound,
        meeting,
        bridge.device.public_key(),
    )
    .await;
    if 送り先.is_empty() {
        return;
    }
    記録!("名乗りを配ります（{} 人へ）", 送り先.len());
    for tx in &送り先 {
        // 届かない相手が居ても止めない。**送る側を待たせない**
        let _ = tx
            .send(warifu_meeting::Notice::Profile {
                meeting,
                from: bridge.device.public_key(),
                名前: 名前.clone(),
                紹介: 紹介.clone(),
            })
            .await;
    }
}

/// **その席の名乗り**を、線に載せる形にする。
///
/// 名乗っていれば `図面くん（zumen）`、名乗っていなければ席そのまま。
/// **どこの席かを落とさない** —— 落とすと、相手の画面で取り違えられる（**D75**）。
#[must_use]
pub fn 席の名札(呼び方: &str) -> String {
    let Ok(面々) = 読む() else {
        return 呼び方.to_owned();
    };
    let 名前 = 面々.find(&Who::Desk(呼び方.to_owned())).map(|p| p.name().to_owned());
    名札にする(呼び方, 名前.as_deref())
}

/// 名乗りと名前から、**画面に出す 1 つの呼び名**を作る。
///
/// **名前の決め方を 1 か所に置く。**2 か所に置くと、
/// **会話と名簿で同じ席が別の名前で出る**（2026-09-09 に実物で見た ——
/// 名簿は「かくにん係」、会話は「kakunin のエージェント」だった）。
#[must_use]
pub fn 名札にする(呼び方: &str, 名前: Option<&str>) -> String {
    let 場所 = 呼び方.strip_suffix(" のエージェント").unwrap_or(呼び方);
    match 名前 {
        Some(名前) if !名前.is_empty() => format!("{名前}（{場所}）"),
        _ => 呼び方.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::名札にする;

    #[test]
    fn 名乗っていれば名前とどこの席かを並べる() {
        assert_eq!(
            名札にする("zumen のエージェント", Some("図面くん")),
            "図面くん（zumen）"
        );
    }

    #[test]
    fn 名乗っていなければ席そのままにする() {
        assert_eq!(名札にする("zumen のエージェント", None), "zumen のエージェント");
    }

    #[test]
    fn 名前が空なら席そのままにする() {
        assert_eq!(名札にする("zumen のエージェント", Some("")), "zumen のエージェント");
    }

    #[test]
    fn どこの席かは落とさない() {
        // 落とすと、**相手の画面で取り違えられる**（D75）
        let 名札 = 名札にする("git-qa のエージェント", Some("図面くん"));
        assert!(名札.contains("git-qa"), "{名札}");
    }
}
