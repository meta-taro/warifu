//! **メールのパスワードを、OS のキーチェーンに置く**（0.1.44）。
//!
//! macOS の `security` に任せる。**割符はパスワードの字を受け取らない** ——
//! 置くときは `security` が人に直接たずね（`-w` を最後に置くと聞き返す）、
//! 読むときは `security` の標準出力から受ける。**引数には載せない**（`ps` に見える）。
//!
//! 作った道具（`security`）が読める項目になるので、画面も CLI も同じ `security` を通して読む。
//! 他の OS はまだ無い（無いと言って断る）。

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

fn この_os_で使えるか() -> Result<(), Error> {
    if cfg!(target_os = "macos") {
        Ok(())
    } else {
        Err(Error::Keychain(
            "この OS のキーチェーンには、まだ対応していません（macOS だけ）".into(),
        ))
    }
}

/// **項目が入っているか**（0.1.45）。**秘密そのものは読まない**（`-w` を付けない）。
#[must_use]
pub fn 入っているか(サービスの名: &str, 利用者: &str) -> bool {
    if この_os_で使えるか().is_err() {
        return false;
    }
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
    let 字: String = 打った字.chars().filter(|c| !c.is_whitespace()).collect();
    if 字.len() < 8 || 字.len() > 秘密の長さ {
        return Err(Error::Keychain("長さが合いません（8〜256 字）".into()));
    }
    if !字
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || "_-.+/=".contains(c))
    {
        return Err(Error::Keychain(
            "使えない字が入っています。ターミナルで warifu mail secret <アドレス> から入れてください".into(),
        ));
    }
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

#[cfg(test)]
mod tests {
    use super::*;

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
