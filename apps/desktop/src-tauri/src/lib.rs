//! warifu Desktop の橋（M5-c2）。
//!
//! **ここに規則を書かない**（baseline §9）。進行は `warifu-app`、封筒は `warifu-meeting`、
//! 経路は `warifu-net` が持っている。この crate がやるのは
//! 「画面から呼べる形に直す」ことと「届いたものを画面へ流す」ことだけである。
//!
//! # 身元
//!
//! **鍵をこの端末に置く**（`warifu-vault` / **D42**）。閉じても同じ人でいられる。
//! **CLI（`warifu id`）と同じ身元**を使うので、画面と端末で別人にならない。
//!
//! 全端末を失ったときの復旧モデル（**D2**）は依然として未決である。
//! だが**平常時の置き場所と、全部失ったときの戻し方は別の話**で、
//! D2 がどれを選んでもこの端末が持っていること自体は変わらない（D42 に根拠）。

mod menu;

use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;

use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::{Mutex, mpsc};

use warifu_app::{Conference, format_invite, introductions_for, parse_invite};
use warifu_core::{Acceptance, Device, PublicKey, Revocations, Tally};
use warifu_door::{Answer as DoorAnswer, Door, Knock, Subject};
use warifu_intent::Channel;
use warifu_meeting::{MeetingId, Notice, Signal, Step};
use warifu_net::{Address, Node};

/// 画面へ流す出来事。**名前は画面側の `events.ts` と揃える。**
const EVENT_JOINED: &str = "warifu://joined";
const EVENT_LEFT: &str = "warifu://left";
const EVENT_SIGNAL: &str = "warifu://signal";
const EVENT_CLOSED: &str = "warifu://closed";
/// 誰かの住所を教わった（**D41**）。画面はこれを見て、自分から呼びに行く。
const EVENT_INTRODUCED: &str = "warifu://introduced";
/// 文字が届いた。`[誰から, 中身]` で渡す。
const EVENT_TEXT: &str = "warifu://text";
/// **この PC のこの機械から出た発言。**`[公開鍵, 中身, 時刻]` で渡す。
///
/// 相手から届いた文字（[`EVENT_TEXT`]）と分けるのは、
/// **同じエージェントの AI の発言だと人に分かる必要がある**ため。
/// 混ぜると、誰が言ったのか画面から読めなくなる。
const EVENT_DESK: &str = "warifu://desk";
/// **この機械に何人つながっているか**が変わった。人数だけを渡す。
///
/// 会議に人が居なくても、**同じエージェントの AI が居るなら人は話しかけられる。**
/// これが無いと、AI が居るのに「入ってきたら送れます」と出たままになる。
const EVENT_DESK_SEATS: &str = "warifu://desk-seats";

/// **札の頼みが来た**（**D119**）。
///
/// **部屋の会話とは別の口である**（**#26** —— 許可を聞く言葉を部屋へ流さない）。
const EVENT_PASS: &str = "warifu://pass";
/// **メニューからテーマを選んだ。**`auto` / `light` / `dark` のどれかを渡す。
///
/// **覚えるのも当てるのも画面側**（`localStorage` は Rust から読めない）。
/// ここは「押された」ことだけを伝える。
const EVENT_THEME: &str = "warifu://theme";
/// **更新を確かめてほしい**（メニューから・**D81**）。確かめるのは画面の側。
const EVENT_CHECK_UPDATE: &str = "warifu://check-update";
/// **プロフィールが変わった。**画面は読み直す。
///
/// この機械につながったエージェントが自分で書くことがあるので、
/// **画面が書いたときだけ**読み直す形にはできない。
const EVENT_PROFILES: &str = "warifu://profiles";
/// **相手が名乗った**（**D75**）。`[公開鍵, 名前, 紹介]` で渡す。
///
/// **本人が名乗ったものであって、本人確認ではない。**
/// こちらが付けた呼び名があれば、**そちらが勝つ**（**D46**）。
const EVENT_CLAIMED: &str = "warifu://claimed";

/// 経路の要所を書き出す。
///
/// **繋がらなかったときに「どこまで進んだか」が分かる**ようにするためだけのもの。
/// ターミナルから起動したときだけ人の目に入る（`open` では消える）。
///
/// **秘密情報を書かない**（baseline §14）。割符の中身・SDP の中身・
/// 公開鍵の全桁は出さない。**長さと種類だけ**を出す。
macro_rules! 記録 {
    ($($arg:tt)*) => {{
        // **crate:: で書く。**別のモジュールから呼ばれても同じ所を指す
        let 行 = format!("[warifu +{:.3}s] {}", $crate::起動からの秒(), format!($($arg)*));
        eprintln!("{行}");
        $crate::書き置く(&行);
    }};
}

// 記録! を使うので、**この宣言はマクロの後ろに置く**（マクロは書いた順にしか見えない）
mod call;
mod cli;
mod contacts;
mod desk;
mod link;
mod notify;
mod postbox;
mod profile;
mod schedule;

/// **決まった場所へ書き置く。**
///
/// `eprintln!` だけだと、**人がアイコンから起動したときログがどこにも残らない。**
/// こちらが `> warifu.log` を付けて起動していたから読めていただけである
/// （2026-09-07 に気づいた）。
///
/// **人が普通に使って不具合を踏んだとき、エージェントが見に行ける記録が要る。**
/// GUI はエージェントが操作できない（端末から動かせない）ので、
/// **人が触り、AI が記録を読む**という分担になる。**記録が無ければ、その分担が成立しない。**
///
/// 置き場所は身元と同じ所（`WARIFU_HOME` で移せる）。**中身は要所だけ**で、
/// **会議の文字も鍵も住所も書かない**（`話の記録` と同じ構え）。
fn 書き置く(行: &str) {
    use std::io::Write as _;
    static 置き場: std::sync::OnceLock<Option<std::sync::Mutex<std::fs::File>>> =
        std::sync::OnceLock::new();
    let 口 = 置き場.get_or_init(|| {
        let vault = warifu_vault::Vault::default_location().ok()?;
        let path = vault.dir().join("warifu.log");
        // **前の分を消さない。**追記で足す。前回落ちた理由が消えると、追えなくなる
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .ok()
            .map(std::sync::Mutex::new)
    });
    // **書けなくても止めない。**ログのために会議を落とさない
    if let Some(f) = 口
        && let Ok(mut f) = f.lock()
    {
        let _ = writeln!(f, "{行}");
    }
}

/// 起動してから何秒経ったか。
///
/// **時刻ではなく経過秒にする。**知りたいのは「何時か」ではなく
/// **「押してから何秒で映ったか」**であり、経過秒ならその場で引き算せずに読める。
/// 時差も夏時間も関係しない。
fn 起動からの秒() -> f64 {
    static 起動: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    起動
        .get_or_init(std::time::Instant::now)
        .elapsed()
        .as_secs_f64()
}

/// 鍵や住所を、追える範囲で短く。**全桁は出さない。**
fn 短く(s: &str) -> String {
    s.chars().take(12).collect::<String>() + "…"
}

/// 画面へ返す失敗。**下の層の理由を捨てない。**
///
/// `code` は**画面が訳すための鍵**（`messages.ts` の鍵と同じ文字列）。
/// 文言そのものをここで作ると、**Rust 側にもう 1 つ辞書ができて必ずずれる**。
/// 訳しようがないもの（下の層の生の理由）は `code` を持たず、`message` だけで出す。
#[derive(Debug, serde::Serialize)]
pub struct Failure {
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    code: Option<String>,
}

impl<E: std::fmt::Display> From<E> for Failure {
    fn from(e: E) -> Self {
        Self {
            message: e.to_string(),
            code: None,
        }
    }
}

type Answer<T> = Result<T, Failure>;

/// 下ごしらえ 1 通。画面とのやり取りはこの形だけ。
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct SignalPayload {
    /// `offer` / `answer` / `candidate` / `end`。
    pub step: String,
    /// SDP / ICE そのもの。**この層は読まない。**
    pub blob: String,
    /// 誰から（受け取ったときだけ入る）。base32 の公開鍵。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    /// 誰へ（送るときだけ入る）。**3 人以上では省けない**（M6）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to: Option<String>,
}

fn step_from_str(s: &str) -> Result<Step, Failure> {
    match s {
        "offer" => Ok(Step::Offer),
        "answer" => Ok(Step::Answer),
        "candidate" => Ok(Step::Candidate),
        "end" => Ok(Step::End),
        other => Err(Failure {
            message: format!("知らない段: {other}"),
            code: None,
        }),
    }
}

fn step_to_str(step: Step) -> &'static str {
    match step {
        Step::Offer => "offer",
        Step::Answer => "answer",
        Step::Candidate => "candidate",
        Step::End => "end",
    }
}

fn key_to_string(key: PublicKey) -> String {
    warifu_core::base32::encode(&key.to_bytes())
}

/// 起動中ずっと持つもの。
///
/// 会議と送り口は `Arc` で持つ。**動かしている最中も画面から触れる必要がある**ため
/// （送り出すのは画面、名簿を動かすのは受信のタスク）。片方へ持ち去ると、
/// もう片方から「まだ会議がありません」に見える。
pub struct Bridge {
    device: Device,
    node: Mutex<Option<Arc<Node>>>,
    /// 発行した割符の手元の半分。**招待 1 本につき 1 つ持つ**（**D47**）。
    ///
    /// 1 本しか持たない形にしていたため、**2 本目を出すと 1 本目が死に、
    /// 三者会議が成り立たなかった。**割符は「1 つの鍵 = 1 人」（D12）なので、
    /// **人数ぶん出すのが正しい形**である。会場鍵（何度でも使える鍵・`issues/009`）とは別の話。
    tally: Arc<Mutex<Vec<Tally>>>,
    /// **部屋ごとの合言葉**（**D118**・2026-09-17）。
    ///
    /// **主催は建てたときに作り、ゲストは戸口を通った直後に受け取る。**
    /// これを使って**証しを見せ、ゲスト同士が互いを通す**（**#28** の本線 D）。
    ///
    /// **割符は 2 人の間のもの。**主催とゲスト A、主催とゲスト B ——
    /// **A と B の間には何も無い**ので、合言葉が無いとゲスト同士は繋がらない
    /// （2026-09-17 に 3 台で実測）。
    ///
    /// **記録に書かない。**`合言葉` の `Debug` は中身を出さない（型で守ってある）。
    部屋の合言葉: Arc<Mutex<HashMap<MeetingId, warifu_core::合言葉>>>,
    /// **部屋ごとの主催**（**#37**・2026-09-16）。
    ///
    /// **ゲストの名簿は先頭が自分**なので、`members().first()` では主催が分からない
    /// （**D111 の迎えが一度も起きなかった**原因）。
    /// **主催は「入るときに呼んだ相手」**なので、そのとき覚える。
    主催たち: Arc<Mutex<HashMap<MeetingId, PublicKey>>>,
    /// 戸口。**割符が合わない相手は、ここで断る**（D31）。
    door: Arc<Mutex<Door>>,
    /// 相手ごとの住所（**D41**）。
    ///
    /// **主催者は、繋がれた相手の住所を知らない**（相手から来たので）。
    /// だから**入る側が自分で名乗る。**それをここに覚えて、次の人へ紹介する。
    addresses: Arc<Mutex<HashMap<[u8; 32], String>>>,
    /// いま居るルーム。**複数持てる**（`issues/015`）。
    ///
    /// 「相手を選ぶ ＝ その人とのルームを開く」が成り立つには、
    /// **air と 2 人で話しながら、3 人のルームにも居る**ことができなければならない。
    /// 1 つしか持てないと、ルームを移るたびに前のルームが切れる。
    ///
    /// **鍵はルームの id。**`Notice` は前からルームの id を持っているので、
    /// 届いた知らせをどのルームのものか振り分けられる。
    conferences: Arc<Mutex<HashMap<MeetingId, Conference>>>,
    /// **画面がいま見ているルーム。**打ったものはここへ流れる。
    ///
    /// ルームそのものとは別に持つ —— **見ていないルームも生きている。**
    いまのルーム: Arc<Mutex<Option<MeetingId>>>,
    /// 相手ごとの送り出し口（**M6**）。
    ///
    /// 1 本しか持たない形にすると、3 人目が来た時点で**前の相手へ届かなくなる。**
    /// 鍵をそのまま鍵にする（`PublicKey` は `Hash` を持たないのでバイト列で持つ）。
    outbound: Arc<Mutex<HashMap<[u8; 32], mpsc::Sender<Notice>>>>,
    /// この機械につながっている相手へ配る口（`desk.rs`）。
    ///
    /// **人が打った行も、相手から届いた行も、ここを通す。**
    /// 通さないと、同じエージェントの AI は人の発言が見えないまま返事をすることになる。
    /// 添えている数は**出所の番号**（`desk::この機械の外` ならこの機械の外から出たもの）。
    /// **言った本人には返さない**ために持つ。
    desk: tokio::sync::broadcast::Sender<(u64, warifu_desk::FromDesk)>,
    /// **相手ごとの経路の札**（`direct` / `relayed` / `unknown`）。
    ///
    /// 経路を知っているのは画面（WebRTC の統計）だけなので、
    /// **画面が変わったときに置いていく**（`note_path`）。
    /// ここに置くのは、**エージェントからも様子を尋ねられるようにするため**
    /// （オーナー・2026-09-11「押したのを検知できたりする MCP いれてください」）。
    ///
    /// **分からないものは入れない。**入っていなければ「不明」である。
    経路: Arc<Mutex<HashMap<[u8; 32], String>>>,
    /// **いま画面に映っているもの**（**#32**・2026-09-18）。
    ///
    /// ASUS のエージェント ——
    /// 「**測るたびに人のマウスを奪って画面を撮っていた。**」
    ///
    /// **知っているのは画面だけ**なので、画面が置きに来る（`経路` と同じ形）。
    /// **置きに来ていなければ `None`** ——「出ていない」ではない（原則 7）。
    画面に映っているもの: Arc<Mutex<Option<画面の映り>>>,
}

/// いま居るルームたち。**複数持てる**（`issues/015`）。
pub(crate) type ルームたち = Arc<Mutex<HashMap<MeetingId, Conference>>>;
/// 画面がいま見ているルーム。
pub(crate) type 見ているルーム = Arc<Mutex<Option<MeetingId>>>;

/// ルームを足して、**見ているルームにする。**
/// **どの部屋を見るか**（**#39**・2026-09-16）。
///
/// # なぜ要るか
///
/// 起動のときに自分の部屋を建て直すが、**そこで「見ている部屋」を奪っていた。**
///
/// ```text
/// 人が［入る］を押す  → 見ている = 入った部屋
/// 起動の続きが走る    → 建て直し → **見ている = 自分の部屋**
/// ```
///
/// 使った人 ——「**入る押したあと、画面がルームとか、に移動しないの謎だし**」。
/// **押した直後は移り、すぐ戻っていた。**
///
/// **すでに見ている部屋があるなら、そのまま。**無いときだけ、建てた部屋を見る。
#[must_use]
pub(crate) const fn 見る部屋を決める(いま: Option<MeetingId>, 建てた: MeetingId) -> MeetingId {
    match いま {
        Some(見ている) => 見ている,
        None => 建てた,
    }
}

/// **部屋を足して、そこを見る。**（人が作った・入ったときはこちら）
pub(crate) async fn ルームを足す(
    ルーム: &ルームたち,
    いま: &見ているルーム,
    c: Conference,
) -> MeetingId {
    let id = c.id();
    ルーム.lock().await.insert(id, c);
    *いま.lock().await = Some(id);
    id
}

/// **部屋を足すが、見ている部屋は奪わない**（**#39**）。
///
/// 起動の建て直しはこちら。**人が押して移った先を、あとから来た建て直しが奪わない。**
pub(crate) async fn ルームを足す_見るのは奪わない(
    ルーム: &ルームたち,
    いま: &見ているルーム,
    c: Conference,
) -> MeetingId {
    let id = c.id();
    ルーム.lock().await.insert(id, c);
    let mut 見ている = いま.lock().await;
    *見ている = Some(見る部屋を決める(*見ている, id));
    id
}

