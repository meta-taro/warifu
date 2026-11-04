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

/// キーチェーンの項目の「サービス」。割符のメールはこの名で置く。
pub const サービス: &str = "warifu-mail";

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

/// **人に聞いて置く。**端末の入力を `security` へそのまま渡す（割符は字を見ない）。
///
/// # Errors
/// macOS でないとき、`security` が断ったとき（人が取り消した・2 回が違った等）。
pub fn 人に聞いて置く(利用者: &str) -> Result<(), Error> {
    この_os_で使えるか()?;
    let 終わり = Command::new(道具)
        .args(置く引数(利用者))
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
    この_os_で使えるか()?;
    let 出 = Command::new(道具)
        .args(消す引数(利用者))
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
    fn 読むのも消すのも_割符の項目だけ() {
        for 引数 in [読む引数("u"), 消す引数("u")] {
            assert!(引数.windows(2).any(|w| w[0] == "-s" && w[1] == サービス));
            assert!(引数.windows(2).any(|w| w[0] == "-a" && w[1] == "u"));
        }
    }
}
