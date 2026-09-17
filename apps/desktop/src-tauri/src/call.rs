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
use warifu_app::{Conference, KNOCK_WITHOUT_TALLY};
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

    // **「割符は持っていない」と名乗る。**何も送らないと、相手は 10 秒待ってから諦める
    session
        .send(KNOCK_WITHOUT_TALLY)
        .await
        .map_err(|_| 居ません())?;
    記録!("呼ぶ: 割符なしで叩いた");

    // **呼んだ相手は、こちらの戸口にも迎える。**
    // そうしないと、時間をおいて相手から呼ばれたときに**無言で断る**
    // （2026-09-11 に気づいた片方向）
    {
        let mut door = bridge.door.lock().await;
        contacts::迎える(&mut door, 相手);
    }

    let mut channel = Channel::new(session);
    let (meeting, roster) = 招待を待つ(&mut channel).await?;
    記録!(
        "呼ぶ: 招かれた（会議 {}）",
        crate::短く(&meeting.to_string())
    );

    let events = {
        crate::ルームを足す(
            &bridge.conferences,
            &bridge.いまのルーム,
            Conference::joined(bridge.device.public_key(), meeting, roster),
        )
        .await;
        vec![warifu_app::Event::Joined(peer)]
    };
    emit_events(app, &events);

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

/// 相手が会議へ招くのを待つ。
async fn 招待を待つ(channel: &mut Channel) -> Answer<(warifu_meeting::MeetingId, Roster)> {
    let 待つ = std::time::Duration::from_secs(招待を待つ秒);
    let Ok(Ok(intent)) = tokio::time::timeout(待つ, channel.recv()).await else {
        // 戸口に断られたときは、相手が黙って口を閉じる（D31）。**理由は返ってこない**
        return Err(居ません());
    };
    match Notice::from_intent(&intent) {
        Ok(Notice::Invite { meeting, roster }) => Ok((meeting, roster)),
        // 招待以外が来た。**知らせの種類を勝手に読み替えない**
        _ => Err(居ません()),
    }
}

/// 迎える側が、割符なしで通した相手へ**会議を教える**。
///
/// `Notice::Invite` は型も試験も前からあったのに、**誰も送っていなかった。**
/// 割符つきで来た相手には送らない —— そちらは会議キーに id が入っている。
pub fn 招く(
    conferences: &crate::ルームたち, いまのルーム: &crate::見ているルーム
) -> Option<Notice> {
    // **いま見ているルームへ招く。**ルームを複数持つので、どこへ招くかを決める必要がある
    let id = (*いまのルーム.try_lock().ok()?)?;
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
    emit_events(app, &events);

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
