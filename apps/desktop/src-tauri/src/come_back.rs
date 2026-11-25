//! **ひとりになったら、自分から戻る**（0.1.28「ずっとつながる」）。
//!
//! ```text
//!   主催が上がり直す ──► ゲストは全員落ちる
//!   ゲストの画面に［前のルームに戻る］が出る ──► **人が押すまで、誰も戻らない**
//! ```
//!
//! 2026-10-20、主催の機械を上げ直したあと、ルームに残ったのは主催の機械の席だけだった。
//! 戻る口はあったが、**押す人が画面の前に居なかった。**
//!
//! だから**ゲストの機械が自分で戻る。**戻る先は帰り道（`rejoin.tsv`・入ったルームの数だけある）で、
//! **自分で抜けたルームには戻らない**（抜けたら、そのルームの帰り道を消している）。
//!
//! **通すかどうかは、相手の戸口が決める**（`rejoin_room` と同じ道を通る）。
//! 断られ続けたら、間を延ばして叩き続ける —— 主催がいつ戻るかは、こちらから分からない。

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter as _, Manager as _};

use crate::{Bridge, RoomRow};

/// 起動してから最初に見るまでの間。**網の口が立ち上がるのを待つ。**
///
/// 0.1.52 までは 20 秒だった。網の口は起動から 1 秒足らずで立つ（記録の「中継: 使います」が +0.5 秒）ので、
/// 20 秒は何もせずに待っているだけだった。部屋 1 つに約 5 秒かかるのと合わせて、
/// 立ち上げ直してから部屋に戻るまで 30 秒〜1 分かかり、その間の発言は「その部屋に居ません」で断られていた（11-20）。
const 起動を待つ秒: u64 = 5;

/// **戻る途中の部屋へ流す発言を、待つ上限**（11-20）。すぐには断らず、戻るまでこの秒数だけ待つ。
///
/// 口（`chat_send`）は返事を 5 秒しか待たないので、それより短くする。長く待つと、口は
/// 「流したか分からない」と返し、席が送り直して二重になる。待っても戻れなければ、戻る途中だと言って断る。
pub(crate) const 戻りを待つ秒: u64 = 4;
/// 戻らなくてよいときに、見直す間。
const 見直す秒: u64 = 30;
/// 繋がってから、戻れたかを見直すまでの間。
const 落ち着くまでの秒: u64 = 5;
/// 戻れなかったときに延ばす上限。**10 分より長くは黙らない。**
const 待つ上限の秒: u64 = 600;

/// 戻っている最中か。**人の［戻る］と見張りが同時に叩かない**ようにする。
static 戻っている: AtomicBool = AtomicBool::new(false);

/// **戻るべきか。**
///
/// - 帰り道が無い …… 戻らない（入ったことが無いか、自分で抜けた）
/// - そのルームを持っていない …… 戻る（この機械が上がり直した）
/// - そのルームの主催が自分 …… 戻らない（自分の部屋で待てばよい）
/// - **主催と繋がっていない** …… 戻る（主催が上がり直した）
/// - **主催がそのルームの名簿に居ない** …… 戻る（0.1.37）。主催が上がり直したあと、
///   **別のルームの用事で**繋ぎ直してくると、線はあってもこのルームの名簿からは落ちたまま。
///   線だけを見ていたので、ゲストは戻らず、主催の部屋から消えたままだった（10-23 に 3 台で見た）
///
/// **人数では決めない。**ゲスト同士が繋がっていると 2 人以上残るが、
/// 主催が居なければ、新しく来る人とは会えない。
pub fn 戻るべきか(
    帰り道のルーム: Option<&str>,
    部屋たち: &[RoomRow],
    繋がっている: impl Fn(&str) -> bool,
) -> bool {
    let Some(帰り道) = 帰り道のルーム else {
        return false;
    };
    let Some(部屋) = 部屋たち.iter().find(|r| r.id == 帰り道) else {
        return true;
    };
    if 部屋.host {
        return false;
    }
    let 主催が名簿に居る = 部屋.member_keys.contains(&部屋.host_key);
    !(繋がっている(&部屋.host_key) && 主催が名簿に居る)
}

