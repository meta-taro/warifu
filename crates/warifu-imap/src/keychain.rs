//! **メールのパスワードを、OS のキーチェーンに置く**（0.1.44）。
//!
//! macOS の `security` に任せる。**割符はパスワードの字を受け取らない** ——
//! 置くときは `security` が人に直接たずね（`-w` を最後に置くと聞き返す）、
//! 読むときは `security` の標準出力から受ける。**引数には載せない**（`ps` に見える）。
//!
//! 作った道具（`security`）が読める項目になるので、画面も CLI も同じ `security` を通して読む。
//!
//! **Windows は資格情報マネージャー**（0.1.48・`keyring` の windows-native）。項目は
//! 「サービス」と「利用者」で引く汎用の資格情報で、字は割符の画面から置く（端末で聞く道は無い）。
//! ほかの OS はまだ無い（無いと言って断る）。

use std::process::{Command, Stdio};

use crate::Error;

/// キーチェーンの項目の「サービス」。割符のメール（IMAP・Gmail のアプリパスワード）はこの名で置く。
pub const サービス: &str = "warifu-mail";

/// 送るだけの口（Resend の API キー）を置くサービスの名（0.1.45）。受信箱のパスワードと混ぜない。
pub const 送る口のサービス: &str = "warifu-resend";

/// 画面から受け取った秘密の字の長さの上限。
const 秘密の長さ: usize = 256;

const 道具: &str = "/usr/bin/security";

