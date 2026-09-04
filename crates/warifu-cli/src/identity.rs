//! 身元と、覚えた相手。**画面なしの口から使う分だけ。**
//!
//! 置き方そのものは `warifu-vault` の担当。ここはその上の、人が打つ言葉との橋。

use warifu_core::{Device, PublicKey};
use warifu_vault::{Contacts, Error, Vault};

/// この端末の身元。**閉じても同じ人でいられる。**
///
/// 以前は起動のたびに `Seed::generate()` していた（D2 が未決だったため）。
/// **それだと相手は「同じ人」だと分からず、連絡先が作れない**（`issues/010`）。
///
/// # Errors
/// 置き場所を開けないとき。
pub fn 開く() -> Result<(Vault, Device), Error> {
    let vault = Vault::default_location()?;
    let device = vault.open_seed()?.profile("Personal").device("cli");
    Ok((vault, device))
}

/// 人が打った一言から相手を決める。**呼び名でも、公開鍵そのものでも引ける。**
///
/// 呼び名を先に見るのは、**打ちやすいほうを優先する**ため。
/// 呼び名は手元にしか無いので、衝突しても相手を取り違える先が自分の名簿の中に限られる。
#[must_use]
pub fn 相手を引く(contacts: &Contacts, 言葉: &str) -> Option<PublicKey> {
    if let Some(c) = contacts.find_by_label(言葉) {
        return Some(c.key());
    }
    言葉.trim().parse().ok()
}

/// 表に出す名前。覚えていれば呼び名、知らなければ鍵の頭。
///
/// **鍵をそのまま並べても人には読めない**が、短くしすぎると別人と見分けが付かない。
/// 覚えていない相手は「知らない相手」だと分かる形で出す。
#[must_use]
pub fn 呼び名(contacts: &Contacts, key: PublicKey) -> String {
    match contacts.find(key) {
        Some(c) => c.label().to_owned(),
        None => {
            let text = key.to_string();
            format!("知らない相手（{}…）", &text[..8.min(text.len())])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use warifu_core::Seed;

    fn 鍵(seed: [u8; 32]) -> PublicKey {
        Seed::from_bytes(seed)
            .profile("Personal")
            .device("PC")
            .public_key()
    }

    fn 名簿() -> Contacts {
        let mut c = Contacts::new();
        c.add(鍵([1u8; 32]), "Mac Air", 100).unwrap();
        c
    }

    #[test]
    fn 呼び名でも鍵でも引ける() {
        let c = 名簿();
        assert_eq!(相手を引く(&c, "Mac Air"), Some(鍵([1u8; 32])));
        assert_eq!(
            相手を引く(&c, &鍵([1u8; 32]).to_string()),
            Some(鍵([1u8; 32]))
        );
    }

    #[test]
    fn 覚えていない鍵でも_鍵として読めれば引ける() {
        // **覚える前に一度つなぐ**ことがあるので、鍵直打ちを塞がない
        let c = 名簿();
        assert_eq!(
            相手を引く(&c, &鍵([9u8; 32]).to_string()),
            Some(鍵([9u8; 32]))
        );
    }

    #[test]
    fn 知らない言葉は引けない() {
        let c = 名簿();
        assert_eq!(相手を引く(&c, "だれか"), None);
        assert_eq!(相手を引く(&c, ""), None);
    }

    #[test]
    fn 覚えていない相手は_知らないと分かる形で出す() {
        let c = 名簿();
        assert_eq!(呼び名(&c, 鍵([1u8; 32])), "Mac Air");
        let 知らない = 呼び名(&c, 鍵([9u8; 32]));
        assert!(知らない.contains("知らない相手"), "{知らない}");
    }
}
