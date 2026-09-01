//! 繋ぎ先と資格情報。

use core::fmt;

use crate::Error;

/// 受信箱への繋ぎ先。
///
/// **秘密情報はここに入るが、外へは出ない。**
/// `Debug` にパスワードを出さないのは、`.env` から読んだものが
/// ログや Issue にそのまま写るのを、書き方の注意ではなく型で止めるため（baseline §14）。
#[derive(Clone)]
pub struct Account {
    host: String,
    port: u16,
    user: String,
    password: Secret,
}

impl Account {
    /// 繋ぎ先を組み立てる。
    ///
    /// **秘密情報を作らない。**呼ぶ側が環境変数などから渡す（`.env.example` を見ること）。
    ///
    /// # 失敗
    ///
    /// ホスト・利用者・パスワードのいずれかが空なら [`Error::NoCredentials`]。
    pub fn new(host: &str, port: u16, user: &str, password: &str) -> Result<Self, Error> {
        if host.is_empty() || user.is_empty() || password.is_empty() {
            return Err(Error::NoCredentials);
        }
        Ok(Self {
            host: host.to_owned(),
            port,
            user: user.to_owned(),
            password: Secret(password.to_owned()),
        })
    }

    /// 繋ぎ先のホスト。
    pub fn host(&self) -> &str {
        &self.host
    }

    /// 繋ぎ先のポート。
    pub fn port(&self) -> u16 {
        self.port
    }

    /// 利用者。
    pub fn user(&self) -> &str {
        &self.user
    }

    pub(crate) fn password(&self) -> &str {
        &self.password.0
    }
}

impl fmt::Debug for Account {
    /// **パスワードを出さない。**
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Account")
            .field("host", &self.host)
            .field("port", &self.port)
            .field("user", &self.user)
            .field("password", &"（伏せる）")
            .finish()
    }
}

/// 表示に出ない文字列。
#[derive(Clone)]
struct Secret(String);

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("（伏せる）")
    }
}