/// 置くときに `security` へ渡す引数。**パスワードは入れない**（最後の `-w` で `security` が人に聞く）。
#[must_use]
pub fn 置く引数(利用者: &str) -> Vec<String> {
    [
        "add-generic-password",
        "-U",
        "-s",
        サービス,
        "-a",
        利用者,
        "-l",
        "warifu メール（IMAP）",
        "-w",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

/// 読むときに `security` へ渡す引数。
#[must_use]
pub fn 読む引数(利用者: &str) -> Vec<String> {
    ["find-generic-password", "-s", サービス, "-a", 利用者, "-w"]
        .into_iter()
        .map(str::to_owned)
        .collect()
}

/// 消すときに `security` へ渡す引数。
#[must_use]
pub fn 消す引数(利用者: &str) -> Vec<String> {
    ["delete-generic-password", "-s", サービス, "-a", 利用者]
        .into_iter()
        .map(str::to_owned)
        .collect()
}

/// **この OS で秘密を置けない理由**（0.1.46）。人が読む文。macOS と Windows（0.1.48）で置ける。
pub const 置けない訳: &str = "この OS では、まだメールのパスワードと API キーを置けません（いまは macOS と Windows だけです）";

/// この OS で秘密を置けるか（0.1.46）。置けない OS では、控える前に止める（行き止まりを作らない）。
/// Windows は資格情報マネージャー（0.1.48）。
#[must_use]
pub const fn 使えるか() -> bool {
    cfg!(any(target_os = "macos", target_os = "windows"))
}

/// **端末で人に聞いて置けるか**（0.1.48）。macOS は `security` が端末で聞く。
/// Windows は端末で字を伏せて聞く道を持たないので、割符の画面から置く（[`画面から入れる案内`]）。
#[must_use]
pub const fn 端末で聞けるか() -> bool {
    cfg!(target_os = "macos")
}

/// 端末で聞けない OS で、秘密をどこから入れるか（0.1.48）。人が読む文。
pub const 画面から入れる案内: &str = "パスワード（API キー）は、割符の画面の［メール］→［控えた口］→［秘密を入れる］から入れてください。伏せ字の欄に打った字は、Windows の資格情報マネージャーにだけ置かれます（エージェントには渡りません）";

fn この_os_で使えるか() -> Result<(), Error> {
    if 使えるか() {
        Ok(())
    } else {
        Err(Error::Keychain(置けない訳.into()))
    }
}

/// **項目が入っているか**（0.1.45）。**秘密そのものは読まない**（`-w` を付けない）。
#[must_use]
pub fn 入っているか(サービスの名: &str, 利用者: &str) -> bool {
    if この_os_で使えるか().is_err() {
        return false;
    }
    #[cfg(target_os = "windows")]
    {
        return 窓::読む(サービスの名, 利用者).is_ok();
    }
    #[allow(unreachable_code)]
    Command::new(道具)
        .args(["find-generic-password", "-s", サービスの名, "-a", 利用者])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// **画面で人が打った秘密を、`security` の標準入力へ渡す 1 行**にする（0.1.45）。
///
/// 引数に載せると `ps` に見えるので、`security -i`（標準入力から命令を読む）へ渡す。
/// 字の途中の空白は落とす（Gmail のアプリパスワードは 4 字ずつ空白で区切って見せられる）。
/// **引用の外へ出られる字は受けない**（英数字と `_-.+/=` だけ）。それ以外の秘密はターミナルから入れてもらう。
///
/// # Errors
/// 使えない字・短すぎる・長すぎる・利用者に引用の字が入っているとき。
pub fn 標準入力の行(
    サービスの名: &str,
    利用者: &str,
    名札: &str,
    打った字: &str,
) -> Result<String, Error> {
    let 字 = 秘密の字を整える(打った字)?;
    let 危ない =
        |s: &str| s.is_empty() || s.chars().any(|c| c == '"' || c == '\\' || c.is_control());
    if 危ない(サービスの名) || 危ない(利用者) || 危ない(名札) {
        return Err(Error::Keychain(
            "利用者の名前に使えない字が入っています".into(),
        ));
    }
    Ok(format!(
        "add-generic-password -U -s \"{サービスの名}\" -a \"{利用者}\" -l \"{名札}\" -w \"{字}\"\n"
    ))
}

/// **画面で人が打った秘密の字を整える**（0.1.48・OS によらず同じ決まり）。
/// 字の途中の空白は落とし、英数字と `_-.+/=` だけを受ける。8〜256 字。
///
/// # Errors
/// 使えない字・短すぎる・長すぎるとき。
pub fn 秘密の字を整える(打った字: &str) -> Result<String, Error> {
    let 字: String = 打った字.chars().filter(|c| !c.is_whitespace()).collect();
    if 字.len() < 8 || 字.len() > 秘密の長さ {
        return Err(Error::Keychain("長さが合いません（8〜256 字）".into()));
    }
    if !字
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || "_-.+/=".contains(c))
    {
        return Err(Error::Keychain(
            "使えない字が入っています。英数字と _-.+/= だけを入れてください".into(),
        ));
    }
    Ok(字)
}

/// **画面で人が打った秘密を置く**（0.1.45）。字は `security` の標準入力だけを通る（引数・記録に載せない）。
///
/// # Errors
/// macOS でないとき、字が使えないとき、`security` が断ったとき。
pub fn 字から置く(
    サービスの名: &str,
    利用者: &str,
    名札: &str,
    打った字: &str,
) -> Result<(), Error> {
    use std::io::Write as _;
    この_os_で使えるか()?;
    #[cfg(target_os = "windows")]
    {
        let _ = 名札;
        let 字 = 秘密の字を整える(打った字)?;
        窓::置く(サービスの名, 利用者, &字)?;
        // 置いたつもりで入っていない、を残さない（読み直して確かめる）
        return if 入っているか(サービスの名, 利用者) {
            Ok(())
        } else {
            Err(Error::Keychain(
                "資格情報マネージャーに置けませんでした".into(),
            ))
        };
    }
    #[allow(unreachable_code)]
    let 行 = 標準入力の行(サービスの名, 利用者, 名札, 打った字)?;
    let mut 子 = Command::new(道具)
        .arg("-i")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| Error::Keychain(format!("security を起こせません: {e}")))?;
    {
        let mut 口 = 子
            .stdin
            .take()
            .ok_or_else(|| Error::Keychain("security の入力を開けません".into()))?;
        口.write_all(行.as_bytes())
            .map_err(|e| Error::Keychain(format!("security へ渡せません: {e}")))?;
    }
    let 終わり = 子
        .wait()
        .map_err(|e| Error::Keychain(format!("security が終わりません: {e}")))?;
    // `security -i` は命令が落ちても 0 で終わることがあるので、入ったかを読み直して確かめる
    if 終わり.success() && 入っているか(サービスの名, 利用者) {
        Ok(())
    } else {
        Err(Error::Keychain("キーチェーンに置けませんでした".into()))
    }
}

/// **人に聞いて置く。**端末の入力を `security` へそのまま渡す（割符は字を見ない）。
///
/// # Errors
/// macOS でないとき、`security` が断ったとき（人が取り消した・2 回が違った等）。
pub fn 人に聞いて置く(利用者: &str) -> Result<(), Error> {
    人に聞いて置く_口(サービス, 利用者, "warifu メール（IMAP）")
}

/// **サービスを決めて、人に聞いて置く**（0.1.45・Resend の API キーなど）。
///
/// # Errors
/// [`人に聞いて置く`] と同じ。
pub fn 人に聞いて置く_口(
    サービスの名: &str,
    利用者: &str,
    名札: &str,
) -> Result<(), Error> {
    この_os_で使えるか()?;
    if !端末で聞けるか() {
        return Err(Error::Keychain(画面から入れる案内.into()));
    }
    let mut 引数 = 置く引数(利用者);
    for (i, 値) in [(2, サービスの名), (6, 名札)] {
        引数[i] = 値.to_owned();
    }
    let 終わり = Command::new(道具)
        .args(引数)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|e| Error::Keychain(format!("security を起こせません: {e}")))?;
    if 終わり.success() {
        Ok(())
    } else {
        Err(Error::Keychain("キーチェーンに置けませんでした".into()))
    }
}

