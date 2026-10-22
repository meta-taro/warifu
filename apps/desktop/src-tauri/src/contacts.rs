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

/// 鍵の頭で引くときの、いちばん短い長さ。これより短いと、たまたま当たるだけになる。
const 鍵の頭の下限: usize = 4;

/// **エージェントが言った「誰を」から、呼ぶ相手の鍵を 1 つに決める**（2026-10-16）。
///
/// 帳は（鍵, 呼び名）の組。名前がぴったり合えばそれ、なければ鍵の頭で引く。
/// **2 人に当たるなら決めない** —— 当て推量で呼ぶと、違う人の画面が動く。
/// 返す `Err` は人が読む訳。
pub fn 呼ぶ相手を決める(帳: &[(String, String)], 指定: &str) -> Result<String, String> {
    let 指定 = 指定.trim();
    let 名で: Vec<&(String, String)> = 帳.iter().filter(|(_, 名)| 名.trim() == 指定).collect();
    let 当たり = if 名で.is_empty() && 指定.chars().count() >= 鍵の頭の下限 {
        帳.iter().filter(|(鍵, _)| 鍵.starts_with(指定)).collect()
    } else {
        名で
    };
    match 当たり.as_slice() {
        [(鍵, _)] => Ok(鍵.clone()),
        [] => Err(format!(
            "「{指定}」は連絡先にいません（contacts_list で名前を確かめてください）"
        )),
        多い => Err(format!(
            "「{指定}」は {} 人に当たります。名前か鍵をもっと長く書いてください",
            多い.len()
        )),
    }
}

/// **相手の住所から、連絡帳に名前と住所を書く**（0.1.33）。足した相手の鍵を返す。
///
/// 会えない相手（受けの口が防火壁に止められている機械など）は、名前を付ける場が無かった。
/// **自分自身は足さない**（自分を呼んでも意味が無く、戸口に自分を載せることになる）。
/// 名前が他の相手と重なれば連絡帳が断る。返す `Err` は人が読む訳。
pub fn 住所から足す(
    帳: &mut warifu_vault::Contacts,
    自分: PublicKey,
    住所: &str,
    名前: &str,
    今: u64,
) -> Result<PublicKey, String> {
    let 宛先 = Address::from_str(住所.trim())
        .map_err(|_| "住所として読めません（WARIFU1-… の住所を渡してください。ルームキーではありません）".to_owned())?;
    let 相手 = 宛先.public_key();
    if 相手 == 自分 {
        return Err("この PC 自身の住所です。自分は連絡先に足せません".to_owned());
    }
    // **既にいる相手は書き換えない**（審査）。名前や住所を黙ってすり替えさせない。
    // 付け直すのは人が画面で行う
    if let Some(行) = 帳.find(相手) {
        return Err(format!("既に「{}」として連絡先にいます", 行.label()));
    }
    let 名前 = 名前.trim();
    // **向きを変える字などの見えない書式文字を、名前に入れさせない**（通知を読み違えさせない）
    if 名前.chars().any(見えない書式文字か) {
        return Err("名前に見えない書式文字（向きを変える字など）は使えません".to_owned());
    }
    // **他の相手の鍵の頭と同じ名前にさせない**（名前が鍵の頭より先に当たるので、呼び先がずれる）
    if 名前.chars().count() >= 鍵の頭の下限
        && 帳.iter().any(|c| c.key().to_string().starts_with(名前))
    {
        return Err("他の相手の鍵の頭と同じ名前は付けられません".to_owned());
    }
    帳.add(相手, 名前, 今)
        .map_err(|e| format!("連絡先に足せません: {e}"))?;
    帳.note_address(相手, &宛先.to_string())
        .map_err(|e| format!("住所を書けません: {e}"))?;
    Ok(相手)
}

/// 見えない書式文字（幅ゼロ・向きを変える字・BOM）か。
fn 見えない書式文字か(c: char) -> bool {
    matches!(
        c,
        '\u{200B}'..='\u{200F}'
            | '\u{202A}'..='\u{202E}'
            | '\u{2060}'..='\u{2064}'
            | '\u{2066}'..='\u{2069}'
            | '\u{FEFF}'
    )
}

