//! **覚えた相手を、割符なしで呼ぶ。**
//!
//! ```text
//!   呼ぶ側                         呼ばれる側（画面が開いていれば待ち受けている）
//!     覚えた住所へ繋ぐ
//!     目印を 1 通 送る    ────►    割符として読めない → 戸口へ回す
//!                                  知り合いなら開ける（D31）／でなければ黙って落とす
//!                        ◄────    Notice::Invite { meeting, roster }
//!     その会議へ入る
//!     Notice::Join を送る ────►
//! ```
//!
//! **会議 id を自分で作らない。**作ると相手の会議と別物になり、
//! 送った知らせが「別の会議あて」として捨てられる（2026-09-04 に実機で踏んだ）。
//!
//! **繋がらなかった理由を分けない。**「居ない」も「断られた」も同じ言い分を返す ——
//! 押した人は叩いた人なので、分けると D31 の「断る理由を相手に返さない」が
//! 画面越しに崩れる。

use std::str::FromStr as _;

use tauri::AppHandle;
use warifu_app::{Conference, KNOCK_WITHOUT_TALLY, knock_for_room};
use warifu_core::{PublicKey, Revocations};
use warifu_intent::Channel;
use warifu_meeting::{Notice, Roster};
use warifu_net::Address;

use crate::{Answer, Bridge, Failure, contacts, emit_events, key_to_string, 始める};

/// 招待を待つ長さ。**永遠に待たない。**
///
/// 相手が居ないのか、戸口に断られたのかは分けない（どちらもこれで終わる）。
const 招待を待つ秒: u64 = 10;

/// 繋がらなかったときの言い分。**1 か所に置く。**
fn 居ません() -> Failure {
    Failure {
        message: "いま居ません".into(),
        code: Some("contact.unreachable".into()),
    }
}

/// 覚えた相手を呼ぶ。
pub async fn 呼ぶ(app: &AppHandle, bridge: &Bridge, 相手: PublicKey) -> Answer<()> {
    // **住所を知らないなら、繋ぎに行かずにすぐ返す。**押した人を待たせない（D49）
    let 住所 = contacts::住所を引く(相手)?;
    住所へ呼ぶ(app, bridge, 相手, &住所, None).await
}

