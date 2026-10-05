//! **エージェントの声**（#50）。
//!
//! ```text
//!   エージェント ── voice_say ── 机（desk.rs）── ここで読み上げを作る（この PC の中だけ）
//!                                                   │ EVENT_AGENT_VOICE（番号と文）
//!                                                   ▼
//!                                         画面が音を取りに来て、会議の音の送り手へ差し替える
//!                                                   │ agent_voice_report（始めた／終えた）
//!                                                   ▼
//!                                         始めたら会話へ「（エージェントの声）…」を流す
//! ```
//!
//! # ここで守ること
//!
//! - **読み上げは OS の機能で作る。**外の読み上げサービスへ文を送らない
//! - **文をシェルに通さない。**macOS は標準入力、Windows は環境変数で渡す
//!   （命令の文字列へつなげると、文の中身が命令として読まれうる）
//! - **人がルームごとに入れた印が無ければ、作りもしない**（既定は切）

use std::collections::BTreeMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;
use warifu_desk::声の結果;

use crate::{Answer, Failure};

/// **声を鳴らしてほしい**（`[番号, 文]`）。画面は番号で音を取りに来る。
pub const EVENT_AGENT_VOICE: &str = "warifu://agent-voice";

/// 会話へ流す行の頭。**声で言ったことが、人の発言と見分けられるように付ける。**
pub const 会話の頭: &str = "（エージェントの声）";

/// **この PC の人が、いま見ているルームで声を入れたか。**
///
/// 画面が入切とルームの移り変わりを置きに来る（ルームを抜けたら切に戻す）。
/// **既定は切。**人が入れていないのに音を出さない。
static 声を流してよい: AtomicBool = AtomicBool::new(false);

/// 声ごとに振る番号。画面が音を取りに来る・結果を返すのに使う。
static 次の声: AtomicU64 = AtomicU64::new(1);

/// 画面へ渡す前の声。**取りに来たら手放す**（同じ音を 2 度渡さない）。
static 声の棚: Mutex<BTreeMap<u64, 預けた声>> = Mutex::new(BTreeMap::new());

struct 預けた声 {
    音: Option<Vec<u8>>,
    知らせ: mpsc::UnboundedSender<画面の報せ>,
}

/// 画面から届く報せ。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum 画面の報せ {
    /// 鳴らし始めた（会話へ行を流す合図）。
    始めた,
    /// 鳴らし終えた、または鳴らさなかった。
    終えた(声の結果),
}

/// **読み上げを作る手**。試験では偽物に差し替える。
pub trait 声を作る: Send + Sync {
    /// 文を WAV のバイト列にする。
    ///
    /// # Errors
    /// 作れなかったとき、人が読める訳を返す。
    fn 作る(&self, 本文: &str) -> Result<Vec<u8>, String>;
}

/// この PC の OS が持つ読み上げ。
pub struct この機械の声;

impl 声を作る for この機械の声 {
    fn 作る(&self, 本文: &str) -> Result<Vec<u8>, String> {
        os::作る(本文)
    }
}

/// **人の印を見てから作る。**印が無ければ作る手を呼ばない。
///
/// # Errors
/// 印が無い・作れない・WAV でないとき、そのまま返せる結果を返す。
pub fn 声を用意する(
    同意: bool,
    作り手: &dyn 声を作る,
    本文: &str,
) -> Result<Vec<u8>, 声の結果> {
    if !同意 {
        return Err(声の結果::許されていない);
    }
    let 音 = 作り手.作る(本文).map_err(|訳| 声の結果::失敗 { 訳 })?;
    if !wavか(&音) {
        return Err(声の結果::失敗 {
            訳: "読み上げが WAV になっていません".to_owned(),
        });
    }
    Ok(音)
}

/// RIFF / WAVE の頭を持つか。**中身が空の音を画面へ渡さない。**
fn wavか(音: &[u8]) -> bool {
    音.len() > 44 && 音.starts_with(b"RIFF") && 音.get(8..12) == Some(b"WAVE".as_slice())
}

/// 画面が送ってくる段の名を読む。**知らない名は受けない。**
#[must_use]
pub fn 報せを読む(段: &str, 訳: Option<String>) -> Option<画面の報せ> {
    let 終えた = |r| Some(画面の報せ::終えた(r));
    match 段 {
        "started" => Some(画面の報せ::始めた),
        "spoken" => 終えた(声の結果::流した),
        "not_allowed" => 終えた(声の結果::許されていない),
        "no_meeting" => 終えた(声の結果::会議が無い),
        "failed" => 終えた(声の結果::失敗 {
            訳: 訳.unwrap_or_else(|| "画面で鳴らせませんでした".to_owned()),
        }),
        _ => None,
    }
}