/// **書き出す会話の「誰が」を、人が読める形にする**（2026-10-16）。
///
/// 帳は（鍵, 呼び名）の組。線の向こうの人は鍵の全桁で、線の向こうのエージェントは
/// 「鍵の頭…（名札）」で来る。覚えている相手なら呼び名を前に出す。
/// この機械の名札（「◯◯ のエージェント」）や覚えていない相手は、そのまま返す。
pub fn 書き出しの名(帳: &[(String, String)], 自分: &str, 誰: &str) -> String {
    // **この PC の人が打った行は、この PC の鍵で来る。**鍵のままでは誰か読めない
    if 誰 == 自分 {
        let 頭: String = 自分.chars().take(12).collect();
        return format!("この PC の人（{頭}…）");
    }
    for (鍵, 名) in 帳 {
        if 誰 == 鍵 {
            let 頭: String = 鍵.chars().take(12).collect();
            return format!("{名}（{頭}…）");
        }
        let 頭: String = 鍵.chars().take(12).collect();
        if let Some(札) = 誰.strip_prefix(&format!("{頭}…")) {
            return format!("{名}{札}");
        }
    }
    誰.to_owned()
}

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

/// **自分から呼んだ相手を、戸口に迎える。**
///
/// 2026-09-11 まで、知り合いが増えるのは**受けたとき**だけだった ——
/// A が B を呼ぶと B の戸口は A を知るが、**A の戸口は B を知らない。**
/// 時間をおいて B が A を呼ぶと、A は無言で断っていた（**片方向**）。
///
/// **自分から呼ぶのは「通してよい」という意思表示である。**
/// 迎えた相手は `known.tsv` へ書き置く（画面から降ろせる）。
pub fn 迎える(door: &mut Door, key: PublicKey) {
    let Some(who) = Subject::new(&key_to_string(key)) else {
        return;
    };
    if door.welcome(who) {
        戸口を書き置く(&door.known().map(str::to_owned).collect::<Vec<_>>());
        記録!("戸口: 自分から呼んだ相手を迎えた（{}）", crate::短く(&key_to_string(key)));
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
    会ったと書き留める(相手, 住所);
    書き留める(相手, 住所)
}

/// **呼び名の無い相手の住所も、別の帳に残す**（0.1.32）。
///
/// 連絡帳は人が名前を付けた相手だけを持つ（上の 3）。そのため、ルームで会っても
/// 名前を付けなかった相手は、**鍵を運ばないと二度と呼べなかった。**
/// こちらは呼ぶときの宛先にだけ使い、表示の名前には使わない（D46）。
fn 会ったと書き留める(相手: PublicKey, 住所: &str) {
    let 結果 = Vault::default_location().and_then(|v| v.save_met(相手, 住所));
    if let Err(e) = 結果 {
        // **握り潰さない。**書けなければ、名前の無い相手は鍵なしで呼べないまま
        記録!("会った相手の住所を書き留められませんでした: {e}");
    }
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
    let 置き場 = Vault::default_location().map_err(|e| Failure {
        message: e.to_string(),
        code: None,
    })?;
    let 名簿 = 置き場.contacts().map_err(|e| Failure {
        message: e.to_string(),
        code: None,
    })?;
    // **名前を付けた相手の住所を先に使う。**無ければ、ルームで会ったときの住所（0.1.32）
    let 会った = || -> Option<String> {
        置き場
            .met()
            .ok()?
            .into_iter()
            .find(|(鍵, _)| *鍵 == 相手)
            .map(|(_, 住所)| 住所)
    };
    let 住所 = 名簿
        .find(相手)
        .and_then(|c| c.address().map(str::to_owned))
        .or_else(会った)
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

/// **その紹介で、相手を呼びに行かせてよいか。**
///
/// # なぜ要るか
///
/// 2026-09-12、Windows で踏んだ（`gh issue 9`）——
/// **画面からルームに入れない。**入室は成功するのに、直後に経路が `lost` になる。
/// ログには `受信: 紹介` と `住所: 名乗りが経路の相手と違う。捨てた` が
/// **177 回**並んでいた。
///
/// 原因は**紹介のピンポン**である ——
///
/// ```text
/// 画面が紹介を受ける → connect(住所) を呼ぶ
///   → connect は「自分の住所」を Introduce で名乗る（D41）
///     → 相手の画面が紹介を受ける → connect を呼ぶ → …（際限なく）
/// ```
///
/// **`who == peer` は「第三者の紹介」ではない。**
/// 「**いま繋がっているこの相手が、自分の住所を名乗った**」だけである（`connect` が必ず送る）。
/// **すでに繋がっている相手を呼び直す意味は無い。**
///
/// 呼びに行かせるのは **D41 の第三者紹介**（`who != peer`）だけにする。
#[must_use]
pub fn 呼びに行かせるか(相手: PublicKey, 名乗り: PublicKey) -> bool {
    相手 != 名乗り
}

#[cfg(test)]
mod 紹介の試験 {
    use super::*;
    use warifu_core::Seed;

    fn 鍵(seed: [u8; 32]) -> PublicKey {
        Seed::from_bytes(seed)
            .profile("Personal")
            .device(warifu_core::端末の呼び名)
            .public_key()
    }

    #[test]
    fn 第三者の紹介なら_呼びに行かせる() {
        // **D41 の本体。**主催が「C さんはここです」と教えてくれた場合
        assert!(呼びに行かせるか(鍵([1; 32]), 鍵([2; 32])));
    }

    #[test]
    fn 相手が自分の住所を名乗っただけなら_呼びに行かせない() {
        // **これが 177 回の嵐の元。**呼び直すと、相手も紹介を送り返して往復する
        let 同じ = 鍵([1; 32]);
        assert!(!呼びに行かせるか(同じ, 同じ));
    }
}

/// **その紹介を、ほかの人へ配ってよいか**（**D41**）。
///
/// # なぜ要るか
///
/// 2026-09-13、3 台で実測して分かった —— **Mac Air から 66 ミリ秒で約 130 件の
/// 紹介が来て、直後に経路が `lost` になった。**全部 `who != peer`
/// （**第三者についての紹介**）である。
///
/// ```text
/// mini（主催）が Air へ「ASUS はここ」を配る
///   → Air も自分を主催だと思って、mini へ「ASUS はここ」を配り返す
///     → mini が受けて、また配る → 際限なく往復
/// ```
///
/// **`Conference` の主催は「名簿の先頭が自分か」で決めている。**入った側の名簿でも
/// 自分が先頭に来ることがあり、**両側が「自分が主催だ」と思う。**
///
/// # 決めたこと
///
/// **配るのは「新入りが自分の住所を名乗った」ときだけ**（`who == peer`）。
///
/// - 自己紹介（`who == peer`）…… 主催が受けて、**ほかの人へ配る**（D41 の本体）
/// - 第三者の紹介（`who != peer`）…… **配り直さない。**受けて使うだけ
///
/// **どちら側が主催だと思っていても、往復しない。**
#[must_use]
pub fn 配ってよいか(主催か: bool, 相手: PublicKey, 名乗り: PublicKey) -> bool {
    主催か && 相手 == 名乗り
}

/// **帰り道が「住所だけ」か**（0.1.35）。鍵なしで入った部屋は、住所を帰り道に書く。
///
/// ルームキー（`住所#割符#部屋`）なら `None`。**`#` を含むものは住所として読まない。**
#[must_use]
pub fn 住所だけの帰り道(帰り道: &str) -> Option<Address> {
    if 帰り道.contains('#') {
        return None;
    }
    Address::from_str(帰り道.trim()).ok()
}

/// **主催が、新入りへ自分の住所を名乗り返すか**（0.1.35）。
///
/// 入る側は自分の住所を名乗るが、**主催は名乗っていなかった。**そのため新入りは
/// 主催の住所を「入るときに使った住所」でしか知らず、同じ網でしか使えない住所だと、
/// 外から呼び直せなかった。
///
/// **1 本の経路・1 部屋につき 1 度だけ返す。**両側が自分を主催だと思っていると、
/// 名乗りに名乗りで返して往復する（2026-09-13 に配り直しで踏んだ形）。
#[must_use]
pub fn 名乗り返すか(主催か: bool, 相手: PublicKey, 名乗り: PublicKey, もう返した: bool) -> bool {
    主催か && 相手 == 名乗り && !もう返した
}

/// **その相手が、この部屋の主催か**（**#37**・2026-09-16）。
///
/// # なぜ名簿の先頭で見てはいけないか
///
/// **ゲストの名簿は、先頭が自分である。**
///
/// ```text
/// 主催側   Roster::with_capacity(自分) → 先頭は主催（＝自分）
/// ゲスト側 Roster::with_capacity(自分) → **先頭は自分**。主催は 2 番目
/// ```
///
/// だから `members().first() == Some(&相手)` で主催を見ると、
/// **ゲストでは必ず false** になる。**D111 の迎えが一度も起きなかった**のはこれ（#37）——
/// Mac Air の実測で **08 は落ち・12 の行も出ない**だった。
///
/// **主催は「入るときに呼んだ相手」である。**それを覚えておいて、ここで突き合わせる。
#[must_use]
pub fn 主催と同じか(主催: Option<PublicKey>, 誰: PublicKey) -> bool {
    主催 == Some(誰)
}

/// **その紹介を信じて、戸口で通してよいか**（**D111**・2026-09-15）。
///
/// 3 台で測ったら**ルームが星形だった** —— 割符を持っているのは主催だけなので、
/// **ゲスト A がゲスト B を呼んでも、B の戸口が黙って落とす**（#28）。
///
/// **信じるのは主催の言い分だけ。**しかも**第三者についての紹介のときだけ** ——
/// 相手が自分の住所を名乗っただけなら、**戸口に足す理由が無い**
/// （割符を持って叩いてくるので、そこで通る）。
///
/// **通るのはそのルームの中だけ。**戸口の書き置きには入れない（`Door::名簿で迎える`）。
#[must_use]
pub fn 名簿として迎えるか(紹介者が主催か: bool, 紹介者: PublicKey, 名乗り: PublicKey) -> bool {
    紹介者が主催か && 紹介者 != 名乗り
}

#[cfg(test)]
mod 配る試験 {
    use super::*;
    use warifu_core::Seed;

    fn 鍵(seed: [u8; 32]) -> PublicKey {
        Seed::from_bytes(seed)
            .profile("Personal")
            .device(warifu_core::端末の呼び名)
            .public_key()
    }

    #[test]
    fn 新入りの自己紹介は_主催が配る() {
        // **D41 の本体。**入ってきた人の住所を、既に居る人へ教える
        let 同じ = 鍵([1; 32]);
        assert!(配ってよいか(true, 同じ, 同じ));
    }

    #[test]
    fn 主催は新入りへ_一度だけ名乗り返す() {
        let 新入り = 鍵([1; 32]);
        let 別 = 鍵([2; 32]);
        assert!(名乗り返すか(true, 新入り, 新入り, false));
        // 二度目は返さない（往復しない）
        assert!(!名乗り返すか(true, 新入り, 新入り, true));
        // 主催でなければ返さない
        assert!(!名乗り返すか(false, 新入り, 新入り, false));
        // 第三者の紹介には返さない
        assert!(!名乗り返すか(true, 新入り, 別, false));
    }

    #[test]
    fn 主催を覚えていれば_突き合わせられる() {
        // **ゲストの名簿は先頭が自分**なので、先頭では判断できない（#37）
        let 主催 = 鍵([1; 32]);
        let ゲスト = 鍵([2; 32]);
        assert!(主催と同じか(Some(主催), 主催));
        assert!(!主催と同じか(Some(主催), ゲスト));
    }

    #[test]
    fn 主催を覚えていなければ_信じない() {
        // **分からないものを主催として扱わない**（原則 7）
        assert!(!主催と同じか(None, 鍵([1; 32])));
    }

    #[test]
    fn 主催が言った第三者は_戸口で通す() {
        // **D111 の本体。**これが無いとゲスト同士が繋がらない（#28）
        assert!(名簿として迎えるか(true, 鍵([1; 32]), 鍵([2; 32])));
    }

    #[test]
    fn 主催でない相手の紹介は_戸口で通さない() {
        // **信じるのは主催の言い分だけ。**ゲストが「この人は居る」と言っても通さない
        assert!(!名簿として迎えるか(false, 鍵([1; 32]), 鍵([2; 32])));
    }

    #[test]
    fn 自分の住所を名乗っただけなら_戸口には足さない() {
        // 割符を持って叩いてくるので、そこで通る。**足す理由が無い**
        let 同じ = 鍵([1; 32]);
        assert!(!名簿として迎えるか(true, 同じ, 同じ));
    }

    #[test]
    fn 第三者の紹介は_配り直さない() {
        // **これが 130 件の嵐の元。**受けて使うだけにする
        assert!(!配ってよいか(true, 鍵([1; 32]), 鍵([2; 32])));
    }

    #[test]
    fn 主催でなければ_配らない() {
        let 同じ = 鍵([1; 32]);
        assert!(!配ってよいか(false, 同じ, 同じ));
    }
}

#[cfg(test)]
mod 呼ぶ相手の試験 {
    use super::呼ぶ相手を決める;

    fn 帳() -> Vec<(String, String)> {
        vec![
            ("AAAAMMMMKKKK1111".to_owned(), "相手A".to_owned()),
            ("BBBBQQQQ22223333".to_owned(), "相手B（エージェント）".to_owned()),
            ("BBBBZZZZ44445555".to_owned(), "別の人".to_owned()),
        ]
    }

    #[test]
    fn 名前で引ける() {
        assert_eq!(
            呼ぶ相手を決める(&帳(), "相手A").unwrap(),
            "AAAAMMMMKKKK1111"
        );
    }

    #[test]
    fn 前後の空白と_鍵の頭でも引ける() {
        assert_eq!(
            呼ぶ相手を決める(&帳(), "  相手B（エージェント） ").unwrap(),
            "BBBBQQQQ22223333"
        );
        assert_eq!(
            呼ぶ相手を決める(&帳(), "AAAAMMMM").unwrap(),
            "AAAAMMMMKKKK1111"
        );
    }

    #[test]
    fn 鍵の頭が_2_人に当たるなら決めない() {
        // **当て推量で呼ばない。**違う人の画面が動く
        let 訳 = 呼ぶ相手を決める(&帳(), "BBBB").unwrap_err();
        assert!(訳.contains("2 人"), "{訳}");
    }

    #[test]
    fn 名前が_2_人に当たるなら決めない() {
        let mut 帳 = 帳();
        帳.push(("ZZZZZZZZZZZZ".to_owned(), "相手A".to_owned()));
        assert!(呼ぶ相手を決める(&帳, "相手A").is_err());
    }

    #[test]
    fn 居ない相手は_居ないと言う() {
        let 訳 = 呼ぶ相手を決める(&帳(), "相手C").unwrap_err();
        assert!(訳.contains("連絡先に"), "{訳}");
    }

    #[test]
    fn 短すぎる鍵の頭では引かない() {
        // 1〜3 文字の頭は、たまたま当たるだけである
        assert!(呼ぶ相手を決める(&帳(), "6L7").is_err());
    }
}

#[cfg(test)]
mod 書き出しの名の試験 {
    use super::書き出しの名;

    fn 帳() -> Vec<(String, String)> {
        vec![("AAAAMMMMKKKK1111".to_owned(), "相手A".to_owned())]
    }

    #[test]
    fn 覚えている相手の鍵なら_呼び名を前に出す() {
        assert_eq!(書き出しの名(&帳(), "SELFSELFSELF0000", "AAAAMMMMKKKK1111"), "相手A（AAAAMMMMKKKK…）");
    }

    #[test]
    fn 相手のエージェントの札が付いていても_呼び名を添える() {
        // 線の向こうのエージェントは「鍵の頭（名札）」で来る
        assert_eq!(
            書き出しの名(&帳(), "SELFSELFSELF0000", "AAAAMMMMKKKK…（クロ のエージェント）"),
            "相手A（クロ のエージェント）"
        );
    }

    #[test]
    fn この機械の人は_そう書く() {
        assert_eq!(
            書き出しの名(&帳(), "SELFSELFSELF0000", "SELFSELFSELF0000"),
            "この PC の人（SELFSELFSELF…）"
        );
    }

    #[test]
    fn 覚えていない相手と_この機械の名札はそのまま() {
        assert_eq!(書き出しの名(&帳(), "SELFSELFSELF0000", "ZZZZZZZZZZZZ9999"), "ZZZZZZZZZZZZ9999");
        assert_eq!(書き出しの名(&帳(), "SELFSELFSELF0000", "warifu のエージェント"), "warifu のエージェント");
    }
}

#[cfg(test)]
mod 住所から足す試験 {
    use super::*;
    use warifu_core::Seed;

    fn 鍵(b: u8) -> PublicKey {
        Seed::from_bytes([b; 32])
            .profile("Personal")
            .device(warifu_core::端末の呼び名)
            .public_key()
    }

    fn 住所(b: u8) -> String {
        Address::from_ip_addrs(鍵(b), ["192.0.2.1:4000".parse().expect("番地")]).to_string()
    }

    #[test]
    fn 帰り道が住所だけなら_その相手へ叩ける() {
        let 宛先 = 住所だけの帰り道(&住所(2)).expect("住所として読める");
        assert_eq!(宛先.public_key(), 鍵(2));
        // ルームキーは住所として読まない（割符で入る道を通す）
        assert!(住所だけの帰り道(&format!("{}#割符#部屋", 住所(2))).is_none());
        assert!(住所だけの帰り道("読めない").is_none());
    }

    #[test]
    fn 住所から_名前と住所が連絡帳に入る() {
        let mut 帳 = warifu_vault::Contacts::new();
        let 相手 = 住所から足す(&mut 帳, 鍵(1), &住所(2), "相手B", 10).expect("足せる");
        assert_eq!(相手, 鍵(2));
        let 行 = 帳.find(鍵(2)).expect("在る");
        assert_eq!(行.label(), "相手B");
        assert_eq!(行.address(), Some(住所(2).as_str()));
    }

    #[test]
    fn 自分の住所や_読めない住所や_ルームキーは足さない() {
        let mut 帳 = warifu_vault::Contacts::new();
        assert!(住所から足す(&mut 帳, 鍵(1), &住所(1), "自分", 10).is_err());
        assert!(住所から足す(&mut 帳, 鍵(1), "WARIFU1-AAAA", "相手", 10).is_err());
        let 鍵付き = format!("{}#AAAA#BBBB", 住所(2));
        assert!(住所から足す(&mut 帳, 鍵(1), &鍵付き, "相手", 10).is_err());
        assert_eq!(帳.len(), 0);
    }

    #[test]
    fn 既にいる相手は_名前も住所も書き換えない() {
        let mut 帳 = warifu_vault::Contacts::new();
        住所から足す(&mut 帳, 鍵(1), &住所(2), "相手B", 10).expect("足せる");
        let 別の番地 = Address::from_ip_addrs(鍵(2), ["198.51.100.1:5000".parse().expect("番地")]).to_string();
        let 訳 = 住所から足す(&mut 帳, 鍵(1), &別の番地, "別名", 20).unwrap_err();
        assert!(訳.contains("相手B"), "{訳}");
        let 行 = 帳.find(鍵(2)).expect("在る");
        assert_eq!(行.label(), "相手B");
        assert_eq!(行.address(), Some(住所(2).as_str()));
    }

    #[test]
    fn 見えない書式文字や_他の鍵の頭と同じ名前は断る() {
        let mut 帳 = warifu_vault::Contacts::new();
        住所から足す(&mut 帳, 鍵(1), &住所(2), "相手B", 10).expect("足せる");
        assert!(住所から足す(&mut 帳, 鍵(1), &住所(3), "相手\u{202E}C", 10).is_err());
        let 頭: String = 鍵(2).to_string().chars().take(8).collect();
        assert!(住所から足す(&mut 帳, 鍵(1), &住所(3), &頭, 10).is_err());
    }

    #[test]
    fn 名前が他の相手と重なれば断る() {
        let mut 帳 = warifu_vault::Contacts::new();
        住所から足す(&mut 帳, 鍵(1), &住所(2), "同じ名", 10).expect("足せる");
        assert!(住所から足す(&mut 帳, 鍵(1), &住所(3), "同じ名", 10).is_err());
    }
}