/// **住所を手に持っているときに呼ぶ。**
///
/// 覚えた相手（[`呼ぶ`]）のほかに、**帰り道の鍵から住所だけ取り出して呼ぶ**道がある
/// （**#21** / **#13**・2026-09-18）——
/// **割符は 24 時間で切れるが、戸口の「知り合い」は消えない。**
/// **切れた鍵でも、前に通してもらった部屋へは戻れるべきである。**
///
/// **戻るときは、戻りたい部屋を添えて叩く**（0.1.41）。主催はその部屋を主催していればそこへ招き、
/// していなければ「もう無い」と返して部屋を建てない（[`招く先を決める`]）。
pub async fn 住所へ呼ぶ(
    app: &AppHandle,
    bridge: &Bridge,
    相手: PublicKey,
    住所: &str,
    戻りたい: Option<warifu_meeting::MeetingId>,
) -> Answer<()> {
    let 住所 = 住所.to_owned();
    記録!(
        "呼ぶ: 覚えた住所へ（{}）",
        crate::短く(&key_to_string(相手))
    );

    let node = bridge.node().await?;
    let 宛先 = Address::from_str(&住所).map_err(|_| 居ません())?;
    let mut session = node
        .connect(&宛先, &Revocations::new())
        .await
        .map_err(|_| 居ません())?;
    let peer = session.peer();
    if peer != 相手 {
        // **別人が同じ住所に居る。**呼んだ相手ではないので繋がない
        記録!("呼ぶ: 住所の先が別人だった");
        return Err(居ません());
    }

    // **「割符は持っていない」と名乗る。**何も送らないと、相手は 10 秒待ってから諦める。
    // 戻るときは戻りたい部屋も添える（古い主催は添え物を見ずに今までどおり扱う）
    let 叩き = match 戻りたい {
        Some(部屋) => knock_for_room(&部屋.to_string()),
        None => KNOCK_WITHOUT_TALLY.to_vec(),
    };
    session.send(&叩き).await.map_err(|_| 居ません())?;
    match 戻りたい {
        Some(部屋) => 記録!(
            "呼ぶ: 割符なしで叩いた（戻りたい部屋 {}）",
            crate::短く(&部屋.to_string())
        ),
        None => 記録!("呼ぶ: 割符なしで叩いた"),
    }

    // **呼んだ相手は、こちらの戸口にも迎える。**
    // そうしないと、時間をおいて相手から呼ばれたときに**無言で断る**
    // （2026-09-11 に気づいた片方向）
    {
        let mut door = bridge.door.lock().await;
        contacts::迎える(&mut door, 相手);
    }

    // **映像の道を足す**（**D125**）。会話に渡す前に口を取り出す
    crate::media_path::足す(session.包みの口());
    let mut channel = Channel::new(session);
    let (meeting, roster) = 招待を待つ(&mut channel).await?;
    let roster = 自分を名簿に足す(roster, bridge.device.public_key());
    記録!(
        "呼ぶ: 招かれた（会議 {}）",
        crate::短く(&meeting.to_string())
    );

    // 影へ流す分を先に写しておく（名簿は部屋へ渡して手放す）
    let (影の名簿, 影の定員) = (roster.members().to_vec(), roster.capacity());
    let events = {
        crate::ルームを足す(
            &bridge.conferences,
            &bridge.いまのルーム,
            Conference::joined(bridge.device.public_key(), meeting, roster),
        )
        .await;
        // **呼んだ相手を、この部屋の主催として覚える**（`connect` と同じ・**#37**）。
        // 覚えないと、ゲストなのに「あなたが建てた」と出て、
        // **帰り道の見張りが「自分の部屋」とみなし、次に主催が落ちても戻らない**
        // （2026-10-21・割符なしで戻った機械で実測）
        bridge.主催たち.lock().await.insert(meeting, peer);
        crate::影へ(
            bridge,
            warifu_app::room::入り口::入った {
                部屋: meeting,
                主催: peer,
                名簿: 影の名簿,
                定員: 影の定員,
                見る: false,
            },
        )
        .await;
        vec![warifu_app::Event::Joined(peer)]
    };
    // **鍵なしで入った部屋にも、帰り道を書き置く**（0.1.35）。
    // 書かないと、主催が上がり直したとき**見張りが戻る先を持たず、ひとりのまま**だった。
    // 中身は住所だけで、割符は入らない（戻るときも鍵なしで叩く）
    if let Ok(vault) = warifu_vault::Vault::default_location()
        && let Err(e) = vault.save_rejoin(&meeting.to_string(), &住所)
    {
        記録!("帰り道を書き置けませんでした: {e}");
    }
    emit_events(app, meeting, &events);

    // 持っている所を先に言う（0.1.42・主催は持っていない分を送り直す）
    crate::持っている所を言う(bridge, &mut channel, meeting).await;
    channel
        .send(&Notice::Join { meeting }.to_intent()?)
        .await
        .map_err(|_| 居ません())?;

    // **名乗りも渡す**（**D75**）。相手の画面に、こちらの名前と紹介が出る
    if let Ok(名乗り) = crate::自分の名乗り() {
        let _ = channel
            .send(
                &Notice::Profile {
                    meeting,
                    from: bridge.device.public_key(),
                    名前: 名乗り.0,
                    紹介: 名乗り.1,
                }
                .to_intent()?,
            )
            .await;
    }

    // **自分の住所を名乗る。**相手は経路からこちらの住所を知れない（D41 と同じ理由）
    if let Ok(自分の住所) = node.address().await {
        let _ = channel
            .send(
                &Notice::Introduce {
                    meeting,
                    who: bridge.device.public_key(),
                    address: 自分の住所.to_string(),
                }
                .to_intent()?,
            )
            .await;
    }

    始める(app, bridge, channel, peer).await;
    Ok(())
}

