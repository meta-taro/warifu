//! 連絡帳と、戸口の知り合い。**画面から呼べる形に直すだけ**（判断は crate 側）。
//!
//! ```text
//!   contacts.tsv   呼び名と、最後に繋がった住所   ← 人が見る
//!   known.tsv      戸口が通してよい相手           ← 戸口が使う
//! ```
//!
//! **2 つを 1 つにしない。**呼び名は表示のため、戸口は判断のためである。
//! 同じにすると、連絡先から呼び名を消した瞬間に相手が入れなくなる（`issues/010`）。

use std::str::FromStr as _;

use warifu_core::PublicKey;
use warifu_door::{Door, Subject};
use warifu_net::Address;
use warifu_vault::Vault;

use crate::{Failure, key_to_string};

/// 置いてある知り合いを連れて、戸口を開ける。
///
/// **読めなくても止めない。**戸口の名簿が壊れていても、
/// 割符さえ渡せば会議はできる。**身元（シード）とは代償が違う**ので、
/// ここは記録に残して空の戸口で立ち上げる。
pub fn 戸口を開く() -> Door {
    let 一覧 = match Vault::default_location().and_then(|v| v.known()) {
        Ok(一覧) => 一覧,
        Err(e) => {
            記録!("戸口の名簿を読めませんでした（誰も知らない状態で始めます）: {e}");
            return Door::new();
        }
    };
    記録!("戸口の名簿を読みました（{} 人）", 一覧.len());
    Door::with_known(
        一覧
            .into_iter()
            .filter_map(|k| Subject::new(&key_to_string(k))),
    )
}

/// 戸口の知り合いを書き置く。
///
/// **ロックを持ったままファイルへ触らない。**呼ぶ側が集めてから渡す。
pub fn 戸口を書き置く(一覧: &[String]) {
    let 鍵たち: Vec<PublicKey> = 一覧.iter().filter_map(|s| s.parse().ok()).collect();
    let 結果 = Vault::default_location().and_then(|v| v.save_known(&鍵たち));
    if let Err(e) = 結果 {
        // **握り潰さない。**書けなければ次の起動で知り合いが減る
        記録!("戸口の名簿を書けませんでした: {e}");
    }
}

/// 相手を戸口から降ろす。**画面から押す。**
///
/// 知り合いを保存した以上、**取り消す口が要る。**
/// 保存する前は、間違って開けた相手もアプリを閉じれば切れていた。
pub fn 降ろす(door: &mut Door, key: PublicKey) -> bool {
    let Some(who) = Subject::new(&key_to_string(key)) else {
        return false;
    };
    let 落とせた = door.forget_known(&who);
    if 落とせた {
        戸口を書き置く(&door.known().map(str::to_owned).collect::<Vec<_>>());
    }
    落とせた
}

/// **最後に繋がった住所**を連絡帳へ書き留める。
///
/// # 名乗りをそのまま信じない
///
/// 相手は「**C さんの住所はここです**」と言える。
/// そのまま書くと、こちらの連絡帳の C の行が書き換わり、
/// **次に人が C の名前を押したとき、別の場所へ呼びに行く。**
/// 割符の「1 本 = 1 人」（**D12**）を、住所の側から迂回されることになる。
///
/// だから 3 つ全部を満たすときだけ書く。
///
/// 1. 名乗った本人が、**経路で確定した相手と同じ**であること
/// 2. 住所の中に入っている公開鍵が、**その相手のもの**であること
/// 3. その相手を**既に覚えている**こと（呼び名が付いている）
///
/// 3 のせいで「まだ呼び名を付けていない相手」の住所は残らない。
/// そこは `remember`（呼び名を付ける）が、同時に住所も書くことで埋める。
pub fn 住所を覚える(相手: PublicKey, 名乗り: PublicKey, 住所: &str) -> bool {
    if 相手 != 名乗り {
        記録!("住所: 名乗りが経路の相手と違う。捨てた");
        return false;
    }
    let Ok(宛先) = Address::from_str(住所) else {
        記録!("住所: 読めない宛先だった。捨てた");
        return false;
    };
    if 宛先.public_key() != 相手 {
        記録!("住所: 宛先に入っている鍵が相手と違う。捨てた");
        return false;
    }
    書き留める(相手, 住所)
}

/// 呼び名を付けるのと同時に、覚えている住所も書く。
///
/// **人の手順は「会って、名前を付ける」のまま。**次からは名前を押すだけになる。
pub fn 書き留める(相手: PublicKey, 住所: &str) -> bool {
    let 結果 = Vault::default_location().and_then(|v| {
        let mut 名簿 = v.contacts()?;
        // **覚えていない相手には書かない**（住所だけの行を作らない）
        if !名簿.note_address(相手, 住所)? {
            return Ok(false);
        }
        v.save_contacts(&名簿)?;
        Ok(true)
    });
    match 結果 {
        Ok(書けた) => 書けた,
        Err(e) => {
            記録!("住所を書き留められませんでした: {e}");
            false
        }
    }
}

/// 覚えている相手の住所を引く。
///
/// # Errors
/// 名簿が読めないとき、覚えていないとき、住所をまだ知らないとき。
pub fn 住所を引く(相手: PublicKey) -> Result<String, Failure> {
    let 名簿 = Vault::default_location()
        .and_then(|v| v.contacts())
        .map_err(|e| Failure {
            message: e.to_string(),
            code: None,
        })?;
    let 住所 = 名簿
        .find(相手)
        .and_then(|c| c.address().map(str::to_owned))
        .ok_or_else(|| Failure {
            message: "この相手の居場所をまだ知りません".into(),
            code: Some("contact.no_address".into()),
        })?;
    // **壊れた行で呼びに行かない。**住所の中の鍵が相手と違えば、居場所を知らないのと同じ
    match Address::from_str(&住所) {
        Ok(宛先) if 宛先.public_key() == 相手 => Ok(住所),
        _ => Err(Failure {
            message: "この相手の居場所をまだ知りません".into(),
            code: Some("contact.no_address".into()),
        }),
    }
}
