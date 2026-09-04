//! warifu Desktop の橋（M5-c2）。
//!
//! **ここに規則を書かない**（baseline §9）。進行は `warifu-app`、封筒は `warifu-meeting`、
//! 経路は `warifu-net` が持っている。この crate がやるのは
//! 「画面から呼べる形に直す」ことと「届いたものを画面へ流す」ことだけである。
//!
//! # まだ踏んでいない所
//!
//! **鍵を保存しない。**起動のたびに新しい種を作る（`Seed::generate`）。
//! 保存する形は **`decisions.md` の D2 が未決**であり、
//! 全端末を失ったときの復旧モデルが決まっていない。
//! **決まる前に「とりあえずファイルへ置く」をやると、それが既成事実になる**（baseline §15）。
//!
//! つまり **今の版はアプリを閉じると別人になる。**これは不具合ではなく、
//! D2 が決まるまで意図的にそうしてある。

mod menu;

use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;

use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::{Mutex, mpsc};

use warifu_app::{Conference, format_invite, introductions_for, parse_invite};
use warifu_core::{Acceptance, Device, PublicKey, Revocations, Seed, Tally};
use warifu_door::{Answer as DoorAnswer, Door, Knock, Subject};
use warifu_intent::Channel;
use warifu_meeting::{Notice, Signal, Step};
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

/// 経路の要所を書き出す。
///
/// **繋がらなかったときに「どこまで進んだか」が分かる**ようにするためだけのもの。
/// ターミナルから起動したときだけ人の目に入る（`open` では消える）。
///
/// **秘密情報を書かない**（baseline §14）。割符の中身・SDP の中身・
/// 公開鍵の全桁は出さない。**長さと種類だけ**を出す。
macro_rules! 記録 {
    ($($arg:tt)*) => {
        eprintln!("[warifu +{:.3}s] {}", 起動からの秒(), format!($($arg)*))
    };
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
    /// 発行した割符の手元の半分。**招待を出すたびに入れ替わる。**
    tally: Arc<Mutex<Option<Tally>>>,
    /// 戸口。**割符が合わない相手は、ここで断る**（D31）。
    door: Arc<Mutex<Door>>,
    /// 相手ごとの住所（**D41**）。
    ///
    /// **主催者は、繋がれた相手の住所を知らない**（相手から来たので）。
    /// だから**入る側が自分で名乗る。**それをここに覚えて、次の人へ紹介する。
    addresses: Arc<Mutex<HashMap<[u8; 32], String>>>,
    conference: Arc<Mutex<Option<Conference>>>,
    /// 相手ごとの送り出し口（**M6**）。
    ///
    /// 1 本しか持たない形にすると、3 人目が来た時点で**前の相手へ届かなくなる。**
    /// 鍵をそのまま鍵にする（`PublicKey` は `Hash` を持たないのでバイト列で持つ）。
    outbound: Arc<Mutex<HashMap<[u8; 32], mpsc::Sender<Notice>>>>,
}