/// いま見ているルームの id。**無ければ `None`。**
pub(crate) async fn いま見ているルーム(いま: &見ているルーム) -> Option<MeetingId> {
    *いま.lock().await
}

/// **そのルームに居る相手だけ**へ送る口を集める。
///
/// ルームを複数持つので、**全員へ配ると別のルームの人にも届く。**
pub(crate) async fn そのルームの相手(
    ルーム: &ルームたち,
    outbound: &Arc<Mutex<HashMap<[u8; 32], mpsc::Sender<Notice>>>>,
    meeting: MeetingId,
    me: PublicKey,
) -> Vec<mpsc::Sender<Notice>> {
    let 面々: Vec<[u8; 32]> = {
        let 棚 = ルーム.lock().await;
        let Some(c) = 棚.get(&meeting) else {
            return Vec::new();
        };
        c.members()
            .iter()
            .filter(|k| **k != me)
            .map(|k| k.to_bytes())
            .collect()
    };
    let out = outbound.lock().await;
    面々.iter().filter_map(|k| out.get(k).cloned()).collect()
}

/// **その相手が居るルーム**を探す。居なければ `None`。
///
/// ルームを複数持つので（`issues/015`）、「その人へ言う」には
/// **どのルームの話か**を決めないと `Notice` が組めない。
pub(crate) async fn 相手が居るルーム(
    ルーム: &ルームたち, 相手: PublicKey
) -> Option<MeetingId> {
    let 棚 = ルーム.lock().await;
    棚.iter()
        .find(|(_, c)| c.members().contains(&相手))
        .map(|(id, _)| *id)
}

/// この端末の人の名乗り（名前・紹介）。**書いていなければ空。**
///
/// # Errors
/// 置き場所を開けないとき。
pub(crate) fn 自分の名乗り() -> Result<(String, String), warifu_vault::Error> {
    let 面々 = warifu_vault::Vault::default_location()?.profiles()?;
    Ok(面々
        .find(&warifu_vault::Who::Me)
        .map(|p| (p.name().to_owned(), p.bio().to_owned()))
        .unwrap_or_default())
}

/// この端末の身元。**CLI と同じものを使う。**
///
/// 画面と端末で別の身元になると、`warifu id` で見せた鍵と、
/// 画面が名乗る鍵が食い違う。**同じ人が 2 人居るように見える。**
/// 置き場所。**開けなくても止めない** —— 口の控えは、無くても動ける。
fn 置き場() -> Option<warifu_vault::Vault> {
    warifu_vault::Vault::default_location().ok()
}

fn 身元() -> Result<Device, warifu_vault::Error> {
    let vault = warifu_vault::Vault::default_location()?;
    Ok(vault.open_seed()?.profile("Personal").device("この端末"))
}

impl Bridge {
    fn new() -> Self {
        // **この端末に置いたシードから導く**（D42）。閉じても同じ人でいられる。
        //
        // ここで落とすのは、**別人として動き出すよりましだから**である。
        // 身元が毎回変われば相手は「同じ人」だと分からず、覚えた相手が全部無駄になる。
        // 落ちる理由（権限が緩い・中身が壊れている）は `warifu-vault` が文言で言う。
        let device = 身元().unwrap_or_else(|e| panic!("身元を開けません: {e}"));
        Self {
            device,
            node: Mutex::new(None),
            // **出した割符を控えから戻す**（**#38**）——
            // これが無いと、**上げ直した瞬間に配った鍵が全部死ぬ**
            tally: Arc::new(Mutex::new(控えた割符())),
            // **控えた合言葉を戻す**（**D118**）——
            // 戻さないと、**主催が落ちて建て直した瞬間に、渡した合言葉が全部死ぬ**
            // （**#38** で踏んだのと同じ形）
            部屋の合言葉: Arc::new(Mutex::new(控えた合言葉())),
            主催たち: Arc::new(Mutex::new(HashMap::new())),
            // **置いてある知り合いを連れて開く。**
            // 2026-09-07 まで毎起動で空になっており、「一度開けた相手は
            // 次から割符なしで開ける」（D31）が再起動をまたいで効かなかった
            door: Arc::new(Mutex::new(contacts::戸口を開く())),
            addresses: Arc::new(Mutex::new(HashMap::new())),
            conferences: Arc::new(Mutex::new(HashMap::new())),
            いまのルーム: Arc::new(Mutex::new(None)),
            outbound: Arc::new(Mutex::new(HashMap::new())),
            desk: tokio::sync::broadcast::Sender::new(desk::配る溜め),
            経路: Arc::new(Mutex::new(HashMap::new())),
            画面に映っているもの: Arc::new(Mutex::new(None)),
        }
    }

    async fn node(&self) -> Answer<Arc<Node>> {
        let mut slot = self.node.lock().await;
        if let Some(node) = slot.as_ref() {
            return Ok(Arc::clone(node));
        }
        // **既定は中継を使わない**（**D13**）。
        // 付けたときだけ通す —— 中継を通すと「誰がいつ誰に繋いだか」が
        // 中継の運用者に見える（**D10**）。**画面でも同じ形にする**（D78 は CLI だけだった）
        // **環境変数が勝つ。**次に控え（画面の設定）を見る（**#5**・2026-09-18）。
        //
        // **画面から入れる道が、環境変数しか無かった** ——
        // `WARIFU_RELAY=1` を付けて `.app` を立ち上げるのは、人の手順ではない。
        // **だから「網を越えた実測がまだ 0 件」のままだった。**
        //
        // **既定は使わない**（**D13**）。中継を使うと、
        // **繋いだことが中継の運用者に見える**ので、**入れるのは人が決めること**である。
        let 使う = match std::env::var("WARIFU_RELAY").ok() {
            Some(言葉) => 中継を使うか(Some(&言葉)),
            None => warifu_vault::Vault::default_location()
                .ok()
                .and_then(|v| v.relay().ok().flatten())
                .unwrap_or(false),
        };
        記録!(
            "中継: {}（WARIFU_RELAY で切り替えます）",
            if 使う { "**使います**" } else { "使いません" }
        );
        let 中継 = if 使う {
            warifu_net::中継の使い方::使う
        } else {
            warifu_net::中継の使い方::使わない
        };

        // **前と同じ口を取りに行く**（**#38 の残り半分**）。
        // 鍵は「出したときの口」を焼き込むので、**口が変われば配った鍵は死ぬ**
        let 控えた口 = 置き場()
            .and_then(|v| v.port().ok())
            .flatten();
        let 決め方 = 控えた口.map_or(warifu_net::口の決め方::まかせる, warifu_net::口の決め方::同じ口);

        let node = Arc::new(Node::bind_at(&self.device, 中継, 決め方).await?);
        self.口を言う(node.口の様子());
        *slot = Some(Arc::clone(&node));
        Ok(node)
    }

    /// **口がどう決まったかを、記録と画面へ出す**（**#38** の案 C）。
    ///
    /// **黙って空きへ落ちない。**落ちたことを言えなければ、
    /// **人は鍵が死んだことを知らないまま待つ**
    /// （2026-09-16 に Mac Air が 6 時間待った）。
    fn 口を言う(&self, 様子: warifu_net::口の様子) {
        match 様子 {
            warifu_net::口の様子::取り直せた(口) => {
                記録!("口 {口} を取り直しました（**前に配った鍵は、そのまま使えます**）");
            }
            warifu_net::口の様子::まかせた(口) => {
                記録!("口 {口} で待ちます（初めてなので、この口を控えます）");
            }
            warifu_net::口の様子::取れなかった { 望んだ, 代わり } => {
                記録!(
                    "**前の口 {望んだ} が取れませんでした。**{代わり} で待ちます —— \
                     **前に配った鍵は、もう使えません。出し直してください**"
                );
            }
        }
        // **取れなかったときは控えを書き換えない。**
        //
        // 取れない理由は 2 つあり、**こちらからは見分けられない。**
        //
        // 1. **別の warifu がその口を持っている** …… 控えはその子にとって正しい。
        //    上書きすると、**動いているほうの鍵を殺すことになる**
        // 2. 関係の無いものが取った …… 控えは古い
        //
        // **見分けられないなら、消さないほうを選ぶ。**
        // 毎回「取れませんでした」と言い続けるが、**言い続けるのは黙るより良い。**
        if matches!(様子, warifu_net::口の様子::取れなかった { .. }) {
            return;
        }
        match 置き場().map(|v| v.save_port(様子.口())) {
            Some(Ok(())) => {}
            Some(Err(e)) => 記録!("口を控えられませんでした: {e}（次の起動で口が変わります）"),
            None => 記録!("置き場所を開けないので、口を控えられません（次の起動で口が変わります）"),
        }
    }
}

/// **鍵の窓について、人に分かる言い方をする**（2026-09-18）。
///
/// **これまでは、期限切れが「宛先に届きませんでした」として出ていた。**
/// **2026-09-17、Mac Air がその文言で 6 時間待った** ——
/// 相手を疑い、網を疑い、口を疑った。**鍵を疑う材料が、どこにも無かった。**
///
/// **鍵には `いつまで` が入っている。**繋ぐ前に読めば言える。
///
/// # 「どうすれば通るか」は言う
///
/// **D31 の「断る理由を相手に返さない」とは別である。**
/// あれは**叩かれた側が叩いた相手へ**返さない話で、
/// こちらは**自分の画面が、自分に**言う話である ——**隠す理由が無い。**
fn 鍵の窓を言う(e: warifu_core::Error) -> Failure {
    use warifu_core::Error;
    match e {
        Error::Expired => Failure {
            message: "この会議キーは期限が切れています。新しいものをもらってください".into(),
            code: Some("meeting.key.expired".into()),
        },
        Error::TooEarly => Failure {
            message: "この会議キーは、まだ始まっていません".into(),
            code: Some("meeting.key.tooearly".into()),
        },
        他 => Failure::from(他),
    }
}

/// **中継を使うか**（環境変数の言葉から決める）。
///
/// **既定は使わない**（**D13**）。`1` / `true` / `yes` / `on` のときだけ使う。
///
/// 環境変数を読むのは呼ぶ側。**ここは渡された言葉だけを見る**（試験できるように）。
fn 中継を使うか(言葉: Option<&str>) -> bool {
    matches!(
        言葉.map(str::trim).map(str::to_ascii_lowercase).as_deref(),
        Some("1" | "true" | "yes" | "on")
    )
}

/// **中継を使う設定を読む**（**#5**）。**いま効いている値**ではなく、**控えてある値**。
#[tauri::command]
fn relay_setting() -> Answer<bool> {
    let vault = warifu_vault::Vault::default_location()?;
    Ok(vault.relay()?.unwrap_or(false))
}

/// **中継を使うかを控える**（**#5**・2026-09-18）。
///
/// **いますぐには変わらない** —— 結び目は起動のときに建てるので、
/// **次の起動から効く。**画面はそう言う（黙って効かないのが、いちばん悪い）。
#[tauri::command]
fn set_relay(on: bool) -> Answer<()> {
    let vault = warifu_vault::Vault::default_location()?;
    vault.save_relay(on)?;
    記録!(
        "中継: 次の起動から {}",
        if on { "使います" } else { "使いません" }
    );
    Ok(())
}

/// 自分の宛先。**これを相手へ渡す**（QR・紙・口頭でも成立する・M1）。
#[tauri::command]
async fn my_address(bridge: State<'_, Bridge>) -> Answer<String> {
    let node = bridge.node().await?;
    Ok(node.address().await?.to_string())
}

/// **招待を出す。**宛先と割符を 1 本の文字列にして返す。
///
/// 宛先だけを渡す形にはしない。**それでは受け取った側が誰でも繋げてしまう**（D31）。
/// 割符は人が渡すものであり、渡した時点で人はもう判断している。
///
/// 出すたびに前の割符は無効になる（手元の半分を入れ替えるため）。
/// **一度に有効な招待は 1 つ** — 配った先が分からなくなる状態を作らない。
/// 会議キーを作る。
///
/// `starts_at` を渡すと、**その時刻までは誰も入れない**（**D43**）。
/// 予定に紐づく鍵（週次 MTG など）を前もって配れるようにするための口で、
/// これが無いと**渡した瞬間から期限までずっと入れる。**
/// 画面にまだ予定の UI が無いので、渡さなければ今までどおり「いまから」になる。
/// **控えから割符を戻す**（**#38**・2026-09-16）。
///
/// オーナー ——「**保存で済むなら保存して試験用につかいまわしてよ。あほらしい**」
///
/// **期限の切れたものは戻さない**（上げ直しで復活させない）。
/// **使い切ったものは戻す** —— 戻さないと**1 本＝1 人が壊れて、同じ鍵で 2 人入れる。**
fn 控えた割符() -> Vec<Tally> {
    let Ok(vault) = warifu_vault::Vault::default_location() else {
        return Vec::new();
    };
    let 控え = match vault.issued() {
        Ok(控え) => 控え,
        Err(e) => {
            記録!("出した割符の控えを読めませんでした: {e}");
            return Vec::new();
        }
    };
    let いま = now_secs();
    let 戻した: Vec<Tally> = 控え
        .iter()
        .filter_map(|(_, bytes)| Tally::控えから戻す(bytes).ok())
        .filter(|t| t.not_after() >= いま)
        .collect();
    if !戻した.is_empty() {
        記録!(
            "出した割符を {} 本戻しました（控え {} 本のうち、期限内）",
            戻した.len(),
            控え.len()
        );
    }
    // **切れた分を、黙って落とさない**（2026-09-18・ASUS の求め）。
    //
    // **出した側には、何も出ていなかった。**
    // ASUS ——「**私は「切れた」を、そちらに言われるまで知りませんでした**」
    //
    // **落ちたことは分かるが、落ちた理由は分からない**——
    // だから**「期限で」と書く。**版の入れ替えや消去と見分けられるように
    let 切れた = 控え.len() - 戻した.len();
    if 切れた > 0 {
        記録!(
            "**配った鍵が {切れた} 本、期限で切れました**（残り {} 本）",
            戻した.len()
        );
    }
    戻した
}

