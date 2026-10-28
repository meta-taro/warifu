//! この機械 —— **同じ PC のエージェントが、人と同じ会話につながる口。**
//!
//! ```text
//!   人（この画面）──┐
//!                   ├── 同じ会話 ── iroh P2P ── 相手の PC
//!   AI（warifu mcp）─┘
//!         ↑ ここがこの機械（同じ機械の中だけ。網には出ない）
//! ```
//!
//! **これが「エージェント同士でチャットし、人がそれを見て入れる」の実体。**
//! エージェントに `warifu join` を叩かせる形は、実際に許可プロンプトで
//! 3 回とも止まった（2026-09-06）。MCP の口なら止まらない。
//!
//! # ここで守ること
//!
//! - **繋いできた側は差出人を名乗れない。**誰が言ったかはここで刻む
//! - **この機械は同じ機械の中だけ。**口の権限は `warifu-desk` が持つ（Unix は 0600）
//! - **届いた文字は相手の言い分。**ここは運ぶだけで、解釈しない

use std::collections::BTreeMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::broadcast;
use warifu_desk::{FromDesk, ToDesk, 受け口, 口, この機械の場所};
use warifu_meeting::MeetingId;

use crate::{Bridge, EVENT_DESK, EVENT_DESK_SEATS};

/// この機械につながっている相手へ配る溜め。
///
/// **溜めきれない相手を待たない。**追いつけない客は取りこぼすが、
/// 会話そのものは止めない（止めると人の画面まで止まる）。
pub const 配る溜め: usize = 128;

/// この機械につながった人に振る番号。
///
/// **自分の発言を自分に返さないため**に要る。返すと、
/// エージェントは `chat_read` で自分が言ったことを「誰かの発言」として読む。
static 次の番号: AtomicU64 = AtomicU64::new(1);

/// 発言に振る通し番号（**既読**を数えるため・**D76**）。
///
/// **立て直しても続きから振る**（2026-10-23）。1 から振り直していたので、
/// エージェントの控え（`heard/<名乗り>`・前の立ち上げで 29 まで）が新しい番号より大きく、
/// 「29 より後」に 1 行も当たらなかった。画面を 0.1.38 に上げたあと、繋ぎ直した
/// エージェントへ #1〜#7 が渡らず、知らせは来るのに本文が読めなかった。
static 次の発言: AtomicU64 = AtomicU64::new(1);

/// 番号の控えの名前（`在り処.控え` の下。`heard/` と同じ所）。
const 番号の控え: &str = "said";

fn 番号の控えの道() -> std::path::PathBuf {
    warifu_desk::この機械の在り処().控え.join(番号の控え)
}

/// 控えの続きから振る。控えが無ければ 1 から。
#[must_use]
const fn 続きの番号(控えた: Option<u64>) -> u64 {
    match 控えた {
        Some(n) => n + 1,
        None => 1,
    }
}

/// 立ち上げたとき、控えの続きから番号を振るようにする。
fn 番号を続ける() {
    let 道 = 番号の控えの道();
    let 控えた = std::fs::read_to_string(&道)
        .ok()
        .and_then(|s| s.trim().parse().ok());
    次の発言.store(続きの番号(控えた), Ordering::Relaxed);
}

/// 告げた版の控えの名前（`在り処.控え` の下）。
const 版の控え: &str = "told-version";

/// **版が上がったら、この機械のエージェントへ 1 度だけ告げる**（0.1.39・2026-10-23）。
///
/// 出してから人が不具合を見つける形が続いた。エージェントは MCP の文字で状態を見ているので、
/// 画面と中身の食い違いを「おかしい」と言えない。**版が上がった直後に、点検して報告するよう頼む。**
/// 行は `直近の発言` に残るので、あとから着いたエージェントにも渡る。
fn 版が上がったと告げる(app: &AppHandle) {
    let 道 = warifu_desk::この機械の在り処().控え.join(版の控え);
    let 版 = env!("CARGO_PKG_VERSION");
    let 前 = std::fs::read_to_string(&道).ok();
    // **控えは毎回書く。**初めて（控えが無い）は告げないが、書かないと次の版でも告げられない
    // （0.1.39 で踏んだ：告げるときだけ書いていたので、控えが一度もできなかった）
    if let Err(e) = 道
        .parent()
        .map_or(Ok(()), std::fs::create_dir_all)
        .and_then(|()| std::fs::write(&道, 版))
    {
        記録!("告げた版を控えられません（{}）: {e}", 道.display());
    }
    if let Some(本文) = 版が上がった知らせ(前.as_deref().map(str::trim), 版) {
        記録!("版が {} から {版} に上がったと、エージェントへ告げます", 前.as_deref().unwrap_or("").trim());
        配る(&app.state::<Bridge>(), 聞いた("割符", &本文));
    }
}

/// 版が上がっていれば、エージェントへ渡す 1 行。**初めて（控えが無い）なら告げない。**
#[must_use]
fn 版が上がった知らせ(前: Option<&str>, 今: &str) -> Option<String> {
    let 前 = 前?;
    if 前 == 今 {
        return None;
    }
    Some(format!(
        "割符の画面が {前} から {今} に上がりました。room_status と app_view（札 screen.read）で、\
         画面に出ている人数・届く先と、本体の知っている相手が合っているかを見比べ、\
         食い違いがあれば開発の席へ知らせてください。合っていれば何もしなくてかまいません。"
    ))
}

/// 発言に番号を振り、控える。**控えられなくても発言は止めない**（記録には残す）。
fn 番号を振る() -> u64 {
    let id = 次の発言.fetch_add(1, Ordering::Relaxed);
    let 道 = 番号の控えの道();
    let 書けた = 道
        .parent()
        .map_or(Ok(()), std::fs::create_dir_all)
        .and_then(|()| std::fs::write(&道, id.to_string()));
    if let Err(e) = 書けた {
        記録!("発言の番号を控えられません（{}）: {e}", 道.display());
    }
    id
}

/// 発言ごとに、**どのエージェントへ届けたか。**
///
/// **「3 人へ流しました」だけでは、誰に届いたか分からない**（`issues/4`）。
/// 古いものから捨てる —— **溜め込む所にしない。**
static 届いた先: Mutex<BTreeMap<u64, Vec<String>>> = Mutex::new(BTreeMap::new());

/// エージェントごとに、**どこまで読んだか。**
///
/// **読んだのは、渡された時点である**（`chat_read` / `chat_wait` が呼び手へ渡したとき）。
/// **中身を理解したかは誰にも分からない**ので、そこは名乗らない。
static 読んだところ: Mutex<BTreeMap<u64, u64>> = Mutex::new(BTreeMap::new());

/// 覚えておく発言の数。**画面に出す既読を数えるのに要るだけ持つ。**
const 覚える発言: usize = 200;

/// **直近の発言そのもの**（番号つき）。
///
/// **繋ぎ直したエージェントへ、切れている間の言葉を渡すために持つ**
/// （内部の Issue 019）。人は画面で過去を読めるが、**エージェントは読めない** ——
/// `warifu agent` は繋ぎ直せても（**D93**）、**その間の言葉は落ちていた。
/// しかも落ちたことが誰にも見えなかった。**
///
/// **誰がどこまで読んだかは覚えない。**「どこから」は読み手が言う
/// （覚えると**既読を作ることになる**・**D94** で作らないと決めた）。
static 直近の発言: Mutex<Vec<(u64, FromDesk)>> = Mutex::new(Vec::new());

/// エージェントへ座らせる。**同じ呼び方が二つ並ばないようにする。**
///
/// 名乗らないエージェントが 2 つつながると、**どちらも同じ呼び方になる。**
/// 画面は呼び方で行を分けているので、**2 人が 1 行に潰れる**（特定の 1 人だけに
/// 聞くことが成り立たなくなる・2026-09-08）。
/// **後から来たほうに番号を振る** —— 先に居るほうの名前を変えない。
fn エージェントにつながれる(エージェント: u64, 呼び方: &str) {
    let mut 棚 = エージェントたち.lock().expect("毒されていない");
    let 使われている =
        |名: &str, 棚: &BTreeMap<u64, String>| 棚.iter().any(|(番, 呼)| *番 != エージェント && 呼 == 名);
    if !使われている(呼び方, &棚) {
        棚.insert(エージェント, 呼び方.to_owned());
        return;
    }
    for n in 2..=99u32 {
        let 候補 = format!("{呼び方} {n}");
        if !使われている(&候補, &棚) {
            棚.insert(エージェント, 候補);
            return;
        }
    }
    // **ここまで来たらエージェント番号で分ける。**同じ名前で潰すよりよい
    棚.insert(エージェント, format!("{呼び方}（{エージェント}）"));
}

/// **古い口が抜けたら、番号付きの名を元に戻す**（0.1.41・内部の Issue 023）。
///
/// Claude Code の `/mcp` の繋ぎ直しは古い `warifu mcp` を閉じないことがあり、
/// 新しい口は「〜のエージェント 2」で着く。古い方を止めても「2」のまま残り、
/// 人には口が入れ替わったように見えなかった（2026-10-28 に mac mini で見た）。
/// **元の名が空いたら、その名に戻す。**
fn 番号を外す(棚: &mut BTreeMap<u64, String>) {
    let 番号付き: Vec<(u64, String)> = 棚
        .iter()
        .filter_map(|(番, 名)| 元の名(名).map(|元| (*番, 元.to_owned())))
        .collect();
    for (番, 元) in 番号付き {
        if !棚.values().any(|名| *名 == 元) {
            棚.insert(番, 元);
        }
    }
}

/// 「◯◯ のエージェント 2」の元の名。番号が付いていなければ `None`。
fn 元の名(名: &str) -> Option<&str> {
    let (元, 番号) = 名.rsplit_once(' ')?;
    let 番号か = !番号.is_empty() && 番号.bytes().all(|b| b.is_ascii_digit());
    (番号か && 元.ends_with("エージェント")).then_some(元)
}

/// エージェントごとの呼び方。**誰がつながっているかを画面へ出すため。**
///
/// 1 行にまとめると、**どれが喋ったのか分からない**だけでなく、
/// **どれがつながっているのかも分からない**（2026-09-08）。
/// **まとめて 1 人ではない。それぞれが 1 人である。**
static エージェントたち: Mutex<BTreeMap<u64, String>> = Mutex::new(BTreeMap::new());

/// **ルームキーの既定の寿命**（24 時間・**#32 の段 2**）。
///
/// **画面と同じ値にしてある**（`+page.svelte` の `KEY_TTL_SECS`）——
/// **2 か所で違う値を持つと、「24 時間」と書いた案内が嘘になる。**
pub(crate) const 既定の鍵の寿命: u64 = 60 * 60 * 24;

/// **人が札に答えたことを、待っている口へ知らせる**（2026-09-24）。
///
/// # なぜ要るか —— **押されたことは、エージェントへ届かなかった**
///
/// `Ask` は「いまの答え」を返すだけで、
/// **人が押しても、エージェントには何も届かない** ——
/// だから毎回「押しましたか」「押しました」の往復が要っていた。
/// **人が、エージェントの目の代わりをしていた。**
///
/// **`notify_waiters` は、待っている者が居なければ何もしない** ——
/// 誰も待っていないときに押しても、あとから届いたりはしない（そこは `Ask` の役）。
static 答えの知らせ: std::sync::OnceLock<tokio::sync::Notify> = std::sync::OnceLock::new();

/// 知らせの口。**初めて使うときに作る**（`Notify::new` は const ではない）。
fn 答えの知らせ口() -> &'static tokio::sync::Notify {
    答えの知らせ.get_or_init(tokio::sync::Notify::new)
}

/// **待つ口の既定**（秒）。書かなければこれで待つ。
const 既定の待ち秒: u64 = 30;

/// **人の答えを待っている札の頼み**（**D119**）。
///
/// **部屋へは流さない。**画面の別の所に出す ——
/// **聞く口と流す口を分ける**（**#26**）。
///
/// **同じ動作の頼みは 1 つだけ持つ。**同じものが並ぶと、
/// **人が何度も同じことを聞かれる**（#33 / #35 で踏んだ「読まれなくなる」形）。
static 待っている頼み: Mutex<Vec<待ち>> = Mutex::new(Vec::new());

/// 人の答えを待っている 1 件。
#[derive(Debug, Clone, serde::Serialize)]
pub struct 待ち {
    /// 何をしたいか。
    pub 動作: String,
    /// **なぜ要るか。**人が一言で読む
    pub 訳: String,
    /// **どのエージェントが頼んだか。**人が「誰に許すか」を判断する材料
    pub 頼んだ人: String,
}

/// この機械の外から出た発言（相手から届いた文字・人が打った行）に使う番号。
///
/// **この機械の誰とも一致しない**ので、全員へ配られる。
pub const この機械の外: u64 = 0;

/// いまの時刻（`HH:MM`）。**秒は出さない。**
///
/// 無いと、あとから読み返せない（2026-09-06 の「何時に投稿したかわからないです」）。
fn いま時刻() -> String {
    chrono::Local::now().format("%H:%M").to_string()
}