impl Bridge {
    fn new() -> Self {
        // **保存しない。**D2 が決まるまで、起動ごとに使い捨てる
        let seed = Seed::generate().expect("種を作れない");
        Self {
            device: seed.profile("Personal").device("この端末"),
            node: Mutex::new(None),
            tally: Arc::new(Mutex::new(None)),
            door: Arc::new(Mutex::new(Door::new())),
            addresses: Arc::new(Mutex::new(HashMap::new())),
            conference: Arc::new(Mutex::new(None)),
            outbound: Arc::new(Mutex::new(HashMap::new())),
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
#[tauri::command]
async fn invite(bridge: State<'_, Bridge>, ttl_secs: u64) -> Answer<String> {
    let node = bridge.node().await?;
    let address = node.address().await?.to_string();
    // **会議 id を鍵に載せる。**載せないと入る側が別の id を名乗り、
    // こちらが「別の会議あて」として捨てる（2026-09-04 に実機で踏んだ）
    let meeting = {
        let mut slot = bridge.conference.lock().await;
        if slot.is_none() {
            *slot = Some(Conference::host(
                bridge.device.public_key(),
                warifu_app::DEFAULT_CAPACITY,
            )?);
        }
        slot.as_ref().expect("直前に入れた").id()
    };
    let (tally, token) = bridge.device.issue_tally(now_secs(), ttl_secs)?;
    *bridge.tally.lock().await = Some(tally);
    記録!("会議キーを作った（会議 {}）", 短く(&meeting.to_string()));
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
    let id = conference.id().to_string();
    *bridge.conference.lock().await = Some(conference);
    Ok(id)
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
        let mut slot = bridge.conference.lock().await;
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
        *slot = Some(Conference::joined(
            bridge.device.public_key(),
            meeting,
            roster,
        ));
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

    始める(&app, &bridge, channel, peer).await;
    Ok(())
}

/// 待ち受けを始める。**呼ぶ側だけでは 2 台は出会えない。**
///
/// 片方が `connect`、もう片方がこれ。宛先を渡した側が待ち、受け取った側が呼ぶ。
#[tauri::command]
async fn listen(app: AppHandle, bridge: State<'_, Bridge>) -> Answer<()> {
    let node = bridge.node().await?;
    let conference = Arc::clone(&bridge.conference);
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
            let 答え = {
                let mut door = door.lock().await;
                let knock = if 合った {
                    Knock::with_verified_tally(subject.clone(), now_secs())
                } else {
                    Knock::new(subject.clone(), now_secs())
                };
                door.answer(&knock)
            };
            記録!("待受: 戸口の答えは {答え:?}");
            if 答え != DoorAnswer::Open {
                // **断る理由を相手に返さない**（D31）。黙って落とす
                continue;
            }

            let channel = Channel::new(session);
            {
                let mut slot = conference.lock().await;
                if slot.is_none() {
                    match Conference::host(me, warifu_app::DEFAULT_CAPACITY) {
                        Ok(c) => *slot = Some(c),
                        Err(_) => continue,
                    }
                }
            }
            汲む(
                app.clone(),
                Arc::clone(&conference),
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
    tally: &Arc<Mutex<Option<Tally>>>,
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
    let mut slot = tally.lock().await;
    let Some(t) = slot.as_mut() else {
        return false;
    };
    t.match_half(&acceptance, now_secs(), &Revocations::new())
        .is_ok()
}

/// 今の時刻（秒）。**割符の期限と戸口の窓に使う。**
fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// 繋がった口を動かし始める。呼ぶ側・受ける側で同じ形にする。
async fn 始める(app: &AppHandle, bridge: &Bridge, channel: Channel, peer: PublicKey) {
    汲む(
        app.clone(),
        Arc::clone(&bridge.conference),
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
    conference: Arc<Mutex<Option<Conference>>>,
    outbound: Arc<Mutex<HashMap<[u8; 32], mpsc::Sender<Notice>>>>,
    addresses: Arc<Mutex<HashMap<[u8; 32], String>>>,
    me: PublicKey,
    mut channel: Channel,
    peer: PublicKey,
) {
    let (tx, mut rx) = mpsc::channel::<Notice>(32);
    tokio::spawn(async move {
        outbound.lock().await.insert(peer.to_bytes(), tx);
        loop {
            tokio::select! {
                // 画面から送るもの
                outgoing = rx.recv() => {
                    let Some(notice) = outgoing else { break };
                    let Ok(intent) = notice.to_intent() else { continue };
                    if channel.send(&intent).await.is_err() { break; }
                }
                // 相手から届くもの
                incoming = channel.recv() => {
                    let Ok(intent) = incoming else { break };
                    let Ok(notice) = Notice::from_intent(&intent) else {
                        // 会議のものでない口は、経路としては通る。**会議は受け取らない**
                        continue;
                    };
                    // **文字は名簿を動かさない。**そのまま画面へ渡す
                    if let Notice::Text { body, .. } = &notice {
                        let _ = app.emit(EVENT_TEXT, (key_to_string(peer), body.clone()));
                        continue;
                    }
                    // **紹介は名簿を動かさない**（D41）
                    記録!("受信: {}", 知らせの名(&notice));
                    if let Notice::Introduce { meeting, who, address } = &notice {
                        addresses.lock().await.insert(who.to_bytes(), address.clone());
                        // 自分が主催者なら、**入った人を既存の面々へ配り、
                        // 入った人へ既存の面々を教える**
                        let 主催 = {
                            let slot = conference.lock().await;
                            slot.as_ref().map(|c| c.members().first() == Some(&me))
                        };
                        if 主催 == Some(true) {
                            紹介を配る(&conference, &outbound, &addresses, me, *who, *meeting).await;
                        } else {
                            // 主催者でなければ、教わった住所を画面へ渡して呼びに行かせる
                            let _ =
                                app.emit(EVENT_INTRODUCED, (key_to_string(*who), address.clone()));
                        }
                        continue;
                    }
                    let mut slot = conference.lock().await;
                    let Some(c) = slot.as_mut() else { continue };
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
        }
        // **自分の口だけを外す。**ほかの相手との経路は生きている（M6）
        outbound.lock().await.remove(&peer.to_bytes());
        let _ = app.emit(EVENT_CLOSED, key_to_string(peer));
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
    let meeting = {
        let slot = bridge.conference.lock().await;
        slot.as_ref().map(Conference::id)
    };
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

/// 紹介を配る（**D41**）。**主催者だけが呼ぶ。**
///
/// 既存の面々へ「入った人の住所」を、入った人へ「既存の面々の住所」を送る。
/// **住所を知らない相手は飛ばす** — まだ名乗っていないだけなので、断りではない。
async fn 紹介を配る(
    conference: &Arc<Mutex<Option<Conference>>>,
    outbound: &Arc<Mutex<HashMap<[u8; 32], mpsc::Sender<Notice>>>>,
    addresses: &Arc<Mutex<HashMap<[u8; 32], String>>>,
    me: PublicKey,
    newcomer: PublicKey,
    meeting: warifu_meeting::MeetingId,
) {
    let 配り先 = {
        let slot = conference.lock().await;
        let Some(c) = slot.as_ref() else { return };
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
/// **残さない。**送ったものも届いたものも、閉じれば消える。
/// 保存するには身元が続く必要があり、それは **D2 が未決**である（`issues/010`）。
#[tauri::command]
async fn send_text(bridge: State<'_, Bridge>, body: String) -> Answer<()> {
    let meeting = {
        let slot = bridge.conference.lock().await;
        slot.as_ref().map(Conference::id)
    };
    let Some(meeting) = meeting else {
        return Err(Failure {
            message: "まだ会議がありません".into(),
            code: None,
        });
    };
    let out = bridge.outbound.lock().await;
    if out.is_empty() {
        return Err(Failure {
            message: "まだ誰も居ません".into(),
            code: None,
        });
    }
    記録!("送信: 文字（{} バイト）を {} 人へ", body.len(), out.len());
    for tx in out.values() {
        // 届かない相手が居ても止めない。**送る側を待たせない**
        let _ = tx
            .send(Notice::Text {
                meeting,
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
    let meeting = {
        let slot = bridge.conference.lock().await;
        slot.as_ref().map(Conference::id)
    };
    let Some(meeting) = meeting else {
        // まだ会議が無い。**断りではない**ので黙って戻る
        return Ok(());
    };
    let out = bridge.outbound.lock().await;
    for tx in out.values() {
        // 届かない相手が居ても止めない。**抜ける側を待たせない**
        let _ = tx.send(Notice::Leave { meeting }).await;
    }
    Ok(())
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
    let slot = bridge.conference.lock().await;
    let Some(conference) = slot.as_ref() else {
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
        .setup(|app| {
            // 最初に呼んで、起点をここに固定する
            起動からの秒();
            記録!("起動しました。ここから経路の要所を書き出します（+秒 は起動からの経過）");
            app.manage(Bridge::new());
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
            set_menu_locale,
        ])
        .run(tauri::generate_context!())
        .expect("warifu の窓を開けませんでした");
}