/// **主催の部屋に居ないのに、居るつもりで送ってくる相手へ「居ない」と告げるか**（0.1.37）。
///
/// 主催が上がり直すと、主催の名簿からゲストは消える。ところが主催が**別のルームの用事で**
/// すぐ繋ぎ直すと、ゲストの側では線が切れたことにならず（入り直しとして扱う）、
/// **ゲストは居るつもりのまま下ごしらえを送り続け、主催は黙って捨て続けた**
/// （10-23・6 分で 60 通）。主催が `Leave` を返せば、ゲストは主催を名簿から外し、
/// 見張り（[`戻るべきか`]）が戸口から入り直す。
///
/// 告げるのは**自分が主催している部屋**についてだけ。1 本の線で 1 部屋 1 度
/// （`まだ告げていない`）。**戸口の答えではない**（線は既に通っている相手）ので、D31 とは別。
pub fn 居ないと告げるか<T: PartialEq>(
    名簿に居ないから断った: bool,
    部屋の主催: Option<&T>,
    自分: &T,
    まだ告げていない: bool,
) -> bool {
    名簿に居ないから断った && まだ告げていない && 部屋の主催.is_none_or(|主催| 主催 == 自分)
}

/// **線がついてから、主催がその相手の名簿を確かめるまでの秒**（0.1.61）。
///
/// 鍵で入る相手は、線がついてから 1 秒足らずで `Join` を送ってくる。それより早く確かめると、
/// 入る途中の相手に「居ない」と告げてしまうので、間を置く。
pub const 名簿を確かめるまでの秒: u64 = 10;

/// **線がついた相手へ、「居ない」と告げる部屋**（0.1.61）。
///
/// 主催が立ち上げ直すと、主催の名簿からゲストは消える。ところが主催が**別の部屋の用事で**
/// （紹介などで）すぐ線を張り直すと、ゲストの側では古い線の始末が新しい線に隠れ
/// （「入り直しているので、何も触らない」）、ゲストは主催を名簿に残したまま戻らない。
/// 11-25、ゲストが 93 分戻らず、発言したときに初めて入り直した（0.1.37 の告げは、話すまで起きない）。
///
/// だから主催は、線がついて [`名簿を確かめるまでの秒`] たったら、**自分が主催している部屋のうち、
/// その相手が名簿に居ない部屋**をすべて「居ない」と告げる。その部屋を持っていない相手は
/// 読み飛ばし、持っている相手は主催を外して帰り道から入り直す。
pub fn 居ないと告げる部屋<M: Copy, K: PartialEq>(
    主催している部屋: &[(M, Vec<K>)],
    相手: &K,
) -> Vec<M> {
    主催している部屋
        .iter()
        .filter(|(_, 面々)| !面々.contains(相手))
        .map(|(部屋, _)| *部屋)
        .collect()
}

/// 影と本物の名簿の食い違いが、何回続いたら入り直すか（0.1.61）。
pub const 食い違いで入り直す回数: u32 = 2;
/// 食い違いで入り直したあと、同じ部屋で次に入り直してよいまでの秒。**入り直し続けない。**
pub const 食い違いで入り直す間の秒: u64 = 600;

/// **ゲストの部屋の名簿が、影と本物で食い違い続けたら入り直す**（0.1.61）。
///
/// 11-25、ゲストは「影: 名簿が違う」を 30 秒おきに 17 分記録していたが、記録するだけで入り直さなかった。
/// 自己点検ごとに、名簿の食い違うゲストの部屋を渡す。[`食い違いで入り直す回数`] 回続いた部屋を返す
/// （続かなかった部屋は数え直す・入り直した部屋は [`食い違いで入り直す間の秒`] 待つ）。
#[derive(Debug, Default)]
pub struct 名簿の食い違い {
    続いた: HashMap<String, u32>,
    入り直した: HashMap<String, Instant>,
}

