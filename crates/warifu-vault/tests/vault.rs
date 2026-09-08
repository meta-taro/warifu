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

// --- 戸口の知り合い（**再起動をまたぐ**） -----------------------------------

#[test]
fn 覚えた知り合いは開き直しても残る() {
    // **2026-09-07 まで、知り合いはメモリの上にしか無かった。**
    // 「一度開けた相手は、次から割符なしで開ける」という決めごとが、
    // アプリを閉じた瞬間に効かなくなっていた（不具合）。
    let dir = 仮の置き場("known-survives");
    let vault = Vault::at(&dir);
    vault.open_seed().unwrap();

    vault.save_known(&[鍵([7u8; 32]), 鍵([8u8; 32])]).unwrap();
    let 戻り = Vault::at(&dir).known().unwrap();

    assert_eq!(戻り.len(), 2);
    assert!(戻り.contains(&鍵([7u8; 32])));
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn 知り合いのファイルが無ければ_誰も知らないところから始まる() {
    // **無いことと壊れていることを混ぜない。**初回は「無い」が正しい
    let dir = 仮の置き場("known-empty");
    let vault = Vault::at(&dir);
    vault.open_seed().unwrap();

    assert!(vault.known().unwrap().is_empty());
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn 知り合いのファイルは自分だけが読める() {
    // **誰を通すかの一覧である。**他人に読ませない
    let dir = 仮の置き場("known-private");
    let vault = Vault::at(&dir);
    vault.open_seed().unwrap();
    vault.save_known(&[鍵([9u8; 32])]).unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(vault.known_path())
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600, "0600 であること");
    }
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn 見出しが違うファイルは知り合いとして読まない() {
    let dir = 仮の置き場("known-bad-header");
    let vault = Vault::at(&dir);
    vault.open_seed().unwrap();
    fs::write(vault.known_path(), "なにかの別のファイル\n").unwrap();

    let err = vault
        .known()
        .expect_err("別のファイルを知り合いとして読んだ");
    assert!(matches!(err, Error::Malformed { .. }), "{err:?}");
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn 読めない行があっても_知り合いごと落とさない() {
    // **1 行壊れただけで全員が入れなくなるのは、代償が大きすぎる**（名簿と同じ構え）
    let dir = 仮の置き場("known-broken-line");
    let vault = Vault::at(&dir);
    vault.open_seed().unwrap();
    let 良い行 = 鍵([5u8; 32]).to_string();
    fs::write(
        vault.known_path(),
        format!("warifu-known-v1\nこわれた行\n{良い行}\n"),
    )
    .unwrap();

    let 戻り = vault.known().unwrap();

    assert_eq!(戻り, vec![鍵([5u8; 32])]);
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn 同じ相手を二度書いても増えない() {
    // **一覧であって履歴ではない**
    let dir = 仮の置き場("known-dedup");
    let vault = Vault::at(&dir);
    vault.open_seed().unwrap();

    vault.save_known(&[鍵([1u8; 32]), 鍵([1u8; 32])]).unwrap();

    assert_eq!(vault.known().unwrap().len(), 1);
    fs::remove_dir_all(&dir).ok();
}

// --- 最後に繋がった住所（**1 つだけ持つ。履歴にしない**） ---------------------

/// 試験で使う住所の形（`warifu-net` の `Address` の表記に合わせた見た目）。
const 住所: &str = "WARIFU1-AAAAAAAABBBBBBBBCCCCCCCC";
const 別の住所: &str = "WARIFU1-DDDDDDDDEEEEEEEEFFFFFFFF";

#[test]
fn 旧版の名簿を読める() {
    // **古いものを黙って壊さない。**手元にある v1 の名簿がそのまま読めること
    let dir = 仮の置き場("contacts-v1-read");
    let vault = Vault::at(&dir);
    vault.open_seed().unwrap();
    fs::write(
        vault.contacts_path(),
        format!("warifu-contacts-v1\n{}\tmini\t100\n", 鍵([2u8; 32])),
    )
    .unwrap();

    let 名簿 = vault.contacts().unwrap();

    let 相手 = 名簿.find(鍵([2u8; 32])).expect("旧版の行を読めていない");
    assert_eq!(相手.label(), "mini");
    assert_eq!(相手.added_at(), 100);
    assert_eq!(相手.address(), None, "旧版に住所は無い");
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn 旧版を読んで書き出すと新版になる() {
    // **移行のための別コマンドを作らない。**次に書いたときに上がる
    let dir = 仮の置き場("contacts-v1-upgrade");
    let vault = Vault::at(&dir);
    vault.open_seed().unwrap();
    fs::write(
        vault.contacts_path(),
        format!("warifu-contacts-v1\n{}\tmini\t100\n", 鍵([2u8; 32])),
    )
    .unwrap();

    let 名簿 = vault.contacts().unwrap();
    vault.save_contacts(&名簿).unwrap();

    let 中身 = fs::read_to_string(vault.contacts_path()).unwrap();
    assert!(中身.starts_with("warifu-contacts-v3\n"), "{中身}");
    // **覚えた日を動かさない**（版が上がっただけで「今日覚えた人」にしない）
    assert_eq!(
        vault
            .contacts()
            .unwrap()
            .find(鍵([2u8; 32]))
            .unwrap()
            .added_at(),
        100
    );
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn 住所を覚えたら開き直しても残る() {
    let dir = 仮の置き場("contacts-address");
    let vault = Vault::at(&dir);
    vault.open_seed().unwrap();
    let mut 名簿 = Contacts::new();
    名簿.add(鍵([3u8; 32]), "air", 100).unwrap();
    assert!(名簿.note_address(鍵([3u8; 32]), 住所).unwrap());
    vault.save_contacts(&名簿).unwrap();

    let 戻り = Vault::at(&dir).contacts().unwrap();

    assert_eq!(戻り.find(鍵([3u8; 32])).unwrap().address(), Some(住所));
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn 住所は一つだけ持つ() {
    // **居場所の履歴にしない**（`issues/010` の「止めるべき条件」）
    let mut 名簿 = Contacts::new();
    名簿.add(鍵([3u8; 32]), "air", 100).unwrap();
    名簿.note_address(鍵([3u8; 32]), 住所).unwrap();
    名簿.note_address(鍵([3u8; 32]), 別の住所).unwrap();

    assert_eq!(名簿.len(), 1);
    assert_eq!(名簿.find(鍵([3u8; 32])).unwrap().address(), Some(別の住所));
}

#[test]
fn 覚えていない相手に住所は書けない() {
    // **住所だけの行を作らない。**呼び名の無い相手は連絡帳に出せない
    let mut 名簿 = Contacts::new();

    assert!(!名簿.note_address(鍵([4u8; 32]), 住所).unwrap());

    assert!(名簿.is_empty());
}

#[test]
fn 呼び名を付け直しても住所は消えない() {
    let mut 名簿 = Contacts::new();
    名簿.add(鍵([3u8; 32]), "air", 100).unwrap();
    名簿.note_address(鍵([3u8; 32]), 住所).unwrap();
    名簿.add(鍵([3u8; 32]), "Mac Air", 200).unwrap();

    assert_eq!(名簿.find(鍵([3u8; 32])).unwrap().label(), "Mac Air");
    assert_eq!(名簿.find(鍵([3u8; 32])).unwrap().address(), Some(住所));
}

#[test]
fn 区切りを壊す住所は断る() {
    let mut 名簿 = Contacts::new();
    名簿.add(鍵([3u8; 32]), "air", 100).unwrap();

    for 壊す in ["WARIFU1-A\tB", "WARIFU1-A\nB", ""] {
        assert!(
            名簿.note_address(鍵([3u8; 32]), 壊す).is_err(),
            "{壊す:?} を受け取った"
        );
    }
}

#[test]
fn 長すぎる住所は断る() {
    let mut 名簿 = Contacts::new();
    名簿.add(鍵([3u8; 32]), "air", 100).unwrap();
    let 長い = format!("WARIFU1-{}", "A".repeat(2000));

    assert!(名簿.note_address(鍵([3u8; 32]), &長い).is_err());
}

#[test]
fn 欄の数が違う行は捨てて数える() {
    let dir = 仮の置き場("contacts-v2-bad-cells");
    let vault = Vault::at(&dir);
    vault.open_seed().unwrap();
    fs::write(
        vault.contacts_path(),
        format!(
            "warifu-contacts-v2\n{k}\tmini\t100\n{k2}\tair\t100\t{住所}\n",
            k = 鍵([2u8; 32]),
            k2 = 鍵([3u8; 32])
        ),
    )
    .unwrap();

    let 名簿 = vault.contacts().unwrap();

    assert_eq!(名簿.len(), 1, "3 欄の行は v2 では読まない");
    assert_eq!(名簿.skipped(), 1);
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn 忘れた相手の住所も消える() {
    let mut 名簿 = Contacts::new();
    名簿.add(鍵([3u8; 32]), "air", 100).unwrap();
    名簿.note_address(鍵([3u8; 32]), 住所).unwrap();

    assert!(名簿.remove(鍵([3u8; 32])));

    assert!(名簿.find(鍵([3u8; 32])).is_none());
}

// --- 預かり所の宛先（**人が書く。割符が拾ってこない**） ----------------------

#[test]
fn 預かり所の宛先は開き直しても残る() {
    let dir = 仮の置き場("postbox-remembered");
    let vault = Vault::at(&dir);
    vault.open_seed().unwrap();
    assert_eq!(vault.postbox().unwrap(), None, "はじめは無い");

    vault.save_postbox(Some("WARIFU1-ABCDEF")).unwrap();
    assert_eq!(
        Vault::at(&dir).postbox().unwrap().as_deref(),
        Some("WARIFU1-ABCDEF")
    );
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn 預かり所は外せる() {
    // **やめると決めたら、置いたものが残らない**
    let dir = 仮の置き場("postbox-cleared");
    let vault = Vault::at(&dir);
    vault.open_seed().unwrap();
    vault.save_postbox(Some("WARIFU1-ABCDEF")).unwrap();
    vault.save_postbox(None).unwrap();
    assert_eq!(vault.postbox().unwrap(), None);
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn 預かり所の宛先は自分だけが読める() {
    // **どこへ預けに行くかは、その人の居場所の手がかりになる**
    let dir = 仮の置き場("postbox-private");
    let vault = Vault::at(&dir);
    vault.open_seed().unwrap();
    vault.save_postbox(Some("WARIFU1-ABCDEF")).unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(vault.postbox_path())
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600, "0600 であること");
    }
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn 見出しが違うファイルは預かり所として読まない() {
    let dir = 仮の置き場("postbox-bad-header");
    let vault = Vault::at(&dir);
    vault.open_seed().unwrap();
    fs::write(vault.postbox_path(), "なにかの別のファイル\n").unwrap();

    let err = vault
        .postbox()
        .expect_err("別のファイルを預かり所として読んだ");
    assert!(matches!(err, Error::Malformed { .. }), "{err:?}");
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn 宛先に改行は書けない() {
    // **1 行に 1 つ。**改行が入ると、次の行が別の意味を持ってしまう
    let dir = 仮の置き場("postbox-newline");
    let vault = Vault::at(&dir);
    vault.open_seed().unwrap();
    assert!(vault.save_postbox(Some("WARIFU1-AB\nCDEF")).is_err());
    fs::remove_dir_all(&dir).ok();
}

// --- プロフィール（**人も、この端末の AI も**） -----------------------------

#[test]
fn 人と_この端末の_ai_を分けて覚える() {
    // **鍵で分けない。**この端末の AI は持ち主の鍵で喋る（D48）ので、
    // 鍵で分けると人と AI が同じ 1 つになる
    let dir = 仮の置き場("profiles-me-and-ai");
    let vault = Vault::at(&dir);
    vault.open_seed().unwrap();

    let mut 面々 = warifu_vault::Profiles::new();
    面々.put(warifu_vault::Profile::new(warifu_vault::Who::Me, "たろう", "この PC の人").unwrap());
    面々.put(
        warifu_vault::Profile::new(
            warifu_vault::Who::Desk("zumen".into()),
            "図面くん",
            "図面まわりを見ています",
        )
        .unwrap(),
    );
    vault.save_profiles(&面々).unwrap();

    let 読み直し = Vault::at(&dir).profiles().unwrap();
    assert_eq!(読み直し.len(), 2);
    assert_eq!(
        読み直し.find(&warifu_vault::Who::Me).unwrap().name(),
        "たろう"
    );
    assert_eq!(
        読み直し
            .find(&warifu_vault::Who::Desk("zumen".into()))
            .unwrap()
            .bio(),
        "図面まわりを見ています"
    );
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn プロフィールは自分だけが読める() {
    let dir = 仮の置き場("profiles-private");
    let vault = Vault::at(&dir);
    vault.open_seed().unwrap();
    let mut 面々 = warifu_vault::Profiles::new();
    面々.put(warifu_vault::Profile::new(warifu_vault::Who::Me, "たろう", "").unwrap());
    vault.save_profiles(&面々).unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(vault.profiles_path())
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600, "0600 であること");
    }
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn 見出しが違うファイルはプロフィールとして読まない() {
    let dir = 仮の置き場("profiles-bad-header");
    let vault = Vault::at(&dir);
    vault.open_seed().unwrap();
    fs::write(vault.profiles_path(), "なにかの別のファイル\n").unwrap();
    let err = vault
        .profiles()
        .expect_err("別のファイルをプロフィールとして読んだ");
    assert!(matches!(err, Error::Malformed { .. }), "{err:?}");
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn 壊れた行があっても_他のプロフィールは残る() {
    let dir = 仮の置き場("profiles-broken-line");
    let vault = Vault::at(&dir);
    vault.open_seed().unwrap();
    fs::write(
        vault.profiles_path(),
        "warifu-profiles-v1\nこわれた行\nme\tたろう\tこの PC の人\t\n",
    )
    .unwrap();
    let 面々 = vault.profiles().unwrap();
    assert_eq!(面々.len(), 1);
    assert_eq!(面々.find(&warifu_vault::Who::Me).unwrap().name(), "たろう");
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn 顔を差し替えても_既定へ戻せる() {
    let mut p = warifu_vault::Profile::new(warifu_vault::Who::Me, "たろう", "").unwrap();
    assert_eq!(p.avatar(), None, "既定は鍵から描く");
    p.set_avatar(Some("kao.png")).unwrap();
    assert_eq!(p.avatar(), Some("kao.png"));
    p.set_avatar(None).unwrap();
    assert_eq!(p.avatar(), None);
}

#[test]
fn 長すぎる名前と紹介は断る() {
    // **黙って切り詰めない。**削られたことに、書いた人が気づかない
    let 長い名 = "あ".repeat(warifu_vault::NAME_MAX + 1);
    assert!(warifu_vault::Profile::new(warifu_vault::Who::Me, &長い名, "").is_err());
    let 長い一言 = "い".repeat(warifu_vault::BIO_MAX + 1);
    assert!(warifu_vault::Profile::new(warifu_vault::Who::Me, "た", &長い一言).is_err());
}

#[test]
fn 改行やタブは入れられない() {
    // 入ると、次の行・次の欄が別の意味を持つ
    assert!(warifu_vault::Profile::new(warifu_vault::Who::Me, "た\nろう", "").is_err());
    assert!(warifu_vault::Profile::new(warifu_vault::Who::Me, "たろう", "あ\tい").is_err());
}

#[test]
fn 同じ人のプロフィールは一つだけ持つ() {
    let mut 面々 = warifu_vault::Profiles::new();
    面々.put(warifu_vault::Profile::new(warifu_vault::Who::Me, "たろう", "").unwrap());
    面々.put(warifu_vault::Profile::new(warifu_vault::Who::Me, "たろう 2", "").unwrap());
    assert_eq!(面々.len(), 1);
    assert_eq!(
        面々.find(&warifu_vault::Who::Me).unwrap().name(),
        "たろう 2"
    );
}

// --- 顔（差し替える画像） ---------------------------------------------------

/// 幅と高さだけを持つ、いちばん短い PNG の頭。
fn pngの頭(幅: u32, 高さ: u32) -> Vec<u8> {
    let mut v = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
    v.extend_from_slice(&13u32.to_be_bytes()); // IHDR の長さ
    v.extend_from_slice(b"IHDR");
    v.extend_from_slice(&幅.to_be_bytes());
    v.extend_from_slice(&高さ.to_be_bytes());
    v
}

#[test]
fn 顔は_png_だけ受ける() {
    // **拡張子を信じない。**中身の頭を見る
    assert_eq!(warifu_vault::顔として読む(&pngの頭(64, 64)), Ok((64, 64)));
    assert!(warifu_vault::顔として読む("\u{ff}\u{d8}\u{ff} JPEG のつもり".as_bytes()).is_err());
    assert!(warifu_vault::顔として読む(b"").is_err());
}

#[test]
fn 縦横が大きすぎる顔は断る() {
    // **小さいファイルでも、桁の大きい画像は描くときに膨らむ**
    let 大きい = pngの頭(warifu_vault::AVATAR_MAX_SIDE + 1, 10);
    assert!(matches!(
        warifu_vault::顔として読む(&大きい),
        Err(warifu_vault::BadImage::TooWide(..))
    ));
}

#[test]
fn 大きすぎるファイルは断る() {
    let mut 重い = pngの頭(10, 10);
    重い.resize(warifu_vault::AVATAR_MAX_BYTES + 1, 0);
    assert!(matches!(
        warifu_vault::顔として読む(&重い),
        Err(warifu_vault::BadImage::TooBig(_))
    ));
}

#[test]
fn 縦横が_0_の画像は絵ではない() {
    // 描こうとした側で割り算が壊れる
    assert!(warifu_vault::顔として読む(&pngの頭(0, 10)).is_err());
    assert!(warifu_vault::顔として読む(&pngの頭(10, 0)).is_err());
}

#[test]
fn 顔のファイル名は置き場所の外へ出られない() {
    // **人が書いた名乗りを、そのままファイル名にしない**
    let 名 =
        warifu_vault::顔のファイル名(&warifu_vault::Who::Desk("../../etc/passwd".into()));
    assert!(!名.contains('/'), "{名}");
    assert!(!名.contains(".."), "{名}");
    assert!(名.ends_with(".png"), "{名}");
    assert_eq!(
        warifu_vault::顔のファイル名(&warifu_vault::Who::Me),
        "me.png"
    );
}

// --- 覚え書き（**こちらが書く。相手の名乗りとは別**） -----------------------

#[test]
fn 相手に覚え書きを残せる() {
    // 「どの機械の、何をするエージェントか」を人が自分の言葉で残す
    let dir = 仮の置き場("contacts-note");
    let vault = Vault::at(&dir);
    vault.open_seed().unwrap();

    let mut 名簿 = Contacts::new();
    名簿.add(鍵([3u8; 32]), "mac air", 100).unwrap();
    assert!(
        名簿
            .set_note(鍵([3u8; 32]), "Air の zumen 担当。図面まわり")
            .unwrap()
    );
    vault.save_contacts(&名簿).unwrap();

    let 読み直し = Vault::at(&dir).contacts().unwrap();
    assert_eq!(
        読み直し.find(鍵([3u8; 32])).unwrap().note(),
        "Air の zumen 担当。図面まわり"
    );
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn 覚えていない相手には覚え書きを残せない() {
    // **行を作らない。**呼び名の無い相手を連絡帳に出さない（住所と同じ構え）
    let mut 名簿 = Contacts::new();
    assert!(!名簿.set_note(鍵([4u8; 32]), "だれか").unwrap());
}

#[test]
fn 覚え書きは空にできる() {
    let mut 名簿 = Contacts::new();
    名簿.add(鍵([5u8; 32]), "air", 100).unwrap();
    名簿.set_note(鍵([5u8; 32]), "いちど書く").unwrap();
    名簿.set_note(鍵([5u8; 32]), "").unwrap();
    assert_eq!(名簿.find(鍵([5u8; 32])).unwrap().note(), "");
}

#[test]
fn 覚え書きに改行やタブは入れられない() {
    // 入ると、次の行・次の欄が別の意味を持つ
    let mut 名簿 = Contacts::new();
    名簿.add(鍵([6u8; 32]), "air", 100).unwrap();
    assert!(名簿.set_note(鍵([6u8; 32]), "あ\tい").is_err());
    assert!(名簿.set_note(鍵([6u8; 32]), "あ\nい").is_err());
}

#[test]
fn 長すぎる覚え書きは断る() {
    // **黙って切り詰めない**
    let mut 名簿 = Contacts::new();
    名簿.add(鍵([7u8; 32]), "air", 100).unwrap();
    let 長い = "あ".repeat(warifu_vault::NOTE_MAX + 1);
    assert!(名簿.set_note(鍵([7u8; 32]), &長い).is_err());
}

#[test]
fn 覚え書きの無い旧版も読める() {
    // v1 / v2 の行にはそもそも欄が無い
    let dir = 仮の置き場("contacts-v2-note");
    let vault = Vault::at(&dir);
    vault.open_seed().unwrap();
    fs::write(
        vault.contacts_path(),
        format!("warifu-contacts-v2\n{}\tair\t100\t\n", 鍵([8u8; 32])),
    )
    .unwrap();
    let 名簿 = vault.contacts().unwrap();
    assert_eq!(名簿.find(鍵([8u8; 32])).unwrap().note(), "");
    fs::remove_dir_all(&dir).ok();
}