/// **招かれた名簿に、自分を足す。**
///
/// 招く側は**自分の名簿**（先頭が招く側・呼んだ側はまだ居ない）を送ってくる。
/// そのまま使うと、**呼んだ側の画面に自分が数えられず**、相手と話していても
/// 「（1 人）まだ自分だけです」と出ていた（2026-10-21）。`connect` は自分から名簿を組むので起きない。
/// 満員なら足さない（入れたかは相手の戸口が決めている）。
///
/// **自分を先頭に組み直す**（0.1.38）。名簿の先頭は外せない（`Roster::remove`）ので、
/// 主催を先頭のまま使うと、主催が上がり直しても「居ない」と告げても客の名簿から主催が消えず、
/// 客は「居るつもり」のまま戻らなかった。鍵で入るとき（`connect`）は前から自分が先頭だった。
pub(crate) fn 自分を名簿に足す(roster: Roster, 自分: PublicKey) -> Roster {
    let Ok(mut 組み直し) = Roster::with_capacity(自分, roster.capacity()) else {
        // 定員が読めないときは、相手の名簿のまま使う（入る・入らないは相手の戸口が決める）
        return roster;
    };
    for k in roster.members() {
        if *k != 自分 {
            // 満員で足せない分は落とす（相手の戸口が決めた人数を超えない）
            let _ = 組み直し.add(*k);
        }
    }
    組み直し
}

/// 相手が会議へ招くのを待つ。
async fn 招待を待つ(channel: &mut Channel) -> Answer<(warifu_meeting::MeetingId, Roster)> {
    let 待つ = std::time::Duration::from_secs(招待を待つ秒);
    let Ok(Ok(intent)) = tokio::time::timeout(待つ, channel.recv()).await else {
        // 戸口に断られたときは、相手が黙って口を閉じる（D31）。**理由は返ってこない**
        return Err(居ません());
    };
    match Notice::from_intent(&intent) {
        Ok(Notice::Invite { meeting, roster }) => Ok((meeting, roster)),
        // **「その部屋はもう無い」**（0.1.41）。戻りたい部屋を添えて叩いたのに、主催がその部屋を主催していない
        Ok(Notice::Leave { meeting }) => Err(もう無い(meeting)),
        // 招待以外が来た。**知らせの種類を勝手に読み替えない**
        _ => Err(居ません()),
    }
}

/// 主催に「その部屋はもう無い」と言われた。**戻る側は 1 回で帰り道を消す**（叩き続けない）。
pub const もう無いの印: &str = "room.gone";

fn もう無い(部屋: warifu_meeting::MeetingId) -> Failure {
    Failure {
        message: format!(
            "部屋 {} は、主催がもう主催していません",
            crate::短く(&部屋.to_string())
        ),
        code: Some(もう無いの印.into()),
    }
}

// **招く先の決め方は warifu-app にある**（計画 1-B・0.1.41）。机上の網（`crates/warifu-app/tests/room_sim.rs`）が
// 同じ関数を動かすので、ここに写しを持たない
pub use warifu_app::room::{招く先, 招く先を決める, 招く部屋を選ぶ};

/// 迎える側が、割符なしで通した相手へ**会議を教える**。
///
/// `Notice::Invite` は型も試験も前からあったのに、**誰も送っていなかった。**
/// 割符つきで来た相手には送らない —— そちらは会議キーに id が入っている。
///
/// 招く部屋は [`招く部屋を選ぶ`] で決めてから渡す。
pub fn 招く(
    conferences: &crate::ルームたち, id: warifu_meeting::MeetingId
) -> Option<Notice> {
    let 棚 = conferences.try_lock().ok()?;
    let c = 棚.get(&id)?;
    let mut roster = Roster::with_capacity(c.me(), c.capacity()).ok()?;
    for m in c.members() {
        if *m != c.me() {
            roster.add(*m).ok()?;
        }
    }
    Some(Notice::Invite {
        meeting: c.id(),
        roster,
    })
}