/// この機械を開く。**開けなくても画面は止めない。**
///
/// この機械が無くても人どうしの会話は成り立つ。
/// **開かなかったことは記録に残す**（黙って落とすと、
/// エージェントが繋がらない理由が誰にも分からない）。
pub fn 開く(app: AppHandle) {
    番号を続ける();
    版が上がったと告げる(&app);
    tauri::async_runtime::spawn(async move {
        let 場所 = この機械の場所();
        let mut 待ち = match 受け口::開く(&場所).await {
            Ok(待ち) => {
                記録!("この機械を開きました: {}", 場所.display());
                待ち
            }
            Err(e) => {
                記録!("この機械を開けませんでした（{}）: {e}", 場所.display());
                return;
            }
        };
        loop {
            match 待ち.受ける().await {
                Ok(一本) => {
                    記録!("この機械に 1 人着きました");
                    座らせる(app.clone(), 一本);
                }
                Err(e) => {
                    記録!("この機械の口が壊れました: {e}");
                    return;
                }
            }
        }
    });
}

/// この機械につながった 1 人の面倒を見る。
fn 座らせる(app: AppHandle, 一本: impl warifu_desk::一本) {
    tauri::async_runtime::spawn(async move {
        let 自分 = 次の番号.fetch_add(1, Ordering::Relaxed);
        // **どこで動いているか。**`Listen` で名乗ってくるまでは分からない
        let mut 名乗り: Option<String> = None;
        // **届いた行に部屋を付けてよいか**（0.1.40・内部の Issue 026）。古い口は知らない欄を断るので、
        // `Listen` で名乗った口にだけ付ける
        let mut 部屋も聞く = false;
        // **様子に食い違いを付けてよい口か**（0.1.41）。古い口には付けない（読めなくなる）
        let mut 点検も聞く = false;
        let mut 口 = 口::新しく(一本);
        let mut 聞く = app.state::<Bridge>().desk.subscribe();
        // **人数が変わったことを画面へ伝える。**伝えないと、
        // AI が居るのに「入ってきたら送れます」と出たままになる
        // **名乗るまでエージェントに着けない**（2026-09-08）。
        // 名乗らないものは一覧に出さず、ターミナルから一度名乗って入るよう案内する。
        //
        // **どこで動いているか分からないエージェントを、一覧に出さない。**
        // 出しても「どの階層のエージェントか不明」で、押しても意味が無い。
        // つながれるのは `ToDesk::Listen { 場所: Some(..) }` が来たときだけ。
        loop {
            tokio::select! {
                来た = 口.受ける() => {
                    let Ok(Some(行)) = 来た else { break };
                    if 応じる(&app, &行, &mut 口, 自分, &mut 名乗り, &mut 部屋も聞く, &mut 点検も聞く)
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
                配られた = 聞く.recv() => {
                    match 配られた {
                        // **自分が言ったことは自分に返さない。**返すと、
                        // エージェントは自分の発言を「誰かの発言」として読む。
                        // 宛先つきなら、**そのエージェントだけが受ける**
                        Ok((出所, _)) if !自分あてか(出所, 自分) => continue,
                        Ok((_, 一つ)) => {
                            let 一つ = 部屋を落とすか(一つ, 部屋も聞く);
                            if 口.送る(&一つ.書く()).await.is_err() {
                                break;
                            }
                        }
                        // 追いつけずに落ちた分がある。**会話は止めない**
                        Err(broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(broadcast::error::RecvError::Closed) => break,
                    }
                }
            }
        }
        記録!("この機械から 1 人抜けました");
        {
            let mut 棚 = エージェントたち.lock().expect("毒されていない");
            棚.remove(&自分);
            番号を外す(&mut 棚);
        }
        // subscribe を落としてから数える。**落とす前だと自分を数えてしまう**
        drop(聞く);
        エージェントの数を伝える(&app);
    });
}

/// この機械に何人つながっているかを、画面へ伝える。
///
/// **「相手が居ない」と「話し相手が 1 人も居ない」は違う。**
/// 会議に人が居なくても、同じエージェントの AI が居るなら人は話しかけられる。
pub fn エージェントの数を伝える(app: &AppHandle) {
    let _ = app.emit(EVENT_DESK_SEATS, つながっている顔ぶれ());
}

/// いまこの機械につながっている顔ぶれ。**呼び方の並び。**
///
/// 数だけでは「どれがつながっているのか」が分からない。
#[must_use]
pub fn つながっている顔ぶれ() -> Vec<String> {
    エージェントたち
        .lock()
        .expect("毒されていない")
        .values()
        .cloned()
        .collect()
}

/// いまこの機械に何人つながっているか。
#[must_use]
pub fn エージェントの数(bridge: &Bridge) -> usize {
    bridge.desk.receiver_count()
}

/// この機械から来た 1 行に応じる。
async fn 応じる(
    app: &AppHandle,
    行: &str,
    口: &mut 口<impl warifu_desk::一本>,
    自分: u64,
    名乗り: &mut Option<String>,
    部屋も聞く: &mut bool,
    点検も聞く: &mut bool,
) -> Result<(), ()> {
    let 中身 = match ToDesk::読む(行) {
        Ok(中身) => 中身,
        Err(e) => {
            // **読めなかったことを、断ったことと混ぜない**
            let _ = 口
                .送る(&FromDesk::Denied { why: e.to_string() }.書く())
                .await;
            return Ok(());
        }
    };
    match 中身 {
        // 「聞く」は名乗りだけ。**過去は残していない**ので、返すものが無い（`issues/010`）。
        // **どこで動いているかを覚える** —— 1 台の PC で複数のエージェントが
        // 同じこの機械につながるので、覚えないとどれが喋ったのか人に分からない（2026-09-08）
        ToDesk::Listen {
            場所,
            どこから,
            部屋も聞く: 聞く,
            点検も聞く: 点検,
        } => {
            *部屋も聞く = 聞く;
            *点検も聞く = 点検;
            let Some(名) = 場所 else {
                // **名乗らない相手はエージェントに着けない。**どうすればつながれるかは言う ——
                // ここは「断る理由を相手に返さない」（D31）とは別で、
                // **同じ PC の中の、こちらが呼んだ相手**である
                let _ = 口
                    .送る(
                        &FromDesk::Denied {
                            why: 名乗ってください.to_owned(),
                        }
                        .書く(),
                    )
                    .await;
                return Ok(());
            };
            記録!("この機械: {名} から着きました");
            *名乗り = Some(名.clone());
            エージェントにつながれる(自分, &呼び方(Some(&名)));
            エージェントの数を伝える(app);
            // **いつつながったかを返す**（`issues/4`）。
            // これが無いと、エージェントから「届いていない」と「つながる前だった」を見分けられない
            let who = エージェントたち
                .lock()
                .expect("毒されていない")
                .get(&自分)
                .cloned()
                .unwrap_or_else(|| 呼び方(Some(&名)));
            // **切れている間の言葉を渡す**（`issues/019`）。
            // **`Seated` より先に渡す** —— `Seated` は「ここから先は生きている」の印なので、
            // 過去はその前に置く（`line.rs` の `Listen` が
            // 「これまでの会話をもらってから、以後を聞く」と言っている形にする）
            let 過去 = {
                let 棚 = 直近の発言.lock().expect("毒されていない");
                過去を選ぶ(&棚, どこから)
            };
            if !過去.is_empty() {
                記録!("この機械: {名} へ、切れている間の {} 行を渡します", 過去.len());
            }
            for 行 in 過去 {
                if 口.送る(&行.書く()).await.is_err() {
                    return Ok(());
                }
            }
            let _ = 口
                .送る(
                    &FromDesk::Seated {
                        at: いま時刻(),
                        who,
                    }
                    .書く(),
                )
                .await;
            Ok(())
        }
        // **自分のエージェントのプロフィールを書く**（2026-09-08）。
        // エージェント自身が書けるほうが、人が代わりに書くより手間が少ない。
        //
        // **どのエージェントかは、この口で決まる。**行の中に「誰の」は書けない ——
        // 書けると、**同じ機械の別のエージェントに化けられる。**
        ToDesk::Profile { 名前, 紹介 } => {
            let 返事 = match 名乗り.as_deref() {
                // **名乗っていないエージェントは、エージェントが決まらない。**
                // どこで動いているか分からないものに、名前だけ付けさせない
                None => FromDesk::Denied {
                    why: "先に名乗ってください（--as か、起動した場所）".to_owned(),
                },
                Some(名) => match crate::profile::エージェントの名乗りを書く(app, 名, &名前, &紹介)
                {
                    Ok(()) => {
                        // **相手の名簿にも出す**（0.1.33）。人が名乗っていない PC では、
                        // この名乗りが相手に見える名前になる。入ったあとで名乗っても届くよう、配り直す
                        crate::profile::配る(&app.state::<Bridge>()).await;
                        FromDesk::Wrote {
                            who: 呼び方(Some(名)),
                        }
                    }
                    Err(訳) => FromDesk::Denied { why: 訳 },
                },
            };
            let _ = 口.送る(&返事.書く()).await;
            Ok(())
        }
        // **札を頼む**（**D119** / **#26** / **#12**）。
        //
        // **部屋へは流さない。**この機械の中で決めて、この口へ返す ——
        // 2026-09-15、相手の画面に「許可が必要です」が届いたのは、
        // **聞く口と流す口が同じだった**からである。
        ToDesk::Ask { 動作, 訳 } => {
            let 答え = 頼みを受ける(app, &動作, &訳, 名乗り.as_deref());
            let _ = 口.送る(&FromDesk::Asked { 動作, 答え }.書く()).await;
            Ok(())
        }
        // **人が答えるまで待つ**（2026-09-24）。
        //
        // **待つだけで、答えは作らない**（**D56** ——「札を出すのは人である」）。
        // 時間切れなら `まだ` を返す。**「押させる」口ではない。**
        ToDesk::札を待つ { 動作, 訳, 秒 } => {
            let 待つ秒 = 秒.unwrap_or(既定の待ち秒).min(warifu_desk::札を待てる秒);
            // **まず帯を出す。**待つ前に頼んでおかないと、人は何も見ない
            let 答え = 頼みを受ける(app, &動作, &訳, 名乗り.as_deref());
            let 答え = if matches!(答え, warifu_desk::頼みの返り::まだ) {
                記録!("札: 待ちます（{動作}・{待つ秒} 秒・{}）", 呼び方(名乗り.as_deref()));
                答えを待つ(&動作, 待つ秒).await
            } else {
                // **すでに答えが出ているものは待たない**（許した／断った／受け付けない）
                答え
            };
            let _ = 口.送る(&FromDesk::Asked { 動作, 答え }.書く()).await;
            Ok(())
        }
        // **ルームキーを出す**（**#32 の段 2**・2026-09-21）。
        //
        // UI/UX の試験を除けば、エージェントが自分で動かせることのほうが大事である。
        //
        // **鍵を出すのに人を介す意味が無かった。**人が押しても判断していない。
        // **人の 1 回は札に置く**（`room.invite` を MCP 側が確かめる・**D119**）。
        //
        // **記録に鍵を書かない。**本数と期限だけ書く ——
        // 鍵は割符の片割れで、記録は追記であり消えない（落とし穴 8）。
        ToDesk::招く { 何本, 秒 } => {
            let bridge = app.state::<Bridge>();
            let ttl = 秒.unwrap_or(既定の鍵の寿命);
            let 開始 = crate::now_secs();
            let mut 鍵たち = Vec::with_capacity(何本 as usize);
            for _ in 0..何本 {
                match crate::鍵を出す(&bridge, ttl, Some(開始)).await {
                    Ok(鍵) => 鍵たち.push(鍵),
                    Err(e) => {
                        // **途中で失敗したら、出せた分だけ返さない。**
                        // 半分渡すと「何本渡したか」が両側で食い違う
                        記録!("招く: 出せませんでした（{} 本目・{}）", 鍵たち.len() + 1, e.message);
                        let _ = 口.送る(&FromDesk::Denied { why: e.message }.書く()).await;
                        return Ok(());
                    }
                }
            }
            記録!(
                "招く: ルームキーを {} 本出しました（{} まで・頼んだのは {}）",
                鍵たち.len(),
                crate::時刻の言い方(開始.saturating_add(ttl)),
                名乗り.as_deref().unwrap_or("名乗りなし")
            );
            let _ = 口
                .送る(
                    &FromDesk::招いた {
                        鍵たち,
                        いつまで: crate::時刻の言い方(開始.saturating_add(ttl)),
                    }
                    .書く(),
                )
                .await;
            Ok(())
        }
        // **ルームキーを、その相手の機械にだけ手渡す**（2026-10-20）。
        //
        // **人が画面で承認してから送る**（`key_give.rs`）。鍵の文字はエージェントへ返さない ——
        // 返りの形（`手渡しの返り`）に欄が無い。
        ToDesk::鍵を渡す { 相手, 訳, 秒 } => {
            let 待つ秒 = 秒.unwrap_or(既定の待ち秒).min(warifu_desk::札を待てる秒);
            let 結果 =
                crate::key_give::頼む(app, &相手, &訳, &呼び方(名乗り.as_deref()), 待つ秒).await;
            let _ = 口.送る(&FromDesk::手渡し { 結果 }.書く()).await;
            Ok(())
        }
        // **覚えている相手を並べる**（2026-10-16）。住所そのものは渡さない
        ToDesk::連絡先 => {
            let 返事 = match 呼べる相手を並べる() {
                Ok(行たち) => FromDesk::連絡先たち { 行たち },
                Err(why) => FromDesk::Denied { why },
            };
            let _ = 口.送る(&返事.書く()).await;
            Ok(())
        }
        // **いま持っている会話を書き出す**（2026-10-16）。
        //
        // 人はあとで必ずエージェントに「どんな話をしたっけ？」と聞く。議事録に使う。
        // **常には残さない** —— 返すのはこのプロセスが控えている直近の分だけで、閉じれば消える
        // **画面の写し**（0.1.34）。札（screen.read）は MCP 側が見る
        ToDesk::画面の写し => {
            let 返事 = match crate::snapshot::写してもらう(app).await {
                Ok(中身) => FromDesk::写し { 中身 },
                Err(why) => FromDesk::Denied { why },
            };
            let _ = 口.送る(&返事.書く()).await;
            Ok(())
        }
        ToDesk::書き出す => {
            let 自分 = crate::key_to_string(app.state::<Bridge>().device.public_key());
            let 全部 = 会話を書き出す(&自分);
            let 全部の数 = 全部.len();
            let 行たち = 書き出しを収める(全部, warifu_desk::返事の上限);
            記録!(
                "書き出し: {} 行を渡しました（頼んだのは {}・入りきらず落とした古い行 {}）",
                行たち.len(),
                名乗り.as_deref().unwrap_or("名乗りなし"),
                全部の数 - 行たち.len()
            );
            let _ = 口.送る(&FromDesk::書き出し { 行たち }.書く()).await;
            Ok(())
        }
        // **覚えている相手を、ルームキーなしで呼ぶ**（2026-10-16）。
        //
        // 画面の連絡帳の［ルームに呼ぶ］と同じ道（`call::呼ぶ`）。札（contacts.call）は MCP 側が見る。
        // **誰が呼んだかを記録に残す** —— 相手の画面が動くので、あとで辿れるように
        ToDesk::呼ぶ { 相手 } => {
            let 返事 = 連絡先から呼ぶ(app, &相手).await;
            記録!(
                "呼ぶ: {}（頼んだのは {}）",
                match &返事 {
                    FromDesk::呼んだ { 相手 } => format!("{相手} を呼びました"),
                    FromDesk::Denied { why } => format!("呼べませんでした: {why}"),
                    _ => String::new(),
                },
                名乗り.as_deref().unwrap_or("名乗りなし")
            );
            let _ = 口.送る(&返事.書く()).await;
            Ok(())
        }
        // **住所から連絡先に足す**（0.1.33）。札（contacts.add）は MCP 側が見て、ここでも消す
        ToDesk::連絡先を足す { 住所, 名前 } => {
            let 返事 = 住所で足す(app, &住所, &名前, &呼び方(名乗り.as_deref())).await;
            記録!(
                "連絡先を足す: {}（頼んだのは {}）",
                match &返事 {
                    FromDesk::足した { 名前 } => format!("{名前} を足しました"),
                    FromDesk::Denied { why } => format!("足せませんでした: {why}"),
                    _ => String::new(),
                },
                名乗り.as_deref().unwrap_or("名乗りなし")
            );
            let _ = 口.送る(&返事.書く()).await;
            Ok(())
        }
        // **ルームキーで入る**（0.1.30）。札（room.join）は MCP 側が見る。
        //
        // 画面の［入る］と同じ道（`connect`）を通る。**誰が入れたかを記録に残す** ——
        // 人が画面の前に居なくても、あとで「なぜこのルームに居るか」を辿れるように
        ToDesk::入る { 鍵 } => {
            let 返事 = 鍵で入る(app, 鍵.trim(), &呼び方(名乗り.as_deref())).await;
            記録!(
                "入る: {}（頼んだのは {}）",
                match &返事 {
                    FromDesk::入った { ルーム } => format!("ルーム {} へ入りました", crate::短く(ルーム)),
                    FromDesk::Denied { why } => format!("入れませんでした: {why}"),
                    _ => String::new(),
                },
                名乗り.as_deref().unwrap_or("名乗りなし")
            );
            let _ = 口.送る(&返事.書く()).await;
            Ok(())
        }
        // **棚にある部屋を並べる**（0.1.40・内部の Issue 026）。見ている部屋だけでは片付けられない
        ToDesk::部屋たち => {
            let bridge = app.state::<Bridge>();
            let 並び = crate::ルームの並び(&bridge).await;
            let 名前たち = crate::room_name::room_names();
            let 見ている = crate::いま見ているルーム(&bridge.いまのルーム)
                .await
                .map(|m| m.to_string());
            let 部屋 = 並び
                .into_iter()
                .map(|r| warifu_desk::部屋の行 {
                    名前: 名前たち.get(&r.id).cloned(),
                    人数: r.members,
                    主催か: r.host,
                    見ているか: 見ている.as_deref() == Some(r.id.as_str()),
                    id: r.id,
                })
                .collect();
            let _ = 口.送る(&FromDesk::部屋たち { 部屋 }.書く()).await;
            Ok(())
        }
        // **向こうに新しい版があるか尋ねる**（0.1.41・内部の Issue 027）。入れ替えは人が押す
        ToDesk::更新を確かめる => {
            let 返事 = 更新を確かめる(app).await;
            記録!("{} が更新を確かめました: {返事:?}", 呼び方(名乗り.as_deref()));
            let _ = 口.送る(&返事.書く()).await;
            Ok(())
        }
        // **部屋を指定して抜ける**（0.1.40）。画面の［このルームを抜ける］と同じ道
        ToDesk::抜ける { 部屋 } => {
            let 返事 = 部屋を抜ける(app, &部屋, &呼び方(名乗り.as_deref())).await;
            記録!(
                "抜ける: {}（頼んだのは {}）",
                match &返事 {
                    FromDesk::抜けた { 部屋 } => format!("部屋 {} を抜けました", crate::短く(部屋)),
                    FromDesk::Denied { why } => format!("抜けませんでした: {why}"),
                    _ => String::new(),
                },
                名乗り.as_deref().unwrap_or("名乗りなし")
            );
            let _ = 口.送る(&返事.書く()).await;
            Ok(())
        }
        // **相手に名前を付ける**（0.1.40）。画面の「名前を付ける」と同じ道
        ToDesk::相手に名前を付ける { 相手, 名前 } => {
            let 返事 = 相手に名前を付ける(app, &相手, &名前, &呼び方(名乗り.as_deref())).await;
            記録!(
                "相手に名前を付ける: {}（頼んだのは {}）",
                match &返事 {
                    FromDesk::名付けた { 鍵, 名前 } =>
                        format!("{} に「{名前}」", crate::短く(鍵)),
                    FromDesk::Denied { why } => format!("付けられませんでした: {why}"),
                    _ => String::new(),
                },
                名乗り.as_deref().unwrap_or("名乗りなし")
            );
            let _ = 口.送る(&返事.書く()).await;
            Ok(())
        }
        // **部屋に名前を付ける**（0.1.40）。画面の［決める］と同じ道
        ToDesk::名前を付ける { 部屋, 名前 } => {
            let 返事 = 部屋に名前を付ける(app, &部屋, &名前, &呼び方(名乗り.as_deref())).await;
            記録!(
                "名前を付ける: {}（頼んだのは {}）",
                match &返事 {
                    FromDesk::付けた { 部屋, 名前 } =>
                        format!("部屋 {} に「{名前}」", crate::短く(部屋)),
                    FromDesk::Denied { why } => format!("付けられませんでした: {why}"),
                    _ => String::new(),
                },
                名乗り.as_deref().unwrap_or("名乗りなし")
            );
            let _ = 口.送る(&返事.書く()).await;
            Ok(())
        }
        // **そこまで読んだ**（**D76**）。返事は要らない —— 数えるだけ
        ToDesk::Read { まで } => {
            読んだ(自分, まで);
            Ok(())
        }
        // **その発言の届き方を尋ねる**（**D76**）
        ToDesk::Status { id } => {
            let (届いた, 読んだ) = 届き方(id);
            let _ = 口
                .送る(
                    &FromDesk::Status {
                        id, 届いた, 読んだ
                    }
                    .書く(),
                )
                .await;
            Ok(())
        }
        // **いまの様子を返す**（2026-09-11）。押されたことを MCP から検知できるようにする。
        // **画面が既に持っている値をそのまま返す。**ここで数え直さない
        ToDesk::Status様子 => {
            let bridge = app.state::<Bridge>();
            let 見ている = crate::いま見ているルーム(&bridge.いまのルーム).await;
            let 名簿: Vec<String> = match 見ている {
                Some(id) => {
                    let 棚 = bridge.conferences.lock().await;
                    棚.get(&id).map_or_else(Vec::new, |c| {
                        c.members()
                            .iter()
                            .filter(|k| **k != bridge.device.public_key())
                            .map(|k| crate::key_to_string(*k))
                            .collect()
                    })
                }
                None => Vec::new(),
            };
            // **経路は「分かっている分だけ」返す。**
            // 相手が 1 人のときはその札、複数なら**全員が同じ札のときだけ**返す
            let 経路 = {
                let 棚 = bridge.経路.lock().await;
                let 札たち: Vec<String> = 名簿
                    .iter()
                    .filter_map(|k| k.parse::<warifu_core::PublicKey>().ok())
                    .filter_map(|k| 棚.get(&k.to_bytes()).cloned())
                    .collect();
                match 札たち.split_first() {
                    Some((先頭, 残り)) if 残り.iter().all(|x| x == 先頭) => Some(先頭.clone()),
                    _ => None,
                }
            };
            let 映り = bridge.画面に映っているもの.lock().await.clone();
            let _ = 口
                .送る(
                    &FromDesk::様子 {
                        ルーム: 見ている.map(|id| id.to_string()),
                        名簿,
                        経路,
                        エージェント: つながっている顔ぶれ(),
                        待っているリンク: crate::link::待っている数(),
                        待っているルーム: crate::link::待っているルームたち(),
                        // **画面が置きに来ている分だけ**（**#32**）。
                        // **置きに来ていなければ `None`** ——「出ていない」ではない
                        送っている: 映り.as_ref().map(|m| m.送っている),
                        受けている: 映り.as_ref().map(|m| m.受けている),
                        掴んでいる: 映り.as_ref().map(|m| m.掴んでいる),
                        題字: 映り.as_ref().map(|m| m.題字.clone()),
                        // **本体が見比べた食い違い**（spec 04 S04-8）。名乗った口にだけ付ける（古い口は読めない）
                        食い違い: if *点検も聞く {
                            bridge.いまの食い違い.lock().await.clone()
                        } else {
                            Vec::new()
                        },
                    }
                    .書く(),
                )
                .await;
            Ok(())
        }
        // **エージェントの声で言う**（#50）。
        // **名乗っていないエージェントの声は流さない**（会話へ流す行の話し手が決まらない）。
        // 流すかは人がルームごとに入れた印で決まる（`voice.rs`）
        ToDesk::声 { .. } if 名乗り.is_none() => {
            let _ = 口
                .送る(
                    &FromDesk::Denied {
                        why: 名乗ってください.to_owned(),
                    }
                    .書く(),
                )
                .await;
            Ok(())
        }
        ToDesk::声 { 本文 } => {
            // **必ず返事をする。**返さないと、言った側は待ち続ける（**D49**）
            let 結果 = crate::voice::言う(app, &本文, 自分, 呼び方(名乗り.as_deref())).await;
            let _ = 口.送る(&FromDesk::声の返り { 結果 }.書く()).await;
            Ok(())
        }
        ToDesk::Say { .. } if 名乗り.is_none() => {
            // **名乗っていないエージェントからは流さない。**
            // 流すと「どこの誰か分からない発言」が人の画面に並ぶ
            let _ = 口
                .送る(
                    &FromDesk::Denied {
                        why: 名乗ってください.to_owned(),
                    }
                    .書く(),
                )
                .await;
            Ok(())
        }
        ToDesk::Say { body, room } => {
            // **部屋を指されたら、その部屋へ**（0.1.40・内部の Issue 026）。
            // 指されなければ画面が見ているルーム（今までどおり）
            let 部屋 = match room.as_deref() {
                Some(指定) => {
                    let 候補: Vec<String> = app
                        .state::<Bridge>()
                        .conferences
                        .lock()
                        .await
                        .keys()
                        .map(ToString::to_string)
                        .collect();
                    match 部屋を引く(&候補, 指定).and_then(|id| {
                        id.parse::<MeetingId>()
                            .map_err(|_| format!("部屋の id が読めません（{id}）"))
                    }) {
                        Ok(m) => Some(m),
                        Err(why) => {
                            let _ = 口.送る(&FromDesk::Denied { why }.書く()).await;
                            return Ok(());
                        }
                    }
                }
                None => None,
            };
            // **必ず返事をする。**返さないと、言った側は待ち続ける（**D49**）
            let 返事 = match 流す(app, &body, 自分, 呼び方(名乗り.as_deref()), 部屋).await {
                Ok((id, 届いた, 流した先)) => FromDesk::Sent {
                    to: 届いた.len(),
                    id,
                    届いた,
                    room: 流した先.map(|m| m.to_string()),
                },
                Err(訳) => {
                    記録!("この機械からの発言を流せませんでした: {訳}");
                    // 相手が居ないのと、会議そのものが無いのは違う。**混ぜない**
                    if 訳 == 誰も居ない {
                        FromDesk::Nobody
                    } else {
                        FromDesk::Denied { why: 訳 }
                    }
                }
            };
            let _ = 口.送る(&返事.書く()).await;
            Ok(())
        }
    }
}

/// 名乗っていない相手へ返す言い分。**どうすればつながれるかまで言う。**
///
/// **同じ PC の中の相手なので、手順を教えてよい。**
/// 「断る理由を相手に返さない」（**D31**）は、**網の向こうの知らない相手**の話である。
const 名乗ってください: &str = concat!(
    // **行を折り返すのに `\` を使わない。**続きの行の字下げがそのまま文に入る
    // （2026-09-08 に実物で出た: 「入り直すか、␣␣␣␣␣エージェントを」）
    "先に名乗ってください。ターミナルで `warifu mcp --as <名前>` として入り直すか、",
    "エージェントを立て直してください（起動した場所のフォルダ名が名乗りになります）"
);

/// 相手が 1 人も居ないときの言い分。**判定に使うので、1 か所に置く。**
///
/// **この画面からは、もう出ない。**この PC の人がいつでも聞いているためである
/// （2026-09-08・`流す` の注記）。この機械の作りが変わったときのために残してある。
const 誰も居ない: &str = "まだ誰も居ません";

/// この機械から来た発言を、会話へ流す。**届いた人数を返す。**
///
/// **人の画面にも、繋がっている相手にも、同じ 1 行が出る。**
/// 片方にしか出ないと、見ている人と話している人がずれる。
///
/// # 画面は「届いた先」に数える
///
/// **チャットは会議を前提にしない**（2026-09-06 / **D55**）。
/// 会議の相手だけを数えていたので、**この PC の人には出ているのに
/// エージェントには「誰にも届いていません」と返っていた**
/// （2026-09-08 に実物で踏んだ）。返された側は言い直すか、諦める。
///
/// # どの部屋へ（0.1.40・内部の Issue 026）
///
/// `部屋` を指されたらそこへ。指されなければ**画面が見ているルーム**へ（今までどおり）。
/// 返事が声掛けの来た部屋へ戻らなかった —— 口は届いた行の部屋をそのまま指してくる。
pub(crate) async fn 流す(
    app: &AppHandle,
    body: &str,
    出所: u64,
    呼び方: String,
    部屋: Option<MeetingId>,
) -> Result<(u64, Vec<String>, Option<MeetingId>), String> {
    let bridge = app.state::<Bridge>();
    let 自分 = bridge.device.public_key();
    let 見ている = crate::いま見ているルーム(&bridge.いまのルーム).await;
    let 流す先 = 部屋.or(見ている);
    if let Some(m) = 部屋
        && !bridge.conferences.lock().await.contains_key(&m)
    {
        return Err(format!("その部屋に居ません（{}）", crate::短く(&m.to_string())));
    }

    // **会話に出す名前は、名簿と同じ決め方を通す**（2026-09-09）。
    // 通さないと、名簿では「かくにん係」、会話では「kakunin のエージェント」と
    // **同じエージェントが 2 つの名前で出る** —— 人はそれを 2 人だと読む。
    let 名札 = crate::profile::エージェントの名札(&呼び方);

    // **先に人の画面へ出す。**会議が無くても、この PC の人には見える。
    // **どの部屋へかも渡す**（0.1.40）。渡さないと画面はその瞬間に見ているルームへ積む
    let 部屋の札 = 流す先.map(|m| m.to_string()).unwrap_or_default();
    let _ = app.emit(
        EVENT_DESK,
        (名札.clone(), body.to_owned(), いま時刻(), 部屋の札.clone()),
    );
    // **同じエージェントの AI が喋ったことにも気づけるようにする。**
    // 窓が前に居るときは鳴らないので、うるさくならない
    crate::notify::届いたと知らせる(app, &名札);
    // **通し番号を振る。**あとで「誰が読んだか」を尋ねられるようにする（**D76**）
    let id = 番号を振る();
    // この機械につながっている**他の**エージェントにも同じ行を見せる（言った本人には返さない）
    let 行 = FromDesk::Heard {
        id,
        from: 名札.clone(),
        body: body.to_owned(),
        at: いま時刻(),
        room: 流す先.map(|m| m.to_string()),
    };
    // **エージェントの発言も控える**（2026-10-16）。控えないと、書き出した議事録から
    // この PC のエージェントが言ったことだけが抜ける
    控える(&行);
    let _ = bridge.desk.send((出所, 行));

    // **誰に届いたか。**規則は `届いた先を並べる` に 1 か所だけ置く
    let mut 届いた = 届いた先を並べる(出所);

    let Some(meeting) = 流す先 else {
        // 会議が無くても、**この PC の人には届いている。**失敗にしない
        届けた(id, 届いた.clone());
        return Ok((id, 名札で並べる(&届いた), None));
    };

    // **そのルームに居る相手だけへ。**全員へ配ると、別のルームの人にも届く
    let 送り先 = crate::そのルームの相手(&bridge.conferences, &bridge.outbound, meeting, 自分).await;
    for tx in &送り先 {
        // 届かない相手が居ても止めない。**送る側を待たせない**
        let _ = tx
            .send(warifu_meeting::Notice::Text {
                meeting,
                // **エージェントの持ち主は自分**（D48）。AI は自分のエージェントから喋っている
                from: 自分,
                // **そのエージェントの誰が言ったかを、線の向こうへも運ぶ**（2026-09-08）。
                // 載せないと、相手の画面では人が打ったのと区別が付かない。
                // **名乗っていれば名前も載せる**（`資料くん（docs）`・**D75**）——
                // ただし**どこのエージェントかは落とさない**（落とすと取り違えられる）
                話し手: Some(名札.clone()),
                body: body.to_owned(),
            })
            .await;
    }
    // ルームの相手は**名前を持たない**（呼び名は画面の側にある）ので、人数だけを足す。
    // **持っていないものを名乗らない**
    for _ in 0..送り先.len() {
        届いた.push(ルームの相手.to_owned());
    }
    届けた(id, 届いた.clone());
    Ok((id, 名札で並べる(&届いた), Some(meeting)))
}

/// 覚えている相手を（鍵, 呼び名）の組で並べる。**読めなければ空**（手渡し・書き出しで使う）。
pub(crate) fn 連絡先の組() -> Vec<(String, String)> {
    名前のある連絡先()
        .unwrap_or_default()
        .into_iter()
        .map(|r| (r.鍵, r.名前))
        .collect()
}

/// **誰かが入った・出たことを、この機械のエージェントにも見せる**（2026-10-20）。
///
/// 連絡先から呼ばれた側は、知り合いなら人に聞かずにルームへ入る。
/// **見せないと、エージェントは自分から `room_status` を見るまで気づけない。**
///
/// 名前は書き出しと同じ決め方にする（覚えている相手なら呼び名を前に出す）。
/// **返した名前は人への通知にも使う**（画面とエージェントで呼び方を揃える）。
pub fn 入退室を配る(bridge: &Bridge, 鍵: &str, 入った: bool) -> String {
    let 名 = 名を引く(bridge, 鍵);
    let 中身 = if 入った {
        FromDesk::Joined { who: 名.clone() }
    } else {
        FromDesk::Left { who: 名.clone() }
    };
    配る(bridge, 中身);
    名
}

/// **エージェントに見せる名**（呼び名があれば呼び名・無ければ鍵）。
pub fn 名を引く(bridge: &Bridge, 鍵: &str) -> String {
    let 帳 = 連絡先の組();
    let 自分 = crate::key_to_string(bridge.device.public_key());
    crate::contacts::書き出しの名(&帳, &自分, 鍵)
}

/// **書き出しを、返事の 1 行に収める**（0.1.38）。収まらなければ**古い行から落とす。**
///
/// 返事は 1 行で、受ける側は [`warifu_desk::返事の上限`] を超えた行を落とす。
/// 超えたまま送ると、エージェントには何も届かず「この機械が返事をしません」になる（0.1.37）。
fn 書き出しを収める(
    mut 行たち: Vec<warifu_desk::書き出しの行>,
    上限: usize,
) -> Vec<warifu_desk::書き出しの行> {
    let 長さ = |行たち: &[warifu_desk::書き出しの行]| {
        FromDesk::書き出し {
            行たち: 行たち.to_vec(),
        }
        .書く()
        .len()
    };
    while !行たち.is_empty() && 長さ(&行たち) > 上限 {
        行たち.remove(0);
    }
    行たち
}

/// **控えている会話を、書き出す形にする**（2026-10-16）。古い順。誰が、には呼び名を添える。
fn 会話を書き出す(自分: &str) -> Vec<warifu_desk::書き出しの行> {
    let 帳: Vec<(String, String)> = 名前のある連絡先()
        .unwrap_or_default()
        .into_iter()
        .map(|r| (r.鍵, r.名前))
        .collect();
    let 棚 = 直近の発言.lock().expect("毒されていない");
    棚.iter()
        .filter_map(|(_, 行)| match 行 {
            FromDesk::Heard {
                id, from, body, at, ..
            } => Some(warifu_desk::書き出しの行 {
                番号: *id,
                誰: crate::contacts::書き出しの名(&帳, 自分, from),
                時刻: at.clone(),
                本文: body.clone(),
            }),
            _ => None,
        })
        .collect()
}

/// **覚えている相手を、エージェントへ渡す形にする**（2026-10-16）。住所そのものは載せない。
fn 呼べる相手を並べる() -> Result<Vec<warifu_desk::連絡先の行>, String> {
    let 置き場 = warifu_vault::Vault::default_location()
        .map_err(|e| format!("連絡先を読めませんでした: {e}"))?;
    let 帳 = 置き場
        .contacts()
        .map_err(|e| format!("連絡先を読めませんでした: {e}"))?;
    // **名前を付けていない相手も、ルームで会っていれば鍵の頭で呼べる**（0.1.32）。
    // 名前は付けない（知らない相手を、知っているように見せない・D46）
    let 会った = 置き場.met().unwrap_or_else(|e| {
        記録!("会った相手を読めませんでした（名前のある相手だけを並べます）: {e}");
        Vec::new()
    });
    let mut 行たち: Vec<warifu_desk::連絡先の行> = 帳
        .iter()
        .map(|c| warifu_desk::連絡先の行 {
            名前: c.label().to_owned(),
            鍵: c.key().to_string(),
            // **呼べるかは `住所を引く` と同じに決める**（0.1.40）：連絡帳に住所が無くても、
            // ルームで会ったときの住所があれば呼べる。CLI の `contacts add` で名前だけ付けた相手が
            // 「住所を知らない（呼べない）」と出ていたが、呼べていた（10-27 に実物で見た）
            住所あり: 呼べるか(c, &会った),
        })
        .collect();
    行たち.extend(名前の無い会った相手(&行たち, 会った));
    Ok(行たち)
}

/// 名前を付けた相手だけ（書き出し・入退室の名前に使う）。**名前の無い行を名前として出さない。**
fn 名前のある連絡先() -> Result<Vec<warifu_desk::連絡先の行>, String> {
    Ok(呼べる相手を並べる()?
        .into_iter()
        .filter(|r| r.名前 != 名前の無い相手)
        .collect())
}

/// 覚えている相手を呼べるか。連絡帳の住所か、ルームで会ったときの住所のどちらかがあればよい。
fn 呼べるか(c: &warifu_vault::Contact, 会った: &[(warifu_core::PublicKey, String)]) -> bool {
    c.address().is_some() || 会った.iter().any(|(k, _)| *k == c.key())
}

/// 会った相手のうち、**連絡帳に無い者だけ**を、名前の無い行にする。新しく会った順。
fn 名前の無い会った相手(
    帳: &[warifu_desk::連絡先の行],
    会った: Vec<(warifu_core::PublicKey, String)>,
) -> Vec<warifu_desk::連絡先の行> {
    会った
        .into_iter()
        .rev()
        .map(|(鍵, _)| 鍵.to_string())
        .filter(|鍵| !帳.iter().any(|r| r.鍵 == *鍵))
        .map(|鍵| warifu_desk::連絡先の行 {
            名前: 名前の無い相手.to_owned(),
            鍵,
            住所あり: true,
        })
        .collect()
}

/// 名前の無い相手の、名前の欄。**鍵の頭で呼んでもらう。**
const 名前の無い相手: &str = "（名前なし・ルームで会った相手）";

/// **相手の住所から、連絡先と戸口に足す**（0.1.33）。
///
/// 戸口にも載せるので、相手から鍵なしで呼べるようになる。**札は 1 回で消え、人に必ず知らせる**
/// （`鍵で入る` と同じ構え）。連絡帳は名前のため、戸口は通すかの判断のため（2 つを 1 つにしない）。
///
/// **札を見るのは、足せると決まってから**（0.1.40・`contacts::札を使って足す`）。断られた足し方で
/// 札を消さない。消す場所は `使い切りの札を受け取る` 1 か所のまま。
async fn 住所で足す(app: &AppHandle, 住所: &str, 名前: &str, 誰: &str) -> FromDesk {
    let bridge = app.state::<Bridge>();
    let 自分 = bridge.device.public_key();
    let 結果 = warifu_vault::Vault::default_location()
        .map_err(|e| format!("連絡先を読めませんでした: {e}"))
        .and_then(|v| {
            let mut 帳 = v.contacts().map_err(|e| format!("連絡先を読めませんでした: {e}"))?;
            let 相手 = crate::contacts::札を使って足す(
                &mut 帳,
                自分,
                住所,
                名前,
                crate::now_secs(),
                || 使い切りの札を受け取る("contacts.add"),
            )?;
            v.save_contacts(&帳)
                .map_err(|e| format!("連絡先を書けませんでした: {e}"))?;
            Ok(相手)
        });
    let 相手 = match 結果 {
        Ok(相手) => 相手,
        Err(why) => return FromDesk::Denied { why },
    };
    {
        let mut door = bridge.door.lock().await;
        crate::contacts::迎える(&mut door, 相手);
    }
    crate::notify::エージェントが連絡先を足したと知らせる(
        app,
        誰,
        名前.trim(),
        &crate::短く(&crate::key_to_string(相手)),
    );
    FromDesk::足した {
        名前: 名前.trim().to_owned(),
    }
}

/// **向こうに新しい版があるか尋ねる**（0.1.41・内部の Issue 027）。
///
/// 確かめるだけで、**落として入れ替えるのは人が画面で押す**。新しい版があれば、
/// メニューの「更新を確認…」と同じ出来事を画面へ出して、更新の帯を見せる。
/// `.app` の外で動いているときは確かめない（`bundle.rs` の決まり。置き場所が消える）。
async fn 更新を確かめる(app: &AppHandle) -> FromDesk {
    use tauri_plugin_updater::UpdaterExt;
    let いまの版 = env!("CARGO_PKG_VERSION").to_owned();
    let 返す = |新しい版: Option<String>, 何が変わったか: Option<String>, 確かめられない: Option<String>| {
        FromDesk::更新 {
            いまの版: いまの版.clone(),
            新しい版,
            何が変わったか,
            確かめられない,
        }
    };
    if !crate::bundle::can_update() {
        return 返す(
            None,
            None,
            Some(".app の外で動いているので、更新は確かめません".to_owned()),
        );
    }
    let 向こう = match app.updater() {
        Ok(更新) => 更新.check().await,
        Err(e) => Err(e),
    };
    match 向こう {
        Ok(Some(新しい)) => {
            // **画面に帯を出す**（人が押す所）。確かめるのは画面の側（`+page.svelte` の `更新を確かめる`）
            let _ = app.emit(crate::EVENT_CHECK_UPDATE, ());
            返す(Some(新しい.version), 新しい.body, None)
        }
        Ok(None) => 返す(None, None, None),
        Err(e) => 返す(
            None,
            None,
            Some(format!("向こうに尋ねられませんでした（{e}）")),
        ),
    }
}

/// **口から部屋を抜ける**（0.1.40・内部の Issue 026）。
///
/// 居ない部屋・読めない指し方は**札を見る前に**断る（札を使った動作ではない）。
/// 抜けたら画面に読み直させ、この機械の人とエージェントに 1 行で知らせる。
async fn 部屋を抜ける(app: &AppHandle, 指定: &str, 誰: &str) -> FromDesk {
    let bridge = app.state::<Bridge>();
    let 候補: Vec<String> = bridge
        .conferences
        .lock()
        .await
        .keys()
        .map(ToString::to_string)
        .collect();
    let id = match 部屋を引く(&候補, 指定) {
        Ok(id) => id,
        Err(why) => return FromDesk::Denied { why },
    };
    let Ok(meeting) = id.parse::<MeetingId>() else {
        return FromDesk::Denied {
            why: format!("部屋の id が読めません（{id}）"),
        };
    };
    if !使い切りの札を受け取る("room.leave") {
        return FromDesk::Denied {
            why: "札（room.leave）を人が許していません（1 回使うと消えます。pass_ask で頼み直してください）"
                .to_owned(),
        };
    }
    crate::部屋を畳む(&bridge, meeting).await;
    // **画面に読み直させる。**抜けたのは口なので、画面は自分では気づけない
    let _ = app.emit(crate::EVENT_LOOK, ());
    配る(
        &bridge,
        聞いた("割符", &format!("{誰} が部屋 {} を抜けました", crate::短く(&id))),
    );
    FromDesk::抜けた { 部屋: id }
}

/// **口から相手に名前を付ける**（0.1.40・内部の Issue 016 の 26）。
///
/// 画面の `remember` と同じ：呼び名を書き、いま知っている住所（線の相手の名乗り、無ければ
/// ルームで会ったときの住所）も書く。居ない相手・2 人に当たる指し方は**札を見る前に**断る。
async fn 相手に名前を付ける(app: &AppHandle, 指定: &str, 名前: &str, 誰: &str) -> FromDesk {
    let 帳 = match 呼べる相手を並べる() {
        Ok(行たち) => 行たち,
        Err(why) => return FromDesk::Denied { why },
    };
    let 組: Vec<(String, String)> = 帳.into_iter().map(|r| (r.鍵, r.名前)).collect();
    let 鍵 = match crate::contacts::呼ぶ相手を決める(&組, 指定) {
        Ok(鍵) => 鍵,
        Err(why) => return FromDesk::Denied { why },
    };
    let Ok(who) = 鍵.parse::<warifu_core::PublicKey>() else {
        return FromDesk::Denied {
            why: format!("鍵が読めません（{}）", crate::短く(&鍵)),
        };
    };
    if !使い切りの札を受け取る("contacts.add") {
        return FromDesk::Denied {
            why: crate::contacts::札が無い.to_owned(),
        };
    }
    let bridge = app.state::<Bridge>();
    let 名前 = 名前.trim();
    let 住所 = match bridge.addresses.lock().await.get(&who.to_bytes()).cloned() {
        Some(住所) => Some(住所),
        None => warifu_vault::Vault::default_location()
            .and_then(|v| v.met())
            .ok()
            .and_then(|会った| 会った.into_iter().find(|(k, _)| *k == who).map(|(_, a)| a)),
    };
    let 結果 = warifu_vault::Vault::default_location()
        .map_err(|e| format!("連絡先を読めませんでした: {e}"))
        .and_then(|v| {
            let mut list = v.contacts().map_err(|e| format!("連絡先を読めませんでした: {e}"))?;
            list.add(who, 名前, crate::now_secs())
                .map_err(|e| format!("名前を付けられません: {e}"))?;
            if let Some(住所) = &住所 {
                let _ = list.note_address(who, 住所);
            }
            v.save_contacts(&list)
                .map_err(|e| format!("連絡先を書けませんでした: {e}"))
        });
    if let Err(why) = 結果 {
        return FromDesk::Denied { why };
    }
    // **画面に読み直させる**（名簿と連絡帳）。付けたのは口なので、画面は自分では気づけない
    let _ = app.emit(crate::EVENT_LOOK, ());
    配る(
        &bridge,
        聞いた(
            "割符",
            &format!("{誰} が {} に「{名前}」と名前を付けました", crate::短く(&鍵)),
        ),
    );
    FromDesk::名付けた {
        鍵,
        名前: 名前.to_owned(),
    }
}

/// **口から部屋に名前を付ける**（0.1.40）。居ない部屋は札を見る前に断る。
async fn 部屋に名前を付ける(app: &AppHandle, 指定: &str, 名前: &str, 誰: &str) -> FromDesk {
    let bridge = app.state::<Bridge>();
    let 候補: Vec<String> = bridge
        .conferences
        .lock()
        .await
        .keys()
        .map(ToString::to_string)
        .collect();
    let id = match 部屋を引く(&候補, 指定) {
        Ok(id) => id,
        Err(why) => return FromDesk::Denied { why },
    };
    if !使い切りの札を受け取る("room.rename") {
        return FromDesk::Denied {
            why: "札（room.rename）を人が許していません（1 回使うと消えます。pass_ask で頼み直してください）"
                .to_owned(),
        };
    }
    let 名前 = 名前.trim();
    if let Err(e) = crate::room_name::付ける(&bridge, &id, 名前).await {
        return FromDesk::Denied {
            why: format!("名前を付けられませんでした: {}", e.message),
        };
    }
    // **画面にも同じ名前を出す**（人が付けたときと同じ出来事）
    let _ = app.emit(crate::room_name::EVENT_ROOM_NAME, (id.clone(), 名前.to_owned()));
    配る(
        &bridge,
        聞いた("割符", &format!("{誰} が部屋 {} に「{名前}」と名前を付けました", crate::短く(&id))),
    );
    FromDesk::付けた {
        部屋: id,
        名前: 名前.to_owned(),
    }
}

/// **ルームキーで入る**（0.1.30）。画面の［入る］と同じ道を、**エージェントの入り方**で通る。
///
/// **見ているルームを奪わない・主催を戸口に載せない・帰り道を残さない**（`誰が入れるか`）。
/// 入ったら、**必ず人に知らせる**（OS の通知と記録）。返すのは鍵から読んだ会議の id。
async fn 鍵で入る(app: &AppHandle, 鍵: &str, 誰: &str) -> FromDesk {
    // **札は画面の側でも確かめて、ここで消す**（口の関所だけに任せない・`使い切りの札を受け取る`）
    if !使い切りの札を受け取る("room.join") {
        return FromDesk::Denied {
            why: "札（room.join）を人が許していません（1 回使うと消えます。pass_ask で頼み直してください）"
                .to_owned(),
        };
    }
    let bridge = app.state::<Bridge>();
    match crate::会議キーで入る(
        app.clone(),
        &bridge,
        鍵.to_owned(),
        crate::誰が入れるか::エージェント,
    )
    .await
    {
        Ok(会議) => {
            let 主催 = bridge
                .主催たち
                .lock()
                .await
                .get(&会議)
                .map(|k| crate::短く(&crate::key_to_string(*k)))
                .unwrap_or_default();
            crate::notify::エージェントが入ったと知らせる(app, 誰, &主催);
            FromDesk::入った {
                ルーム: 会議.to_string(),
            }
        }
        Err(e) => FromDesk::Denied {
            why: format!("入れませんでした: {}", e.message),
        },
    }
}

/// **札の控え（`passes`）を読み書きするときの錠**（0.1.30）。
///
/// 人が札を押すのと、使い切りの札を消すのが同時に走ると、片方の書き換えが消える
/// （「札が残る」か「人が許した札が消える」）。読み書きはこの錠の中でだけ行う。
static 札の控えの錠: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// **使い切りの札を受け取る**（0.1.30）。人が許していれば控えから落として `true`。
///
/// `room.join` は **1 回使ったら終わり**。確かめて消すのは**ここ 1 か所**にする ——
/// 口（MCP）の関所は口ごとに札を覚えるので、口が 2 つ・同時の呼び・時間切れ・
/// `--allow` で、人に聞かずにもう 1 回入れていた（2026-10-21 のセキュリティ審査）。
/// **入る前に消す。**入れなかったときも札は戻らない（次はまた人に頼む）。
fn 使い切りの札を受け取る(動作: &str) -> bool {
    let _錠 = 札の控えの錠.lock().expect("毒されていない");
    let 結果 = warifu_vault::Vault::default_location().and_then(|v| {
        let 一覧 = v.passes_with_until()?;
        let (通す, 残す) = 札を受け取る(一覧, 動作, crate::now_secs());
        v.save_passes_with_until(&残す)?;
        Ok(通す)
    });
    match 結果 {
        Ok(受け取れた) => 受け取れた,
        Err(e) => {
            // **読めない・書けないなら通さない。**消せない札で入ると、使い切りにならない
            記録!("札 {動作} を確かめられませんでした: {e}");
            false
        }
    }
}

/// **札を 1 つ受け取る**（純粋・0.1.41・D129）。（通すか, 残す一覧）を返す。
///
/// - 期限の無い札（人が［通す］を押した分）は、受け取ったら消える（今までどおり・10-21 の審査）
/// - **期限のある札（［24 時間聞かない］）は、期限内なら消さずに通す。**期限を過ぎていれば消して通さない
/// - 無い・断っている札は通さない
pub fn 札を受け取る(
    一覧: Vec<(String, bool, Option<u64>)>,
    動作: &str,
    今: u64,
) -> (bool, Vec<(String, bool, Option<u64>)>) {
    let 通す = 一覧
        .iter()
        .any(|(a, 許した, まで)| a == 動作 && *許した && まで.is_none_or(|t| t > 今));
    let 残す = 一覧
        .into_iter()
        .filter(|(a, _, まで)| {
            if a != 動作 {
                return true;
            }
            // 期限内の持続の札だけ残す。使い切りと期限切れは消す
            まで.is_some_and(|t| t > 今)
        })
        .collect();
    (通す, 残す)
}

/// **人が前もって出している札**（期限内のもの・0.1.41）。画面が一覧に出し、取り消せる。
#[must_use]
pub fn 持続の札() -> Vec<(String, u64)> {
    let 今 = crate::now_secs();
    warifu_vault::Vault::default_location()
        .and_then(|v| v.passes_with_until())
        .unwrap_or_default()
        .into_iter()
        .filter_map(|(a, 許した, まで)| match まで {
            Some(t) if 許した && t > 今 => Some((a, t)),
            _ => None,
        })
        .collect()
}

/// **人が札を取り消す**（0.1.41）。持続の札も使い切りの札も、その動作の行を消す。
///
/// # Errors
/// 控えへ書けないとき。
pub fn 札を取り消す(動作: &str) -> Result<(), String> {
    let _錠 = 札の控えの錠.lock().expect("毒されていない");
    let vault = warifu_vault::Vault::default_location().map_err(|e| e.to_string())?;
    let 残す: Vec<(String, bool, Option<u64>)> = vault
        .passes_with_until()
        .unwrap_or_default()
        .into_iter()
        .filter(|(a, _, _)| a != 動作)
        .collect();
    vault.save_passes_with_until(&残す).map_err(|e| e.to_string())?;
    記録!("札: 取り消した（{動作}）");
    Ok(())
}

/// **名前（か鍵の頭）から相手を決めて、画面の［ルームに呼ぶ］と同じ道で呼ぶ**（2026-10-16）。
async fn 連絡先から呼ぶ(app: &AppHandle, 指定: &str) -> FromDesk {
    let 行たち = match 呼べる相手を並べる() {
        Ok(行たち) => 行たち,
        Err(why) => return FromDesk::Denied { why },
    };
    let 帳: Vec<(String, String)> = 行たち
        .iter()
        .map(|r| (r.鍵.clone(), r.名前.clone()))
        .collect();
    let 鍵 = match crate::contacts::呼ぶ相手を決める(&帳, 指定) {
        Ok(鍵) => 鍵,
        Err(why) => return FromDesk::Denied { why },
    };
    let 名前 = 行たち
        .iter()
        .find(|r| r.鍵 == 鍵)
        .filter(|r| r.名前 != 名前の無い相手)
        .map_or_else(|| crate::短く(&鍵), |r| r.名前.clone());
    let Ok(相手) = 鍵.parse::<warifu_core::PublicKey>() else {
        return FromDesk::Denied {
            why: "連絡先の鍵が読めません".to_owned(),
        };
    };
    let bridge = app.state::<Bridge>();
    match crate::call::呼ぶ(app, &bridge, 相手).await {
        Ok(()) => FromDesk::呼んだ { 相手: 名前 },
        Err(e) => FromDesk::Denied {
            why: format!("{名前} を呼べませんでした: {}", e.message),
        },
    }
}

/// 棚に置いた**生の名乗り**を、画面と同じ**名札**に直す。
///
/// **棚は生のまま、出すときだけ名札にする** —— 棚を名札で持つと、
/// 名前を書き換えたときに古い名札と食い違って**既読が外れる。**
fn 名札で並べる(名たち: &[String]) -> Vec<String> {
    名たち
        .iter()
        .map(|名| crate::profile::エージェントの名札(名))
        .collect()
}

/// 画面（この PC の人）の呼び方。**エージェントではないが、届いた先ではある。**
const 画面: &str = "画面";
/// ルームの相手の呼び方。**名前は画面の側にあるので、ここでは数えるだけ。**
const ルームの相手: &str = "ルームの相手";

/// **その行が誰に届くか**を並べる。
///
/// 規則は 2 つだけである ——
///
/// - **画面（人）は必ず 1 つとして並べる。**「3 人」だけでは誰に届いたか分からない（D49）
/// - **言った本人には返さない。**返すと、自分の言葉に自分で反応して止まらなくなる
///   （`warifu agent` は届いた行で起きるので、ここが崩れると回り続ける）
///
/// 人が打った行は「この機械の外」（[`この機械の外`]）から来る ——
/// **どのエージェントとも一致しないので、つながっている全員が受け取る。**
/// だから **A のエージェントの言葉が B のエージェントに届く**（人を介さずに往復できる）。
///
/// **棚には生の名乗りを置く。**名札（画面に出す名前）は人が書き換えると変わるので、
/// これで照合すると**あとから既読が外れる**（D76）。
fn 届いた先を並べる(出所: u64) -> Vec<String> {
    let 他のエージェントの名: Vec<String> = エージェントたち
        .lock()
        .expect("毒されていない")
        .iter()
        .filter(|(エージェント, _)| **エージェント != 出所)
        .map(|(_, 名)| 名.clone())
        .collect();
    let mut 届いた = vec![画面.to_owned()];
    // **並びを固定する。**棚は `HashMap` なので、そのままでは順が揺れて
    // 「誰に届いたか」の行が毎回入れ替わる（読む人が差分だと勘違いする）
    let mut 他 = 他のエージェントの名;
    他.sort();
    届いた.extend(他);
    届いた
}

/// 会話に出たことを、この機械につながっている全員へ配る。
///
/// **人が打った行も、相手から届いた行も、ここを通す。**
/// 通さないと、エージェントは人の発言が見えないまま返事をすることになる。
pub fn 配る(bridge: &Bridge, 中身: FromDesk) {
    // 誰も着いていなければ落ちる。**それは失敗ではない**
    let _ = bridge.desk.send((この機械の外, 中身));
}

/// **そのエージェントにだけ渡す。**宛先が着いていなければ `false`。
///
/// 3 つも 4 つもこの機械につながっていると、**1 人だけに聞きたくても全員に飛ぶ** ——
/// それでは 1 対 1 が成り立たない（2026-09-08）。同じ機械の中で円滑にやりとりできなければ、
/// 別の端末とはなおさらできない。
///
/// **配る仕組みは 1 つのまま。**宛先のエージェント以外が捨てる形にする ——
/// エージェントごとに別の口を張ると、**誰に届いたかを数える所が 2 つになる。**
pub fn エージェントへ配る(bridge: &Bridge, 宛先: &str, 中身: FromDesk) -> bool {
    let 番号 = エージェントたち
        .lock()
        .expect("毒されていない")
        .iter()
        .find(|(_, 呼び方)| 呼び方.as_str() == 宛先)
        .map(|(n, _)| *n);
    let Some(番号) = 番号 else {
        // **つながっていない相手へ送ったことにしない**
        return false;
    };
    let _ = bridge.desk.send((宛先の印(番号), 中身));
    true
}

/// **そのエージェントに「止まれ」と言う。**着いていなければ `false`。
///
/// 常駐（`warifu agent`）は人が居ない間も動く。
/// **落とすしか止め方が無い状態にしない**（`issues/014`）。
///
/// **相手のプロセスを殺さない。**受けた側が自分で降りる ——
/// 割符が他人の機械の中まで手を伸ばさない、という線はここでも引く。
pub fn エージェントを止める(bridge: &Bridge, 宛先: &str) -> bool {
    エージェントへ配る(bridge, 宛先, FromDesk::Stop)
}

/// 宛先つきで配るときの出所。
///
/// 出所は「**返さない相手**」を指す番号なので、
/// **宛先のエージェントだけが受け取るように、それ以外を指す**形にはできない。
/// だから宛先は別の帯で運ぶ —— 上の 1 桁を立てて「これは宛先つき」と示し、
/// 受ける側が自分あてかどうかを見る。
#[must_use]
const fn 宛先の印(エージェント: u64) -> u64 {
    エージェント | 宛先の帯
}

/// 「これは宛先つき」を示す帯。**エージェントの番号とぶつからない所に置く。**
const 宛先の帯: u64 = 1 << 63;

/// 配られたものが、自分あてか（宛先つきなら宛先のエージェントだけが受ける）。
#[must_use]
const fn 自分あてか(出所: u64, 自分: u64) -> bool {
    if 出所 & 宛先の帯 == 0 {
        // 宛先なし。**言った本人以外へ配る**
        return 出所 != 自分;
    }
    出所 & !宛先の帯 == 自分
}

/// 会話に出た文字を、この機械へ配る形にする。
#[must_use]
pub fn 聞いた(from: &str, body: &str) -> FromDesk {
    聞いた_部屋で(from, body, None)
}

/// **どの部屋で聞いたかも載せて**行にする（0.1.40・内部の Issue 026）。
/// 口はこの部屋を指して返事を返す。この機械の中の知らせは `None`。
pub fn 聞いた_部屋で(from: &str, body: &str, 部屋: Option<String>) -> FromDesk {
    let 行 = FromDesk::Heard {
        id: 番号を振る(),
        from: from.to_owned(),
        body: body.to_owned(),
        at: いま時刻(),
        room: 部屋,
    };
    控える(&行);
    行
}

/// **部屋を知らない口には、部屋の欄を落として渡す。**古い口は知らない欄で 1 行も読めなくなる。
fn 部屋を落とすか(行: FromDesk, 部屋も聞く: bool) -> FromDesk {
    match 行 {
        FromDesk::Heard {
            id,
            from,
            body,
            at,
            room: Some(_),
        } if !部屋も聞く => FromDesk::Heard {
            id,
            from,
            body,
            at,
            room: None,
        },
        FromDesk::Sent {
            to,
            id,
            届いた,
            room: Some(_),
        } if !部屋も聞く => FromDesk::Sent {
            to,
            id,
            届いた,
            room: None,
        },
        他 => 他,
    }
}

/// **指された部屋を、居る部屋の中から引く。**id の全文か、頭（`…` 付きでも）で 1 つに決まるもの。
///
/// # Errors
/// 居ない・2 つ以上に当たる。人が読む訳。
pub fn 部屋を引く(候補: &[String], 指定: &str) -> Result<String, String> {
    let 指定 = 指定.trim().trim_end_matches('…');
    if 指定.is_empty() {
        return Err("部屋が指されていません".to_owned());
    }
    if let Some(そのもの) = 候補.iter().find(|c| c.as_str() == 指定) {
        return Ok(そのもの.clone());
    }
    let 当たり: Vec<&String> = 候補.iter().filter(|c| c.starts_with(指定)).collect();
    match 当たり.as_slice() {
        [一つ] => Ok((*一つ).clone()),
        [] => Err(format!("その部屋に居ません（{}）", crate::短く(指定))),
        多い => Err(format!(
            "「{}」は {} 部屋に当たります。もっと長く書いてください",
            crate::短く(指定),
            多い.len()
        )),
    }
}

/// **直近の発言として控える。**繋ぎ直した相手へ渡すため（`issues/019`）。
///
/// 古いものから捨てる。**溜め込む所にしない。**
fn 控える(行: &FromDesk) {
    let FromDesk::Heard { id, .. } = 行 else {
        return;
    };
    let mut 棚 = 直近の発言.lock().expect("毒されていない");
    棚.push((*id, 行.clone()));
    if 棚.len() > 覚える発言 {
        let 捨てる = 棚.len() - 覚える発言;
        棚.drain(..捨てる);
    }
}

/// **どこから聞かせるか。**`どこから` より後の発言を、古い順に返す。
///
/// - `None` …… 何も渡さない（**今までと同じ。新しい分だけ聞く**）
/// - `Some(0)` …… 持っている全部（`0` は「番号が無い」と読む約束・`line.rs`）
/// - `Some(n)` …… `n` より大きい番号だけ
///
/// **持っていない分は渡せない。**`覚える発言` より古い所は落ちている ——
/// **そこは「渡した」と言わない**（取りこぼしを黙って埋めたことにしない）。
#[must_use]
fn 過去を選ぶ(履歴: &[(u64, FromDesk)], どこから: Option<u64>) -> Vec<FromDesk> {
    let Some(から) = どこから else {
        return Vec::new();
    };
    履歴
        .iter()
        .filter(|(id, _)| *id > から)
        .map(|(_, 行)| 行.clone())
        .collect()
}

/// **その発言を、どのエージェントへ届けたか**を控える（**D76**）。
///
/// 古いものから捨てる。**溜め込む所にしない** ——
/// ここが要るのは「さっき流したものが読まれたか」を返すためだけである。
fn 届けた(id: u64, 先: Vec<String>) {
    let mut 棚 = 届いた先.lock().expect("毒されていない");
    棚.insert(id, 先);
    while 棚.len() > 覚える発言 {
        let 古い = *棚.keys().next().expect("空でない");
        棚.remove(&古い);
    }
}

/// **そこまで読んだ**と控える（**D76**）。
fn 読んだ(エージェント: u64, まで: u64) {
    let mut 棚 = 読んだところ.lock().expect("毒されていない");
    let いま = 棚.entry(エージェント).or_insert(0);
    // **戻らない。**古い番号で上書きしない
    if まで > *いま {
        *いま = まで;
    }
}

/// その発言の**届き方**を返す（届いたエージェント・読んだエージェント）。
fn 届き方(id: u64) -> (Vec<String>, Vec<String>) {
    let 届いた = 届いた先
        .lock()
        .expect("毒されていない")
        .get(&id)
        .cloned()
        .unwrap_or_default();
    let 読んだ棚 = 読んだところ.lock().expect("毒されていない");
    let エージェント棚 = エージェントたち.lock().expect("毒されていない");
    // **番号ではなく名前で返す。**呼び手に番号の意味は無い
    let 読んだ: Vec<String> = エージェント棚
        .iter()
        .filter(|(エージェント, 名)| 届いた.contains(名) && 読んだ棚.get(*エージェント).copied().unwrap_or(0) >= id)
        .map(|(_, 名)| 名.clone())
        .collect();
    (名札で並べる(&届いた), 名札で並べる(&読んだ))
}

/// **この機械が呼び方を刻む。**エージェントは「どこで動いているか」しか名乗れない。
///
/// 名乗りをそのまま出さないのは、**人の発言と見分けが付かなくなる**のを避けるため
/// （`ToDesk::Say` に差出人が無いのと同じ約束）。
/// 「◯◯ の AI」という形に包むので、名乗りが何であれ**AI だと分かる。**
#[must_use]
pub fn 呼び方(名乗り: Option<&str>) -> String {
    match 名乗り {
        // **「AI」ではなく「エージェント」と呼ぶ**（2026-09-08）。
        // 「この PC の AI」だと 1 人に見えるが、**つながっているのは 1 人ずつのエージェント**である
        Some(名) => format!("{名} のエージェント"),
        // 名乗らなかった。**どこか分からないことを、分かるように見せない**
        None => "名乗りのないエージェント".to_owned(),
    }
}

/// **札の頼みを受ける**（**D119**）。返すのは**いまの答え**だけ。
///
/// **札はここで出さない。**出るのは**人が画面で押したとき**である（**D56** を保つ）。
///
/// **人に聞くことになったら、棚へ積んで画面へ知らせる。**
/// **同じ動作の頼みは 1 つだけ持つ** —— 並ぶと人が何度も同じことを聞かれる。
fn 頼みを受ける(
    app: &AppHandle,
    動作: &str,
    訳: &str,
    頼んだ人: Option<&str>,
) -> warifu_desk::頼みの返り {
    use warifu_capability::{受け付けない訳, 札の答え, 頼みの行き先, 頼みをどうするか};
    use warifu_desk::頼みの返り;

    let いまの控え = 札の控え();
    match 頼みをどうするか(&いまの控え, 動作, 訳) {
        頼みの行き先::答えが出ている(札の答え::許した) => 頼みの返り::許した,
        頼みの行き先::答えが出ている(札の答え::断った) => 頼みの返り::断った,
        頼みの行き先::答えが出ている(札の答え::まだ) => 頼みの返り::まだ,
        頼みの行き先::受け付けない(訳け) => {
            // **「どうすれば通るか」は書かない。**何が悪かったかだけ
            let 言い方 = match 訳け {
                受け付けない訳::動作が読めない => "動作の名が読めません",
                受け付けない訳::訳が無い => "訳がありません",
                受け付けない訳::訳が長い => "訳が長すぎます",
            };
            記録!("札: 受け付けませんでした（{動作}・{言い方}）");
            頼みの返り::受け付けない {
                訳: 言い方.to_owned(),
            }
        }
        頼みの行き先::人に聞く => {
            let 積んだ = {
                let mut 棚 = 待っている頼み.lock().expect("毒されていない");
                // **同じ動作は 1 つだけ。**訳は新しいほうで上書きする
                if let Some(在る) = 棚.iter_mut().find(|待| 待.動作 == 動作) {
                    在る.訳 = 訳.to_owned();
                    false
                } else {
                    棚.push(待ち {
                        動作: 動作.to_owned(),
                        訳: 訳.to_owned(),
                        頼んだ人: 呼び方(頼んだ人),
                    });
                    true
                }
            };
            if 積んだ {
                記録!("札: 人に聞きます（{動作}・{}）", 呼び方(頼んだ人));
                // **画面へ知らせる。**落ちても頼みは棚に在る（次の知らせで出る）
                if let Err(e) = app.emit(crate::EVENT_PASS, 待っている札()) {
                    記録!("札: 画面へ渡せませんでした: {e}");
                }
            }
            頼みの返り::まだ
        }
    }
}

/// **人が答えるまで待つ**（2026-09-24）。
///
/// **`Notify` を先に構えてから控えを見る** ——
/// 逆にすると、**見たあとに押された 1 回を取り落とす。**
///
/// **時間切れは失敗ではない。**`まだ` を返す ——
/// **待った側が「まだ押されていない」と分かればよい。**
async fn 答えを待つ(動作: &str, 待つ秒: u64) -> warifu_desk::頼みの返り {
    答えを待つ_控えは(動作, 待つ秒, 札の控え).await
}

/// **控えを読む手を渡して待つ**（試験のために開けてある）。
///
/// **本物の置き場所を読む試験は書けない** —— その機械の `pass.tsv` を見てしまう。
async fn 答えを待つ_控えは(
    動作: &str,
    待つ秒: u64,
    控えを読む: impl Fn() -> 控え,
) -> warifu_desk::頼みの返り {
    use warifu_capability::札の答え;
    use warifu_desk::頼みの返り;

    let 終わり = tokio::time::Instant::now() + std::time::Duration::from_secs(待つ秒);
    loop {
        // **構えてから見る。**この順でないと、取り落とす
        let 知らせ = 答えの知らせ口().notified();
        // **動作の名は、ここで読み直す** —— 読めない名は待つ前に落ちているので、
        // ここへ来た名は必ず読める（読めなければ `まだ` で返す）
        let Ok(名) = warifu_capability::Action::new(動作) else {
            return 頼みの返り::まだ;
        };
        match warifu_capability::札の控え::答え(&控えを読む(), &名) {
            Some(札の答え::許した) => return 頼みの返り::許した,
            Some(札の答え::断った) => return 頼みの返り::断った,
            // **`まだ` は控えに書かれない**ので、ここへは来ない
            Some(札の答え::まだ) | None => {}
        }
        let 残り = 終わり.saturating_duration_since(tokio::time::Instant::now());
        if 残り.is_zero() {
            return 頼みの返り::まだ;
        }
        if tokio::time::timeout(残り, 知らせ).await.is_err() {
            return 頼みの返り::まだ;
        }
    }
}

/// 控えから作る、引ける形。
struct 控え(BTreeMap<String, bool>);

impl warifu_capability::札の控え for 控え {
    fn 答え(&self, 動作: &warifu_capability::Action) -> Option<warifu_capability::札の答え> {
        self.0.get(動作.as_str()).map(|許した| {
            if *許した {
                warifu_capability::札の答え::許した
            } else {
                warifu_capability::札の答え::断った
            }
        })
    }
}

/// 人が答えた札を、置き場所から読む。**読めなくても止めない**（空で始める）。
fn 札の控え() -> 控え {
    let 今 = crate::now_secs();
    let 棚 = warifu_vault::Vault::default_location()
        .and_then(|v| v.passes_with_until())
        .unwrap_or_default();
    // **期限切れの札は答えとして数えない**（人にもう一度聞く）
    控え(
        棚.into_iter()
            .filter(|(_, _, まで)| まで.is_none_or(|t| t > 今))
            .map(|(a, 許した, _)| (a, 許した))
            .collect(),
    )
}

/// **人の答えを待っている札**を並べる（画面が読む）。
#[must_use]
pub fn 待っている札() -> Vec<待ち> {
    待っている頼み.lock().expect("毒されていない").clone()
}

/// **人が押した**（**D119**）。控えへ書いて、棚から降ろす。
///
/// **ここが札を出す唯一の所である**（**D56** ——「札を出すのは人である」）。
///
/// # Errors
/// 控えへ書けないとき。
pub fn 人が答えた(動作: &str, 許した: bool, まで: Option<u64>) -> Result<(), String> {
    let _錠 = 札の控えの錠.lock().expect("毒されていない");
    let vault = warifu_vault::Vault::default_location().map_err(|e| e.to_string())?;
    let mut 一覧 = vault.passes_with_until().unwrap_or_default();
    // **同じ動作は 1 行だけ。**書き換える（期限も書き換える）
    if let Some(在る) = 一覧.iter_mut().find(|(名, _, _)| 名 == 動作) {
        在る.1 = 許した;
        在る.2 = まで;
    } else {
        一覧.push((動作.to_owned(), 許した, まで));
    }
    vault.save_passes_with_until(&一覧).map_err(|e| e.to_string())?;
    待っている頼み
        .lock()
        .expect("毒されていない")
        .retain(|待| 待.動作 != 動作);
    // **待っている口を起こす**（2026-09-24）。
    // **起こさないと、押されたことがエージェントへ永久に届かない。**
    答えの知らせ口().notify_waiters();
    // **「人が」と書かない**（2026-09-18）。
    //
    // **この機械からは、押したのが人か合成クリックか分からない。**
    // 同じ機械で画面を操れるエージェントは、**自分で押せる。**
    //
    // **2026-09-18、実際にそれが起きた** ——
    // エージェントが自分で `pass_ask` を出し、自分で【許す】を押した。
    // **機構は確かめられるが、「人が決める」という性質は確かめられない。**
    //
    // **記録が知らないことを言わない。**「画面で」までが、この機械に言えることである。
    記録!(
        "札: 画面で{}（{動作}）**押したのが人かは、この機械からは分かりません**",
        // **「許す」をやめて「通す」にした**（2026-09-22）。「許す」は硬い。
        // **「許す」は、上の人が下の人に与える形に読める** ——
        // 実際にやっているのは、そばに居る人が「いいよ」と言うだけのことである。
        // **関所の中も `通るか` なので、言葉が揃う。**
        //
        // **`pass.tsv` の中身（許した／断った）は変えない** ——
        // あれは置き場所の形であって、変えると**前に書いたファイルが読めなくなる。**
        if 許した { "通されました" } else { "通されませんでした" }
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn 古い口が抜けたら_番号付きの名は元に戻る() {
        let mut 棚 = BTreeMap::new();
        棚.insert(1, "git-qa のエージェント".to_owned());
        棚.insert(2, "git-qa のエージェント 2".to_owned());
        棚.insert(3, "zumen のエージェント".to_owned());
        // 両方生きている間は番号のまま
        super::番号を外す(&mut 棚);
        assert_eq!(棚[&2], "git-qa のエージェント 2");
        // 古い口（1）が抜けたら、2 が元の名に戻る
        棚.remove(&1);
        super::番号を外す(&mut 棚);
        assert_eq!(棚[&2], "git-qa のエージェント");
        assert_eq!(棚[&3], "zumen のエージェント");
        // 名乗りが数字で終わっていても、「エージェント」で終わらなければ触らない
        assert_eq!(super::元の名("席 2 のエージェント"), None);
        assert_eq!(super::元の名("席 のエージェント 10"), Some("席 のエージェント"));
    }

    #[test]
    fn 版が上がったときだけ_初めては告げない() {
        assert!(super::版が上がった知らせ(None, "0.1.39").is_none());
        assert!(super::版が上がった知らせ(Some("0.1.39"), "0.1.39").is_none());
        let 文 = super::版が上がった知らせ(Some("0.1.38"), "0.1.39").expect("告げる");
        assert!(文.contains("0.1.38") &&文.contains("0.1.39") && 文.contains("app_view"));
    }

    #[test]
    fn 立て直したら_控えの続きから番号を振る() {
        // 1 から振り直すと、エージェントの控え（前の立ち上げの番号）より小さくなり、
        // 「控えより後」に 1 行も当たらない（2026-10-23）
        assert_eq!(super::続きの番号(Some(29)), 30);
        assert_eq!(super::続きの番号(None), 1);
    }


    #[test]
    fn 書き出しは返事の上限に収め_古い行から落とす() {
        let 行 = |番号: u64| warifu_desk::書き出しの行 {
            番号,
            誰: "相手".to_owned(),
            時刻: "12:00".to_owned(),
            本文: "あ".repeat(1000),
        };
        let 行たち: Vec<_> = (1..=20).map(行).collect();
        let 収めた = 書き出しを収める(行たち, 20_000);
        assert!(FromDesk::書き出し { 行たち: 収めた.clone() }.書く().len() <= 20_000);
        assert!(収めた.len() < 20 && !収めた.is_empty());
        assert_eq!(収めた.last().unwrap().番号, 20, "新しい行は残す");
        // 収まるなら、そのまま
        assert_eq!(書き出しを収める((1..=3).map(行).collect(), 20_000).len(), 3);
    }
    // 内部の Issue 019 —— **繋ぎ直したエージェントへ、切れている間の言葉を渡す。**
    //
    // 繋ぎ直したエージェントは、切れている間に来た返事を見落としていた
    // （実際は 33 分前に来ていた）。**人は画面で過去を読めるが、エージェントは読めない。**

    fn 発言(id: u64, body: &str) -> FromDesk {
        FromDesk::Heard {
            id,
            from: "だれか".to_owned(),
            body: body.to_owned(),
            at: "18:00".to_owned(),
            room: None,
        }
    }

    /// **人が押した瞬間に返る**（2026-09-24）。
    mod 押されるのを待つ {
        use super::super::{答えの知らせ口, 答えを待つ_控えは, 控え};
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};

        fn 空の控え() -> 控え {
            控え(std::collections::BTreeMap::new())
        }

        #[tokio::test]
        async fn 押された瞬間に返る() {
            // **時間切れを待たずに返る**ことが要点である ——
            // 返らなければ、人が「押しました」と言う手間が残る
            let 押した = Arc::new(AtomicBool::new(false));
            let 写し = Arc::clone(&押した);
            tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                写し.store(true, Ordering::SeqCst);
                答えの知らせ口().notify_waiters();
            });

            let 始め = std::time::Instant::now();
            let 答え = 答えを待つ_控えは("chat.send", 30, move || {
                if 押した.load(Ordering::SeqCst) {
                    控え([("chat.send".to_owned(), true)].into_iter().collect())
                } else {
                    空の控え()
                }
            })
            .await;

            assert_eq!(答え, warifu_desk::頼みの返り::許した);
            assert!(
                始め.elapsed() < std::time::Duration::from_secs(5),
                "**30 秒待たずに返る**（かかった: {:?}）",
                始め.elapsed()
            );
        }

        #[tokio::test]
        async fn 断られたことも_その場で返る() {
            // **断りも押下である。**「まだ」で待ち続けない
            let 押した = Arc::new(AtomicBool::new(false));
            let 写し = Arc::clone(&押した);
            tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                写し.store(true, Ordering::SeqCst);
                答えの知らせ口().notify_waiters();
            });

            let 答え = 答えを待つ_控えは("chat.send", 30, move || {
                if 押した.load(Ordering::SeqCst) {
                    控え([("chat.send".to_owned(), false)].into_iter().collect())
                } else {
                    空の控え()
                }
            })
            .await;

            assert_eq!(答え, warifu_desk::頼みの返り::断った);
        }

        #[tokio::test]
        async fn 時間切れは_まだで返る() {
            // **時間切れは失敗ではない。**待った側が「まだ」と分かればよい
            let 答え = 答えを待つ_控えは("chat.send", 1, 空の控え).await;
            assert_eq!(答え, warifu_desk::頼みの返り::まだ);
        }

        #[tokio::test]
        async fn すでに答えが在れば_待たない() {
            // **押す前から答えが在るものは、その場で返る**（帯も出さない）
            let 始め = std::time::Instant::now();
            let 答え = 答えを待つ_控えは("chat.send", 30, || {
                控え([("chat.send".to_owned(), true)].into_iter().collect())
            })
            .await;
            assert_eq!(答え, warifu_desk::頼みの返り::許した);
            assert!(始め.elapsed() < std::time::Duration::from_secs(1));
        }
    }

    #[test]
    fn どこからを言わなければ_過去は渡さない() {
        // **今までと同じ。**新しい分だけ聞くエージェントの動きを変えない
        let 履歴 = vec![(1, 発言(1, "あ")), (2, 発言(2, "い"))];
        assert!(過去を選ぶ(&履歴, None).is_empty());
    }

    #[test]
    fn ゼロを言えば_持っている全部を渡す() {
        // `0` は「番号が無い」と読む約束（`line.rs`）
        let 履歴 = vec![(1, 発言(1, "あ")), (2, 発言(2, "い"))];
        assert_eq!(過去を選ぶ(&履歴, Some(0)).len(), 2);
    }

    #[test]
    fn その番号より後だけを渡す() {
        let 履歴 = vec![(1, 発言(1, "あ")), (2, 発言(2, "い")), (3, 発言(3, "う"))];
        let 出た = 過去を選ぶ(&履歴, Some(1));
        assert_eq!(出た.len(), 2);
        assert_eq!(出た[0], 発言(2, "い"));
        assert_eq!(出た[1], 発言(3, "う"));
    }

    #[test]
    fn 古い順に渡す() {
        // **新しい順に渡すと、会話が逆さに読まれる**
        let 履歴 = vec![(5, 発言(5, "さき")), (6, 発言(6, "あと"))];
        let 出た = 過去を選ぶ(&履歴, Some(0));
        assert_eq!(出た[0], 発言(5, "さき"));
    }

    #[test]
    fn 追いついていれば_何も渡さない() {
        let 履歴 = vec![(1, 発言(1, "あ")), (2, 発言(2, "い"))];
        assert!(過去を選ぶ(&履歴, Some(2)).is_empty());
    }

    #[test]
    fn 覚えている数を超えたら_古いほうから落ちる() {
        // **持っていない分は渡せない。**そこを「渡した」と言わない
        let _順 = 発言も一つずつ();
        for i in 1..=(覚える発言 as u64 + 10) {
            控える(&発言(i, "あ"));
        }
        let 棚 = 直近の発言.lock().expect("毒されていない");
        assert_eq!(棚.len(), 覚える発言);
        // **落ちたのは古いほう**（新しいほうを捨てると、いまの会話が消える）
        assert_eq!(棚.first().expect("空でない").0, 11);
    }

    #[test]
    fn 発言でない行は控えない() {
        // 入退室の知らせは会話ではない。**渡し直しても意味が無い**
        let _順 = 発言も一つずつ();
        let 前 = 直近の発言.lock().expect("毒されていない").len();
        控える(&FromDesk::Joined {
            who: "だれか".to_owned(),
        });
        assert_eq!(直近の発言.lock().expect("毒されていない").len(), 前);
    }

    use super::*;

    /// **エージェントの棚は静的なので、1 つずつ使う。**
    ///
    /// 空にしてから確かめる形なので、**並んで走ると互いのエージェントが混ざる**
    /// （2026-09-09 に、試験を 1 本足したら順番が変わって落ちた）。
    /// 「たまに落ちる試験」を残さない —— **順を明示して固定する。**
    static 順番: Mutex<()> = Mutex::new(());

    /// 試験のあいだだけ、エージェントの棚を空にする。
    fn 棚を空に() {
        エージェントたち.lock().expect("毒されていない").clear();
    }

    /// 棚を触る試験は、これを持っているあいだだけ走る。
    fn 一つずつ() -> std::sync::MutexGuard<'static, ()> {
        let 番 = 順番
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        棚を空に();
        番
    }

    /// **発言の棚も静的なので、1 つずつ使う。**
    ///
    /// エージェントの棚（`順番`）と同じ理由だが、**別の棚なので別の錠**にする
    /// （同じ錠にすると、関係の無い試験どうしが待ち合う）。
    ///
    /// **2026-09-17 に踏んだ。**`発言でない行は控えない` が
    /// **前 0 / 後 200** で落ちた —— 前を読んだ後に、隣の試験が 200 本積んだ。
    /// **控える側は壊れていない。測る側が並んでいた。**
    static 発言の順番: Mutex<()> = Mutex::new(());

    /// 発言の棚を触る試験は、これを持っているあいだだけ走る。
    fn 発言も一つずつ() -> std::sync::MutexGuard<'static, ()> {
        let 番 = 発言の順番
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        直近の発言
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clear();
        番
    }

    fn いまの呼び方(エージェント: u64) -> String {
        エージェントたち
            .lock()
            .expect("毒されていない")
            .get(&エージェント)
            .cloned()
            .unwrap_or_default()
    }

    // D129 —— 持続の札（2026-10-28）
    #[test]
    fn 使い切りの札は受け取ると消え_持続の札は期限内なら残る_期限切れは消して通さない() {
        use super::札を受け取る;
        let 一覧 = vec![
            ("contacts.add".to_owned(), true, None),
            ("room.join".to_owned(), true, Some(2_000)),
            ("room.leave".to_owned(), true, Some(500)),
            ("voice.say".to_owned(), false, None),
        ];
        let (通す, 残す) = 札を受け取る(一覧.clone(), "contacts.add", 1_000);
        assert!(通す);
        assert!(!残す.iter().any(|(a, _, _)| a == "contacts.add"), "使い切りは消える");
        let (通す, 残す) = 札を受け取る(一覧.clone(), "room.join", 1_000);
        assert!(通す);
        assert!(残す.iter().any(|(a, _, _)| a == "room.join"), "期限内の持続の札は残る");
        let (通す, 残す) = 札を受け取る(一覧.clone(), "room.leave", 1_000);
        assert!(!通す, "期限切れは通さない");
        assert!(!残す.iter().any(|(a, _, _)| a == "room.leave"), "期限切れは消す");
        let (通す, _) = 札を受け取る(一覧.clone(), "voice.say", 1_000);
        assert!(!通す, "断っている札は通さない");
        let (通す, _) = 札を受け取る(一覧, "room.rename", 1_000);
        assert!(!通す, "無い札は通さない");
    }

    // 026 —— 返事が声掛けの部屋へ戻らない（2026-10-27）
    #[test]
    fn 指された部屋は_全文か頭で_1_つに引く() {
        let 候補 = vec![
            "NBIW3PA2TUQ52DTJNVXSQOL3G4".to_owned(),
            "NBIWAAAAAAAAAAAAAAAAAAAAAA".to_owned(),
            "4QNXPYQIHAYIAAAAAAAAAAAAAA".to_owned(),
        ];
        assert_eq!(部屋を引く(&候補, "4QNX…").as_deref(), Ok("4QNXPYQIHAYIAAAAAAAAAAAAAA"));
        assert_eq!(
            部屋を引く(&候補, "NBIW3PA2TUQ52DTJNVXSQOL3G4").as_deref(),
            Ok("NBIW3PA2TUQ52DTJNVXSQOL3G4")
        );
        assert!(部屋を引く(&候補, "NBIW").is_err(), "2 つに当たる");
        assert!(部屋を引く(&候補, "ZZZZ").is_err(), "居ない");
        assert!(部屋を引く(&候補, " ").is_err());
    }

    #[test]
    fn 部屋を知らない口には_部屋の欄を落とす() {
        let 行 = FromDesk::Heard {
            id: 1,
            from: "A".to_owned(),
            body: "b".to_owned(),
            at: "09:00".to_owned(),
            room: Some("NBIW".to_owned()),
        };
        assert!(matches!(
            部屋を落とすか(行.clone(), false),
            FromDesk::Heard { room: None, .. }
        ));
        assert_eq!(部屋を落とすか(行.clone(), true), 行);
    }

    #[test]
    fn 名乗りのないエージェントが_二つ着いても潰れない() {
        // **画面は呼び方で行を分けている。**同じ呼び方だと 2 人が 1 行に潰れ、
        // 特定の 1 人だけに聞くことが成り立たなくなる（2026-09-08）
        let _順 = 一つずつ();
        エージェントにつながれる(1, &呼び方(None));
        エージェントにつながれる(2, &呼び方(None));
        エージェントにつながれる(3, &呼び方(None));
        assert_eq!(いまの呼び方(1), "名乗りのないエージェント");
        assert_eq!(いまの呼び方(2), "名乗りのないエージェント 2");
        assert_eq!(いまの呼び方(3), "名乗りのないエージェント 3");
    }

    #[test]
    fn 先に居るほうの名前は変えない() {
        // 後から来たほうに番号を振る。**呼んでいる途中で相手の名前が変わらない**
        let _順 = 一つずつ();
        エージェントにつながれる(10, "docs のエージェント");
        エージェントにつながれる(11, "docs のエージェント");
        assert_eq!(いまの呼び方(10), "docs のエージェント");
        assert_eq!(いまの呼び方(11), "docs のエージェント 2");
    }

    #[test]
    fn 同じエージェントが名乗り直しても番号は付かない() {
        // `Listen` はつながった後に来る。**自分自身と重なったことにしない**
        let _順 = 一つずつ();
        エージェントにつながれる(20, &呼び方(None));
        エージェントにつながれる(20, "docs のエージェント");
        エージェントにつながれる(20, "docs のエージェント");
        assert_eq!(いまの呼び方(20), "docs のエージェント");
    }

    #[test]
    fn 言った本人には返さず_ほかのエージェントには配る() {
        // **エージェント同士が、人を介さずに往復できるか**（2026-09-11）
        //
        // 配る規則はここ 1 か所である ——
        // **言った本人には返さない**（返すと、自分の言葉に自分で反応して止まらない）。
        // **ほかのエージェントには配る**（だから A の言葉が B に届く）。
        let _順 = 一つずつ();
        エージェントにつながれる(1, "A のエージェント");
        エージェントにつながれる(2, "B のエージェント");

        let 届く先 = 届いた先を並べる(1);
        // **画面（人）は必ず 1 つとして並べる。**「2 人」だけでは誰に届いたか分からない
        assert_eq!(届く先, vec![画面.to_owned(), "B のエージェント".to_owned()]);

        let 逆 = 届いた先を並べる(2);
        assert_eq!(逆, vec![画面.to_owned(), "A のエージェント".to_owned()]);
    }

    #[test]
    fn エージェントが一つだけなら_届く先は画面だけ() {
        // **「流しました」だけでは 0 人と区別が付かない**（D49）。
        // ほかのエージェントが居ないことは、ここで分かる形にする
        let _順 = 一つずつ();
        エージェントにつながれる(1, "A のエージェント");
        assert_eq!(届いた先を並べる(1), vec![画面.to_owned()]);
    }

    #[test]
    fn 人が打った行は_つながっている全員へ配る() {
        // 人の発言は「この機械の外」から来る（`この機械の外`）。
        // **誰とも一致しないので、全員が受け取る**
        let _順 = 一つずつ();
        エージェントにつながれる(1, "A のエージェント");
        エージェントにつながれる(2, "B のエージェント");
        assert_eq!(
            届いた先を並べる(この機械の外),
            vec![
                画面.to_owned(),
                "A のエージェント".to_owned(),
                "B のエージェント".to_owned()
            ]
        );
    }

    #[test]
    fn 呼び方はエージェントと分かる形にする() {
        assert_eq!(呼び方(Some("docs")), "docs のエージェント");
        assert!(呼び方(None).contains("エージェント"));
    }

    /// **名前の無い相手は、連絡帳に無い者だけ・新しく会った順に足す**（0.1.32）。
    #[test]
    fn 名前だけ付けた相手も_会った住所があれば呼べる() {
        use super::呼べるか;
        use warifu_core::Seed;
        let 鍵 = |b: u8| {
            Seed::from_bytes([b; 32])
                .profile("Personal")
                .device(warifu_core::端末の呼び名)
                .public_key()
        };
        let mut 帳 = warifu_vault::Contacts::new();
        帳.add(鍵(1), "相手A", 1).expect("足せる");
        帳.add(鍵(2), "相手B", 1).expect("足せる");
        let 会った = vec![(鍵(1), "住所1".to_owned())];
        let a = 帳.find(鍵(1)).expect("在る");
        let b = 帳.find(鍵(2)).expect("在る");
        assert!(呼べるか(a, &会った), "連絡帳に住所が無くても、会った住所で呼べる");
        assert!(!呼べるか(b, &会った), "どちらの住所も無ければ呼べない");
    }

    #[test]
    fn 会った相手は_名前の無い行として連絡帳の後ろに並ぶ() {
        use super::{名前の無い会った相手, 名前の無い相手};
        use warifu_core::Seed;
        let 鍵 = |b: u8| {
            Seed::from_bytes([b; 32])
                .profile("Personal")
                .device(warifu_core::端末の呼び名)
                .public_key()
        };
        let 帳 = vec![warifu_desk::連絡先の行 {
            名前: "相手A".to_owned(),
            鍵: 鍵(1).to_string(),
            住所あり: true,
        }];
        let 会った = vec![
            (鍵(2), "住所2".to_owned()),
            (鍵(1), "住所1".to_owned()),
            (鍵(3), "住所3".to_owned()),
        ];
        let 足す = 名前の無い会った相手(&帳, 会った);
        let 鍵たち: Vec<String> = 足す.iter().map(|r| r.鍵.clone()).collect();
        assert_eq!(鍵たち, vec![鍵(3).to_string(), 鍵(2).to_string()]);
        assert!(足す.iter().all(|r| r.名前 == 名前の無い相手 && r.住所あり));
    }
}