/// **読む。**無ければ [`Error::NoCredentials`]。
///
/// # Errors
/// macOS でないとき、項目が無いとき。
pub fn 読む(利用者: &str) -> Result<String, Error> {
    この_os_で使えるか()?;
    #[cfg(target_os = "windows")]
    {
        return 窓::読む(サービス, 利用者);
    }
    #[allow(unreachable_code)]
    let 出 = Command::new(道具)
        .args(読む引数(利用者))
        .stdin(Stdio::null())
        .output()
        .map_err(|e| Error::Keychain(format!("security を起こせません: {e}")))?;
    if !出.status.success() {
        return Err(Error::NoCredentials);
    }
    let mut 字 = String::from_utf8(出.stdout).map_err(|_| Error::NoCredentials)?;
    // `security -w` は末尾に改行を付ける
    while 字.ends_with('\n') || 字.ends_with('\r') {
        字.pop();
    }
    if 字.is_empty() {
        return Err(Error::NoCredentials);
    }
    Ok(字)
}

/// **消す。**無かったときも「消えている」ので通す。
///
/// # Errors
/// macOS でないとき。
pub fn 消す(利用者: &str) -> Result<bool, Error> {
    消す_口(サービス, 利用者)
}

/// **サービスを決めて消す**（0.1.45）。
///
/// # Errors
/// macOS でないとき。
pub fn 消す_口(サービスの名: &str, 利用者: &str) -> Result<bool, Error> {
    この_os_で使えるか()?;
    #[cfg(target_os = "windows")]
    {
        return 窓::消す(サービスの名, 利用者);
    }
    #[allow(unreachable_code)]
    let mut 引数 = 消す引数(利用者);
    引数[2] = サービスの名.to_owned();
    let 出 = Command::new(道具)
        .args(引数)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|e| Error::Keychain(format!("security を起こせません: {e}")))?;
    Ok(出.success())
}

