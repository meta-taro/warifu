//! **閉じても同じ人でいられるか**を確かめる。
//!
//! ここが成り立たないと「友達登録」は作れない —— 自分の身元が毎回変われば、
//! 相手は「同じ人」だと分からない（`issues/010`）。

use std::fs;
use std::path::PathBuf;

use warifu_core::{PublicKey, Seed};
use warifu_vault::{Contact, Contacts, Error, Vault};

/// 試験ごとに別の場所を使う。**本物の置き場所（$HOME 配下）に触らない。**
fn 仮の置き場(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "warifu-vault-test-{name}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = fs::remove_dir_all(&dir);
    dir
}

fn 鍵(seed: [u8; 32]) -> PublicKey {
    Seed::from_bytes(seed)
        .profile("Personal")
        .device("PC")
        .public_key()
}

// --- シード -----------------------------------------------------------------

#[test]
fn 二度開いても同じ身元になる() {
    let dir = 仮の置き場("same-identity");
    let vault = Vault::at(&dir);

    let 一度目 = vault.open_seed().expect("一度目");
    let 二度目 = vault.open_seed().expect("二度目");

    assert_eq!(
        一度目.profile("Personal").device("PC").public_key(),
        二度目.profile("Personal").device("PC").public_key(),
        "閉じて開き直したら別人になった"
    );
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn 置き場所ごとに別の身元になる() {
    let a = 仮の置き場("dir-a");
    let b = 仮の置き場("dir-b");

    let ka = Vault::at(&a)
        .open_seed()
        .unwrap()
        .profile("Personal")
        .device("PC")
        .public_key();
    let kb = Vault::at(&b)
        .open_seed()
        .unwrap()
        .profile("Personal")
        .device("PC")
        .public_key();

    assert_ne!(ka, kb, "別の置き場所なのに同じ身元が出た");
    fs::remove_dir_all(&a).ok();
    fs::remove_dir_all(&b).ok();
}

#[cfg(unix)]
#[test]
fn 置いたシードは自分だけが読める() {
    use std::os::unix::fs::PermissionsExt as _;

    let dir = 仮の置き場("perm");
    let vault = Vault::at(&dir);
    vault.open_seed().unwrap();

    let mode = fs::metadata(vault.seed_path())
        .unwrap()
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(mode, 0o600, "シードの権限が 0600 ではない: {mode:o}");

    let dir_mode = fs::metadata(&dir).unwrap().permissions().mode() & 0o777;
    assert_eq!(
        dir_mode, 0o700,
        "置き場所の権限が 0700 ではない: {dir_mode:o}"
    );
    fs::remove_dir_all(&dir).ok();
}

#[cfg(unix)]
#[test]
fn 他人にも読めるシードは受け取らない() {
    use std::os::unix::fs::PermissionsExt as _;

    let dir = 仮の置き場("loose-perm");
    let vault = Vault::at(&dir);
    vault.open_seed().unwrap();
    fs::set_permissions(vault.seed_path(), fs::Permissions::from_mode(0o644)).unwrap();

    // **黙って使わない。**他人に読める鍵は、もう鍵ではない
    let err = vault.open_seed().expect_err("緩い権限のまま開けてしまった");
    assert!(
        matches!(err, Error::Exposed { .. }),
        "権限の話だと分かる形で断っていない: {err:?}"
    );
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn 壊れたシードを黙って作り直さない() {
    let dir = 仮の置き場("broken");
    let vault = Vault::at(&dir);
    vault.open_seed().unwrap();
    fs::write(vault.seed_path(), "これはシードではありません\n").unwrap();

    // **作り直すと身元を失う。**読めないことを言って止まる
    let err = vault
        .open_seed()
        .expect_err("壊れたシードを黙って作り直した");
    assert!(
        matches!(err, Error::Malformed { .. }),
        "壊れていると言っていない: {err:?}"
    );
    assert!(
        vault.seed_path().exists(),
        "読めないファイルを消してしまった"
    );
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn 復旧フレーズから同じ身元が戻る() {
    let dir = 仮の置き場("phrase");
    let vault = Vault::at(&dir);
    let もとの鍵 = vault
        .open_seed()
        .unwrap()
        .profile("Personal")
        .device("PC")
        .public_key();

    let phrase = vault.recovery_phrase().expect("復旧フレーズ");
    assert_eq!(phrase.len(), 52, "base32 52 文字ではない: {phrase}");

    let 別の場所 = 仮の置き場("phrase-restored");
    let 復旧 = Vault::at(&別の場所);
    復旧.restore(&phrase).expect("復旧");
    let 戻った鍵 = 復旧
        .open_seed()
        .unwrap()
        .profile("Personal")
        .device("PC")
        .public_key();

    assert_eq!(もとの鍵, 戻った鍵, "復旧フレーズから別人が出てきた");
    fs::remove_dir_all(&dir).ok();
    fs::remove_dir_all(&別の場所).ok();
}

#[test]
fn 既に身元があるところへ上書き復旧しない() {
    let dir = 仮の置き場("no-overwrite");
    let vault = Vault::at(&dir);
    vault.open_seed().unwrap();
    let 別の場所 = 仮の置き場("other");
    let 別の身元 = Vault::at(&別の場所).open_seed().unwrap();
    let phrase = warifu_core::base32::encode(&別の身元.to_bytes());

    // **上書きは、いまの身元を消すこと。**黙ってやらない
    let err = vault.restore(&phrase).expect_err("黙って身元を上書きした");
    assert!(
        matches!(err, Error::AlreadyExists { .. }),
        "既にあると言っていない: {err:?}"
    );
    fs::remove_dir_all(&dir).ok();
    fs::remove_dir_all(&別の場所).ok();
}

#[test]
fn 復旧フレーズが壊れていたら受け取らない() {
    let dir = 仮の置き場("bad-phrase");
    let vault = Vault::at(&dir);

    for 壊れた in ["", "みじかい", &"A".repeat(51), &"1".repeat(52)] {
        let err = vault
            .restore(壊れた)
            .expect_err("壊れたフレーズを受け取った");
        assert!(
            matches!(err, Error::Malformed { .. }),
            "{壊れた:?} → {err:?}"
        );
    }
    assert!(!vault.seed_path().exists(), "断ったのにファイルを作った");
    fs::remove_dir_all(&dir).ok();
}

// --- 連絡先 -----------------------------------------------------------------

#[test]
fn 覚えた相手は開き直しても残る() {
    let dir = 仮の置き場("contacts-persist");
    let vault = Vault::at(&dir);
    let 自分 = vault
        .open_seed()
        .unwrap()
        .profile("Personal")
        .device("PC")
        .public_key();

    let mut 名簿 = vault.contacts().expect("空の名簿");
    assert!(名簿.is_empty(), "はじめから誰か入っている");
    名簿.add(鍵([7u8; 32]), "Mac Air", 1_700_000_000).unwrap();
    vault.save_contacts(&名簿).unwrap();

    let 読み直し = vault.contacts().unwrap();
    assert_eq!(読み直し.len(), 1);
    let c = 読み直し.find_by_label("Mac Air").expect("呼び名で引けない");
    assert_eq!(c.key(), 鍵([7u8; 32]));
    assert_eq!(c.added_at(), 1_700_000_000);
    assert_ne!(c.key(), 自分);
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn 同じ相手を二度足しても増えない() {
    let mut 名簿 = Contacts::new();
    名簿.add(鍵([1u8; 32]), "Mac Air", 100).unwrap();
    名簿.add(鍵([1u8; 32]), "エアの方", 200).unwrap();

    assert_eq!(名簿.len(), 1, "同じ鍵が二重に載った");
    let c = 名簿.find(鍵([1u8; 32])).unwrap();
    assert_eq!(c.label(), "エアの方", "呼び名を付け直せていない");
    assert_eq!(c.added_at(), 100, "覚えた日まで書き換わっている");
}

#[test]
fn 呼び名が重なったら断る() {
    let mut 名簿 = Contacts::new();
    名簿.add(鍵([1u8; 32]), "Mac Air", 100).unwrap();

    // **同じ呼び名が 2 つあると、`warifu chat Mac Air` がどちらか分からない**
    let err = 名簿
        .add(鍵([2u8; 32]), "Mac Air", 100)
        .expect_err("重なった呼び名を通した");
    assert!(matches!(err, Error::DuplicateLabel { .. }), "{err:?}");
    assert_eq!(名簿.len(), 1);
}

#[test]
fn 区切りを壊す呼び名は断る() {
    let mut 名簿 = Contacts::new();
    for 悪い in ["", "  ", "タブ\tあり", "改行\nあり"] {
        let err = 名簿
            .add(鍵([1u8; 32]), 悪い, 100)
            .expect_err("{悪い:?} を通した");
        assert!(matches!(err, Error::BadLabel { .. }), "{悪い:?} → {err:?}");
    }
    assert!(名簿.is_empty());
}

#[test]
fn 忘れたい相手を消せる() {
    let mut 名簿 = Contacts::new();
    名簿.add(鍵([1u8; 32]), "Mac Air", 100).unwrap();
    名簿.add(鍵([2u8; 32]), "mini", 100).unwrap();

    assert!(名簿.remove(鍵([1u8; 32])), "消したと言わなかった");
    assert!(!名簿.remove(鍵([1u8; 32])), "居ないのに消したと言った");
    assert_eq!(名簿.len(), 1);
    assert!(名簿.find_by_label("mini").is_some());
}

#[test]
fn 読めない行があっても_名簿ごと落とさない() {
    let dir = 仮の置き場("contacts-broken-line");
    let vault = Vault::at(&dir);
    vault.open_seed().unwrap();

    let 良い鍵 = 鍵([3u8; 32]);
    fs::write(
        vault.contacts_path(),
        format!("warifu-contacts-v1\nこわれた行\n{良い鍵}\tmini\t100\n"),
    )
    .unwrap();

    // 1 行の破損で**覚えた相手を全部失う**のは代償が大きすぎる。読める行は残す
    let 名簿 = vault.contacts().expect("読めない行で名簿ごと落ちた");
    assert_eq!(名簿.len(), 1);
    assert_eq!(名簿.skipped(), 1, "捨てた行の数を言っていない");
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn 見出しが違うファイルは読まない() {
    let dir = 仮の置き場("contacts-bad-header");
    let vault = Vault::at(&dir);
    vault.open_seed().unwrap();
    fs::write(vault.contacts_path(), "なにかの別のファイル\n").unwrap();

    let err = vault
        .contacts()
        .expect_err("別のファイルを名簿として読んだ");
    assert!(matches!(err, Error::Malformed { .. }), "{err:?}");
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn 連絡先の一覧は呼び名の順で返る() {
    let mut 名簿 = Contacts::new();
    名簿.add(鍵([3u8; 32]), "mini", 100).unwrap();
    名簿.add(鍵([1u8; 32]), "Mac Air", 100).unwrap();
    名簿.add(鍵([2u8; 32]), "あいさん", 100).unwrap();

    let labels: Vec<_> = 名簿.iter().map(Contact::label).collect();
    assert_eq!(
        labels,
        vec!["Mac Air", "mini", "あいさん"],
        "並びが呼び名の順ではない"
    );
}
