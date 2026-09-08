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
/// **この PC の机から出た発言。**`[公開鍵, 中身, 時刻]` で渡す。
///
/// 相手から届いた文字（[`EVENT_TEXT`]）と分けるのは、
/// **同じ席の AI の発言だと人に分かる必要がある**ため。
/// 混ぜると、誰が言ったのか画面から読めなくなる。
const EVENT_DESK: &str = "warifu://desk";
/// **机に何人着いているか**が変わった。人数だけを渡す。
///
/// 会議に人が居なくても、**同じ席の AI が居るなら人は話しかけられる。**
/// これが無いと、AI が居るのに「入ってきたら送れます」と出たままになる。
const EVENT_DESK_SEATS: &str = "warifu://desk-seats";

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
mod contacts;
mod desk;
mod notify;

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
    /// 戸口。**割符が合わない相手は、ここで断る**（D31）。
    door: Arc<Mutex<Door>>,
    /// 相手ごとの住所（**D41**）。
    ///
    /// **主催者は、繋がれた相手の住所を知らない**（相手から来たので）。
    /// だから**入る側が自分で名乗る。**それをここに覚えて、次の人へ紹介する。
    addresses: Arc<Mutex<HashMap<[u8; 32], String>>>,
    /// いま居る部屋。**複数持てる**（`issues/015`）。
    ///
    /// 「相手を選ぶ ＝ その人との部屋を開く」が成り立つには、
    /// **air と 2 人で話しながら、3 人の部屋にも居る**ことができなければならない。
    /// 1 つしか持てないと、部屋を移るたびに前の部屋が切れる。
    ///
    /// **鍵は部屋の id。**`Notice` は前から部屋の id を持っているので、
    /// 届いた知らせをどの部屋のものか振り分けられる。
    conferences: Arc<Mutex<HashMap<MeetingId, Conference>>>,
    /// **画面がいま見ている部屋。**打ったものはここへ流れる。
    ///
    /// 部屋そのものとは別に持つ —— **見ていない部屋も生きている。**
    いまの部屋: Arc<Mutex<Option<MeetingId>>>,
    /// 相手ごとの送り出し口（**M6**）。
    ///
    /// 1 本しか持たない形にすると、3 人目が来た時点で**前の相手へ届かなくなる。**
    /// 鍵をそのまま鍵にする（`PublicKey` は `Hash` を持たないのでバイト列で持つ）。
    outbound: Arc<Mutex<HashMap<[u8; 32], mpsc::Sender<Notice>>>>,
    /// 机に着いている相手へ配る口（`desk.rs`）。
    ///
    /// **人が打った行も、相手から届いた行も、ここを通す。**
    /// 通さないと、同じ席の AI は人の発言が見えないまま返事をすることになる。
    /// 添えている数は**出所の番号**（`desk::机の外` なら机の外から出たもの）。
    /// **言った本人には返さない**ために持つ。
    desk: tokio::sync::broadcast::Sender<(u64, warifu_desk::FromDesk)>,
}

/// いま居る部屋たち。**複数持てる**（`issues/015`）。
pub(crate) type 部屋たち = Arc<Mutex<HashMap<MeetingId, Conference>>>;
/// 画面がいま見ている部屋。
pub(crate) type 見ている部屋 = Arc<Mutex<Option<MeetingId>>>;

/// 部屋を足して、**見ている部屋にする。**
pub(crate) async fn 部屋を足す(
    部屋: &部屋たち,
    いま: &見ている部屋,
    c: Conference,
) -> MeetingId {
    let id = c.id();
    部屋.lock().await.insert(id, c);
    *いま.lock().await = Some(id);
    id
}

/// いま見ている部屋の id。**無ければ `None`。**
pub(crate) async fn いま見ている部屋(いま: &見ている部屋) -> Option<MeetingId> {
    *いま.lock().await
}