/// **終わりの報せを待つ。**始めたら一度だけ `始めたら` を呼ぶ。
///
/// 永遠には待たない —— 画面が黙ったまま止まると、エージェントの呼びが戻らない。
pub async fn 終わりを待つ(
    受け: &mut mpsc::UnboundedReceiver<画面の報せ>,
    上限: Duration,
    mut 始めたら: impl FnMut(),
) -> 声の結果 {
    let 期限 = tokio::time::Instant::now() + 上限;
    let mut 始めた = false;
    loop {
        match tokio::time::timeout_at(期限, 受け.recv()).await {
            Ok(Some(画面の報せ::始めた)) => {
                if !始めた {
                    始めた = true;
                    始めたら();
                }
            }
            Ok(Some(画面の報せ::終えた(結果))) => return 結果,
            Ok(None) => {
                return 声の結果::失敗 {
                    訳: "画面が閉じました".to_owned(),
                };
            }
            Err(_) => {
                return 声の結果::失敗 {
                    訳: "画面が鳴らし終えませんでした（時間切れ）".to_owned(),
                };
            }
        }
    }
}

/// **声で言う**（机から呼ぶ）。鳴らし終えるか、鳴らさないと決まるまで待つ。
///
/// `出所` と `呼び方` は、会話へ流す行の話し手に使う（どのエージェントの声か）。
pub async fn 言う(app: &AppHandle, 本文: &str, 出所: u64, 呼び方: String) -> 声の結果 {
    let 同意 = 声を流してよい.load(Ordering::SeqCst);
    let 文 = 本文.to_owned();
    // **作るのは別の糸で。**OS の読み上げは数秒かかり、その間ほかの口を止めない
    let 音 = match tokio::task::spawn_blocking(move || 声を用意する(同意, &この機械の声, &文)).await
    {
        Ok(Ok(音)) => 音,
        Ok(Err(結果)) => return 結果,
        Err(e) => {
            return 声の結果::失敗 { 訳: e.to_string() };
        }
    };

    let id = 次の声.fetch_add(1, Ordering::Relaxed);
    let (知らせ, mut 受け) = mpsc::unbounded_channel();
    声の棚.lock().expect("毒されていない").insert(
        id,
        預けた声 {
            音: Some(音),
            知らせ,
        },
    );
    // **文の中身は記録に書かない。**長さだけ
    記録!("声: 画面へ渡します（#{id}・{} 文字）", 本文.chars().count());
    if let Err(e) = app.emit(EVENT_AGENT_VOICE, (id, 本文)) {
        声の棚.lock().expect("毒されていない").remove(&id);
        return 声の結果::失敗 {
            訳: format!("画面へ渡せませんでした: {e}"),
        };
    }

    let 行 = format!("{会話の頭}{本文}");
    let 結果 = 終わりを待つ(
        &mut 受け,
        Duration::from_secs(warifu_desk::声を待てる秒),
        || {
            // **鳴り始めたら、会話にも同じ文を流す。**聞いた人が、誰の声かを読めるように
            let app = app.clone();
            let 行 = 行.clone();
            let 呼び方 = 呼び方.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(訳) = crate::desk::流す(&app, &行, 出所, 呼び方).await {
                    記録!("声: 会話へ流せませんでした: {訳}");
                }
            });
        },
    )
    .await;
    声の棚.lock().expect("毒されていない").remove(&id);
    記録!("声: #{id} の結果 {結果:?}");
    結果
}

/// **人が声を入れた／切った**（ルームを移ったら、画面が切を置きに来る）。
#[tauri::command]
pub async fn agent_voice_set(on: bool) -> Answer<()> {
    声を流してよい.store(on, Ordering::SeqCst);
    記録!(
        "声: この機械のエージェントの声を{}",
        if on {
            "入れました"
        } else {
            "切りました"
        }
    );
    Ok(())
}

/// **声の音を取りに来る。**1 度だけ渡す（生のバイト列で返す）。
#[tauri::command]
pub async fn agent_voice_take(id: u64) -> Answer<tauri::ipc::Response> {
    let 音 = 声の棚
        .lock()
        .expect("毒されていない")
        .get_mut(&id)
        .and_then(|預| 預.音.take());
    音.map(tauri::ipc::Response::new).ok_or_else(|| Failure {
        message: "その声はもうありません".into(),
        code: None,
    })
}