/// **Windows の資格情報マネージャー**（0.1.48）。汎用の資格情報を「サービス」と「利用者」で引く。
#[cfg(target_os = "windows")]
mod 窓 {
    use crate::Error;

    fn 項目(サービスの名: &str, 利用者: &str) -> Result<keyring::Entry, Error> {
        keyring::Entry::new(サービスの名, 利用者)
            .map_err(|e| Error::Keychain(format!("資格情報マネージャーを開けません: {e}")))
    }

    pub(super) fn 置く(
        サービスの名: &str, 利用者: &str, 字: &str
    ) -> Result<(), Error> {
        項目(サービスの名, 利用者)?
            .set_password(字)
            .map_err(|e| Error::Keychain(format!("資格情報マネージャーに置けません: {e}")))
    }

    pub(super) fn 読む(サービスの名: &str, 利用者: &str) -> Result<String, Error> {
        let 字 = 項目(サービスの名, 利用者)?
            .get_password()
            .map_err(|_| Error::NoCredentials)?;
        if 字.is_empty() {
            return Err(Error::NoCredentials);
        }
        Ok(字)
    }

    pub(super) fn 消す(サービスの名: &str, 利用者: &str) -> Result<bool, Error> {
        match 項目(サービスの名, 利用者)?.delete_credential() {
            Ok(()) => Ok(true),
            Err(keyring::Error::NoEntry) => Ok(false),
            Err(e) => Err(Error::Keychain(format!(
                "資格情報マネージャーから消せません: {e}"
            ))),
        }
    }
}

/// **秘密が入っていないときの案内**（11-10）。置き場の名と入れ方を OS で分ける。
///
/// Windows でも「キーチェーン」「warifu mail secret」と案内していた。Windows の置き場は資格情報マネージャーで、
/// 端末で字を伏せて聞く道は無い（入れるのは画面の［秘密を入れる］だけ）。
#[must_use]
pub fn 無いときの案内(macos: bool) -> &'static str {
    if macos {
        "パスワードがキーチェーンにありません（画面の［メール］→［控えた口］→［秘密を入れる］か、ターミナルの warifu mail secret で入れてください）"
    } else {
        "パスワードが資格情報マネージャーにありません（画面の［メール］→［控えた口］→［秘密を入れる］で入れてください）"
    }
}

/// この機械の OS での [`無いときの案内`]。
#[must_use]
pub fn 無いとき() -> &'static str {
    無いときの案内(cfg!(target_os = "macos"))
}

#[cfg(test)]
mod tests {
    #[test]
    fn 無いときの案内は_windows_でキーチェーンと端末の道を言わない() {
        let w = super::無いときの案内(false);
        assert!(
            w.contains("資格情報マネージャー")
                && !w.contains("キーチェーン")
                && !w.contains("warifu mail")
        );
        let m = super::無いときの案内(true);
        assert!(m.contains("キーチェーン") && m.contains("warifu mail secret"));
    }

    use super::*;

    #[test]
    fn 置けるのは_macos_と_windows_で_置けない_os_では行き止まりにせず訳を言う() {
        assert_eq!(
            使えるか(),
            cfg!(any(target_os = "macos", target_os = "windows"))
        );
        assert!(置けない訳.contains("macOS と Windows"));
        if !使えるか() {
            let e = 字から置く(サービス, "a@example.com", "試験", "abcdefgh")
                .expect_err("置けない OS では断る");
            assert!(e.to_string().contains("macOS と Windows"), "{e}");
        }
    }

    #[test]
    fn 端末で聞けない_os_では_画面から入れるよう案内する() {
        // Windows は端末で字を伏せて聞く道を持たない。黙って失敗せず、画面の押し所を言う
        assert_eq!(端末で聞けるか(), cfg!(target_os = "macos"));
        assert!(画面から入れる案内.contains("［秘密を入れる］"));
        assert!(画面から入れる案内.contains("エージェントには渡りません"));
        if 使えるか() && !端末で聞けるか() {
            let e = 人に聞いて置く("a@example.com").expect_err("端末では聞かない");
            assert!(e.to_string().contains("［秘密を入れる］"), "{e}");
        }
    }

    #[test]
    fn 画面で打った字は_空白を落とし_使える字だけを受ける() {
        assert_eq!(
            秘密の字を整える("abcd efgh ijkl mnop").expect("受ける"),
            "abcdefghijklmnop"
        );
        assert!(秘密の字を整える("short").is_err());
        assert!(秘密の字を整える("abcd\"efgh").is_err());
    }