impl 名簿の食い違い {
    /// 今回食い違った部屋を渡し、入り直す部屋を受け取る。
    pub fn 見る(&mut self, 食い違う部屋: &[String], 今: Instant) -> Vec<String> {
        self.続いた.retain(|id, _| 食い違う部屋.contains(id));
        let mut 出 = Vec::new();
        for id in 食い違う部屋 {
            let 回 = self.続いた.entry(id.clone()).or_insert(0);
            *回 += 1;
            if *回 < 食い違いで入り直す回数 {
                continue;
            }
            let 待った = self.入り直した.get(id).is_none_or(|前| {
                今.saturating_duration_since(*前) >= Duration::from_secs(食い違いで入り直す間の秒)
            });
            if 待った {
                self.入り直した.insert(id.clone(), 今);
                self.続いた.remove(id);
                出.push(id.clone());
            }
        }
        出
    }
}

/// **戻れなかった回数から、次に叩くまでの秒を決める。**30 秒から倍にして、10 分で止める。
pub fn 次に待つ秒(しくじった: u32) -> u64 {
    let 倍 = 1u64 << しくじった.min(10);
    (見直す秒 * 倍).min(待つ上限の秒)
}

/// **戻る。**人の［戻る］も、見張りも、ここを通る。
///
/// 戻っている最中なら、**何もせずに返す**（同じ相手を 2 度叩かない）。
pub async fn 戻る(
    app: AppHandle,
    部屋: Option<warifu_meeting::MeetingId>,
    鍵: String,
) -> crate::Answer<()> {
    if 戻っている.swap(true, Ordering::SeqCst) {
        記録!("戻る: もう戻っている最中です（重ねて叩きません）");
        return Ok(());
    }
    let 結果 = crate::前のルームへ戻る(app.clone(), app.state::<Bridge>(), 部屋, 鍵).await;
    戻っている.store(false, Ordering::SeqCst);
    結果
}

/// ルームごとの、しくじった回数と、次に叩いてよい時刻。
/// しくじった回数・次に叩いてよい時刻・**主催に繋がったのに別の部屋を教えられた回数**（続けて）。
type 控え = HashMap<String, (u32, Instant, u32)>;

/// 主催に「その部屋はもう無い」と言われた返りか（0.1.41・`call::もう無いの印`）。
pub(crate) fn もう無いと言われたか(結果: &crate::Answer<()>) -> bool {
    matches!(結果, Err(e) if e.code.as_deref() == Some(crate::call::もう無いの印))
}

/// **入り直せていないと人とエージェントに言う回数**（0.1.48）。
///
/// 11-10、客は主催と切れたまま 2 時間戻らず、どの機械も鳴らなかった。叩き直しは続けるが、
/// この回数しくじったら 1 度だけ「主催へ入り直せていません」と会話に出す（黙って叩き続けない）。
const 言う回数: u32 = 3;

/// しくじった回数が、入り直せていないと言う回か（ちょうど [`言う回数`] のときだけ・言い続けない）。
#[must_use]
pub fn 入り直せていないと言うか(しくじった: u32) -> bool {
    しくじった == 言う回数
}

/// **主催が「その部屋は知らない」と続けて言ったら、見切る回数**（0.1.36）。
const 見切る回数: u32 = 3;

/// **その部屋は、もう主催の居ない部屋か。**
///
/// 主催に繋がれたのに別の部屋を教えられるのは、主催がその部屋を主催していないとき。
/// 0.1.32 までの取り違えで、**2 台が互いに「主催は相手」と覚えた部屋**が残っていた
/// （10-22 に mac mini と mac air で見た）。そのままだと、10 分ごとに叩き合い続ける。
pub fn 見切るか(続けて違った: u32) -> bool {
    続けて違った >= 見切る回数
}

