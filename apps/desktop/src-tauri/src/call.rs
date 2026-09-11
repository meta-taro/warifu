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