/// **部屋の合言葉の証しを見せて、同じ部屋のゲストを呼ぶ**（**D118** / **#28**）。
///
/// ```text
///   呼ぶ側（ゲスト A）                   呼ばれる側（ゲスト B）
///     教わった住所へ繋ぐ
///     部屋の叩きを 1 通 送る   ────►    割符ではない → 部屋の証しとして検める
///                                       合えば開ける（**知り合いには入れない**）
///     その部屋へ相手を足す
///     Notice::Join を送る      ────►
/// ```
///
/// # なぜ `呼ぶ`（割符なし）と別なのか
///
/// **あちらは「相手がこちらを覚えていること」に頼っている。**
/// ゲスト同士は互いを覚えていないので、**それでは通らない** ——
/// 2026-09-17 に 3 台で測って出た：`割符が付いていません（宛先だけでは繋げません）`。
///
/// # なぜ招待を待たないのか
///
/// **もう同じ部屋に居る。**部屋 id は主催から教わっている（**D111** の名簿）ので、
/// **待つと、相手が招待を送らない限り繋がらない**ことになる。
///
/// # 繋がらなかった理由を分けない
///
/// `呼ぶ` と同じ。**「居ない」も「断られた」も同じ言い分**を返す（**D31**）。
pub async fn 部屋の合言葉で呼ぶ(
    app: &AppHandle,
    bridge: &Bridge,
    相手: PublicKey,
    住所: &str,
    部屋: warifu_meeting::MeetingId,
) -> Answer<()> {
    // **合言葉を持っていなければ、繋ぎに行かない。**
    // 行っても戸口で落ちるので、**押した人を待たせない**（D49 と同じ筋）
    let 証し = {
        let 棚 = bridge.部屋の合言葉.lock().await;
        let Some(言) = 棚.get(&部屋) else {
            記録!(
                "呼ぶ: この部屋の合言葉がありません（部屋 {}）",
                crate::短く(&部屋.to_string())
            );
            return Err(居ません());
        };
        言.証しを作る(
            部屋.to_string().as_bytes(),
            bridge.device.public_key(),
            相手,
        )
    };

    記録!(
        "呼ぶ: 部屋の証しを見せて（{}・部屋 {}）",
        crate::短く(&key_to_string(相手)),
        crate::短く(&部屋.to_string())
    );

    let node = bridge.node().await?;
    let 宛先 = Address::from_str(住所).map_err(|_| 居ません())?;
    let mut session = node
        .connect(&宛先, &Revocations::new())
        .await
        .map_err(|_| 居ません())?;
    let peer = session.peer();
    if peer != 相手 {
        // **別人が同じ住所に居る。**証しはこの相手に縛ってあるので、出しても通らない
        記録!("呼ぶ: 住所の先が別人だった");
        return Err(居ません());
    }

    session
        .send(&warifu_core::部屋の叩き::new(部屋.to_string().as_bytes(), 証し).to_bytes())
        .await
        .map_err(|_| 居ません())?;
    記録!("呼ぶ: 部屋の証しを差し出した");

    // **相手を、いま居る部屋の名簿へ足す。**
    // 部屋を作り直さない —— **もう入っている部屋**である
    let events = 部屋へ足す(bridge, 部屋, peer).await?;
    emit_events(app, 部屋, &events);

    // **映像の道を足す**（**D125**）。会話に渡す前に口を取り出す
    crate::media_path::足す(session.包みの口());
    let mut channel = Channel::new(session);
    channel
        .send(&Notice::Join { meeting: 部屋 }.to_intent()?)
        .await
        .map_err(|_| 居ません())?;

    // **名乗りも渡す**（**D75**）
    if let Ok(名乗り) = crate::自分の名乗り() {
        let _ = channel
            .send(
                &Notice::Profile {
                    meeting: 部屋,
                    from: bridge.device.public_key(),
                    名前: 名乗り.0,
                    紹介: 名乗り.1,
                }
                .to_intent()?,
            )
            .await;
    }

    // **住所は名乗らない。**
    //
    // `呼ぶ` は名乗るが、こちらは名乗らない —— **相手はもう主催から教わっている**
    // （D111 の名簿）。名乗ると、相手が「新入りが名乗った」と読んで
    // **紹介を配り直し、往復が増える**（0.1.5 で 177 回捨てた形）。

    始める(app, bridge, channel, peer).await;
    Ok(())
}