/// **見張る。**起動したら 1 本だけ立てる。
///
/// **入っているルームそれぞれを見る**（帰り道はルームの数だけある）。
/// 叩く間はルームごとに延ばす —— 1 つの主催が落ちたままでも、ほかの部屋へは早く戻る。
pub async fn 見張る(app: AppHandle) {
    tokio::time::sleep(Duration::from_secs(起動を待つ秒)).await;
    let mut 控え: 控え = HashMap::new();
    loop {
        let 戻る先 = いま戻る先(&app).await;
        // **戻らなくてよくなった部屋の控えは捨てる**（次に落ちたら 30 秒から）
        控え.retain(|id, _| 戻る先.iter().any(|(部屋, _)| 部屋 == id));
        for (id, 鍵) in 戻る先 {
            if 控え.get(&id).is_some_and(|(_, 次, _)| Instant::now() < *次) {
                continue;
            }
            一つ戻る(&app, &mut 控え, id, 鍵).await;
        }
        tokio::time::sleep(Duration::from_secs(見直す秒)).await;
    }
}

/// **前に見ていたルームを見続けるか。**在って、自分のほかに誰か居るか、自分が鍵を配った部屋なら（0.1.42）。
///
/// 人が話しているルームは奪わない（0.1.33）。ただし、上げ直した直後に建てた
/// **自分しか居ない仮のルーム**まで守ると、戻れたのに仮のルームを見続け、
/// `room_status` は「相手 居ません」を返し続けた（11-02 に 2 台で見た）。
///
/// **鍵を配った部屋は、空でも見続ける。**主催が上げ直すと、客が戻るまでの数十秒は
/// 自分しか居ない。それを仮のルームと見なして、見ている所を別のルームへ移していた（11-02 23:06）
fn 見続けるか(在る: bool, 人数: usize, 鍵を配った: bool) -> bool {
    在る && (人数 > 1 || 鍵を配った)
}

/// この機械が鍵を配った部屋か（`issued.tsv` に、その部屋の行がある）。読めなければ配っていない扱い。
fn 鍵を配った部屋か(部屋: &str) -> bool {
    warifu_vault::Vault::default_location()
        .ok()
        .and_then(|v| v.issued().ok())
        .is_some_and(|控え| 控え.iter().any(|(id, _)| id == 部屋))
}

/// **見張りが戻ったあと、どのルームを見るか。**前に見ていたルームを見続けてよければ、そこに戻す。
fn 見張りのあとに見るルーム<T: Copy>(
    前: Option<T>,
    いま: Option<T>,
    前を見続ける: bool,
) -> Option<T> {
    match 前 {
        Some(前) if 前を見続ける => Some(前),
        _ => いま,
    }
}

/// **戻るときに、頼んでいない部屋へ招かれた分**（0.1.36）。
///
/// 割符なしで叩くと、主催が選んだ部屋へ招かれる。戻りたい部屋でなければ、
/// その部屋は持ち続けない —— 叩き直すたびに知らない部屋が増え、通話も作り直していた。
pub(crate) fn 頼んでいない部屋<T: PartialEq + Copy>(
    前: &[T],
    後: &[T],
    戻る先: Option<T>,
) -> Vec<T> {
    後.iter()
        .filter(|id| !前.contains(id) && Some(**id) != 戻る先)
        .copied()
        .collect()
}