/// **画面が鳴らし始めた・鳴らし終えた・鳴らさなかった**を返す。
#[tauri::command]
pub async fn agent_voice_report(id: u64, stage: String, why: Option<String>) -> Answer<()> {
    let Some(報せ) = 報せを読む(&stage, why) else {
        return Err(Failure {
            message: format!("知らない段です: {stage}"),
            code: None,
        });
    };
    let 棚 = 声の棚.lock().expect("毒されていない");
    if let Some(預) = 棚.get(&id) {
        // 待つ側が居なくなっていても構わない（時間切れのあと）
        let _ = 預.知らせ.send(報せ);
    }
    Ok(())
}

/// 文に日本語の字（かな・漢字）が入っているか。**声の言語を選ぶのに使う。**
fn 日本語を含む(本文: &str) -> bool {
    本文
        .chars()
        .any(|c| matches!(c as u32, 0x3040..=0x30FF | 0x4E00..=0x9FFF | 0xFF66..=0xFF9F))
}

/// 一時の置き場。**走りごと・声ごとに別の名にする**（同時に 2 つ作っても混ざらない）。
fn 一時の置き場() -> std::path::PathBuf {
    static 次: AtomicU64 = AtomicU64::new(1);
    std::env::temp_dir().join(format!(
        "warifu-voice-{}-{}.wav",
        std::process::id(),
        次.fetch_add(1, Ordering::Relaxed)
    ))
}

/// 置き場から読んで消す。**読めても読めなくても消す。**
#[cfg(any(target_os = "macos", windows))]
fn 読んで消す(置き場: &std::path::Path) -> Result<Vec<u8>, String> {
    let 読めた = std::fs::read(置き場).map_err(|e| format!("読み上げを読めません: {e}"));
    let _ = std::fs::remove_file(置き場);
    読めた
}

/// 命令の失敗を、人が読める訳にする。
#[cfg(any(target_os = "macos", windows))]
fn 失敗の訳(名: &str, 出力: &std::process::Output) -> String {
    let 言い分 = String::from_utf8_lossy(&出力.stderr).trim().to_owned();
    if 言い分.is_empty() {
        format!("{名} が失敗しました（{}）", 出力.status)
    } else {
        format!("{名} が失敗しました: {言い分}")
    }
}

/// `say -v ?` の一覧から、日本語の声を 1 つ選ぶ。**Kyoko が在ればそれ。**
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn 日本語の声を選ぶ(一覧: &str) -> Option<String> {
    let 声たち: Vec<String> = 一覧
        .lines()
        .filter_map(|行| {
            let (名, 残り) = 行.split_once("ja_JP")?;
            // 名と言語の間には空白がある。`#` より後ろは見本の文なので見ない
            if 残り.trim_start().starts_with('#') || 残り.trim().is_empty() {
                let 名 = 名.trim();
                (!名.is_empty() && !名.contains('#')).then(|| 名.to_owned())
            } else {
                None
            }
        })
        .collect();
    声たち
        .iter()
        .find(|名| *名 == "Kyoko")
        .or_else(|| 声たち.first())
        .cloned()
}

#[cfg(target_os = "macos")]
mod os {
    use std::io::Write as _;
    use std::process::{Command, Stdio};

    /// **macOS の `say` で WAV を作る。**文は標準入力で渡す（引数にもシェルにも置かない）。
    pub fn 作る(本文: &str) -> Result<Vec<u8>, String> {
        let 置き場 = super::一時の置き場();
        let mut 命令 = Command::new("/usr/bin/say");
        if super::日本語を含む(本文)
            && let Some(声) = 日本語の声()
        {
            命令.arg("-v").arg(声);
        }
        命令
            .arg("-o")
            .arg(&置き場)
            .args(["--file-format=WAVE", "--data-format=LEI16@24000", "-f", "-"])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped());
        let mut 子 = 命令
            .spawn()
            .map_err(|e| format!("say を起こせません: {e}"))?;
        if let Some(mut 入り口) = 子.stdin.take() {
            入り口
                .write_all(本文.as_bytes())
                .map_err(|e| format!("say へ文を渡せません: {e}"))?;
        }
        let 出力 = 子
            .wait_with_output()
            .map_err(|e| format!("say を待てません: {e}"))?;
        if !出力.status.success() {
            let _ = std::fs::remove_file(&置き場);
            return Err(super::失敗の訳("say", &出力));
        }
        super::読んで消す(&置き場)
    }

    fn 日本語の声() -> Option<String> {
        let 出力 = Command::new("/usr/bin/say")
            .args(["-v", "?"])
            .output()
            .ok()?;
        super::日本語の声を選ぶ(&String::from_utf8_lossy(&出力.stdout))
    }
}

