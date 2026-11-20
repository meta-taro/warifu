//! **相手の名乗りを、エージェントへ渡す差出人にも使う**（11-20）。
//!
//! 名乗り（`Notice::Profile`）はこれまで画面にだけ渡していた。人の発言をエージェントへ渡すときは
//! 鍵しか付かず、`chat_read` と知らせには鍵の頭だけが出ていた。**人が書いた頼みなのか、
//! 名乗っていない席の発言なのかを、席の側から見分けられなかった。**
//!
//! 名は「連絡先の呼び名 → 相手の名乗り → 鍵」の順で決める（呼び名を名乗りで上書きしない）。
//! 形は入室の行と同じ `名（鍵の頭…）`。名乗りは相手が書いた字なので、区切り（タブ・改行）を消し、長さを切る。

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

/// 名乗りの長さの上限（字）。行が崩れない長さ。
const 名の上限: usize = 40;

/// 控えておく相手の数の上限。超えたら全部捨てて控え直す（名乗りは入るたびに届く）。
const 控えの上限: usize = 4096;

static 名乗りの棚: LazyLock<Mutex<HashMap<String, String>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// 届いた名乗りを控える。空の名乗りは控えを消す（名乗りを取り下げた）。
pub fn 控える(鍵: &str, 名前: &str) {
    let Ok(mut 棚) = 名乗りの棚.lock() else {
        return;
    };
    let 名 = 整える(名前);
    if 名.is_empty() {
        棚.remove(鍵);
        return;
    }
    if 棚.len() >= 控えの上限 && !棚.contains_key(鍵) {
        棚.clear();
    }
    棚.insert(鍵.to_owned(), 名);
}

/// 控えた名乗り。無ければ `None`。
pub fn 引く(鍵: &str) -> Option<String> {
    名乗りの棚.lock().ok()?.get(鍵).cloned()
}

/// 相手が書いた名を、行に載せてよい形にする（区切りの字を空白に・前後の空白を落とす・長さを切る）。
fn 整える(名前: &str) -> String {
    let 字: String = 名前
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    字.trim().chars().take(名の上限).collect()
}

/// **人の発言の差出人**。呼び名があれば呼び名、無ければ名乗り、どちらも無ければ鍵のまま。
#[must_use]
pub fn 人の差出人(帳: &[(String, String)], 名乗り: Option<&str>, 鍵: &str) -> String {
    let 名 = crate::contacts::書き出しの名(帳, "", 鍵);
    if 名 != 鍵 {
        return 名;
    }
    match 名乗り.map(str::trim).filter(|n| !n.is_empty()) {
        Some(n) => {
            let 頭: String = 鍵.chars().take(12).collect();
            format!("{n}（{頭}…）")
        }
        None => 鍵.to_owned(),
    }
}

/// **エージェントへ渡す差出人**。席の発言（札つき）は画面と同じ名、人の発言は [`人の差出人`]。
pub fn 渡す差出人(画面の名: &str, 席か: bool, 鍵: &str) -> String {
    if 席か {
        return 画面の名.to_owned();
    }
    人の差出人(&crate::desk::連絡先の組(), 引く(鍵).as_deref(), 鍵)
}

#[cfg(test)]
mod tests {
    use super::*;

    const 鍵: &str = "ABCDEFGHJKMNAAAAAAAAAAAAAAAAAAAA";

    #[test]
    fn 名乗りがあれば_名乗りと鍵の頭() {
        assert_eq!(
            人の差出人(&[], Some("NOTE-PC の中の人"), 鍵),
            "NOTE-PC の中の人（ABCDEFGHJKMN…）"
        );
    }

    #[test]
    fn 呼び名があれば_名乗りより呼び名() {
        let 帳 = [(鍵.to_owned(), "経理の人".to_owned())];
        assert_eq!(
            人の差出人(&帳, Some("NOTE-PC の中の人"), 鍵),
            "経理の人（ABCDEFGHJKMN…）"
        );
    }

    #[test]
    fn どちらも無ければ_鍵のまま() {
        assert_eq!(人の差出人(&[], None, 鍵), 鍵);
        assert_eq!(人の差出人(&[], Some("  "), 鍵), 鍵);
    }

    #[test]
    fn 名乗りの区切りの字は消し_長さを切る() {
        let 長い = "あ".repeat(100);
        控える("試しの鍵1", &format!("名\tの\n人{長い}"));
        let 名 = 引く("試しの鍵1").expect("控えた");
        assert!(!名.contains('\t') && !名.contains('\n'), "{名}");
        assert_eq!(名.chars().count(), 名の上限);
    }

    #[test]
    fn 空の名乗りは控えを消す() {
        控える("試しの鍵2", "NOTE-PC の中の人");
        控える("試しの鍵2", "");
        assert_eq!(引く("試しの鍵2"), None);
    }
}