async fn 一つ戻る(app: &AppHandle, 控え: &mut 控え, id: String, 鍵: String) {
    let 頭 = crate::短く(&id);
    記録!("戻る: ルーム {頭} の主催と繋がっていないので、帰り道から戻ります");
    // **見ているルームを奪わない**（0.1.33）。戻る道は人が押したときと同じで、
    // 戻った先を「いま話すルーム」にしていた。人は別のルームを見たまま打つので、
    // **打った文字が、戻った先のルームへ流れていた**（10-22 に 3 台で確かめた）
    let bridge = app.state::<Bridge>();
    let 前 = crate::いま見ているルーム(&bridge.いまのルーム).await;
    let 前の部屋: Vec<_> = bridge.conferences.lock().await.keys().copied().collect();
    let 結果 = 戻る(app.clone(), id.parse().ok(), 鍵).await;
    {
        let 見続ける = match 前 {
            Some(前) => {
                let 棚 = bridge.conferences.lock().await;
                let 人数 = 棚.get(&前).map_or(0, |c| c.members().len());
                let 在る = 棚.contains_key(&前);
                drop(棚);
                見続けるか(在る, 人数, 鍵を配った部屋か(&前.to_string()))
            }
            None => false,
        };
        let mut いま = bridge.いまのルーム.lock().await;
        *いま = 見張りのあとに見るルーム(前, *いま, 見続ける);
        let 見る = *いま;
        drop(いま);
        if let Some(部屋) = 見る {
            crate::影へ(&bridge, warifu_app::room::入り口::見る { 部屋 }).await;
        }
    }
    // **画面にも読み直させる。**戻った瞬間に画面は戻った先を読んでいる
    let _ = app.emit(crate::EVENT_LOOK, ());
    // **繋がった直後に断られることがある**（主催が別の部屋を建て直していると、
    // 割符を見てから口を閉じる）。すぐ見直すと「戻れた」と言い、30 秒ごとに叩き続けた
    if 結果.is_ok() {
        tokio::time::sleep(Duration::from_secs(落ち着くまでの秒)).await;
    }
    // **頼んでいない部屋へ招かれていたら、抜ける**（0.1.36）
    let 後の部屋: Vec<_> = bridge.conferences.lock().await.keys().copied().collect();
    let 戻る先 = 後の部屋.iter().copied().find(|m| m.to_string() == id);
    for 部屋 in 頼んでいない部屋(&前の部屋, &後の部屋, 戻る先) {
        記録!(
            "戻る: 頼んでいない部屋 {} へ招かれたので抜けます",
            crate::短く(&部屋.to_string())
        );
        crate::部屋を畳む(&bridge, 部屋).await;
    }
    // **主催が「その部屋はもう無い」と言った**（0.1.41）。3 回叩いて見切るのを待たず、1 回で帰り道を消す
    if もう無いと言われたか(&結果) {
        見切る(app, &id, "主催が「その部屋はもう無い」と言った").await;
        控え.remove(&id);
        return;
    }
    // **繋がっても、戻る先とは別のルームへ招かれることがある**
    // （割符なしで叩くと、主催がいま見ているルームへ招かれる）。戻れたかは見直して決める
    let まだ = いま戻る先(app).await.iter().any(|(部屋, _)| *部屋 == id);
    if !まだ {
        記録!("戻る: ルーム {頭} へ戻れました");
        控え.remove(&id);
        return;
    }
    let (前のしくじり, _, 前の違い) = 控え.get(&id).copied().unwrap_or((0, Instant::now(), 0));
    let しくじった = 前のしくじり.saturating_add(1);
    // 繋がれなかった回は数えない（主催が落ちているだけかもしれない）
    let 違った = if 結果.is_ok() {
        前の違い.saturating_add(1)
    } else {
        0
    };
    if 見切るか(違った) {
        見切る(
            app,
            &id,
            &format!("主催が別の部屋を {見切る回数} 回続けて教えた"),
        )
        .await;
        控え.remove(&id);
        return;
    }
    let 秒 = 次に待つ秒(しくじった);
    if 入り直せていないと言うか(しくじった) {
        let 文 = format!(
            "主催へ入り直せていません: ルーム {頭} に {しくじった} 回戻れませんでした。{秒} 秒ごとに試し続けます（届かない間の発言は、戻れたら 1 回だけ届きます）"
        );
        記録!("戻る: {文}");
        crate::desk::配る(&bridge, crate::desk::聞いた("割符", &文));
    }
    match 結果 {
        Ok(()) => 記録!(
            "戻る: 繋がりましたが、ルーム {頭} ではありませんでした。{秒} 秒あとにもう一度叩きます"
        ),
        Err(e) => 記録!(
            "戻る: ルーム {頭} へ戻れませんでした（{}）。{秒} 秒あとにもう一度叩きます",
            e.message
        ),
    }
    控え.insert(
        id,
        (しくじった, Instant::now() + Duration::from_secs(秒), 違った),
    );
}