#[cfg(windows)]
mod os {
    use std::os::windows::process::CommandExt as _;
    use std::process::Command;

    /// 窓を出さずに走らせる（`CREATE_NO_WINDOW`）。
    const 窓を出さない: u32 = 0x0800_0000;

    /// **命令は固定の文字列。**文・置き場・言語は環境変数で渡す
    /// （命令へつなげると、文の中の `'` や `;` が命令として読まれうる）。
    const 読み上げの命令: &str = "$ErrorActionPreference = 'Stop'; \
        Add-Type -AssemblyName System.Speech; \
        $s = New-Object System.Speech.Synthesis.SpeechSynthesizer; \
        try { \
          if ($env:WARIFU_VOICE_CULTURE) { \
            $v = $s.GetInstalledVoices() | Where-Object { $_.Enabled -and $_.VoiceInfo.Culture.Name -eq $env:WARIFU_VOICE_CULTURE } | Select-Object -First 1; \
            if ($v) { $s.SelectVoice($v.VoiceInfo.Name) } \
          }; \
          $s.SetOutputToWaveFile($env:WARIFU_VOICE_OUT); \
          $s.Speak($env:WARIFU_VOICE_TEXT) \
        } finally { $s.Dispose() }";

    /// **Windows の System.Speech で WAV を作る。**
    pub fn 作る(本文: &str) -> Result<Vec<u8>, String> {
        let 置き場 = super::一時の置き場();
        let mut 命令 = Command::new("powershell");
        命令
            .args(["-NoProfile", "-NonInteractive", "-Command", 読み上げの命令])
            .env("WARIFU_VOICE_TEXT", 本文)
            .env("WARIFU_VOICE_OUT", &置き場)
            .creation_flags(窓を出さない);
        if super::日本語を含む(本文) {
            命令.env("WARIFU_VOICE_CULTURE", "ja-JP");
        }
        let 出力 = 命令
            .output()
            .map_err(|e| format!("PowerShell を起こせません: {e}"))?;
        if !出力.status.success() {
            let _ = std::fs::remove_file(&置き場);
            return Err(super::失敗の訳("PowerShell", &出力));
        }
        super::読んで消す(&置き場)
    }
}