    /// **本物の資格情報マネージャーで確かめる**（Windows の CI で走る・0.1.48）。
    /// 試験用の利用者に秘密ではない字を置き、割符の読む口で読めて、消せることを見る。
    #[cfg(target_os = "windows")]
    #[test]
    fn windows_では_画面から置いた字を資格情報マネージャーで読めて消せる() {
        let 人 = format!("keychain-test-{}@example.invalid", std::process::id());
        let _ = 消す_口(サービス, &人);
        assert!(!入っているか(サービス, &人));
        字から置く(サービス, &人, "warifu 試験", "dummy value 123").expect("置ける");
        assert!(入っているか(サービス, &人));
        assert_eq!(読む(&人).expect("読める"), "dummyvalue123", "空白は落ちる");
        assert!(消す_口(サービス, &人).expect("消せる"));
        assert!(!入っているか(サービス, &人));
        assert!(!消す_口(サービス, &人).expect("2 回目は無い"));
    }

    #[test]
    fn 置くときは_w_を最後に置き_パスワードを引数に載せない() {
        // `-w` の後ろに値が無いと、security が人に聞き返す。値を書くと ps に見える
        let 引数 = 置く引数("someone@example.com");
        assert_eq!(引数.last().map(String::as_str), Some("-w"));
        assert!(引数.contains(&"-U".to_owned()), "2 回目は上書きする");
        assert!(引数.windows(2).any(|w| w[0] == "-s" && w[1] == サービス));
        assert!(
            引数
                .windows(2)
                .any(|w| w[0] == "-a" && w[1] == "someone@example.com")
        );
    }

    #[test]
    fn 画面から置く行は_引用の外へ出られる字を受けない() {
        let 行 = 標準入力の行(
            送る口のサービス,
            "a@example.com",
            "warifu 送る口",
            "re_abc DEF_123",
        )
        .unwrap();
        assert!(行.ends_with("-w \"re_abcDEF_123\"\n"), "空白は落とす: {行}");
        assert!(行.contains("-s \"warifu-resend\""));
        for 悪い in ["abc\"; rm -rf /", "abcdefgh\\x", "abc$(x)defg", "短い"] {
            assert!(
                標準入力の行(サービス, "a@example.com", "x", 悪い).is_err(),
                "{悪い}"
            );
        }
        assert!(標準入力の行(サービス, "a\"@example.com", "x", "abcdefgh").is_err());
    }

    /// **本物のキーチェーンで確かめる**（CI では走らせない・`cargo test -- --ignored` で手元だけ）。
    /// 試験用のサービス名に、秘密ではない字を置き、読めて、消せることを見る。
    #[test]
    #[ignore = "本物のキーチェーンを触る。手元で --ignored を付けて走らせる"]
    fn 画面から置いた字を_本物のキーチェーンで読めて消せる() {
        let (口, 人) = ("warifu-test", "keychain-test@example.com");
        let _ = 消す_口(口, 人);
        assert!(!入っているか(口, 人));
        字から置く(口, 人, "warifu 試験", "dummy value 123").expect("置ける");
        assert!(入っているか(口, 人));
        let mut 引数 = 読む引数(人);
        引数[2] = 口.to_owned();
        let 出 = Command::new(道具).args(引数).output().expect("読める");
        assert_eq!(
            String::from_utf8_lossy(&出.stdout).trim_end(),
            "dummyvalue123",
            "空白は落ちる"
        );
        assert!(消す_口(口, 人).expect("消せる"));
        assert!(!入っているか(口, 人));
    }

    #[test]
    fn 読むのも消すのも_割符の項目だけ() {
        for 引数 in [読む引数("u"), 消す引数("u")] {
            assert!(引数.windows(2).any(|w| w[0] == "-s" && w[1] == サービス));
            assert!(引数.windows(2).any(|w| w[0] == "-a" && w[1] == "u"));
        }
    }
}
