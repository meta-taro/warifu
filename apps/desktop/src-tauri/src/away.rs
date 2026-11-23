//! **画面の前に居ない人の承認**（0.1.55）。
//!
//! 札・メールの送信・入れ替えは、割符の画面で人が押す形だった。人が外に居る（Claude Code を
//! スマホのリモートで見ているだけ）と、帯は押されず運用の席が止まった。
//!
//! 人が画面で一度「画面の前に居ないときは、エージェントの道具の許可で通す」を入れた機械では、
//! エージェントの道具（Claude Code）が許可を求めるときに出す中身を、人の承認として扱う。
//! 人はリモートでもその確認を見て許す。**道具の側で「いつも許す」にすると、この承認は無いのと同じ**
//! なので、画面・口の説明・文書でそう言う。
//!
//! 切り替えはエージェントのカーソルでは押せない（画面の印）。控えを読めなければ通さない側に倒す。

use tauri::{AppHandle, Emitter as _, Manager as _};
use warifu_desk::FromDesk;

use crate::{Answer, Failure};

/// 入れていない機械で、道具の許可の頼みを断るときの文。
pub(crate) const 入れていない訳: &str = "この PC では、画面の前に居ないときの承認（道具の許可で通す）が入っていません。\
     人が割符の画面で入れるか、画面の帯で押してもらってください";

/// **道具の許可で通すか**（人が画面で入れたときだけ）。控えを読めなければ通さない。
pub(crate) fn 通すか() -> bool {
    match warifu_vault::Vault::default_location().and_then(|v| v.terminal_approval()) {
        Ok(入れた) => 入れた,
        Err(e) => {
            記録!("画面の前に居ないときの承認の控えを読めませんでした（通しません）: {e}");
            false
        }
    }
}

/// 画面が読む。
#[tauri::command]
pub fn terminal_approval() -> bool {
    通すか()
}

/// **人が画面で、画面の前に居ないときの承認を入れた／切った。**エージェントのカーソルは押せない（画面の印）。
#[tauri::command]
pub fn set_terminal_approval(on: bool) -> Answer<()> {
    warifu_vault::Vault::default_location()
        .and_then(|v| v.save_terminal_approval(on))
        .map_err(|e| Failure {
            message: e.to_string(),
            code: None,
        })?;
    記録!(
        "画面の前に居ないときの承認を{}しました",
        if on { "入れ" } else { "切り" }
    );
    Ok(())
}

/// **新しい版を落として入れ替え、立て直す**（0.1.55）。
///
/// 呼ぶ側が承認を確かめてから呼ぶ（道具の許可で通す機械か、任せる相手の人の一言）。
/// 入れ替えを始めたら返りを返し、落とし終えたら立て直す（机の線は一度切れる）。
/// `.app` の外で動いているときは入れ替えない（`bundle.rs` の決まり。置き場所が消える）。
pub(crate) async fn 入れ替える(app: &AppHandle, 誰: &str) -> FromDesk {
    use tauri_plugin_updater::UpdaterExt as _;
    let いまの版 = env!("CARGO_PKG_VERSION").to_owned();
    let 返す =
        |新しい版: Option<String>, 始めた: bool, 訳: Option<String>| FromDesk::入れ替え {
            いまの版: いまの版.clone(),
            新しい版,
            始めた,
            訳,
        };
    if !crate::bundle::can_update() {
        return 返す(
            None,
            false,
            Some(".app の外で動いているので、入れ替えません".to_owned()),
        );
    }
    let 向こう = match app.updater() {
        Ok(u) => u.check().await,
        Err(e) => Err(e),
    };
    let 新しい = match 向こう {
        Ok(Some(新しい)) => 新しい,
        Ok(None) => return 返す(None, false, Some("新しい版はありません".to_owned())),
        Err(e) => {
            return 返す(
                None,
                false,
                Some(format!("向こうに尋ねられませんでした（{e}）")),
            );
        }
    };
    let 版 = 新しい.version.clone();
    記録!("入れ替え: {誰} の承認で {版} を落として入れ替えます");
    // 画面にも出す（人が戻ったとき、何が起きたか分かるように）
    let _ = app.emit(crate::EVENT_CHECK_UPDATE, ());
    let app = app.clone();
    let 誰 = 誰.to_owned();
    tauri::async_runtime::spawn(async move {
        match 新しい.download_and_install(|_, _| {}, || {}).await {
            Ok(()) => {
                記録!(
                    "入れ替え: {} を入れました。立て直します（{誰} の承認）",
                    新しい.version
                );
                app.restart();
            }
            Err(e) => {
                let 文 = format!("入れ替えられませんでした（{e}）");
                記録!("入れ替え: {文}");
                crate::desk::配る(
                    &app.state::<crate::Bridge>(),
                    crate::desk::聞いた("割符", &文),
                );
            }
        }
    });
    返す(Some(版), true, None)
}

/// **入れ替えの返りを、人とエージェントが読む 1 行にする。**
pub(crate) fn 入れ替えの言い方(返り: &FromDesk) -> String {
    match 返り {
        FromDesk::入れ替え {
            新しい版: Some(版),
            始めた: true,
            ..
        } => format!("{版} を落として入れ替えています。済んだら割符が立て直ります"),
        FromDesk::入れ替え { 訳: Some(訳), .. } => format!("入れ替えませんでした：{訳}"),
        _ => "入れ替えませんでした".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 入れ替えの言い方は_始めたか訳を言う() {
        let 始めた = FromDesk::入れ替え {
            いまの版: "0.1.55".into(),
            新しい版: Some("0.1.56".into()),
            始めた: true,
            訳: None,
        };
        assert!(入れ替えの言い方(&始めた).contains("0.1.56"));
        let 無い = FromDesk::入れ替え {
            いまの版: "0.1.55".into(),
            新しい版: None,
            始めた: false,
            訳: Some("新しい版はありません".into()),
        };
        assert_eq!(
            入れ替えの言い方(&無い),
            "入れ替えませんでした：新しい版はありません"
        );
    }
}