/// 時刻を、人が読める形にする（`MM-DD HH:MM`）。
///
/// **秒までは出さない。**鍵の期限は分の粒度で足りる。
fn 時刻の言い方(秒: u64) -> String {
    let Ok(時) = std::time::UNIX_EPOCH.checked_add(std::time::Duration::from_secs(秒)).ok_or(())
    else {
        return "（読めません）".to_owned();
    };
    let 経過 = 時
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    // **手で組む。**時刻の箱をこのために足さない（`chrono` を入れない）
    let 日 = 経過 / 86400;
    let 秒内 = 経過 % 86400;
    let (時間, 分) = (秒内 / 3600, (秒内 % 3600) / 60);
    // 1970-01-01 からの日数を年月日へ（グレゴリオ暦・**協定世界時**）
    let mut 年 = 1970u64;
    let mut 残 = 日;
    loop {
        let 閏 = (年 % 4 == 0 &&年 % 100 != 0) || 年 % 400 == 0;
        let 年の日 = if 閏 { 366 } else { 365 };
        if 残 < 年の日 {
            break;
        }
        残 -= 年の日;
        年 += 1;
    }
    let 閏 = (年 % 4 == 0 &&年 % 100 != 0) || 年 % 400 == 0;
    let 月の日 = [
        31,
        if 閏 { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut 月 = 1usize;
    for 日数 in 月の日 {
        if 残 < 日数 {
            break;
        }
        残 -= 日数;
        月 += 1;
    }
    // **UTC であることを書く。**書かないと、人が自分の時計と比べて混乱する
    format!("{月:02}-{:02} {時間:02}:{分:02} UTC", 残 + 1)
}

/// **控えを、いま持っている姿に揃える**（**#38**）。
///
/// - **期限の切れたものを落とす**
/// - **使った印を書き直す**（`match_half` が付けた `used_by` を残す）
///
/// **書けなくても会話は止めない** —— 控えが古いほうが、止まるより良い。
async fn 割符の控えを揃える(bridge: &Bridge) {
    let Ok(vault) = warifu_vault::Vault::default_location() else {
        return;
    };
    let Ok(控え) = vault.issued() else { return };
    let いま持っている = bridge.tally.lock().await;
    let なう = now_secs();
    let 新しい: Vec<(String, Vec<u8>)> = 控え
        .into_iter()
        .filter_map(|(ルーム, bytes)| {
            let 古い = Tally::控えから戻す(&bytes).ok()?;
            if 古い.not_after() < なう {
                return None;
            }
            // **いま持っているほうが新しい**（使った印が付いている）
            let 今 = いま持っている.iter().find(|t| t.id() == 古い.id());
            Some((ルーム, 今.map_or(bytes, Tally::控えるバイト列)))
        })
        .collect();
    if let Err(e) = vault.replace_issued(&新しい) {
        記録!("出した割符の控えを書き直せませんでした: {e}");
    }
}

#[tauri::command]
async fn invite(
    bridge: State<'_, Bridge>,
    ttl_secs: u64,
    starts_at: Option<u64>,
) -> Answer<String> {
    let node = bridge.node().await?;
    let address = node.address().await?.to_string();
    // **会議 id を鍵に載せる。**載せないと入る側が別の id を名乗り、
    // こちらが「別の会議あて」として捨てる（2026-09-04 に実機で踏んだ）
    let meeting = {
        // **ルームが無ければ建てる。**鍵はルームへの招待なので、ルームが要る
        if let Some(id) = いま見ているルーム(&bridge.いまのルーム).await {
            id
        } else {
            let c = Conference::host(bridge.device.public_key(), warifu_app::DEFAULT_CAPACITY)?;
            ルームを足す(&bridge.conferences, &bridge.いまのルーム, c).await
        }
    };
    // **鍵を出す前に合言葉を用意する**（**D118**）。
    // 鍵で入ってきた人へ、戸口を通った直後に渡す
    部屋の合言葉を用意する(&bridge, meeting).await;
    let 開始 = starts_at.unwrap_or_else(now_secs);
    let (tally, token) = bridge
        .device
        .issue_tally_between(開始, 開始.saturating_add(ttl_secs))?;
    // **前の招待を殺さない。**足していく（D47）
    //
    // **控えにも置く**（**#38**）—— これが無いと、
    // **アプリを落とした瞬間に、配った鍵が全部死ぬ。**
    // 版を上げるたびに再起動が要るので、開発中は毎回起きていた
    if let Ok(vault) = warifu_vault::Vault::default_location() {
        if let Err(e) = vault.save_issued(&meeting.to_string(), &tally.控えるバイト列()) {
            記録!("出した割符を控えられませんでした: {e}");
        }
    }
    bridge.tally.lock().await.push(tally);
    // **いつまで使えるかを、そのまま言う**（2026-09-18・ASUS の求め）。
    //
    // **これまでは「開始と秒数」だけだった。**渡す人は、そこから期限を計算できない ——
    // **鍵を復号しないと分からなかった。**
    // ASUS ——「**それは、渡された側がやる作業ではないと思います**」
    //
    // **2026-09-18、鍵が時間で切れて 1 往復した。**
    // 出した側は「切れた」を、相手に言われるまで知らなかった。
    記録!(
        "会議キーを作った（会議 {} / {} から {} 秒・**{} まで使えます**）",
        短く(&meeting.to_string()),
        開始,
        ttl_secs,
        時刻の言い方(開始.saturating_add(ttl_secs))
    );
    Ok(format_invite(&address, &token, meeting))
}

/// **OS のメニューを、画面と同じ言語にする**（D35）。
///
/// 画面側が `navigator.languages` から決めた答えをそのまま渡す。
/// ここで OS へ聞き直すと、**2 か所が別の答えを出しうる。**
/// **同じ機械の CLI が、画面と同じ版か。**
///
/// 画面だけ上げた人は「直ったつもりで直っていない」（`.claude/issues/017`）——
/// **繋がらないという症状だけが残り、CLI が古いせいだと気づけない。**
///
/// **見つからないのは不具合ではない**（CLI を入れていない人が普通）。
#[tauri::command]
fn cli_state() -> cli::CLIの様子 {
    let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) else {
        return cli::CLIの様子::無い;
    };
    let 画面の版 = env!("CARGO_PKG_VERSION");
    for 場所 in cli::探す場所(std::path::Path::new(&home)) {
        if !場所.is_file() {
            continue;
        }
        // **版を聞くだけ。**他の口は叩かない
        let Ok(出た) = std::process::Command::new(&場所).arg("version").output() else {
            continue;
        };
        let 文 = String::from_utf8_lossy(&出た.stdout).to_string()
            + &String::from_utf8_lossy(&出た.stderr);
        return cli::見比べる(&場所, 画面の版, &文);
    }
    cli::CLIの様子::無い
}

#[tauri::command]
fn set_menu_locale(app: AppHandle, locale: String, theme: Option<String>) -> Answer<()> {
    // **メニューはメインスレッドでしか触れない。**macOS では別スレッドから差し替えると
    // 黙って何も起きない（例外も出ない）。1 回それで「英語のまま」を踏んだ。
    if !menu::LOCALES.contains(&locale.as_str()) {
        // 落とす先は英語だが、**黙って落とさない。**画面側と綴りがずれたときに
        // 「なぜか英語のまま」になるのを、ここで読めるようにしておく
        eprintln!("知らないロケール '{locale}' が来たので英語にします");
    }
    // **いま選んでいるテーマに印を付ける。**画面側が持っている値をそのまま受ける
    // （Rust からは `localStorage` を読めない）
    let theme = theme.unwrap_or_else(|| "auto".to_owned());
    let handle = app.clone();
    app.run_on_main_thread(move || match menu::build(&handle, &locale, &theme) {
        Ok(m) => {
            if let Err(e) = handle.set_menu(m) {
                // **握り潰さない。**差し替えに失敗したこと自体が読めないと、原因を追えない
                eprintln!("メニューを差し替えられませんでした: {e}");
            }
        }
        Err(e) => eprintln!("メニューを組めませんでした: {e}"),
    })?;
    Ok(())
}

/// 自分の公開鍵。画面が「自分かどうか」を見分けるのに使う。
#[tauri::command]
fn my_key(bridge: State<'_, Bridge>) -> String {
    key_to_string(bridge.device.public_key())
}

/// **この画面**に、外から届く口が開いているか。
///
/// 経路が付かないとき、画面は「ファイアウォールかもしれません」と言う。
/// **言う前に見る**ための口である。判定は `warifu-guard` が持つ。
///
/// `state` は 3 つある。**`unknown` を `blocked` に倒さない** —— 調べる手段が
/// 失敗しただけで「足してください」と言うと、**足りている人を止める**（線 7）。
///
/// 2026-09-12 に Windows でそうなった。規則を 3 つ足したあとも同じ文言が出続け、
/// 足した人が「足したのに直らない」で止まった。
#[derive(serde::Serialize)]
pub struct FirewallState {
    /// `"open"` / `"blocked"` / `"unknown"`
    state: &'static str,
    /// 規則の件数。`open` のときだけ入る
    rules: Option<usize>,
    /// 調べられなかった理由。`unknown` のときだけ入る
    detail: Option<String>,
    /// 見た実行ファイル。**人にそのまま見せてよい**（直し方に要る）
    program: Option<String>,
}

#[tauri::command]
fn firewall_state() -> FirewallState {
    // **いま動いているこの実行ファイル**を見る。名前で当てない ——
    // `*warifu*` だと CLI の規則まで数え、画面が塞がっていることを隠す
    let Ok(道) = std::env::current_exe() else {
        return FirewallState {
            state: "unknown",
            rules: None,
            detail: Some("自分の在り処が読めませんでした".to_owned()),
            program: None,
        };
    };
    let 道の文字 = Some(道.display().to_string());
    match warifu_guard::調べる(&道) {
        warifu_guard::遮り::開いている(n) => FirewallState {
            state: "open",
            rules: Some(n),
            detail: None,
            program: 道の文字,
        },
        warifu_guard::遮り::塞がっている => FirewallState {
            state: "blocked",
            rules: None,
            detail: None,
            program: 道の文字,
        },
        warifu_guard::遮り::分からない(理由) => FirewallState {
            state: "unknown",
            rules: None,
            detail: Some(理由),
            program: 道の文字,
        },
    }
}

/// 会議を作る。定員は `2..=16`（**D27**）。
#[tauri::command]
async fn host_meeting(bridge: State<'_, Bridge>, capacity: usize) -> Answer<String> {
    let 私 = bridge.device.public_key();
    // **前に主催していたルームがあれば、同じ id で建て直す**（2026-09-11）——
    // id が起動ごとに変わると、付けた名前も、渡した鍵の指す先も持ち越せない
    // （オーナー「ルーム名決めても、リセットされてますね」）。
    // **一回性は崩れない**（D12。割符は鍵ごとに 1 回）。
    let 覚えていた = warifu_vault::Vault::default_location()
        .and_then(|v| v.my_room())
        .ok()
        .flatten()
        .and_then(|(id, _)| id.parse::<MeetingId>().ok());
    let conference = match 覚えていた {
        Some(id) => Conference::host_with_id(私, capacity, id)?,
        None => Conference::host(私, capacity)?,
    };
    // **起動の建て直しは、見ている部屋を奪わない**（**#39**）——
    // 人が［入る］を押した直後に、これが走って**自分の部屋へ戻していた**
    let id =
        ルームを足す_見るのは奪わない(&bridge.conferences, &bridge.いまのルーム, conference).await;
    // **自分が建てた部屋の主催は自分**（**#37**）
    bridge.主催たち.lock().await.insert(id, 私);
    // **合言葉もここで用意する**（**D118**）——
    // 入った人へ渡すものなので、**渡す前に在る**必要がある
    部屋の合言葉を用意する(&bridge, id).await;
    // **建てた id を書き置く。**名前は画面から付けるので、ここでは触らない
    if let Ok(vault) = warifu_vault::Vault::default_location() {
        let 名前 = vault.my_room().ok().flatten().map_or_else(String::new, |(_, n)| n);
        if let Err(e) = vault.save_my_room(&id.to_string(), &名前) {
            記録!("ルームを書き置けませんでした: {e}");
        }
    }
    Ok(id.to_string())
}

/// **ルームに名前を付ける。**置き場所に書くので、閉じても消えない。
///
/// オーナー ——「ルーム名決めても、リセットされてますね」（2026-09-11）。
/// それまで名前は画面の中だけで、**閉じると消えていた。**
#[tauri::command]
async fn name_room(id: String, name: String) -> Answer<()> {
    let vault = warifu_vault::Vault::default_location()?;
    vault.save_my_room(&id, &name)?;
    Ok(())
}

/// **まだ人が答えていないリンクの鍵**（画面が起き上がったときに取りに来る）。
///
/// **`app.emit` は聞き手が居なければ落ちる**（**#25**・2026-09-15、Mac Air が実機で見つけた）——
/// **リンクでアプリが起動した回は、画面がまだ出来ていない。**
/// 数だけ増えて鍵は消え、**人には押すものが画面のどこにも出なかった。**
///
/// **D102 と同じ構え** ——「読む側が、どこから読むかを言う」。
#[tauri::command]
fn pending_links() -> Vec<String> {
    link::待っている鍵たち()
}

/// 主催しているルームの名前。**無ければ空。**
#[tauri::command]
async fn room_name() -> Answer<String> {
    let vault = warifu_vault::Vault::default_location()?;
    Ok(vault.my_room()?.map_or_else(String::new, |(_, n)| n))
}

/// 相手の宛先へ繋ぎ、会議に入ると告げる。
///
/// **繋がった後は、届いたものを画面へ流し続ける。**
#[tauri::command]
async fn connect(app: AppHandle, bridge: State<'_, Bridge>, invite: String) -> Answer<()> {
    // **宛先だけでは繋がない。**会議キーに割符が付いていなければここで止まる（D31）
    let (address, token, meeting) = parse_invite(&invite)?;
    // **自分の会議キーを貼ったときは、ここで気づく。**
    // 下の層（iroh）は "Connecting to ourself is not supported" としか言わない。
    // 画面が訳せるように、文言そのものではなく**鍵**を返す
    if warifu_app::is_own_invite(bridge.device.public_key(), &token) {
        return Err(Failure {
            message: "自分の会議キーです".into(),
            code: Some("meeting.key.own".into()),
        });
    }
    記録!("入室: 会議キーを読んだ（宛先 {}）", 短く(&address));

    // **呼ぶ前に窓を見る**（2026-09-18）。
    //
    // **これまでは、繋いでから窓を見ていた。**だから期限切れの鍵で押すと、
    // **「宛先に届きませんでした」と出ていた** ——
    // **期限が切れたことは、どこにも出なかった。**
    //
    // **2026-09-17、Mac Air がその文言で 6 時間待った。**
    // **割符には `いつまで` が入っているので、繋ぐ前に読めば言える。**
    //
    // CLI（`warifu join`）は前からこうしてある ——
    // 「**呼ぶ前に窓を見る。始まっていない・切れている鍵で相手を叩かない**」。
    // **画面だけが揃っていなかった。**
    let acceptance = bridge.device.accept(&token, now_secs()).map_err(鍵の窓を言う)?;

    let node = bridge.node().await?;
    let to = Address::from_str(&address)?;
    let mut session = node.connect(&to, &Revocations::new()).await?;
    let peer = session.peer();
    記録!(
        "入室: 経路がつながった（相手 {}）",
        短く(&key_to_string(peer))
    );

    // **応じるものは、もう作ってある**（上で窓を見たときに作った）。
    // ここは Intent の下（生のバイト列）で済ませる。「何を話すか」ではなく
    // 「そもそも話してよいか」の段なので、口の語彙を増やさない（D11）
    session.send(&acceptance.to_bytes()).await?;
    記録!("入室: 割符に応じた");

    let mut channel = Channel::new(session);

    // **会議キーに書かれた会議へ入る。**自分で id を作らない。
    // 作ると相手の会議と別物になり、送った知らせが「別の会議あて」として捨てられる
    let events = {
        let mut roster = warifu_meeting::Roster::with_capacity(
            bridge.device.public_key(),
            warifu_app::DEFAULT_CAPACITY,
        )
        .map_err(|e| Failure {
            message: e.to_string(),
            code: None,
        })?;
        roster.add(peer).map_err(|e| Failure {
            message: e.to_string(),
            code: None,
        })?;
        ルームを足す(
            &bridge.conferences,
            &bridge.いまのルーム,
            Conference::joined(bridge.device.public_key(), meeting, roster),
        )
        .await;
        // **呼んだ相手が、この部屋の主催**（**#37**）。
        // **ゲストの名簿は先頭が自分**なので、名簿からは主催が分からない ——
        // 覚えないと **D111 の迎えが一度も起きない**（Mac Air の実測で 08 が落ちた）
        bridge.主催たち.lock().await.insert(meeting, peer);
        vec![warifu_app::Event::Joined(peer)]
    };
    let meeting_id = meeting;
    記録!("入室: 会議 {} に入る", 短く(&meeting.to_string()));
    // **呼んだ側にも「入った」を流す。**
    // ここを落としていたので、**呼んだ側は通話を作らず、相手の映像が来なかった**
    // （2026-09-04 に実機で判明）。受けた側だけが Call を持っている状態になる
    記録!("入室: 名簿に入れた（{} 件の出来事を画面へ）", events.len());
    emit_events(&app, &events);

    // **帰り道を控える**（`gh issue 13`）。
    //
    // ASUS のエージェント ——「**アプリの更新は再起動を伴います。更新のたびに
    // 鍵を貼り直すことになり、鍵は 1 本 = 1 人なので、出し直してもらう手間が
    // 毎回かかります。**」
    //
    // **`room.tsv` には書かない。**あれは**主催しているルーム**の id で、
    // 他人の id を書くと**次の起動で他人の id で建ててしまう。**
    //
    // **ここには割符の片割れが入る。**0600 で置き、画面には出さず、抜けたら消す。
    // 再入場は **D44** で通る（同じ人が同じ割符で戻るのは `rematch`）。
    if let Ok(vault) = warifu_vault::Vault::default_location() {
        if let Err(e) = vault.save_rejoin(&meeting.to_string(), &invite) {
            記録!("帰り道を書き置けませんでした: {e}");
        }
    }

    // 入ると告げる
    channel
        .send(
            &Notice::Join {
                meeting: meeting_id,
            }
            .to_intent()?,
        )
        .await?;

    // **名乗りも渡す**（**D75**）。相手の画面に、こちらの名前と紹介が出る。
    // **相手が呼び名を付けていれば、そちらが勝つ**（D46）ので、上書きにはならない
    if let Ok(名乗り) = 自分の名乗り() {
        let _ = channel
            .send(
                &Notice::Profile {
                    meeting: meeting_id,
                    from: bridge.device.public_key(),
                    名前: 名乗り.0,
                    紹介: 名乗り.1,
                }
                .to_intent()?,
            )
            .await;
    }

    // **自分の住所を名乗る。**相手は経路からこちらの住所を知れない（D41 と同じ理由）。
    // これが無いと、主催側は相手を「呼び返す」ことが永遠にできない
    if let Ok(自分の住所) = node.address().await {
        let _ = channel
            .send(
                &Notice::Introduce {
                    meeting: meeting_id,
                    who: bridge.device.public_key(),
                    address: 自分の住所.to_string(),
                }
                .to_intent()?,
            )
            .await;
    }

    // **繋がったいま覚える。**この住所はたったいま届いたことが分かっている。
    // 切れた時に覚えると、証明が古くなるうえ、落ちたら書けない
    contacts::書き留める(peer, &address);

    始める(&app, &bridge, channel, peer).await;
    Ok(())
}

/// **覚えた相手を、割符なしで呼ぶ。**
///
/// 名前を押して繋ぐ道（`issues/010` 段 3）。会議キーを手で渡さない。
/// 住所を知らなければ、**繋ぎに行かずにすぐ返す**（押した人を待たせない・D49）。
#[tauri::command]
async fn call_contact(app: AppHandle, bridge: State<'_, Bridge>, key: String) -> Answer<()> {
    let 相手: PublicKey = key.parse().map_err(|_| Failure {
        message: "公開鍵として読めません".into(),
        code: None,
    })?;
    call::呼ぶ(&app, &bridge, 相手).await
}

/// **貼られた鍵が、いつまで使えるか。**繋がずに読む（**#18**・2026-09-18）。
///
/// オーナー（2026-09-15）——
/// 「**毎回「入る」押してるけど、「入らない」押したらどうなるの？永遠に入れない？**」
///
/// **答えは「割符は減らない」である**（戸口は使った割符を控えていない）。
/// **困るのは、画面がその鍵を忘れること**だった ——
/// **「永遠に入れなくなるかも」と思えば、人は断れない。断る自由が無いのと同じである。**
///
/// **「24 時間」と決め打ちしない。**鍵に `いつまで` が入っているので、そのまま読む
/// （他所が出した鍵の窓は 24 時間とは限らない）。
#[tauri::command]
fn invite_window(invite: String) -> Answer<String> {
    let (_住所, token, _部屋) = parse_invite(&invite)?;
    Ok(時刻の言い方(token.not_after()))
}

/// **前に通してもらった部屋へ戻る**（**#21** / **#13**・2026-09-18）。
///
/// # なぜ `connect` では戻れないのか
///
/// **割符は 24 時間で切れる。戸口の「知り合い」は消えない**（`known.tsv`）。
///
/// ```text
/// 通した側   known.tsv に残る          → **期限が無い**
/// 通された側 帰り道の鍵（割符入り）    → **24 時間で切れる**
/// ```
///
/// **つまり、相手はまだ通してくれるのに、こちらのアプリが先に断っていた** ——
/// `connect` は繋ぐ前に窓を見るので（2026-09-18 に足した）、
/// **期限が切れた鍵はそこで止まる。**
///
/// 戸口は `if 知り合い { Answer::Open }` である（`warifu-door`）——
/// **割符が無くても、知り合いなら開く。**だから**割符なしで叩けば戻れる。**
///
/// **通すかどうかは、相手の戸口が決める。**こちらでは決めない ——
/// 断られたら、そのとき初めて「鍵をもらってください」と言う。
#[tauri::command]
async fn rejoin_room(app: AppHandle, bridge: State<'_, Bridge>) -> Answer<()> {
    let vault = warifu_vault::Vault::default_location()?;
    let Some((_部屋, 鍵)) = vault.rejoin()? else {
        return Err(Failure {
            message: "戻る先を覚えていません".into(),
            code: Some("room.back.none".into()),
        });
    };
    let (address, token, _meeting) = parse_invite(&鍵)?;
    // **窓の中なら、いつもの道。**割符を持っているほうが強い（名簿にも載る）
    if bridge.device.accept(&token, now_secs()).is_ok() {
        return connect(app, bridge, 鍵).await;
    }
    // **窓の外。**割符なしで叩く —— 通すかは相手の戸口が決める
    記録!("戻る: 鍵の窓は過ぎている。割符なしで叩きます");
    let 宛先 = Address::from_str(&address)?;
    call::住所へ呼ぶ(&app, &bridge, 宛先.public_key(), &address).await
}

/// **同じ部屋のゲストを、部屋の合言葉の証しで呼ぶ**（**D118** / **#28**）。
///
/// 紹介で教わった住所へ繋ぐ道。**`connect` は使えない** ——
/// あちらは会議キー（`宛先#割符#部屋`）を待っており、
/// **住所だけ渡すと「割符が付いていません」で止まる**
/// （2026-09-17 に 3 台で実測した、まさにその行）。
#[tauri::command]
async fn connect_in_room(
    app: AppHandle,
    bridge: State<'_, Bridge>,
    key: String,
    address: String,
    meeting: String,
) -> Answer<()> {
    let 相手: PublicKey = key.parse().map_err(|_| Failure {
        message: "公開鍵として読めません".into(),
        code: None,
    })?;
    let 部屋: MeetingId = meeting.parse().map_err(|_| Failure {
        message: "部屋 id として読めません".into(),
        code: None,
    })?;
    call::部屋の合言葉で呼ぶ(&app, &bridge, 相手, &address, 部屋).await
}

/// **人の答えを待っている札**を並べる（**D119**）。
///
/// **部屋の会話とは別の口である**（**#26**）。
#[tauri::command]
fn pending_passes() -> Vec<desk::待ち> {
    desk::待っている札()
}

/// **人が札に答えた**（**D119**）。
///
/// **ここが札を出す唯一の所である**（**D56** ——「札を出すのは人である」）。
/// **エージェントはこの口を呼べない** —— 机（`desk.sock`）には無い。
#[tauri::command]
fn answer_pass(動作: String, 許す: bool) -> Answer<()> {
    desk::人が答えた(&動作, 許す).map_err(|message| Failure {
        message,
        code: None,
    })
}

/// **いま画面に映っているもの**（**#32**）。**画面だけが知っている。**
#[derive(Debug, Clone)]
pub(crate) struct 画面の映り {
    pub 送っている: bool,
    pub 受けている: bool,
    /// **カメラかマイクを掴んでいるか。**「送っていない」と別のこと（**D116**）
    pub 掴んでいる: bool,
    /// **出ている題字そのもの**（訳したあとの文字）。札ではなく文字を渡す
    pub 題字: String,
}

/// **画面に映っているものを置く**（**#32**・2026-09-18）。
///
/// ASUS のエージェント（#32）——
///
/// > **自分の枠に映像が出ているか** —— 今日いちばん要った
/// > **相手の枠に映像が出ているか** —— **本題そのもの**
/// > **`経路` は在るが、画面の文言と一致する保証が無い**
///
/// **これが無いので、測るたびに人のマウスを奪って画面を撮っていた。**
/// **押す口ではない** —— **読むだけ。**カメラを点ける口はここに作らない。
#[tauri::command]
async fn note_screen(
    bridge: State<'_, Bridge>,
    sending: bool,
    receiving: bool,
    holding: bool,
    title: String,
) -> Answer<()> {
    *bridge.画面に映っているもの.lock().await = Some(画面の映り {
        送っている: sending,
        受けている: receiving,
        掴んでいる: holding,
        題字: title,
    });
    Ok(())
}

/// **経路の札を置く**（画面から）。
///
/// 経路を知っているのは画面（WebRTC の統計）だけである。
/// ここに置くと、**エージェントからも様子を尋ねられる**
/// （オーナー・2026-09-11「押したのを検知できたりする MCP いれてください」）。
///
/// `unknown` は**置かずに消す** —— 「分からない」を札として残すと、
/// 古い `direct` が消えないまま残るより悪い（読む側が「不明だと分かった」と誤読する）。
#[tauri::command]
async fn note_path(bridge: State<'_, Bridge>, peer: String, path: String) -> Answer<()> {
    let 相手: PublicKey = peer.parse().map_err(|_| Failure {
        message: "公開鍵として読めません".into(),
        code: None,
    })?;
    let mut 棚 = bridge.経路.lock().await;
    if path == "unknown" || path.is_empty() {
        棚.remove(&相手.to_bytes());
    } else {
        棚.insert(相手.to_bytes(), path);
    }
    Ok(())
}

/// **人がリンクに答えた**（入る／入らない）。待っている数を 1 つ減らす。
#[tauri::command]
async fn link_answered() -> Answer<()> {
    link::答えた();
    Ok(())
}

/// **いま鍵なしで入れる相手**を並べる。
///
/// 画面は、この一覧に居る相手にだけ「やめる」を出す。
/// **出しても効かない口を出さない**（D49）——
/// 覚えている相手と、鍵なしで入れる相手は**別の集まり**である。
#[tauri::command]
async fn known_keys(bridge: State<'_, Bridge>) -> Answer<Vec<String>> {
    let door = bridge.door.lock().await;
    Ok(door.known().map(str::to_owned).collect())
}

/// **相手を戸口から降ろす。**次からは割符が要る。
///
/// 知り合いを保存した以上、**取り消す口が要る。**
/// 保存する前は、間違って開けた相手もアプリを閉じれば切れていた。
#[tauri::command]
async fn stop_knowing(bridge: State<'_, Bridge>, key: String) -> Answer<bool> {
    let 相手: PublicKey = key.parse().map_err(|_| Failure {
        message: "公開鍵として読めません".into(),
        code: None,
    })?;
    let mut door = bridge.door.lock().await;
    Ok(contacts::降ろす(&mut door, 相手))
}

/// 待ち受けを始める。**呼ぶ側だけでは 2 台は出会えない。**
///
/// 片方が `connect`、もう片方がこれ。宛先を渡した側が待ち、受け取った側が呼ぶ。
#[tauri::command]
async fn listen(app: AppHandle, bridge: State<'_, Bridge>) -> Answer<()> {
    let node = bridge.node().await?;
    let conferences = Arc::clone(&bridge.conferences);
    let いまのルーム = Arc::clone(&bridge.いまのルーム);
    let outbound = Arc::clone(&bridge.outbound);
    let me = bridge.device.public_key();

    let tally = Arc::clone(&bridge.tally);
    let door = Arc::clone(&bridge.door);
    let addresses = Arc::clone(&bridge.addresses);
    // **部屋ごとの主催**（**#37**）。名簿の先頭では分からない
    let 主催たち = Arc::clone(&bridge.主催たち);
    // **部屋ごとの合言葉**（**D118**）。主催は渡し、ゲストは受け取る
    let 合言葉たち = Arc::clone(&bridge.部屋の合言葉);

    tokio::spawn(async move {
        loop {
            let Ok(mut session) = node.accept(&Revocations::new()).await else {
                // **失効した相手は下の層が止める。**ここで理由を分けない
                continue;
            };
            let peer = session.peer();
            let Some(subject) = Subject::new(&key_to_string(peer)) else {
                continue;
            };

            // **割符を先に確かめる。**会議の話をする前に、通してよいかを決める（D31）
            記録!("待受: 誰かが来た（{}）", 短く(&key_to_string(peer)));
            let 中身 =
                割符を確かめる(&mut session, &tally, &合言葉たち, me, &subject).await;
            let 割符が合った = 中身 == 叩きの中身::割符が合った;
            // **使った印を控えへ写す**（**#38**）——
            // 写さないと、**上げ直したときに使い切った鍵で 2 人目が入れる**
            if 割符が合った {
                割符の控えを揃える(&app.state::<Bridge>()).await;
            }
            記録!(
                "待受: 最初の 1 通は{}",
                match 中身 {
                    叩きの中身::割符が合った => "割符（合った）",
                    // **部屋の証しは、それ自体が「合った」しか無い**
                    // （合わなければ `どちらでもない` になる）
                    叩きの中身::部屋の証しが合った => "部屋の証し（合った）",
                    叩きの中身::どちらでもない => "どちらでもなかった",
                }
            );
            let (答え, 知っていた) = {
                let mut door = door.lock().await;
                // **answer は通した相手を知り合いに入れる。**前に見ておかないと
                // 「新しく知り合いになったか」が分からなくなる
                let 知っていた = door.knows(&subject);
                // **部屋の証しで通った相手は、知り合いにしない**（**D118**）——
                // 通るのは**その部屋の中だけ**である
                let knock = match 中身 {
                    叩きの中身::割符が合った => {
                        Knock::with_verified_tally(subject.clone(), now_secs())
                    }
                    叩きの中身::部屋の証しが合った => {
                        Knock::with_verified_room_proof(subject.clone(), now_secs())
                    }
                    叩きの中身::どちらでもない => Knock::new(subject.clone(), now_secs()),
                };
                (door.answer(&knock), 知っていた)
            };
            記録!("待受: 戸口の答えは {答え:?}");
            if 答え != DoorAnswer::Open {
                // **断る理由を相手に返さない**（D31）。黙って落とす
                continue;
            }

            // **新しく知り合いになったなら書き置く。**
            // 毎回書かない —— 叩かれるたびにディスクへ触ることになる。
            // ロックは集めるまで。**持ったままファイルへ触らない**
            //
            // **「名簿で通した」だけの相手は、ここに来ない**（**D111**）——
            // 戸口の書き置きは `known()` だけで、名簿の相手は入らない。
            // **いま本当に知り合いになったかを見てから書く** ——
            // 見ないと「知り合いに加えて書き置いた」と記録に出るのに、
            // **実際には何も増えていない**という嘘になる
            let 知り合いになった = !知っていた && door.lock().await.knows(&subject);
            if 知り合いになった {
                let 一覧: Vec<String> = {
                    let door = door.lock().await;
                    door.known().map(str::to_owned).collect()
                };
                contacts::戸口を書き置く(&一覧);
                記録!("待受: 知り合いに加えて書き置いた");
            }

            let mut channel = Channel::new(session);

            // **割符なしで来た相手には、会議を教える。**
            // 相手は会議 id を知りようがない（会議キーを持っていない）。
            // 割符つきで来た相手には送らない —— そちらは鍵に id が入っている
            // **部屋の証しで来た相手にも教えない。**もう同じ部屋に居るので、
            // 教えると**別の部屋へ入れ直すことになる**
            if 中身 == 叩きの中身::どちらでもない
                && let Some(招待) = call::招く(&conferences, &いまのルーム)
                && let Ok(intent) = 招待.to_intent()
            {
                if channel.send(&intent).await.is_err() {
                    continue;
                }
                記録!("待受: 割符なしの相手へ会議を教えた");
            }
            {
                // **ルームが 1 つも無ければ建てる。**受けた相手を入れる先が要る
                if conferences.lock().await.is_empty() {
                    match Conference::host(me, warifu_app::DEFAULT_CAPACITY) {
                        Ok(c) => {
                            ルームを足す(&conferences, &いまのルーム, c).await;
                        }
                        Err(_) => continue,
                    }
                }
            }
            // **迎える側も名乗る**（**D75**）。
            // 呼ぶ側だけが名乗ると、**片方向にしか名前が出ない**
            if let Ok((名前, 紹介)) = 自分の名乗り()
                && let Some(id) = いま見ているルーム(&いまのルーム).await
                && let Ok(intent) = (Notice::Profile {
                    meeting: id,
                    from: me,
                    名前,
                    紹介,
                })
                .to_intent()
            {
                let _ = channel.send(&intent).await;
            }

            汲む(
                app.clone(),
                Arc::clone(&conferences),
                Arc::clone(&outbound),
                Arc::clone(&addresses),
                Arc::clone(&door),
                Arc::clone(&主催たち),
                Arc::clone(&合言葉たち),
                me,
                channel,
                peer,
            );
        }
    });
    Ok(())
}

/// 相手が最初に送ってくる片割れを、手元の割符と照らす。
///
/// **待ち続けない。**黙って繋いだだけの相手に、待ち受けを塞がせない。
/// **最初の 1 通が何だったか**（**D118**）。
///
/// **「合った／合わなかった」の 2 値では足りなくなった。**
/// 部屋の証しで通った相手は**知り合いとして書き置かない**ので、
/// **どちらで通ったかを、呼んだ側が知っている必要がある。**
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum 叩きの中身 {
    /// 割符の片割れが合った。**知り合いとして書き置く。**
    割符が合った,
    /// 部屋の合言葉の証しが合った。**書き置かない**（その部屋の中だけ）。
    部屋の証しが合った,
    /// どちらでもない。**戸口が「知り合いか」で決める。**
    どちらでもない,
}

async fn 割符を確かめる(
    session: &mut warifu_net::Session,
    tally: &Arc<Mutex<Vec<Tally>>>,
    合言葉たち: &Arc<Mutex<HashMap<MeetingId, warifu_core::合言葉>>>,
    me: PublicKey,
    _subject: &Subject,
) -> 叩きの中身 {
    let Ok(Ok(bytes)) =
        tokio::time::timeout(std::time::Duration::from_secs(10), session.recv()).await
    else {
        return 叩きの中身::どちらでもない;
    };
    // **部屋の証しかどうかを先に見る**（**D118**）。
    // 種別が別なので、**割符として読もうとして壊れることはない**
    if let Ok(叩き) = warifu_core::部屋の叩き::from_bytes(&bytes) {
        return 部屋の証しを確かめる(&叩き, 合言葉たち, session.peer(), me).await;
    }
    let Ok(acceptance) = Acceptance::from_bytes(&bytes) else {
        return 叩きの中身::どちらでもない;
    };
    // **署名した本人と、経路で確定した相手が同じか。**
    // `Acceptance` は本人の鍵で署名されているが、**どこで署名されたかまでは言っていない。**
    // 突き合わせないと、写し取った片割れを別の経路で出せてしまう
    if acceptance.accepter() != session.peer() {
        return 叩きの中身::どちらでもない;
    }
    let mut list = tally.lock().await;
    // **どの招待に対する片割れかは、相手が名乗っている。**総当たりで試さない ——
    // 試すと、別の招待の窓（`not_before` / `not_after`）で通ってしまう
    let Some(t) = list.iter_mut().find(|t| t.id() == acceptance.tally()) else {
        return 叩きの中身::どちらでもない;
    };
    // **まだ誰も入っていなければ初回、一度入った相手が戻ってきたなら再入場**（D44）。
    // 回線が一瞬切れただけで、10 時から 11 時の会議が終わってはいけない
    let now = now_secs();
    let 名簿 = Revocations::new();
    let 合った = if t.used_by().is_none() {
        t.match_half(&acceptance, now, &名簿).is_ok()
    } else {
        t.rematch_half(&acceptance, now, &名簿).is_ok()
    };
    if 合った {
        叩きの中身::割符が合った
    } else {
        叩きの中身::どちらでもない
    }
}

/// **部屋の合言葉の証しを検める**（**D118**）。
///
/// # どこで縛っているか
///
/// 証しは `(部屋, 呼ぶ側の鍵, 受ける側の鍵)` に縛られている。
/// **鍵は経路が確定させたものを使う**（`session.peer()` と自分）——
/// **相手が名乗った鍵は見ない。**見ると「名乗りと経路が食い違ったらどちらを信じるか」
/// という要らない判断が生まれる。
///
/// # 知らない部屋の証しは通さない
///
/// **その部屋の合言葉を持っていなければ、検めようがない。**
/// 持っていない＝その部屋に居ない、なので**断る**。
async fn 部屋の証しを確かめる(
    叩き: &warifu_core::部屋の叩き,
    合言葉たち: &Arc<Mutex<HashMap<MeetingId, warifu_core::合言葉>>>,
    呼ぶ側: PublicKey,
    me: PublicKey,
) -> 叩きの中身 {
    let 部屋の文字 = String::from_utf8_lossy(叩き.部屋()).into_owned();
    let Ok(部屋) = 部屋の文字.parse::<MeetingId>() else {
        記録!("戸口: 部屋の証しが来たが、部屋 id として読めません");
        return 叩きの中身::どちらでもない;
    };
    let 棚 = 合言葉たち.lock().await;
    let Some(言) = 棚.get(&部屋) else {
        // **黙って落とさない**（#37 で直した形と同じ）——
        // 「合言葉を持っていない」と「証しが合わない」は、次に見る所が違う
        記録!(
            "戸口: 部屋の証しが来たが、その部屋の合言葉がありません（部屋 {}）",
            短く(&部屋の文字)
        );
        return 叩きの中身::どちらでもない;
    };
    if 言.証しが合うか(&叩き.証し(), 叩き.部屋(), 呼ぶ側, me) {
        記録!(
            "戸口: 部屋の証しが合いました（{}・部屋 {}）",
            短く(&key_to_string(呼ぶ側)),
            短く(&部屋の文字)
        );
        叩きの中身::部屋の証しが合った
    } else {
        記録!(
            "戸口: 部屋の証しが合いませんでした（{}・部屋 {}）",
            短く(&key_to_string(呼ぶ側)),
            短く(&部屋の文字)
        );
        叩きの中身::どちらでもない
    }
}

/// 今の時刻（秒）。**割符の期限と戸口の窓に使う。**
pub(crate) fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// 繋がった口を動かし始める。呼ぶ側・受ける側で同じ形にする。
async fn 始める(app: &AppHandle, bridge: &Bridge, channel: Channel, peer: PublicKey) {
    汲む(
        app.clone(),
        Arc::clone(&bridge.conferences),
        Arc::clone(&bridge.outbound),
        Arc::clone(&bridge.addresses),
        Arc::clone(&bridge.door),
        Arc::clone(&bridge.主催たち),
        Arc::clone(&bridge.部屋の合言葉),
        bridge.device.public_key(),
        channel,
        peer,
    );
}

/// 画面から送るものと相手から届くものを、1 本のタスクで捌く。
#[allow(clippy::too_many_arguments)]
fn 汲む(
    app: AppHandle,
    conferences: ルームたち,
    outbound: Arc<Mutex<HashMap<[u8; 32], mpsc::Sender<Notice>>>>,
    addresses: Arc<Mutex<HashMap<[u8; 32], String>>>,
    // **主催が言った相手を通すために要る**（**D111**）。
    door: Arc<Mutex<Door>>,
    // **部屋ごとの主催**（**#37**）。名簿の先頭では分からない
    主催たち: Arc<Mutex<HashMap<MeetingId, PublicKey>>>,
    // **部屋ごとの合言葉**（**D118**）。主催は渡し、ゲストは受け取る
    合言葉たち: Arc<Mutex<HashMap<MeetingId, warifu_core::合言葉>>>,
    me: PublicKey,
    mut channel: Channel,
    peer: PublicKey,
) {
    let (tx, mut rx) = mpsc::channel::<Notice>(32);
    tokio::spawn(async move {
        // **自分の経路を見分ける印**（**D83**）。
        //
        // 送り口は**相手の公開鍵で引いている**ので、同じ相手が入り直すと
        // 新しい経路が同じ場所に入る。**後片付けが遅れて走ると、
        // 古い経路の始末が新しい経路を消す**（2026-09-10 に実物で踏んだ ——
        // 入り直した直後に落ちた）。
        //
        // **自分が置いたものだけを片付ける。**
        let 私の口 = tx.clone();
        outbound.lock().await.insert(peer.to_bytes(), tx);
        let 訳 = loop {
            tokio::select! {
                // 画面から送るもの
                outgoing = rx.recv() => {
                    // 画面が口を閉じた（会議を降りた）。**落ちたのではない**
                    let Some(notice) = outgoing else { break "left" };
                    let Ok(intent) = notice.to_intent() else { continue };
                    // **送れないのは経路が落ちたということ**
                    if channel.send(&intent).await.is_err() { break "lost"; }
                }
                // 相手から届くもの
                incoming = channel.recv() => {
                    // **「帰った」と「落ちた」を混ぜない。**人はこの 2 つで次の手が変わる
                    // （帰ったなら会議は終わり、落ちたなら**会議キーを作り直す**・D12）
                    let intent = match incoming {
                        Ok(i) => i,
                        Err(warifu_intent::Error::Closed) => break "left",
                        Err(_) => break "lost",
                    };
                    let Ok(notice) = Notice::from_intent(&intent) else {
                        // 会議のものでない口は、経路としては通る。**会議は受け取らない**
                        continue;
                    };
                    // **文字は名簿を動かさない。**そのまま画面へ渡す
                    if let Notice::Text {
                        from, 話し手, body, ..
                    } = &notice
                    {
                        // **名乗った差出人と、繋いできた相手が違うなら通さない。**
                        // 主催から配られた文字（三者会議）はここへ来ない ——
                        // こちらが主催であり、配るのはこちらだからである
                        if *from != peer {
                            記録!("受信: 差出人が経路の相手と違う。捨てた");
                            continue;
                        }
                        // **札が付いていれば、そのエージェントの誰が言ったかまで画面へ渡す**
                        // （2026-09-08）。付いていなければ人が言ったもの
                        let 名 = match 話し手 {
                            Some(札) => format!("{}（{札}）", 短く(&key_to_string(peer))),
                            None => key_to_string(peer),
                        };
                        let _ = app.emit(EVENT_TEXT, (名, body.clone()));
                        // **窓が後ろに居ると、届いたことに気づけない。**押し出す
                        notify::届いたと知らせる(&app, &短く(&key_to_string(*from)));
                        // **同じエージェントの AI にも、同じ行を見せる。**
                        // 見せないと、人にだけ見えている話に AI が返事をすることになる
                        desk::配る(
                            &app.state::<Bridge>(),
                            desk::聞いた(&key_to_string(peer), body),
                        );
                        // **主催は、聞いた文字をほかの人へ配る**（**D48**）。
                        //
                        // 三者会議は星形である —— 参加者どうしは繋がっていないので、
                        // **配らないと B と C はお互いの発言を 1 通も受け取らない**
                        // （2026-09-06 に実測）。**言った人はそのまま載っている**ので、
                        // 配られた側にも誰の発言かが分かる。
                        let 主催 = {
                            let 棚 = conferences.lock().await;
                            棚.get(&notice.meeting())
                                .map(|c| c.members().first() == Some(&me))
                        };
                        if 主催 == Some(true) {
                            他へ配る(&outbound, peer, &notice).await;
                        }
                        continue;
                    }
                    // **紹介は名簿を動かさない**（D41）
                    記録!("受信: {}", 知らせの名(&notice));
                    // **相手が名乗った**（**D75**）。**呼び名は上書きしない**（D46）——
                    // こちらが付けた呼び名があれば、そちらが勝つ
                    if let Notice::Profile {
                        from, 名前, 紹介, ..
                    } = &notice
                    {
                        if *from != peer {
                            記録!("受信: 名乗りが経路の相手と違う。捨てた");
                            continue;
                        }
                        記録!(
                            "受信: 名乗り（{} / 名前 {} 文字・紹介 {} 文字）",
                            短く(&key_to_string(peer)),
                            名前.chars().count(),
                            紹介.chars().count()
                        );
                        let _ = app.emit(
                            EVENT_CLAIMED,
                            (key_to_string(peer), 名前.clone(), 紹介.clone()),
                        );
                        continue;
                    }
                    // **合言葉を受け取ってしまう**（**D118**）。
                    //
                    // **主催以外から来たものは捨てる。**合言葉は主催が決めるものであり、
                    // 誰からでも受けると**部屋を乗っ取られる**（別の合言葉に差し替えられる）
                    if let Notice::RoomSecret { meeting, 合言葉 } = &notice {
                        let 部屋の主催 = 主催たち.lock().await.get(meeting).copied();
                        if contacts::主催と同じか(部屋の主催, peer) {
                            合言葉たち.lock().await.insert(*meeting, 合言葉.clone());
                            記録!(
                                "部屋の合言葉を受け取りました（部屋 {}・主催から）",
                                短く(&meeting.to_string())
                            );
                            // **ゲストも控える**（**D118**）。
                            // 控えないと、**立ち上げ直した瞬間に他のゲストへ届かなくなる**
                            // —— 主催から貰い直すまで、部屋の中で孤立する
                            合言葉の控えを揃える(&app.state::<Bridge>()).await;
                        } else {
                            // **黙って捨てない。**捨てたことが記録に無いと、
                            // 「届いていない」と「捨てた」が区別できない
                            記録!(
                                "部屋の合言葉を捨てました（部屋 {}・主催でない {} から）",
                                短く(&meeting.to_string()),
                                短く(&key_to_string(peer))
                            );
                        }
                    }
                    if let Notice::Introduce { meeting, who, address } = &notice {
                        // **名乗りをそのまま連絡帳へ落とさない。**
                        // 「C さんの住所はここです」と言われるまま書くと、
                        // 次に人が C の名前を押したとき別の場所へ呼びに行く（D12 の迂回）
                        contacts::住所を覚える(peer, *who, address);
                        addresses.lock().await.insert(who.to_bytes(), address.clone());
                        // 自分が主催者なら、**入った人を既存の面々へ配り、
                        // 入った人へ既存の面々を教える**
                        // **主催は覚えてある**（**#37**）。
                        // **名簿の先頭では分からない** —— ゲストの名簿は先頭が自分で、
                        // `first() == peer` は**ゲストでは必ず false** になっていた。
                        // そのため **D111 の迎えが一度も起きなかった**
                        let 部屋の主催 = 主催たち.lock().await.get(meeting).copied();
                        let 主催 = Some(contacts::主催と同じか(部屋の主催, me));
                        let 紹介者が主催か = contacts::主催と同じか(部屋の主催, peer);
                        // **主催が「この部屋に居る」と言った相手を、戸口で通す**（**D111**）。
                        //
                        // これが無いと**ルームが星形になる** —— 割符を持っているのは主催だけなので、
                        // **ゲスト同士は呼び合っても戸口で黙って落ちる**（#28・2026-09-15 に 3 台で実測）。
                        //
                        // **信じるのは主催の言い分だけ。持ち越さない**（書き置く一覧には入らない）
                        if contacts::名簿として迎えるか(紹介者が主催か, peer, *who)
                            && let Some(印) = warifu_door::Subject::new(&key_to_string(*who))
                        {
                            let 迎えた = door.lock().await.名簿で迎える(印);
                            if 迎えた {
                                記録!(
                                    "戸口: 主催が言った相手を通します（{}・名簿）",
                                    短く(&key_to_string(*who))
                                );
                            }
                        }
                        // **配るのは「新入りが自分の住所を名乗った」ときだけ**
                        // （`配ってよいか`）。第三者の紹介を受けて配り直すと、
                        // **両側が「自分が主催だ」と思っている場合に往復する** ——
                        // 2026-09-13 に実測: **66 ミリ秒で約 130 件**来て `lost` になった
                        if contacts::配ってよいか(主催 == Some(true), peer, *who) {
                            // **合言葉を、紹介と同じ瞬間に渡す**（**D118**）。
                            //
                            // **ここでしか渡さない。**「戸口を通り、名簿に載り、
                            // 主催が紹介を配る」が揃った 1 点であり、
                            // **通る前に渡す理由が無い。**
                            部屋の合言葉を渡す(&合言葉たち, &outbound, *who, *meeting).await;
                            紹介を配る(&conferences, &outbound, &addresses, me, *who, *meeting)
                                .await;
                        } else if contacts::呼びに行かせるか(peer, *who) {
                            // 主催者でなければ、教わった住所を画面へ渡して呼びに行かせる。
                            //
                            // **ただし「相手が自分の住所を名乗っただけ」は渡さない**
                            // （`gh issue 9`）—— `connect` は必ず自分の住所を名乗るので、
                            // これを渡すと画面が呼び直し、相手も名乗り返して
                            // **紹介が往復する。**ASUS では 177 回往復して経路が落ちた
                            // **投げたことを残す**（**#37**・ASUS の指摘）——
                            // `let _ =` で投げていたので、**画面へ渡ったかどうかが記録に無かった。**
                            // 「何も起きていない」と「記録していない」が区別できなかった
                            記録!(
                                "紹介: 呼びに行かせます（{}・住所 {} 文字）",
                                短く(&key_to_string(*who)),
                                address.len()
                            );
                            // **部屋も渡す**（**D118**）。
                            // 画面は**どの部屋の合言葉で呼ぶか**を言えないと、証しを作れない
                            if let Err(e) = app.emit(
                                EVENT_INTRODUCED,
                                (
                                    key_to_string(*who),
                                    address.clone(),
                                    meeting.to_string(),
                                ),
                            ) {
                                記録!("紹介: 画面へ渡せませんでした: {e}");
                            }
                        } else {
                            記録!("受信: 紹介は自分の住所の名乗りだった。呼び直さない");
                        }
                        continue;
                    }
                    // **どのルームあてかで振り分ける。**知らないルームのものは受け取らない
                    let mut 棚 = conferences.lock().await;
                    let Some(c) = 棚.get_mut(&notice.meeting()) else {
                        記録!("受信: 知らないルームあてだった。捨てた");
                        continue;
                    };
                    match c.on_notice(peer, &notice) {
                        Ok(events) => {
                            // **名簿に居る相手が入り直したら、画面へそう言う**（**#23**）。
                            //
                            // `Notice::Join` は冪等なので、**名簿に既に居れば何も返さない。**
                            // それは名簿としては正しいが、**画面は通話を作り直せない。**
                            //
                            // **2026-09-18、両側の記録でそろった。**
                            //
                            // ```text
                            // （受ける側）待受: 割符は合った → 戸口 Open → 合言葉 渡した
                            //             **「入った人がいる」が出ない → 通話を作らない**
                            //             下ごしらえが来た: offer（**「溜めた」が付かない**）
                            //             → **古い通話へ渡る → 何も起きない → 答えが出ない**
                            // （呼ぶ側）  6 分 23 秒、答えを待ち続けた
                            // ```
                            //
                            // ASUS のエージェントが**「溜めた」が付いていないこと**に気づいて
                            // 突き止めた —— **在る行ではなく、無い行を探していた。**
                            //
                            // **名簿は動かさない**（冪等のままでよい）——**通話だけ作り直す。**
                            if events.is_empty()
                                && matches!(notice, Notice::Join { .. })
                                && c.members().contains(&peer)
                            {
                                記録!(
                                    "受信: 参加（{}）—— **名簿に居るので入り直し。通話を作り直します**",
                                    短く(&key_to_string(peer))
                                );
                                // **新しい出来事を足さない。**
                                // **既にある入室の道を通す** —— 画面はその頭で
                                // 「もう通話があるなら閉じる」を見るので、
                                // **入り直しも普通の入室として正しく流れる。**
                                if let Err(e) = app.emit(EVENT_JOINED, key_to_string(peer)) {
                                    記録!("入り直しを画面へ渡せませんでした: {e}");
                                }
                            }
                            emit_events(&app, &events);
                        }
                        Err(e) => {
                            // 相手へは理由を返さない（D31）。**手元のログには出す** —
                            // 黙って捨てると、無言の不通の原因が追えない
                            記録!("受信: 受け取らなかった（{e}）");
                            continue;
                        }
                    }
                }
            }
        };
        // **自分の口だけを外す。**ほかの相手との経路は生きている（M6）。
        //
        // **入れ替わっていたら触らない** —— 同じ相手が入り直していれば、
        // そこに居るのは新しい経路である。消すと、繋がったばかりの相手が落ちる。
        let 私のままだった = {
            let mut 棚 = outbound.lock().await;
            match 棚.get(&peer.to_bytes()) {
                Some(いま) if いま.same_channel(&私の口) => {
                    棚.remove(&peer.to_bytes());
                    true
                }
                _ => false,
            }
        };
        if !私のままだった {
            // **画面にも言わない。**
            //
            // 名簿だけ守って知らせを出していたため、**生きている相手について
            // 「経路が切れました」と画面に出て、届く先が空になった**
            // （2026-09-10 に実物で見た）。**古い経路の始末は、誰にも影響させない。**
            記録!(
                "経路が終わった: {}（{訳}）。**入り直しているので、何も触らない**",
                短く(&key_to_string(peer))
            );
            return;
        }

        // **経路が落ちたら、その人は名簿にも居ない**（**D83**）。
        //
        // 落ちたときに外していなかったため、**同じ相手が入り直しても
        // 画面に戻らなかった**（2026-09-10 に実物で踏んだ）——
        // `Notice::Join` は冪等で、名簿に既に居れば何も返さない。
        // 結果、**相手の発言だけが届いて、画面は「誰も居ない」と思ったまま**になり、
        // 返信もできない状態になっていた。
        //
        // **知らせは画面へ渡さない。**画面には `EVENT_CLOSED` で伝わっていて、
        // ここで `Left` も出すと**退室の行が 2 本出る。**
        // 外すのは、次の入り直しを通すためである。
        {
            let mut 棚 = conferences.lock().await;
            for c in 棚.values_mut() {
                let ルーム = c.id();
                let _ = c.on_notice(peer, &Notice::Leave { meeting: ルーム });
            }
        }

        記録!("経路が終わった: {}（{訳}）", 短く(&key_to_string(peer)));
        let _ = app.emit(EVENT_CLOSED, (key_to_string(peer), 訳));
    });
}

/// 下ごしらえを 1 通送る。**中身は解釈しない。**
///
/// 宛先（`to`）は**相手が 1 人のときだけ省ける。**
/// 3 人以上で省かれたら、どこへ送るか決められないので断る —
/// **黙って全員へ配らない**（SDP は組ごとのもので、他人に配ると経路が壊れる）。
#[tauri::command]
async fn send_signal(bridge: State<'_, Bridge>, payload: SignalPayload) -> Answer<()> {
    let step = step_from_str(&payload.step)?;
    let meeting = いま見ているルーム(&bridge.いまのルーム).await;
    let Some(meeting) = meeting else {
        return Err(Failure {
            message: "まだ会議がありません".into(),
            code: None,
        });
    };
    記録!(
        "送信: 下ごしらえ {} を {} へ（{} バイト）",
        payload.step,
        payload.to.as_deref().map_or("（唯一の相手）".into(), 短く),
        payload.blob.len()
    );
    let notice = Notice::Signal(Signal::new(meeting, step, payload.blob.into_bytes()));

    let slot = bridge.outbound.lock().await;
    let tx = match payload.to.as_deref() {
        Some(to) => {
            let bytes = warifu_core::base32::decode(to).ok_or_else(|| Failure {
                message: "宛先を公開鍵として読めません".into(),
                code: None,
            })?;
            let key: [u8; 32] = bytes.try_into().map_err(|_| Failure {
                message: "公開鍵の長さが違います".into(),
                code: None,
            })?;
            slot.get(&key)
        }
        None if slot.len() == 1 => slot.values().next(),
        None => {
            return Err(Failure {
                message: "宛先が要ります（相手が複数います）".into(),
                code: None,
            });
        }
    };
    let Some(tx) = tx else {
        return Err(Failure {
            message: "まだ繋がっていません".into(),
            code: None,
        });
    };
    tx.send(notice).await.map_err(|_| Failure {
        message: "経路が閉じています".into(),
        code: None,
    })
}

/// 聞いた知らせを、**言った人以外の全員へ配る**（**D48**）。**主催者だけが呼ぶ。**
///
/// 三者会議は星形で、参加者どうしは繋がっていない。主催が配らないと、
/// **B と C はお互いの発言を 1 通も受け取らない**（2026-09-06 に実測）。
///
/// 配るのは**そのままの知らせ**である。差出人（`Notice::Text::from`）を書き換えない ——
/// 書き換えると、受け取った側には**全部が主催の発言に見える。**
async fn 他へ配る(
    outbound: &Arc<Mutex<HashMap<[u8; 32], mpsc::Sender<Notice>>>>,
    言った人: PublicKey,
    notice: &Notice,
) {
    let 送り口 = outbound.lock().await;
    for (鍵, tx) in 送り口.iter() {
        if *鍵 == 言った人.to_bytes() {
            continue;
        }
        // **届かない相手で止めない。**1 人が落ちていても、ほかへは配る
        let _ = tx.send(notice.clone()).await;
    }
}

/// **この部屋の合言葉を用意する**（**D118**）。**主催だけが呼ぶ。**
///
/// **同じ部屋で作り直さない。**作り直すと、**前に渡した合言葉で通れなくなる。**
/// 建てた覚えのない部屋（ゲストとして入った部屋）では呼ばない ——
/// **合言葉は主催が決めるもの**であり、両側が別々に作ると噛み合わない。
async fn 部屋の合言葉を用意する(bridge: &Bridge, 部屋: MeetingId) {
    let mut 棚 = bridge.部屋の合言葉.lock().await;
    if 棚.contains_key(&部屋) {
        return;
    }
    match warifu_core::合言葉::作る() {
        Ok(言) => {
            棚.insert(部屋, 言);
            記録!(
                "部屋の合言葉を作りました（部屋 {}）",
                短く(&部屋.to_string())
            );
            // **作った瞬間に控える。**渡す前に落ちても、次の起動で同じものを使う
            drop(棚);
            合言葉の控えを揃える(bridge).await;
        }
        // **黙って進まない。**合言葉が無いと、ゲスト同士は繋がらない（#28）
        Err(e) => 記録!("部屋の合言葉を作れませんでした: {e}（ゲスト同士は繋がりません）"),
    }
}

/// **控えた合言葉を戻す**（**D118**）。
///
/// **戻さないと、主催が落ちて建て直した瞬間に、渡した合言葉が全部死ぬ。**
/// 同じ部屋 id で新しい合言葉を作ってしまうためである ——
/// **#38 で踏んだのと同じ形**（あちらは割符と口だった）。
///
/// **読めなくても止めない。**空で始めれば、新しい合言葉を作って続けられる。
fn 控えた合言葉() -> HashMap<MeetingId, warifu_core::合言葉> {
    let Ok(vault) = warifu_vault::Vault::default_location() else {
        return HashMap::new();
    };
    let 控え = match vault.room_secrets() {
        Ok(控え) => 控え,
        Err(e) => {
            記録!("部屋の合言葉の控えを読めませんでした: {e}");
            return HashMap::new();
        }
    };
    let 戻した: HashMap<MeetingId, warifu_core::合言葉> = 控え
        .into_iter()
        .filter_map(|(部屋, 中身)| {
            let id = 部屋.parse::<MeetingId>().ok()?;
            Some((id, warifu_core::合言葉::から(中身)))
        })
        .collect();
    if !戻した.is_empty() {
        // **本数だけ言う。中身は言わない**
        記録!("部屋の合言葉を {} 室ぶん戻しました", 戻した.len());
    }
    戻した
}

/// **いま持っている合言葉を、そのまま控えへ写す**（**D118**）。
///
/// **書けなくても会話は止めない** —— 控えが古いほうが、止まるより良い
/// （`割符の控えを揃える` と同じ構え）。
async fn 合言葉の控えを揃える(bridge: &Bridge) {
    let Ok(vault) = warifu_vault::Vault::default_location() else {
        return;
    };
    let 一覧: Vec<(String, [u8; 32])> = {
        let 棚 = bridge.部屋の合言葉.lock().await;
        棚.iter()
            .map(|(部屋, 言)| (部屋.to_string(), 言.バイト列()))
            .collect()
    };
    if let Err(e) = vault.save_room_secrets(&一覧) {
        記録!("部屋の合言葉を控えられませんでした: {e}");
    }
}

/// **合言葉を、入った人へ渡す**（**D118**）。**主催だけが呼ぶ。**
///
/// # 渡す条件（**1 点しかない**）
///
/// 「**戸口を通り、名簿に載り、主催が紹介を配る**」が揃った瞬間だけ。
/// **通る前に渡す理由が無い。**
///
/// # 口が無ければ、黙って諦めない
///
/// 相手への口が無いのは「まだ繋がっていない」ときである。
/// **記録に残す** —— 残さないと、**合言葉が渡っていないのに
/// 「ゲスト同士が繋がらない」という別の症状として現れる。**
async fn 部屋の合言葉を渡す(
    合言葉たち: &Arc<Mutex<HashMap<MeetingId, warifu_core::合言葉>>>,
    outbound: &Arc<Mutex<HashMap<[u8; 32], mpsc::Sender<Notice>>>>,
    相手: PublicKey,
    部屋: MeetingId,
) {
    let 言 = {
        let 棚 = 合言葉たち.lock().await;
        let Some(言) = 棚.get(&部屋) else {
            記録!(
                "合言葉: この部屋の合言葉がありません（部屋 {}）。渡せません",
                短く(&部屋.to_string())
            );
            return;
        };
        言.clone()
    };
    let 口 = {
        let out = outbound.lock().await;
        out.get(&相手.to_bytes()).cloned()
    };
    let Some(tx) = 口 else {
        記録!(
            "合言葉: 渡せません（{} への口がありません）",
            短く(&key_to_string(相手))
        );
        return;
    };
    // **中身は記録しない。**渡したという事実だけ
    if tx
        .send(Notice::RoomSecret {
            meeting: 部屋,
            合言葉: 言,
        })
        .await
        .is_err()
    {
        記録!(
            "合言葉: 渡せませんでした（{} の口が閉じていました）",
            短く(&key_to_string(相手))
        );
        return;
    }
    記録!(
        "合言葉: 渡しました（{}・部屋 {}）",
        短く(&key_to_string(相手)),
        短く(&部屋.to_string())
    );
}

/// 紹介を配る（**D41**）。**主催者だけが呼ぶ。**
///
/// 既存の面々へ「入った人の住所」を、入った人へ「既存の面々の住所」を送る。
/// **住所を知らない相手は飛ばす** — まだ名乗っていないだけなので、断りではない。
async fn 紹介を配る(
    conferences: &ルームたち,
    outbound: &Arc<Mutex<HashMap<[u8; 32], mpsc::Sender<Notice>>>>,
    addresses: &Arc<Mutex<HashMap<[u8; 32], String>>>,
    me: PublicKey,
    newcomer: PublicKey,
    meeting: warifu_meeting::MeetingId,
) {
    let 配り先 = {
        let 棚 = conferences.lock().await;
        let Some(c) = 棚.get(&meeting) else { return };
        let Some(plan) = introductions_for(c, newcomer, me) else {
            // **黙って戻らない**（**#37**）—— 配り先が無いのか、部屋が無いのかが読めなかった
            記録!(
                "紹介: 配り先がありません（新入り {}）",
                短く(&key_to_string(newcomer))
            );
            return;
        };
        plan
    };
    let book = addresses.lock().await;
    let out = outbound.lock().await;

    let 送る = |to: PublicKey, who: PublicKey| {
        let (Some(tx), Some(address)) = (out.get(&to.to_bytes()), book.get(&who.to_bytes())) else {
            // **黙って送らない、をやめる**（**#37**）——
            // **口が無いのか、住所を知らないのか**で、次に見る所が変わる
            記録!(
                "紹介: 送れません（{} へ {} の話。口 {} / 住所 {}）",
                短く(&key_to_string(to)),
                短く(&key_to_string(who)),
                if out.contains_key(&to.to_bytes()) { "あり" } else { "なし" },
                if book.contains_key(&who.to_bytes()) { "あり" } else { "なし" }
            );
            return None;
        };
        Some(tx.send(Notice::Introduce {
            meeting,
            who,
            address: address.clone(),
        }))
    };

    // 既存の面々へ「入った人」を
    let mut 既存へ = 0usize;
    for p in &配り先.tell_existing {
        if let Some(f) = 送る(*p, newcomer) {
            let _ = f.await;
            既存へ += 1;
        }
    }
    // 入った人へ「既存の面々」を
    let mut 新入りへ = 0usize;
    for p in &配り先.tell_newcomer {
        if let Some(f) = 送る(newcomer, *p) {
            let _ = f.await;
            新入りへ += 1;
        }
    }
    // **配れたことを言う**（2026-09-18・ASUS が 2 度求めた行）。
    //
    // **失敗だけ記録して成功を記録しないと、
    // 「呼ばれたが成功した」と「呼ばれなかった」が同じ顔になる。**
    //
    // ASUS（2026-09-17）——
    // 「**主催側の 0 件は 2 通りに読めます。見分けられません。**」
    // 「**失敗だけ記録して成功を記録しないと、
    //   『呼ばれたが成功した』と『呼ばれなかった』が同じ顔になります。**」
    //
    // **2026-09-18、3 台そろった回でも `紹介:` が 0 件だった。**
    // **配れていたのに、主催側からはそれが見えなかった**
    // （ゲスト側の `紹介: 呼べました` で、初めて配れたと分かった）。
    // **予言どおりだったので、足す。**
    記録!(
        "紹介: 配りました（新入り {} → 既存 {} 人へ／既存 {} 人を新入りへ）",
        短く(&key_to_string(newcomer)),
        既存へ,
        新入りへ
    );
}

/// **画面の出来事を、同じログへ流す。**
///
/// 覚えている相手 1 人。**呼び名と鍵の組だけ**を画面へ渡す。
#[derive(Debug, serde::Serialize)]
pub struct ContactRow {
    /// 公開鍵（base32・全桁）。画面は**これで引く**。
    key: String,
    /// 人が付けた呼び名。
    label: String,
    /// **居場所を覚えているか。**覚えていれば、名前を押して呼べる。
    ///
    /// **住所そのものは画面へ渡さない。**画面に要るのは
    /// 「押せるかどうか」だけであり、中身を出しても人には読めない。
    has_address: bool,
    /// **こちらが書いた覚え書き。**空なら書いていない。
    ///
    /// 相手が名乗ったものとは別である（名乗りは相手の都合で変わる）。
    note: String,
}

/// 覚えている相手を並べる。
///
/// **CLI（`warifu contacts`）と同じ置き場所を読む。**別の機械で動いているエージェントに
/// `warifu contacts add` で名前を付けておけば、画面のチャットにもその名前で出る。
/// **鍵の頭 12 文字だけでは、人にもエージェントにも見分けが付かない。**
#[tauri::command]
fn contacts() -> Answer<Vec<ContactRow>> {
    let vault = warifu_vault::Vault::default_location()?;
    let list = vault.contacts()?;
    Ok(list
        .iter()
        .map(|c| ContactRow {
            key: c.key().to_string(),
            label: c.label().to_owned(),
            has_address: c.address().is_some(),
            note: c.note().to_owned(),
        })
        .collect())
}

/// 相手を覚える（呼び名を付ける）。**同じ鍵に付け直せる。**
///
/// 覚えるのは**呼び名と鍵だけ**で、住所（いまどこに居るか）は入らない。
/// 住所は会議キーが運ぶ（`issues/010` の会場鍵で埋める予定）。
#[tauri::command]
async fn remember(bridge: State<'_, Bridge>, key: String, label: String) -> Answer<()> {
    let who = key.parse::<PublicKey>()?;
    let vault = warifu_vault::Vault::default_location()?;
    let mut list = vault.contacts()?;
    let 名 = label.trim();
    if 名.is_empty() {
        list.remove(who);
        vault.save_contacts(&list)?;
        記録!("名簿: 忘れた（{}）", 短く(&key));
        return Ok(());
    }
    list.add(who, 名, now_secs())?;
    // **呼び名を付けるのと同時に、いま知っている住所も書く。**
    // 住所は「覚えている相手」にしか書けない（住所だけの行を作らない）ので、
    // ここで拾わないと、**呼び名を付けた相手の居場所が永遠に入らない**
    if let Some(住所) = bridge.addresses.lock().await.get(&who.to_bytes()).cloned() {
        let _ = list.note_address(who, &住所);
    }
    vault.save_contacts(&list)?;
    記録!("名簿: 覚えた（{}）", 短く(&key));
    Ok(())
}

/// **こちらが書いた覚え書き**を残す（オーナー・2026-09-08）。
///
/// 「どの機械の、何をするエージェントか」を、人が自分の言葉で残す所である。
/// **相手が名乗ったものとは別に持つ** —— 名乗りは相手の都合で変わるが、
/// **これは変わらない**（こちらが書いたものだから）。
///
/// 空にすると消える。**覚えていない相手には書けない**（行を作らない）。
#[tauri::command]
async fn remember_note(key: String, note: String) -> Answer<()> {
    let who = key.parse::<PublicKey>()?;
    let vault = warifu_vault::Vault::default_location()?;
    let mut list = vault.contacts()?;
    if !list.set_note(who, &note)? {
        return Err(Failure {
            message: "先に呼び名を付けてください".into(),
            code: Some("contact.unknown".into()),
        });
    }
    vault.save_contacts(&list)?;
    記録!(
        "名簿: 覚え書きを書いた（{} / {} 文字）",
        短く(&key),
        note.chars().count()
    );
    Ok(())
}

/// WebView のコンソールはターミナルに出ない。**画面側だけで起きたことが見えないと、
/// 切り分けが「Rust までは来ていた」で止まる。**
///
/// 画面が渡すのは**短い一言だけ**にしてある。中身（SDP・鍵・住所）は渡さない。
#[tauri::command]
fn log(message: String) {
    記録!("画面: {message}");
}

/// **文字を送る**（チャット）。
///
/// **会議に入っている全員へ送る。**下ごしらえ（SDP）と違って、
/// 文字は組ごとのものではないので、宛先を指定しない。
///
/// **まだ残さない。**送ったものも届いたものも、閉じれば消える。
/// 身元は続くようになった（**D42**）ので、次は履歴を置く形を決める（`issues/010`）。
#[tauri::command]
async fn send_text(bridge: State<'_, Bridge>, body: String, to: Option<String>) -> Answer<()> {
    // **人が打った行は、まずこの機械へ配る。**
    // 会議に人が 1 人も居なくても、**同じエージェントの AI には届く** ——
    // 「会議ありきのチャットじゃない」（オーナー・2026-09-06）。
    // AI は画面に書けるのに人は返せない、という片側だけの経路にしない。
    //
    // **2026-09-07 に実物で踏んだ。**画面は「この機械 1 人」でボタンを押せるのに、
    // ここが会議の相手だけを見ていたので「まだ誰も居ません」と断っていた。
    let 話 = desk::聞いた(&key_to_string(bridge.device.public_key()), &body);

    // **宛先が決まっていれば、そのエージェントにだけ渡して終わる。**
    // 3 つも 4 つもこの機械につながっていると、zumen だけに聞きたくても全員に飛ぶ ——
    // それでは 1 対 1 が成り立たない（2026-09-08）
    if let Some(宛先) = &to {
        if !desk::エージェントへ配る(&bridge, 宛先, 話) {
            return Err(Failure {
                message: "その相手はもう居ません".into(),
                code: Some("desk.gone".into()),
            });
        }
        記録!("送信: 文字（{} バイト）を 1 エージェントへ", body.len());
        return Ok(());
    }

    let この機械に居る = desk::エージェントの数(&bridge) > 0;
    if この機械に居る {
        desk::配る(&bridge, 話);
    }

    let meeting = いま見ているルーム(&bridge.いまのルーム).await;
    let Some(meeting) = meeting else {
        // この機械に居るなら、届いている。**届いたものを失敗にしない**
        if この機械に居る {
            記録!(
                "送信: 文字（{} バイト）をこの機械へ（会議はまだ無い）",
                body.len()
            );
            return Ok(());
        }
        return Err(Failure {
            message: "まだ会議がありません".into(),
            code: None,
        });
    };
    // **そのルームに居る相手だけへ。**全員へ配ると、別のルームの人にも届く
    let 送り先 = そのルームの相手(
        &bridge.conferences,
        &bridge.outbound,
        meeting,
        bridge.device.public_key(),
    )
    .await;
    if 送り先.is_empty() {
        if この機械に居る {
            記録!(
                "送信: 文字（{} バイト）をこの機械へ（ルームに人は居ない）",
                body.len()
            );
            return Ok(());
        }
        return Err(Failure {
            message: "まだ誰も居ません".into(),
            code: None,
        });
    }
    記録!(
        "送信: 文字（{} バイト）を {} 人へ",
        body.len(),
        送り先.len()
    );
    for tx in &送り先 {
        // 届かない相手が居ても止めない。**送る側を待たせない**
        let _ = tx
            .send(Notice::Text {
                meeting,
                // **自分が言ったと載せる**（D48）。相手はこれで誰の発言かが分かる
                from: bridge.device.public_key(),
                話し手: None,
                body: body.clone(),
            })
            .await;
    }
    Ok(())
}

/// **覚えている相手へ、1 対 1 で言う。**
///
/// 同じルームに居るならその場で渡し、**居なければ預かり所へ預ける**（D71）。
/// 預かり所を置いていなければ「いま居ません」で終わる ——
/// **黙って中央へ繋ぎに行かない**（D68）。
///
/// **この機械へは配らない。**これは 1 人へ宛てた言葉である（D66 と同じ構え）。
#[tauri::command]
async fn send_to_contact(bridge: State<'_, Bridge>, key: String, body: String) -> Answer<()> {
    let 相手: PublicKey = key.parse().map_err(|_| Failure {
        message: "相手の鍵として読めません".into(),
        code: None,
    })?;

    if let Some(meeting) = 相手が居るルーム(&bridge.conferences, 相手).await {
        let tx = bridge.outbound.lock().await.get(&相手.to_bytes()).cloned();
        if let Some(tx) = tx {
            let 送れた = tx
                .send(Notice::Text {
                    meeting,
                    // **自分が言ったと載せる**（D48）
                    from: bridge.device.public_key(),
                    話し手: None,
                    body: body.clone(),
                })
                .await
                .is_ok();
            if 送れた {
                記録!("送信: 文字（{} バイト）を 1 人へ（同じルーム）", body.len());
                return Ok(());
            }
            // 経路が死んでいた。**落としてしまわず、預かり所へ回す**
            記録!("送信: 経路が死んでいたので預かり所へ回します");
        }
    }

    postbox::預ける(&bridge, 相手, &body).await
}

/// **会議から抜けると告げる。**
///
/// 告げないと、相手の名簿からは**経路が切れたときにしか**消えない。
/// 2 人なら経路が切れれば分かるが、**3 人以上では他の人の名簿に残り続ける**
/// （その人との経路は生きているため）。
///
/// **全員へ送る。**抜けたことは、繋がっている全員に関係がある。
#[tauri::command]
async fn leave(bridge: State<'_, Bridge>) -> Answer<()> {
    let meeting = いま見ているルーム(&bridge.いまのルーム).await;
    let Some(meeting) = meeting else {
        // まだ会議が無い。**断りではない**ので黙って戻る
        return Ok(());
    };
    // **そのルームの相手だけへ。**別のルームには居続ける
    let 送り先 = そのルームの相手(
        &bridge.conferences,
        &bridge.outbound,
        meeting,
        bridge.device.public_key(),
    )
    .await;
    for tx in &送り先 {
        // 届かない相手が居ても止めない。**抜ける側を待たせない**
        let _ = tx.send(Notice::Leave { meeting }).await;
    }
    // **抜けたルームは畳む。**居ないルームを持ち続けない
    bridge.conferences.lock().await.remove(&meeting);
    // **その部屋の合言葉も落とす**（**D118**）——
    // **抜けた部屋へ、証しで入り直せてしまう。**
    // 使わない秘密を持ち続けない（`issued.tsv` と同じ構え）
    bridge.部屋の合言葉.lock().await.remove(&meeting);
    合言葉の控えを揃える(&bridge).await;
    let mut いま = bridge.いまのルーム.lock().await;
    if *いま == Some(meeting) {
        *いま = bridge.conferences.lock().await.keys().next().copied();
    }
    // **出した割符の控えも落とす**（**#38**）——
    // 抜けた部屋の鍵を持ち続けない。**使わない秘密を持ち続けない**（D103 と同じ構え）
    if let Ok(vault) = warifu_vault::Vault::default_location() {
        if let Ok(控え) = vault.issued() {
            let 残す: Vec<(String, Vec<u8>)> = 控え
                .into_iter()
                .filter(|(ルーム, _)| *ルーム != meeting.to_string())
                .collect();
            if let Err(e) = vault.replace_issued(&残す) {
                記録!("出した割符の控えを落とせませんでした: {e}");
            }
        }
    }
    // **名簿で通していた相手を降ろす**（**D111**）。
    // **持ち越さない** —— 「主催が言ったから通した」が次の部屋まで効かないように
    {
        let mut door = bridge.door.lock().await;
        let 数 = door.名簿の数();
        door.名簿を忘れる();
        if 数 > 0 {
            記録!("戸口: 名簿で通していた {数} 人を降ろしました");
        }
    }
    // **帰り道を忘れる**（`gh issue 13`）。
    // **自分で抜けた人に「戻る」を出さない**し、**使わない秘密を持ち続けない**
    // （帰り道には割符の片割れが入っている）
    if let Ok(vault) = warifu_vault::Vault::default_location() {
        let 帰り道 = vault.rejoin().ok().flatten();
        if 帰り道.is_some_and(|(id, _)| id == meeting.to_string()) {
            if let Err(e) = vault.forget_rejoin() {
                記録!("帰り道を消せませんでした: {e}");
            }
        }
    }
    Ok(())
}

/// **前に入ったルームへの帰り道**（`gh issue 13`）。
///
/// 返すのは **ルーム id と、入るのに使ったルームキー**。無ければ `None`。
///
/// # 画面はルームキーを出さない
///
/// ここに入っているのは**割符の片割れ**である。画面は「**前のルームに戻る**」の
/// 押し口を出すだけで、**文字そのものを見せない**（見せると、人が別の相手へ渡せてしまう）。
#[tauri::command]
async fn rejoin_key() -> Answer<Option<(String, String)>> {
    let vault = warifu_vault::Vault::default_location()?;
    Ok(vault.rejoin()?)
}

/// **この機械につながっている顔ぶれ。**画面が一覧に出し、「送れるかどうか」も決める。
///
/// **数だけでは足りない。**1 台の PC で複数のエージェントが同じこの機械につながるので、
/// どれがつながっているのかが分からない（2026-09-08 オーナー指摘）。
#[tauri::command]
fn desk_seats() -> Vec<String> {
    desk::つながっている顔ぶれ()
}

/// ルーム 1 つ分。**画面が一覧に出す。**
#[derive(Debug, serde::Serialize)]
pub struct RoomRow {
    /// ルームの id（全桁）。**押したときに使う。**
    id: String,
    /// いま居る人数（自分を含む）。
    members: usize,
    /// 自分が主催か。
    host: bool,
}

/// **いま居るルームを並べる。**
///
/// ルームを複数持てるようになった以上（`issues/015`）、
/// **一覧が無ければ切り替えようがない。**持てても見えなければ意味がない。
#[tauri::command]
async fn rooms(bridge: State<'_, Bridge>) -> Answer<Vec<RoomRow>> {
    let me = bridge.device.public_key();
    // **主催は覚えてある**（**#37** / **D111**）。
    //
    // **`members().first()` では主催が分からない** ——
    // **ゲストの名簿は先頭が自分**なので、**ゲストでも `first() == me` が真になる。**
    //
    // **2026-09-16 に戸口の迎えで同じ所を踏み、`主催たち` を足して直した。**
    // **ここだけ直っていなかった** ——
    // 2026-09-18、Mac Air が「**入った部屋が『あなたが建てた』と出ています**」と報告した。
    //
    // **誰の部屋かは、招く／抜けるの判断に効く。**出所が分からなくなる。
    let 主催たち = bridge.主催たち.lock().await;
    let 棚 = bridge.conferences.lock().await;
    let mut 並び: Vec<RoomRow> = 棚
        .values()
        .map(|c| RoomRow {
            id: c.id().to_string(),
            members: c.members().len(),
            // **覚えていない部屋は、自分が建てたものである**
            // （入るときに必ず覚えるので・`connect` / `call`）。
            // **覚えていない＝誰にも入られていない自分の部屋**
            host: 主催たち.get(&c.id()).is_none_or(|主催| *主催 == me),
        })
        .collect();
    // **並びを固定する。**`HashMap` の順は読むたびに変わる
    並び.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(並び)
}

/// **見るルームを選ぶ。**
///
/// 見ていないルームも生きている —— 選び直すだけで、経路は切れない。
#[tauri::command]
async fn look_at_room(bridge: State<'_, Bridge>, id: String) -> Answer<bool> {
    let 選ぶ = bridge
        .conferences
        .lock()
        .await
        .keys()
        .find(|k| k.to_string() == id)
        .copied();
    let Some(選ぶ) = 選ぶ else {
        // **知らないルームを見ていることにしない**
        return Ok(false);
    };
    *bridge.いまのルーム.lock().await = Some(選ぶ);
    Ok(true)
}

/// **いま見ているルームの id。**画面が会話をルームごとに分けるのに使う。
///
/// ルームを複数持つので（`issues/015`）、**どのルームの会話を出すか**を
/// 画面が知っている必要がある。
#[tauri::command]
async fn current_room(bridge: State<'_, Bridge>) -> Answer<Option<String>> {
    Ok(いま見ているルーム(&bridge.いまのルーム)
        .await
        .map(|id| id.to_string()))
}

/// **同じ PC の AI に「止まれ」と言う。**
///
/// 常駐（`warifu agent`）は人が居ない間も動く。
/// **落とすしか止め方が無い状態にしない**（`issues/014`）。
#[tauri::command]
async fn stop_agent(bridge: State<'_, Bridge>, name: String) -> Answer<bool> {
    Ok(desk::エージェントを止める(&bridge, &name))
}

/// 相手が offer を出す側か（**D38**）。画面が交渉の向きを決めるのに使う。
#[tauri::command]
async fn should_offer_to(bridge: State<'_, Bridge>, peer: String) -> Answer<bool> {
    // base32 は同じバイト列に複数の表記を許さない。読めない表記はここで止める
    let bytes = warifu_core::base32::decode(&peer).ok_or_else(|| Failure {
        message: "公開鍵として読めません".into(),
        code: None,
    })?;
    let key = PublicKey::from_bytes(bytes.try_into().map_err(|_| Failure {
        message: "公開鍵の長さが違います".into(),
        code: None,
    })?)?;
    let Some(meeting) = いま見ているルーム(&bridge.いまのルーム).await else {
        return Err(Failure {
            message: "まだ会議がありません".into(),
            code: None,
        });
    };
    let 棚 = bridge.conferences.lock().await;
    let Some(conference) = 棚.get(&meeting) else {
        return Err(Failure {
            message: "まだ会議がありません".into(),
            code: None,
        });
    };
    Ok(conference.should_offer_to(&key))
}

/// 知らせの種類だけを言う。**中身は出さない。**
///
/// **`Notice` は `non_exhaustive` なので、`_` の枝は外せない。**
/// だから**足したのに名を付け忘れても、`cargo build` は通る** ——
/// 記録に「知らない知らせ」と出るだけである。
///
/// **2026-09-17 に、それで 1 往復した。**`Profile` に名が無く、
/// `受信: 知らない知らせ` と出ていた。ASUS へ「そちらで同じ行が出ていないか」と
/// 聞いてしまった —— **こちらの書き漏らしだった。**
/// `知らせに名が付いているか` が、いまは見張っている。
fn 知らせの名(n: &Notice) -> &'static str {
    match n {
        Notice::Invite { .. } => "招待",
        Notice::Join { .. } => "参加",
        Notice::Leave { .. } => "退出",
        Notice::Signal(_) => "下ごしらえ（SDP / ICE）",
        Notice::Link { .. } => "回線の報せ",
        Notice::Introduce { .. } => "紹介",
        Notice::Text { .. } => "文字",
        Notice::Profile { .. } => "名乗り",
        // **中身は出さない。**種別だけ言う —— これは合言葉を運ぶ知らせである（**D118**）
        Notice::RoomSecret { .. } => "部屋の合言葉",
        _ => "知らない知らせ",
    }
}