/// **主催の居ない部屋を見切る。**帰り道を忘れ、自分のほかに誰も居なければ部屋も畳む。
async fn 見切る(app: &AppHandle, id: &str, 訳: &str) {
    let 頭 = crate::短く(id);
    記録!("戻る: {訳}ので、ルーム {頭} へはもう戻りません");
    let bridge = app.state::<Bridge>();
    let 部屋たち = crate::ルームの並び(&bridge).await;
    let 誰も居ない = 部屋たち
        .iter()
        .find(|r| r.id == id)
        .is_none_or(|r| r.member_keys.len() <= 1);
    let 部屋 = bridge
        .conferences
        .lock()
        .await
        .keys()
        .copied()
        .find(|m| m.to_string() == id);
    match 部屋 {
        // 畳むと帰り道も忘れる
        Some(部屋) if 誰も居ない => {
            記録!("戻る: ルーム {頭} には誰も居ないので畳みます");
            crate::部屋を畳む(&bridge, 部屋).await;
        }
        _ => {
            if let Ok(vault) = warifu_vault::Vault::default_location() {
                if let Err(e) = vault.forget_rejoin_room(id) {
                    記録!("帰り道を消せませんでした: {e}");
                }
            }
        }
    }
}

/// **戻る途中なら、様子の題字をそう言い換える**（0.1.42）。
///
/// 画面を上げ直すと、まず自分の新しいルームを建て、帰り道のルームへ戻るのは少し後になる。
/// その間の `room_status` は「相手: 居ません／相手を待っています」を返し、
/// エージェントが「ルームから出た」と読みかけた（11-02）。戻る先がある間は、そう言う。
pub fn 戻る途中の題字(戻る先: &[String], 元の題字: Option<String>) -> Option<String> {
    if 戻る先.is_empty() {
        return 元の題字;
    }
    let 頭たち: Vec<String> = 戻る先.iter().map(|id| crate::短く(id)).collect();
    Some(format!(
        "前のルーム（{}）へ戻ろうとしています。いま見ているルームは戻るまでの仮で、相手が居ないのはそのためです",
        頭たち.join("・")
    ))
}

/// **戻るべきルームの帰り道**（ルーム id と鍵）。
/// **帰り道に載っている部屋の id**（戻る途中かを見分けるのに使う）。読めなければ空。
pub(crate) fn 帰り道の部屋() -> Vec<String> {
    warifu_vault::Vault::default_location()
        .ok()
        .and_then(|v| v.rejoins().ok())
        .map(|道| 道.into_iter().map(|(id, _)| id).collect())
        .unwrap_or_default()
}

pub(crate) async fn いま戻る先(app: &AppHandle) -> Vec<(String, String)> {
    let Ok(vault) = warifu_vault::Vault::default_location() else {
        return Vec::new();
    };
    // 読めないときは戻らない。**半端な帰り道で叩かない**
    let Ok(帰り道) = vault.rejoins() else {
        return Vec::new();
    };
    let bridge = app.state::<Bridge>();
    let 部屋たち = crate::ルームの並び(&bridge).await;
    let 繋がっている: Vec<String> = bridge
        .outbound
        .lock()
        .await
        .keys()
        .filter_map(|k| warifu_core::PublicKey::from_bytes(*k).ok())
        .map(crate::key_to_string)
        .collect();
    帰り道
        .into_iter()
        .filter(|(id, _)| {
            戻るべきか(Some(id), &部屋たち, |鍵| {
                繋がっている.iter().any(|k| k == 鍵)
            })
        })
        .collect()
}