/// **その部屋に居る相手だけ**へ送る口を集める。
///
/// 部屋を複数持つので、**全員へ配ると別の部屋の人にも届く。**
pub(crate) async fn その部屋の相手(
    部屋: &部屋たち,
    outbound: &Arc<Mutex<HashMap<[u8; 32], mpsc::Sender<Notice>>>>,
    meeting: MeetingId,
    me: PublicKey,
) -> Vec<mpsc::Sender<Notice>> {
    let 面々: Vec<[u8; 32]> = {
        let 棚 = 部屋.lock().await;
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

/// この端末の身元。**CLI と同じものを使う。**
///
/// 画面と端末で別の身元になると、`warifu id` で見せた鍵と、
/// 画面が名乗る鍵が食い違う。**同じ人が 2 人居るように見える。**
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
            tally: Arc::new(Mutex::new(Vec::new())),
            // **置いてある知り合いを連れて開く。**
            // 2026-09-07 まで毎起動で空になっており、「一度開けた相手は
            // 次から割符なしで開ける」（D31）が再起動をまたいで効かなかった
            door: Arc::new(Mutex::new(contacts::戸口を開く())),
            addresses: Arc::new(Mutex::new(HashMap::new())),
            conferences: Arc::new(Mutex::new(HashMap::new())),
            いまの部屋: Arc::new(Mutex::new(None)),
            outbound: Arc::new(Mutex::new(HashMap::new())),
            desk: tokio::sync::broadcast::Sender::new(desk::配る溜め),
        }
    }

    async fn node(&self) -> Answer<Arc<Node>> {
        let mut slot = self.node.lock().await;
        if let Some(node) = slot.as_ref() {
            return Ok(Arc::clone(node));
        }
        let node = Arc::new(Node::bind_without_relay(&self.device).await?);
        *slot = Some(Arc::clone(&node));
        Ok(node)
    }
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
        // **部屋が無ければ建てる。**鍵は部屋への招待なので、部屋が要る
        if let Some(id) = いま見ている部屋(&bridge.いまの部屋).await {
            id
        } else {
            let c = Conference::host(bridge.device.public_key(), warifu_app::DEFAULT_CAPACITY)?;
            部屋を足す(&bridge.conferences, &bridge.いまの部屋, c).await
        }
    };
    let 開始 = starts_at.unwrap_or_else(now_secs);
    let (tally, token) = bridge
        .device
        .issue_tally_between(開始, 開始.saturating_add(ttl_secs))?;
    // **前の招待を殺さない。**足していく（D47）
    bridge.tally.lock().await.push(tally);
    記録!(
        "会議キーを作った（会議 {} / {} から {} 秒）",
        短く(&meeting.to_string()),
        開始,
        ttl_secs
    );
    Ok(format_invite(&address, &token, meeting))
}