fn emit_events(app: &AppHandle, events: &[warifu_app::Event]) {
    for event in events {
        let _ = match event {
            warifu_app::Event::Joined(key) => app.emit(EVENT_JOINED, key_to_string(*key)),
            warifu_app::Event::Left(key) => app.emit(EVENT_LEFT, key_to_string(*key)),
            warifu_app::Event::Signal { from, step, blob } => app.emit(
                EVENT_SIGNAL,
                SignalPayload {
                    step: step_to_str(*step).to_string(),
                    blob: String::from_utf8_lossy(blob).into_owned(),
                    from: Some(key_to_string(*from)),
                    // 受け取ったものに宛先は要らない（自分あてに決まっている）
                    to: None,
                },
            ),
        };
    }
}

/// ルームの鍵を、**渡せる 1 本のリンク**にする（**D79**）。
#[tauri::command]
fn room_link(key: String) -> String {
    link::ルームのリンク(&key)
}

/// そのリンクを **QR** にする（**D79**）。目の前の相手に読ませる用。
///
/// # Errors
/// QR に入らないとき。
#[tauri::command]
fn room_qr(key: String) -> Result<String, String> {
    link::qrにする(&link::ルームのリンク(&key))
}

pub fn run() {
    tauri::Builder::default()
        // **届いたことを窓の外へ押し出すため**（`notify.rs`）。
        // 押し出せないとチャットにならない（オーナー・2026-09-07）
        .plugin(tauri_plugin_notification::init())
        // **`warifu://` を受ける**（**D79**）。鍵を貼り付けさせない。
        // **押しただけでは入らない** —— 受け取ったあと、画面が人に尋ねる
        .plugin(tauri_plugin_deep_link::init())
        // **更新を確かめて入れ替える**（**D81**）。
        // 落とす前に、埋め込んだ公開鍵で `latest.json` の署名を検める ——
        // **検めないと、更新の口が「何でも実行させる口」になる**
        .plugin(tauri_plugin_updater::Builder::new().build())
        // 入れ替えたあとに立て直す（`relaunch`）
        .plugin(tauri_plugin_process::init())
        // **メニューから来た操作を、画面へ渡す。**
        // メニューは OS の側に居るので、画面の状態（いま何を選んでいるか）は知らない
        .on_menu_event(|app, event| {
            if let Some(選び) = event.id().0.strip_prefix(menu::THEME_PREFIX) {
                let _ = app.emit(EVENT_THEME, 選び.to_owned());
            }
            // **更新を確かめる**（**D81**）。確かめるのは画面の側
            if event.id().0 == menu::CHECK_UPDATE_ID {
                let _ = app.emit(EVENT_CHECK_UPDATE, ());
            }
        })
        .setup(|app| {
            // 最初に呼んで、起点をここに固定する
            起動からの秒();
            記録!("起動しました。ここから経路の要所を書き出します（+秒 は起動からの経過）");
            // **置き場所を、起動のたびに出す**（**#22**・2026-09-15）。
            //
            // Windows で「`rejoin.tsv` が無い」と報告された。**書けてはいて、
            // 人が見ている所とは別**という筋がある —— `HOME` を先に見るので、
            // **端末から起動すると `USERPROFILE` とは別の所になる。**
            // **どこに書いているかを、こちらが言わないと切り分けられない。**
            {
                let 出どころ = warifu_vault::家の出どころ(
                    std::env::var_os("WARIFU_HOME").as_deref(),
                    std::env::var_os("HOME").as_deref(),
                    std::env::var_os("USERPROFILE").as_deref(),
                );
                match warifu_vault::Vault::default_location() {
                    Ok(v) => 記録!("置き場所: {}（{出どころ} で決まりました）", v.dir().display()),
                    Err(e) => 記録!("置き場所が決まりません（{出どころ}）: {e}"),
                }
            }
            app.manage(Bridge::new());
            // **画面が立ったらこの機械も開く。**人が別の操作をしなくても、
            // 同じ PC のエージェントが会話につながれる状態にする
            desk::開く(app.handle().clone());

            // **窓が開いている間に来たリンク**を受ける
            {
                use tauri_plugin_deep_link::DeepLinkExt as _;
                let handle = app.handle().clone();
                app.deep_link().on_open_url(move |event| {
                    let urls: Vec<String> = event.urls().iter().map(ToString::to_string).collect();
                    link::受ける(&handle, &urls);
                });
                // **立ち上がる前に押されたリンク**も拾う。
                // 落としたら、人は「押したのに何も起きない」を見る
                if let Ok(Some(urls)) = app.deep_link().get_current() {
                    let urls: Vec<String> = urls.iter().map(ToString::to_string).collect();
                    link::受ける(app.handle(), &urls);
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            room_link,
            room_qr,
            my_address,
            my_key,
            firewall_state,
            host_meeting,
            name_room,
            room_name,
            pending_links,
            connect,
            listen,
            invite,
            send_signal,
            should_offer_to,
            leave,
            send_text,
            log,
            contacts,
            remember,
            remember_note,
            desk_seats,
            call_contact,
            connect_in_room,

            rejoin_room,
            invite_window,
            note_screen,
            relay_setting,
            set_relay,
            pending_passes,
            answer_pass,
            stop_knowing,
            known_keys,
            note_path,
            link_answered,
            stop_agent,
            current_room,
            rooms,
            look_at_room,
            set_menu_locale,
            cli_state,
            rejoin_key,
            postbox::postbox,
            postbox::set_postbox,
            postbox::fetch_postbox,
            schedule::schedule_list,
            schedule::schedule_add,
            schedule::schedule_remove,
            profile::profiles,
            profile::set_profile,
            profile::set_avatar,
            profile::set_avatar_bytes,
            profile::read_image,
            profile::clear_avatar,
            profile::avatar_bytes,
            send_to_contact,
        ])
        .run(tauri::generate_context!())
        .expect("warifu の窓を開けませんでした");
}

#[cfg(test)]
mod 時刻の言い方の試験 {
    use super::時刻の言い方;

    /// **時刻の箱を足さずに手で組んでいる**（`chrono` を入れない）ので、
    /// **境目を押さえておく** —— 閏年・月の変わり目・年の変わり目。
    #[test]
    fn 決まった秒が_決まった文になる() {
        // 1970-01-01 00:00 UTC
        assert_eq!(時刻の言い方(0), "01-01 00:00 UTC");
        // 2026-09-19 05:51 UTC（ASUS が出した鍵の期限・JST 14:51）
        assert_eq!(時刻の言い方(1_789_797_060), "09-19 05:51 UTC");
    }

    #[test]
    fn 閏年の2月29日を_飛ばさない() {
        // 2024-02-29 00:00 UTC = 1709164800
        assert_eq!(時刻の言い方(1_709_164_800), "02-29 00:00 UTC");
        // その 1 日後は 3 月 1 日
        assert_eq!(時刻の言い方(1_709_164_800 + 86400), "03-01 00:00 UTC");
    }

    #[test]
    fn 平年の2月は28日で終わる() {
        // 2023-02-28 00:00 UTC = 1677542400
        assert_eq!(時刻の言い方(1_677_542_400), "02-28 00:00 UTC");
        assert_eq!(時刻の言い方(1_677_542_400 + 86400), "03-01 00:00 UTC");
    }

    #[test]
    fn 年をまたぐ() {
        // 2025-12-31 23:59 UTC = 1767225540
        assert_eq!(時刻の言い方(1_767_225_540), "12-31 23:59 UTC");
        assert_eq!(時刻の言い方(1_767_225_540 + 60), "01-01 00:00 UTC");
    }

    #[test]
    fn utc_と書く() {
        // **書かないと、人が自分の時計と比べて混乱する。**
        // 2026-09-18 の鍵は JST 14:51 だが、記録は UTC 05:51 で出る
        assert!(時刻の言い方(1_789_797_060).ends_with(" UTC"));
    }
}

#[cfg(test)]
mod 知らせの名の試験 {
    use super::知らせの名;
    use warifu_core::Seed;
    use warifu_meeting::{MeetingId, Notice};

    fn 鍵() -> warifu_core::PublicKey {
        Seed::from_bytes([9u8; 32])
            .profile("Personal")
            .device("PC")
            .public_key()
    }

    /// **いま在る知らせを、全部 1 つずつ作る。**
    ///
    /// `Notice` は `non_exhaustive` なので、`知らせの名` の `_` の枝は外せない。
    /// **だから「足したのに名を付け忘れた」を build は教えてくれない。**
    /// ここに 1 行足す手間で、**記録が「知らない知らせ」と言うのを止める。**
    ///
    /// **知らせを足したら、ここにも足す。**
    fn 知らせたち() -> Vec<Notice> {
        let 部屋 = MeetingId::generate();
        vec![
            Notice::Invite {
                meeting: 部屋,
                roster: warifu_meeting::Roster::new(鍵()),
            },
            Notice::Join { meeting: 部屋 },
            Notice::Leave { meeting: 部屋 },
            Notice::Signal(warifu_meeting::Signal::new(
                部屋,
                warifu_meeting::Step::Offer,
                Vec::new(),
            )),
            Notice::Link {
                meeting: 部屋,
                report: warifu_meeting::Report::new(0, 0, 0),
            },
            Notice::Introduce {
                meeting: 部屋,
                who: 鍵(),
                address: String::new(),
            },
            Notice::Text {
                meeting: 部屋,
                from: 鍵(),
                話し手: None,
                body: String::new(),
            },
            Notice::Profile {
                meeting: 部屋,
                from: 鍵(),
                名前: String::new(),
                紹介: String::new(),
            },
            Notice::RoomSecret {
                meeting: 部屋,
                合言葉: warifu_core::合言葉::作る().expect("作れる"),
            },
        ]
    }

    #[test]
    fn 知らせに名が付いている() {
        // **2026-09-17 に、`Profile` の名が無くて 1 往復した。**
        // 記録に `受信: 知らない知らせ` と出て、相手の機械を疑ってしまった
        for 知らせ in 知らせたち() {
            let 名 = 知らせの名(&知らせ);
            assert_ne!(名, "知らない知らせ", "名が付いていない: {知らせ:?}");
            assert!(!名.is_empty());
        }
    }

    #[test]
    fn 名に中身を混ぜない() {
        // **種類だけを言う。**本文や鍵を記録へ出さない（`話の記録` と同じ構え）
        let 知らせ = Notice::Text {
            meeting: MeetingId::generate(),
            from: 鍵(),
            話し手: Some("たろう".to_owned()),
            body: "ひみつ".to_owned(),
        };
        let 名 = 知らせの名(&知らせ);
        assert_eq!(名, "文字");
        assert!(!名.contains("ひみつ"));
        assert!(!名.contains("たろう"));
    }
}

#[cfg(test)]
mod 中継の試験 {
    use super::中継を使うか;

    #[test]
    fn 既定では中継を使わない() {
        // **D13。**既定は直接だけ —— 中継を通すと、
        // 「誰がいつ誰に繋いだか」が中継の運用者に見える（**D10**）
        assert!(!中継を使うか(None));
        assert!(!中継を使うか(Some("")));
        assert!(!中継を使うか(Some("0")));
        assert!(!中継を使うか(Some("no")));
    }

    #[test]
    fn 明示したときだけ使う() {
        // **D78 の「付けたときだけ」を画面でも同じ形にする**
        for 言葉 in ["1", "true", "TRUE", "yes", "on"] {
            assert!(中継を使うか(Some(言葉)), "{言葉} で使う");
        }
    }

    #[test]
    fn 前後の空白は無視する() {
        assert!(中継を使うか(Some(" 1 ")));
        assert!(!中継を使うか(Some("  ")));
    }
}

#[cfg(test)]
mod 見る部屋の試験 {
    use super::見る部屋を決める;
    use warifu_meeting::MeetingId;

    fn 部屋() -> MeetingId {
        MeetingId::generate()
    }

    #[test]
    fn 見ている部屋があれば_そのまま() {
        // **人が［入る］で移った先を、あとから来た建て直しが奪わない**（#39）
        let 見ている = 部屋();
        let 建てた = 部屋();
        assert_eq!(見る部屋を決める(Some(見ている), 建てた), 見ている);
    }

    #[test]
    fn 見ている部屋が無ければ_建てた部屋を見る() {
        let 建てた = 部屋();
        assert_eq!(見る部屋を決める(None, 建てた), 建てた);
    }
}