#[cfg(test)]
mod 試験 {
    #[test]
    fn 主催が線のついた相手へ_名簿に居ない自分の部屋だけ居ないと告げる() {
        use super::居ないと告げる部屋;
        let 部屋 = [("R1", vec!["主催", "甲"]), ("R2", vec!["主催"])];
        assert_eq!(居ないと告げる部屋(&部屋, &"甲"), vec!["R2"]);
        assert_eq!(居ないと告げる部屋(&部屋, &"乙"), vec!["R1", "R2"]);
        assert!(居ないと告げる部屋::<&str, &str>(&[], &"甲").is_empty());
    }

    #[test]
    fn 名簿の食い違いが_2_回続いたら入り直し_10_分は繰り返さない() {
        use super::名簿の食い違い;
        use std::time::{Duration, Instant};
        let mut 見張り = 名簿の食い違い::default();
        let t0 = Instant::now();
        let r = vec!["R1".to_owned()];
        assert!(見張り.見る(&r, t0).is_empty(), "1 回目では入り直さない");
        assert_eq!(見張り.見る(&r, t0), r, "2 回続いたら入り直す");
        assert!(見張り.見る(&r, t0).is_empty());
        assert!(
            見張り.見る(&r, t0 + Duration::from_secs(60)).is_empty(),
            "10 分は繰り返さない"
        );
        assert_eq!(見張り.見る(&r, t0 + Duration::from_secs(601)), r);
    }

    #[test]
    fn 名簿の食い違いが途切れたら_数え直す() {
        use super::名簿の食い違い;
        use std::time::Instant;
        let mut 見張り = 名簿の食い違い::default();
        let t0 = Instant::now();
        let r = vec!["R1".to_owned()];
        assert!(見張り.見る(&r, t0).is_empty());
        assert!(見張り.見る(&[], t0).is_empty());
        assert!(見張り.見る(&r, t0).is_empty(), "途切れたので 1 回目から");
    }

    #[test]
    fn 戻る先がある間だけ_題字を戻る途中と言い換える() {
        use super::戻る途中の題字;
        let 元 = Some("相手を待っています".to_owned());
        assert_eq!(戻る途中の題字(&[], 元.clone()), 元);
        assert_eq!(戻る途中の題字(&[], None), None);
        let 言った =
            戻る途中の題字(&["NBIW3PA2TUQ52DTJNVXSQOL3G4".to_owned()], 元).expect("言い換える");
        assert!(言った.contains("戻ろうとしています"), "{言った}");
        assert!(
            言った.contains("NBIW3PA2"),
            "どのルームへかを言う: {言った}"
        );
        assert!(
            !言った.contains("NBIW3PA2TUQ52DTJNVXSQOL3G4"),
            "id は頭だけ: {言った}"
        );
    }

    #[test]
    fn もう無いと言われた返りだけを見分ける() {
        let もう無い: crate::Answer<()> = Err(crate::Failure {
            message: "x".into(),
            code: Some(crate::call::もう無いの印.into()),
        });
        assert!(super::もう無いと言われたか(&もう無い));
        let 居ない: crate::Answer<()> = Err(crate::Failure {
            message: "x".into(),
            code: Some("contact.unreachable".into()),
        });
        assert!(!super::もう無いと言われたか(&居ない));
        assert!(!super::もう無いと言われたか(&Ok(())));
    }

    #[test]
    fn 見張りが戻っても_前に見ていたルームを奪わない() {
        use super::見張りのあとに見るルーム;
        assert_eq!(見張りのあとに見るルーム(Some(1), Some(2), true), Some(1));
        // 前のルームが無くなっていれば、戻った先を見る
        assert_eq!(見張りのあとに見るルーム(Some(1), Some(2), false), Some(2));
        // 何も見ていなかったなら、戻った先を見る
        assert_eq!(見張りのあとに見るルーム(None, Some(2), false), Some(2));
    }

    #[test]
    fn 上げ直した直後の_自分しか居ない仮のルームは見続けず_戻った先を見る() {
        // 11-02 に 2 台で見た。上げ直すとまず自分のルームを建て、そこを見ている。
        // 戻れたのに仮のルームを見続け、room_status は「相手 居ません」を返し続けた
        use super::{見張りのあとに見るルーム, 見続けるか};
        assert!(
            !見続けるか(true, 1, false),
            "自分しか居ないルームは見続けない"
        );
        assert!(
            見続けるか(true, 2, false),
            "相手の居るルームは奪わない（0.1.33）"
        );
        assert!(!見続けるか(false, 2, true), "無くなったルームは見続けない");
        assert_eq!(
            見張りのあとに見るルーム(Some(1), Some(2), 見続けるか(true, 1, false)),
            Some(2)
        );
    }

    #[test]
    fn 主催が鍵を配った部屋は_皆が戻る前で空でも見続ける() {
        // 11-02 23:06、全機の部屋の主催（本番）を上げ直したら、客が戻る前の 20 秒で
        // 自分の帰り道（別のルーム）へ戻り、見ている所が全機の部屋から移った
        use super::見続けるか;
        assert!(見続けるか(true, 1, true));
    }

    use super::*;

    #[test]
    fn 主催が別の部屋を_3_回続けて教えたら見切る() {
        assert!(!見切るか(2));
        assert!(見切るか(3));
    }

    #[test]
    fn 戻る先とは別に増えた部屋は_頼んでいない部屋() {
        assert_eq!(頼んでいない部屋(&[1, 2], &[1, 2, 7], Some(5)), vec![7]);
    }

    #[test]
    fn 戻る先に戻れたなら_それは抜けない() {
        assert_eq!(頼んでいない部屋(&[1], &[1, 5], Some(5)), Vec::<i32>::new());
    }

    #[test]
    fn 前から居た部屋は抜けない() {
        assert_eq!(頼んでいない部屋(&[1, 7], &[1, 7], None), Vec::<i32>::new());
    }

    fn 部屋(id: &str, host: bool, host_key: &str) -> RoomRow {
        RoomRow {
            id: id.into(),
            members: 1,
            host,
            host_key: host_key.into(),
            // 主催がそのルームの名簿に居る（ふだんの形）
            member_keys: vec![host_key.into()],
            host_linked: true,
        }
    }

    #[test]
    fn 帰り道が無ければ戻らない() {
        assert!(!戻るべきか(None, &[], |_| false));
    }

    #[test]
    fn ルームを持っていなければ戻る() {
        assert!(戻るべきか(Some("R"), &[部屋("別", true, "私")], |_| {
            false
        }));
    }

    #[test]
    fn 自分の部屋なら戻らない() {
        assert!(!戻るべきか(Some("R"), &[部屋("R", true, "私")], |_| {
            false
        }));
    }

    #[test]
    fn 入り直せていないと言うのは_3_回目の_1_度だけ() {
        assert!(!入り直せていないと言うか(1));
        assert!(!入り直せていないと言うか(2));
        assert!(入り直せていないと言うか(3));
        assert!(!入り直せていないと言うか(4), "言い続けない");
    }

    #[test]
    fn 主催と繋がっていれば戻らない() {
        assert!(!戻るべきか(
            Some("R"),
            &[部屋("R", false, "主")],
            |k| k == "主"
        ));
    }

    #[test]
    fn 主催が落ちていれば_ゲストが残っていても戻る() {
        // 主催以外とは繋がっている（ゲスト同士）
        assert!(戻るべきか(Some("R"), &[部屋("R", false, "主")], |k| k == "客"));
    }

    #[test]
    fn 主催と繋がっていても_そのルームの名簿に主催が居なければ戻る() {
        // 主催が上がり直したあと、**別のルームの用事で**繋ぎ直してきた。
        // 線はあるが、このルームの名簿からは落ちたまま（切れたときに外している）
        let mut r = 部屋("R", false, "主");
        r.member_keys = vec!["私".into()];
        assert!(戻るべきか(Some("R"), &[r], |k| k == "主"));
    }

    #[test]
    fn 自分の部屋に居ない相手からの下ごしらえには_1度だけ居ないと告げる() {
        assert!(居ないと告げるか(true, None, &"私", true)); // 覚えていない＝自分が建てた
        assert!(居ないと告げるか(true, Some(&"私"), &"私", true));
        assert!(!居ないと告げるか(true, Some(&"私"), &"私", false)); // もう告げた
    }

    #[test]
    fn 他人の部屋や_ほかの理由で断ったときは告げない() {
        assert!(!居ないと告げるか(true, Some(&"主"), &"私", true));
        assert!(!居ないと告げるか(false, Some(&"私"), &"私", true));
    }

    #[test]
    fn 待つ間は倍に延びて_10分で止まる() {
        assert_eq!(次に待つ秒(0), 30);
        assert_eq!(次に待つ秒(1), 60);
        assert_eq!(次に待つ秒(2), 120);
        assert_eq!(次に待つ秒(5), 600);
        assert_eq!(次に待つ秒(u32::MAX), 600);
    }
}