/// **OS のメニューを、画面と同じ言語にする**（D35）。
///
/// 画面側が `navigator.languages` から決めた答えをそのまま渡す。
/// ここで OS へ聞き直すと、**2 か所が別の答えを出しうる。**
#[tauri::command]
fn set_menu_locale(app: AppHandle, locale: String) -> Answer<()> {
    // **メニューはメインスレッドでしか触れない。**macOS では別スレッドから差し替えると
    // 黙って何も起きない（例外も出ない）。1 回それで「英語のまま」を踏んだ。
    if !menu::LOCALES.contains(&locale.as_str()) {
        // 落とす先は英語だが、**黙って落とさない。**画面側と綴りがずれたときに
        // 「なぜか英語のまま」になるのを、ここで読めるようにしておく
        eprintln!("知らないロケール '{locale}' が来たので英語にします");
    }
    let handle = app.clone();
    app.run_on_main_thread(move || match menu::build(&handle, &locale) {
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

/// 会議を作る。定員は `2..=16`（**D27**）。
#[tauri::command]
async fn host_meeting(bridge: State<'_, Bridge>, capacity: usize) -> Answer<String> {
    let conference = Conference::host(bridge.device.public_key(), capacity)?;
    let id = 部屋を足す(&bridge.conferences, &bridge.いまの部屋, conference).await;
    Ok(id.to_string())
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
    let node = bridge.node().await?;
    let to = Address::from_str(&address)?;
    let mut session = node.connect(&to, &Revocations::new()).await?;
    let peer = session.peer();
    記録!(
        "入室: 経路がつながった（相手 {}）",
        短く(&key_to_string(peer))
    );

    // **最初に割符へ応じる。**会議の話をする前に、通ってよい相手かを相手が決める。
    // ここは Intent の下（生のバイト列）で済ませる。「何を話すか」ではなく
    // 「そもそも話してよいか」の段なので、口の語彙を増やさない（D11）
    let acceptance = bridge.device.accept(&token, now_secs())?;
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
        部屋を足す(
            &bridge.conferences,
            &bridge.いまの部屋,
            Conference::joined(bridge.device.public_key(), meeting, roster),
        )
        .await;
        vec![warifu_app::Event::Joined(peer)]
    };
    let meeting_id = meeting;
    記録!("入室: 会議 {} に入る", 短く(&meeting.to_string()));
    // **呼んだ側にも「入った」を流す。**
    // ここを落としていたので、**呼んだ側は通話を作らず、相手の映像が来なかった**
    // （2026-09-04 に実機で判明）。受けた側だけが Call を持っている状態になる
    記録!("入室: 名簿に入れた（{} 件の出来事を画面へ）", events.len());
    emit_events(&app, &events);

    // 入ると告げる
    channel
        .send(
            &Notice::Join {
                meeting: meeting_id,
            }
            .to_intent()?,
        )
        .await?;

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
    let いまの部屋 = Arc::clone(&bridge.いまの部屋);
    let outbound = Arc::clone(&bridge.outbound);
    let me = bridge.device.public_key();

    let tally = Arc::clone(&bridge.tally);
    let door = Arc::clone(&bridge.door);
    let addresses = Arc::clone(&bridge.addresses);

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
            let 合った = 割符を確かめる(&mut session, &tally, &subject).await;
            記録!(
                "待受: 割符は{}",
                if 合った {
                    "合った"
                } else {
                    "合わなかった"
                }
            );
            let (答え, 知っていた) = {
                let mut door = door.lock().await;
                // **answer は通した相手を知り合いに入れる。**前に見ておかないと
                // 「新しく知り合いになったか」が分からなくなる
                let 知っていた = door.knows(&subject);
                let knock = if 合った {
                    Knock::with_verified_tally(subject.clone(), now_secs())
                } else {
                    Knock::new(subject.clone(), now_secs())
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
            if !知っていた {
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
            if !合った
                && let Some(招待) = call::招く(&conferences, &いまの部屋)
                && let Ok(intent) = 招待.to_intent()
            {
                if channel.send(&intent).await.is_err() {
                    continue;
                }
                記録!("待受: 割符なしの相手へ会議を教えた");
            }
            {
                // **部屋が 1 つも無ければ建てる。**受けた相手を入れる先が要る
                if conferences.lock().await.is_empty() {
                    match Conference::host(me, warifu_app::DEFAULT_CAPACITY) {
                        Ok(c) => {
                            部屋を足す(&conferences, &いまの部屋, c).await;
                        }
                        Err(_) => continue,
                    }
                }
            }
            汲む(
                app.clone(),
                Arc::clone(&conferences),
                Arc::clone(&outbound),
                Arc::clone(&addresses),
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
async fn 割符を確かめる(
    session: &mut warifu_net::Session,
    tally: &Arc<Mutex<Vec<Tally>>>,
    _subject: &Subject,
) -> bool {
    let Ok(Ok(bytes)) =
        tokio::time::timeout(std::time::Duration::from_secs(10), session.recv()).await
    else {
        return false;
    };
    let Ok(acceptance) = Acceptance::from_bytes(&bytes) else {
        return false;
    };
    // **署名した本人と、経路で確定した相手が同じか。**
    // `Acceptance` は本人の鍵で署名されているが、**どこで署名されたかまでは言っていない。**
    // 突き合わせないと、写し取った片割れを別の経路で出せてしまう
    if acceptance.accepter() != session.peer() {
        return false;
    }
    let mut list = tally.lock().await;
    // **どの招待に対する片割れかは、相手が名乗っている。**総当たりで試さない ——
    // 試すと、別の招待の窓（`not_before` / `not_after`）で通ってしまう
    let Some(t) = list.iter_mut().find(|t| t.id() == acceptance.tally()) else {
        return false;
    };
    // **まだ誰も入っていなければ初回、一度入った相手が戻ってきたなら再入場**（D44）。
    // 回線が一瞬切れただけで、10 時から 11 時の会議が終わってはいけない
    let now = now_secs();
    let 名簿 = Revocations::new();
    if t.used_by().is_none() {
        t.match_half(&acceptance, now, &名簿).is_ok()
    } else {
        t.rematch_half(&acceptance, now, &名簿).is_ok()
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
        bridge.device.public_key(),
        channel,
        peer,
    );
}

/// 画面から送るものと相手から届くものを、1 本のタスクで捌く。
#[allow(clippy::too_many_arguments)]
fn 汲む(
    app: AppHandle,
    conferences: 部屋たち,
    outbound: Arc<Mutex<HashMap<[u8; 32], mpsc::Sender<Notice>>>>,
    addresses: Arc<Mutex<HashMap<[u8; 32], String>>>,
    me: PublicKey,
    mut channel: Channel,
    peer: PublicKey,
) {
    let (tx, mut rx) = mpsc::channel::<Notice>(32);
    tokio::spawn(async move {
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
                        // **札が付いていれば、その席の誰が言ったかまで画面へ渡す**
                        // （2026-09-08）。付いていなければ人が言ったもの
                        let 名 = match 話し手 {
                            Some(札) => format!("{}（{札}）", 短く(&key_to_string(peer))),
                            None => key_to_string(peer),
                        };
                        let _ = app.emit(EVENT_TEXT, (名, body.clone()));
                        // **窓が後ろに居ると、届いたことに気づけない。**押し出す
                        notify::届いたと知らせる(&app, &短く(&key_to_string(*from)));
                        // **同じ席の AI にも、同じ行を見せる。**
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
                    if let Notice::Introduce { meeting, who, address } = &notice {
                        // **名乗りをそのまま連絡帳へ落とさない。**
                        // 「C さんの住所はここです」と言われるまま書くと、
                        // 次に人が C の名前を押したとき別の場所へ呼びに行く（D12 の迂回）
                        contacts::住所を覚える(peer, *who, address);
                        addresses.lock().await.insert(who.to_bytes(), address.clone());
                        // 自分が主催者なら、**入った人を既存の面々へ配り、
                        // 入った人へ既存の面々を教える**
                        let 主催 = {
                            let 棚 = conferences.lock().await;
                            棚.get(meeting).map(|c| c.members().first() == Some(&me))
                        };
                        if 主催 == Some(true) {
                            紹介を配る(&conferences, &outbound, &addresses, me, *who, *meeting)
                                .await;
                        } else {
                            // 主催者でなければ、教わった住所を画面へ渡して呼びに行かせる
                            let _ =
                                app.emit(EVENT_INTRODUCED, (key_to_string(*who), address.clone()));
                        }
                        continue;
                    }
                    // **どの部屋あてかで振り分ける。**知らない部屋のものは受け取らない
                    let mut 棚 = conferences.lock().await;
                    let Some(c) = 棚.get_mut(&notice.meeting()) else {
                        記録!("受信: 知らない部屋あてだった。捨てた");
                        continue;
                    };
                    match c.on_notice(peer, &notice) {
                        Ok(events) => emit_events(&app, &events),
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
        // **自分の口だけを外す。**ほかの相手との経路は生きている（M6）
        outbound.lock().await.remove(&peer.to_bytes());
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
    let meeting = いま見ている部屋(&bridge.いまの部屋).await;
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

/// 紹介を配る（**D41**）。**主催者だけが呼ぶ。**
///
/// 既存の面々へ「入った人の住所」を、入った人へ「既存の面々の住所」を送る。
/// **住所を知らない相手は飛ばす** — まだ名乗っていないだけなので、断りではない。
async fn 紹介を配る(
    conferences: &部屋たち,
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
            return;
        };
        plan
    };
    let book = addresses.lock().await;
    let out = outbound.lock().await;

    let 送る = |to: PublicKey, who: PublicKey| {
        let (Some(tx), Some(address)) = (out.get(&to.to_bytes()), book.get(&who.to_bytes())) else {
            return None;
        };
        Some(tx.send(Notice::Introduce {
            meeting,
            who,
            address: address.clone(),
        }))
    };

    // 既存の面々へ「入った人」を
    for p in &配り先.tell_existing {
        if let Some(f) = 送る(*p, newcomer) {
            let _ = f.await;
        }
    }
    // 入った人へ「既存の面々」を
    for p in &配り先.tell_newcomer {
        if let Some(f) = 送る(newcomer, *p) {
            let _ = f.await;
        }
    }
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
    // **人が打った行は、まず机へ配る。**
    // 会議に人が 1 人も居なくても、**同じ席の AI には届く** ——
    // 「会議ありきのチャットじゃない」（オーナー・2026-09-06）。
    // AI は画面に書けるのに人は返せない、という片側だけの経路にしない。
    //
    // **2026-09-07 に実物で踏んだ。**画面は「机 1 人」でボタンを押せるのに、
    // ここが会議の相手だけを見ていたので「まだ誰も居ません」と断っていた。
    let 話 = desk::聞いた(&key_to_string(bridge.device.public_key()), &body);

    // **宛先が決まっていれば、その席にだけ渡して終わる。**
    // 3 つも 4 つも机に着いていると、zumen だけに聞きたくても全員に飛ぶ ——
    // それでは 1 対 1 が成り立たない（2026-09-08）
    if let Some(宛先) = &to {
        if !desk::席へ配る(&bridge, 宛先, 話) {
            return Err(Failure {
                message: "その相手はもう居ません".into(),
                code: Some("desk.gone".into()),
            });
        }
        記録!("送信: 文字（{} バイト）を 1 席へ", body.len());
        return Ok(());
    }

    let 机に居る = desk::席の数(&bridge) > 0;
    if 机に居る {
        desk::配る(&bridge, 話);
    }

    let meeting = いま見ている部屋(&bridge.いまの部屋).await;
    let Some(meeting) = meeting else {
        // 机に居るなら、届いている。**届いたものを失敗にしない**
        if 机に居る {
            記録!(
                "送信: 文字（{} バイト）を机へ（会議はまだ無い）",
                body.len()
            );
            return Ok(());
        }
        return Err(Failure {
            message: "まだ会議がありません".into(),
            code: None,
        });
    };
    // **その部屋に居る相手だけへ。**全員へ配ると、別の部屋の人にも届く
    let 送り先 = その部屋の相手(
        &bridge.conferences,
        &bridge.outbound,
        meeting,
        bridge.device.public_key(),
    )
    .await;
    if 送り先.is_empty() {
        if 机に居る {
            記録!(
                "送信: 文字（{} バイト）を机へ（部屋に人は居ない）",
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

/// **会議から抜けると告げる。**
///
/// 告げないと、相手の名簿からは**経路が切れたときにしか**消えない。
/// 2 人なら経路が切れれば分かるが、**3 人以上では他の人の名簿に残り続ける**
/// （その人との経路は生きているため）。
///
/// **全員へ送る。**抜けたことは、繋がっている全員に関係がある。
#[tauri::command]
async fn leave(bridge: State<'_, Bridge>) -> Answer<()> {
    let meeting = いま見ている部屋(&bridge.いまの部屋).await;
    let Some(meeting) = meeting else {
        // まだ会議が無い。**断りではない**ので黙って戻る
        return Ok(());
    };
    // **その部屋の相手だけへ。**別の部屋には居続ける
    let 送り先 = その部屋の相手(
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
    // **抜けた部屋は畳む。**居ない部屋を持ち続けない
    bridge.conferences.lock().await.remove(&meeting);
    let mut いま = bridge.いまの部屋.lock().await;
    if *いま == Some(meeting) {
        *いま = bridge.conferences.lock().await.keys().next().copied();
    }
    Ok(())
}

/// **机に着いている顔ぶれ。**画面が一覧に出し、「送れるかどうか」も決める。
///
/// **数だけでは足りない。**1 台の PC で複数のエージェントが同じ机に着くので、
/// どれが着いているのかが分からない（2026-09-08 オーナー指摘）。
#[tauri::command]
fn desk_seats() -> Vec<String> {
    desk::着いている顔ぶれ()
}

/// 部屋 1 つ分。**画面が一覧に出す。**
#[derive(Debug, serde::Serialize)]
pub struct RoomRow {
    /// 部屋の id（全桁）。**押したときに使う。**
    id: String,
    /// いま居る人数（自分を含む）。
    members: usize,
    /// 自分が主催か。
    host: bool,
}

/// **いま居る部屋を並べる。**
///
/// 部屋を複数持てるようになった以上（`issues/015`）、
/// **一覧が無ければ切り替えようがない。**持てても見えなければ意味がない。
#[tauri::command]
async fn rooms(bridge: State<'_, Bridge>) -> Answer<Vec<RoomRow>> {
    let me = bridge.device.public_key();
    let 棚 = bridge.conferences.lock().await;
    let mut 並び: Vec<RoomRow> = 棚
        .values()
        .map(|c| RoomRow {
            id: c.id().to_string(),
            members: c.members().len(),
            host: c.members().first() == Some(&me),
        })
        .collect();
    // **並びを固定する。**`HashMap` の順は読むたびに変わる
    並び.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(並び)
}

/// **見る部屋を選ぶ。**
///
/// 見ていない部屋も生きている —— 選び直すだけで、経路は切れない。
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
        // **知らない部屋を見ていることにしない**
        return Ok(false);
    };
    *bridge.いまの部屋.lock().await = Some(選ぶ);
    Ok(true)
}

/// **いま見ている部屋の id。**画面が会話を部屋ごとに分けるのに使う。
///
/// 部屋を複数持つので（`issues/015`）、**どの部屋の会話を出すか**を
/// 画面が知っている必要がある。
#[tauri::command]
async fn current_room(bridge: State<'_, Bridge>) -> Answer<Option<String>> {
    Ok(いま見ている部屋(&bridge.いまの部屋)
        .await
        .map(|id| id.to_string()))
}

/// **同じ PC の AI に「止まれ」と言う。**
///
/// 常駐（`warifu agent`）は人が居ない間も動く。
/// **落とすしか止め方が無い状態にしない**（`issues/014`）。
#[tauri::command]
async fn stop_agent(bridge: State<'_, Bridge>, name: String) -> Answer<bool> {
    Ok(desk::席を止める(&bridge, &name))
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
    let Some(meeting) = いま見ている部屋(&bridge.いまの部屋).await else {
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
fn 知らせの名(n: &Notice) -> &'static str {
    match n {
        Notice::Invite { .. } => "招待",
        Notice::Join { .. } => "参加",
        Notice::Leave { .. } => "退出",
        Notice::Signal(_) => "下ごしらえ（SDP / ICE）",
        Notice::Link { .. } => "回線の報せ",
        Notice::Introduce { .. } => "紹介",
        Notice::Text { .. } => "文字",
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

pub fn run() {
    tauri::Builder::default()
        // **届いたことを窓の外へ押し出すため**（`notify.rs`）。
        // 押し出せないとチャットにならない（オーナー・2026-09-07）
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            // 最初に呼んで、起点をここに固定する
            起動からの秒();
            記録!("起動しました。ここから経路の要所を書き出します（+秒 は起動からの経過）");
            app.manage(Bridge::new());
            // **画面が立ったら机も開く。**人が別の操作をしなくても、
            // 同じ PC のエージェントが会話に着ける状態にする
            desk::開く(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            my_address,
            my_key,
            host_meeting,
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
            desk_seats,
            call_contact,
            stop_knowing,
            known_keys,
            stop_agent,
            current_room,
            rooms,
            look_at_room,
            set_menu_locale,
        ])
        .run(tauri::generate_context!())
        .expect("warifu の窓を開けませんでした");
}