#[cfg(not(any(target_os = "macos", windows)))]
mod os {
    /// **この OS では作らない。**外のサービスへ逃がさない。
    pub fn 作る(_本文: &str) -> Result<Vec<u8>, String> {
        Err("この OS では読み上げを作れません".to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    /// 偽の読み上げ。**呼ばれた数を数える。**
    struct 偽の声 {
        返す: Result<Vec<u8>, String>,
        呼ばれた: AtomicUsize,
    }

    impl 偽の声 {
        fn new(返す: Result<Vec<u8>, String>) -> Self {
            Self {
                返す,
                呼ばれた: AtomicUsize::new(0),
            }
        }
    }

    impl 声を作る for 偽の声 {
        fn 作る(&self, _本文: &str) -> Result<Vec<u8>, String> {
            self.呼ばれた.fetch_add(1, Ordering::SeqCst);
            self.返す.clone()
        }
    }

    fn 小さな_wav() -> Vec<u8> {
        let mut 音 = b"RIFF\0\0\0\0WAVEfmt ".to_vec();
        音.resize(64, 0);
        音
    }

    #[test]
    fn 人が入れていなければ_作る手を呼ばない() {
        let 偽 = 偽の声::new(Ok(小さな_wav()));
        assert_eq!(
            声を用意する(false, &偽, "やあ"),
            Err(声の結果::許されていない)
        );
        assert_eq!(偽.呼ばれた.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn 入れてあれば_作った音を返す() {
        let 偽 = 偽の声::new(Ok(小さな_wav()));
        assert_eq!(声を用意する(true, &偽, "やあ"), Ok(小さな_wav()));
        assert_eq!(偽.呼ばれた.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn 作れなければ_訳をそのまま返す() {
        let 偽 = 偽の声::new(Err("読み上げが使えません".to_owned()));
        assert_eq!(
            声を用意する(true, &偽, "やあ"),
            Err(声の結果::失敗 {
                訳: "読み上げが使えません".to_owned()
            })
        );
    }

    #[test]
    fn wav_でない音は_画面へ渡さない() {
        let 偽 = 偽の声::new(Ok(vec![0; 100]));
        assert!(matches!(
            声を用意する(true, &偽, "やあ"),
            Err(声の結果::失敗 { .. })
        ));
        let 空 = 偽の声::new(Ok(Vec::new()));
        assert!(matches!(
            声を用意する(true, &空, "やあ"),
            Err(声の結果::失敗 { .. })
        ));
    }

    #[test]
    fn 画面の段の名を読む() {
        assert_eq!(報せを読む("started", None), Some(画面の報せ::始めた));
        assert_eq!(
            報せを読む("spoken", None),
            Some(画面の報せ::終えた(声の結果::流した))
        );
        assert_eq!(
            報せを読む("not_allowed", None),
            Some(画面の報せ::終えた(声の結果::許されていない))
        );
        assert_eq!(
            報せを読む("no_meeting", None),
            Some(画面の報せ::終えた(声の結果::会議が無い))
        );
        assert_eq!(
            報せを読む("failed", Some("解けません".to_owned())),
            Some(画面の報せ::終えた(声の結果::失敗 {
                訳: "解けません".to_owned()
            }))
        );
        assert_eq!(報せを読む("exec", None), None);
    }

    #[tokio::test]
    async fn 始めたら一度だけ合図し_終えたら結果を返す() {
        let (送る, mut 受け) = mpsc::unbounded_channel();
        送る.send(画面の報せ::始めた).unwrap();
        送る.send(画面の報せ::始めた).unwrap();
        送る.send(画面の報せ::終えた(声の結果::流した)).unwrap();
        let mut 合図 = 0;
        let 結果 = 終わりを待つ(&mut 受け, Duration::from_secs(1), || 合図 += 1).await;
        assert_eq!(結果, 声の結果::流した);
        assert_eq!(合図, 1, "会話へ流すのは 1 回だけ");
    }

    #[tokio::test]
    async fn 鳴らさなかったら_会話へ流さない() {
        let (送る, mut 受け) = mpsc::unbounded_channel();
        送る.send(画面の報せ::終えた(声の結果::会議が無い)).unwrap();
        let mut 合図 = 0;
        let 結果 = 終わりを待つ(&mut 受け, Duration::from_secs(1), || 合図 += 1).await;
        assert_eq!(結果, 声の結果::会議が無い);
        assert_eq!(合図, 0);
    }

    #[tokio::test]
    async fn 画面が黙ったら_時間切れで失敗を返す() {
        let (_送る, mut 受け) = mpsc::unbounded_channel::<画面の報せ>();
        let 結果 = 終わりを待つ(&mut 受け, Duration::from_millis(20), || {}).await;
        assert!(matches!(結果, 声の結果::失敗 { .. }), "{結果:?}");
    }

    #[test]
    fn 日本語の字を見分ける() {
        assert!(日本語を含む("資料を共有しました"));
        assert!(日本語を含む("ok です"));
        assert!(!日本語を含む("Shared the doc."));
    }

    #[test]
    fn 一覧から日本語の声を選ぶ() {
        let 一覧 = "Albert              en_US    # Hello! My name is Albert.\n\
                   Eddy (日本語（日本）)      ja_JP    # こんにちは! 私の名前はEddyです。\n\
                   Kyoko               ja_JP    # こんにちは! 私の名前はKyokoです。\n";
        assert_eq!(日本語の声を選ぶ(一覧).as_deref(), Some("Kyoko"));
        let kyoko_なし = "Eddy (日本語（日本）)      ja_JP    # こんにちは!\n";
        assert_eq!(
            日本語の声を選ぶ(kyoko_なし).as_deref(),
            Some("Eddy (日本語（日本）)")
        );
        assert_eq!(日本語の声を選ぶ("Albert  en_US  # Hi\n"), None);
    }

    /// **実物の OS で 1 回作る**（手で走らせる: `cargo test --lib -- --ignored 実物`）。
    #[cfg(target_os = "macos")]
    #[test]
    #[ignore = "OS の読み上げを実際に呼ぶ"]
    fn 実物の_say_で_wav_ができる() {
        // 頭が `-` の文も、選択肢として読まれない（標準入力で渡すため）
        for 文 in ["-v テストです", "Hello; rm -rf /tmp/x", "それは 'ok' です"] {
            let 音 = この機械の声.作る(文).expect("作れる");
            assert!(wavか(&音), "{文}");
        }
    }
}