/// いま居る部屋の名簿へ、相手を足す。**部屋を作り直さない。**
async fn 部屋へ足す(
    bridge: &Bridge,
    部屋: warifu_meeting::MeetingId,
    相手: PublicKey,
) -> Answer<Vec<warifu_app::Event>> {
    let mut 棚 = bridge.conferences.lock().await;
    let Some(c) = 棚.get_mut(&部屋) else {
        // **部屋が無い。**合言葉を持っているのに部屋が無いのは、抜けた直後くらいである
        return Ok(Vec::new());
    };
    // **もう名簿に居るなら、何も起きない。**冪等にしておく ——
    // 紹介が 2 度来ることはあり、そのたびに「入った」を画面へ流すと行が二重になる
    if c.members().contains(&相手) {
        return Ok(Vec::new());
    }
    c.admit(相手).map_err(|e| Failure {
        message: e.to_string(),
        code: None,
    })?;
    Ok(vec![warifu_app::Event::Joined(相手)])
}

#[cfg(test)]
mod 試験 {
    #[test]
    fn 添えられた部屋を主催していればそこへ_していなければもう無い_添えが無ければ選ぶ() {
        assert_eq!(
            super::招く先を決める(Some(1), &[1, 2]),
            super::招く先::その部屋(1)
        );
        assert_eq!(
            super::招く先を決める(Some(3), &[1, 2]),
            super::招く先::もう無い(3)
        );
        assert_eq!(
            super::招く先を決める(Some(3), &[]),
            super::招く先::もう無い(3)
        );
        assert_eq!(super::招く先を決める::<u8>(None, &[1]), super::招く先::選ぶ);
    }

    use super::*;

    fn 鍵(n: u8) -> PublicKey {
        warifu_core::Seed::from_bytes([n; 32])
            .profile("Personal")
            .device("PC")
            .public_key()
    }

    #[test]
    fn 見ている部屋を主催していれば_そこへ招く() {
        assert_eq!(招く部屋を選ぶ(Some(2), &[1, 2]), Some(2));
    }

    #[test]
    fn 他人の部屋を見ていても_自分の部屋へ招く() {
        // 3 はゲストとして入っている部屋（主催していない）
        assert_eq!(招く部屋を選ぶ(Some(3), &[1, 2]), Some(1));
        assert_eq!(招く部屋を選ぶ(None, &[1]), Some(1));
    }

    #[test]
    fn 主催している部屋が無ければ_建てる() {
        assert_eq!(招く部屋を選ぶ::<u8>(Some(3), &[]), None);
    }

    #[test]
    fn 招かれた名簿に_呼んだ自分が数えられる() {
        let 名簿 = 自分を名簿に足す(Roster::new(鍵(1)), 鍵(2));
        assert_eq!(名簿.len(), 2);
        assert!(名簿.contains(&鍵(1)));
        assert!(名簿.contains(&鍵(2)));
    }

    #[test]
    fn 招かれた名簿は自分が先頭で_主催を外せる() {
        // **名簿の先頭は外せない**（`Roster::remove`）。主催を先頭のまま使うと、
        // 主催が上がり直しても、「居ない」と告げても、客の名簿から主催が消えず、
        // 客は「居るつもり」のまま戻らなかった（0.1.37・机上の網で見つけた）。
        // 鍵で入るとき（`connect`）と同じく、自分を先頭に組み直す
        let mut 名簿 = 自分を名簿に足す(Roster::new(鍵(1)), 鍵(2));
        assert_eq!(名簿.host(), 鍵(2));
        assert!(名簿.remove(&鍵(1)), "主催を外せる");
    }

    #[test]
    fn もう居るなら_二度は足さない() {
        let mut 元 = Roster::new(鍵(1));
        元.add(鍵(2)).unwrap();
        assert_eq!(自分を名簿に足す(元, 鍵(2)).len(), 2);
    }
}
