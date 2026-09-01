//! 置き場所。**Downloads へ直接置かない。**

use std::cell::RefCell;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::{Incoming, Verdict, inspect};

/// 隔離の置き場。
///
/// **この層は何も書かない。**返すのは「どこへ置くべきか」までで、
/// 置くのも開くのも呼ぶ側（最後は人）が決める。
#[derive(Debug)]
pub struct Quarantine {
    root: PathBuf,
    使った: RefCell<HashSet<String>>,
}

impl Quarantine {
    /// 置き場を決める。
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            使った: RefCell::new(HashSet::new()),
        }
    }

    /// 置き場の根。
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// 届いたものを検めて預かる。**信頼している相手でも同じ。**
    pub fn accept(&self, incoming: &Incoming) -> Verdict {
        inspect(incoming)
    }

    /// この名前をどこへ置くべきか。
    ///
    /// **二重に守る。**[`inspect`] を通し忘れても、道の成分が入っていればここで断る。
    ///
    /// 同じ名前が二度来たら、**別の置き場所を返す。**
    /// 上書きできると、**後から来たファイルで前のものを消せる。**
    ///
    /// # 返らない場合
    ///
    /// 名前が空、道の成分を含む、`..` を含むとき。
    pub fn path_for(&self, safe_name: &str) -> Option<PathBuf> {
        if safe_name.is_empty()
            || safe_name.contains(['/', '\\'])
            || safe_name.contains("..")
            || safe_name.chars().any(char::is_control)
        {
            return None;
        }
        let mut 名 = safe_name.to_owned();
        let mut n = 1;
        while self.使った.borrow().contains(&名) {
            名 = 番号を足す(safe_name, n);
            n += 1;
        }
        Some(self.root.join(名))
    }

    /// この名前を使ったと記す。
    ///
    /// 実際に置くのは呼ぶ側なので、**置けたことをこちらへ伝えてもらう。**
    pub fn reserve(&self, safe_name: &str) {
        self.使った.borrow_mut().insert(safe_name.to_owned());
    }
}

/// `a.pdf` → `a (1).pdf`。
fn 番号を足す(名: &str, n: u32) -> String {
    match 名.rsplit_once('.') {
        Some((語幹, 拡張子)) => format!("{語幹} ({n}).{拡張子}"),
        None => format!("{名} ({n})"),
    }
}
