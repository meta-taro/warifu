//! **3 台を 1 プロセスの中で本物の経路（warifu-net）で繋ぎ、星形の部屋が通ることを確かめる**（計画 1-C・0.1.41）。
//!
//! `two_peers.rs` は 2 台で「届くか」を見る。ここは **3 台で「名簿がそろい、文字が主催ごしに配られ、
//! 1 台が落ちたら残りの名簿が揃う」**までを見る。**規則は [`部屋たち`] のもの**で、
//! 試験が書くのは皮（線を持つ・送る・届いた文字を控える）だけ。机上の網（`room_sim.rs`）と同じ本物を、
//! 今度は本物の経路の上で動かす。
//!
//! 実機もカメラも要らない。**ここで通らないものは、実機でも通らない。**
//! 戸口と割符の握手は warifu-door / 画面の担当なので、ここでは繋がった相手をそのまま迎える。

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{Mutex, mpsc};
use warifu_app::DEFAULT_CAPACITY;
use warifu_app::room::{入り口, 手配, 部屋たち};
use warifu_core::{Device, PublicKey, Revocations, Seed};
use warifu_intent::Channel;
use warifu_meeting::{MeetingId, Notice, Roster};
use warifu_net::Node;

const 待つ限度: Duration = Duration::from_secs(20);

fn 端末(seed: u8, label: &str) -> Device {
    Seed::from_bytes([seed; 32])
        .profile("Personal")
        .device(label)
}

async fn 時間を切る<T>(f: impl std::future::Future<Output = T>) -> T {
    tokio::time::timeout(待つ限度, f)
        .await
        .expect("時間内に終わらない")
}

/// 条件が満つまで待つ（限度あり）。
async fn 満つまで(条件: impl Fn() -> bool, 何を: &str) {
    let 始め = std::time::Instant::now();
    while !条件() {
        assert!(始め.elapsed() < 待つ限度, "時間内にならない: {何を}");
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

/// 1 台ぶんの皮。**部屋の状態は本物（`部屋たち`）。**
#[derive(Clone)]
struct 席 {
    鍵: PublicKey,
    部屋: Arc<Mutex<部屋たち>>,
    /// 相手ごとの送り口。線を持つ輪が置き、閉じたら外す。
    口たち: Arc<Mutex<HashMap<PublicKey, mpsc::Sender<Notice>>>>,
    届いた文字: Arc<Mutex<Vec<(PublicKey, String)>>>,
}

impl 席 {
    fn new(device: &Device) -> Self {
        let 鍵 = device.public_key();
        Self {
            鍵,
            部屋: Arc::new(Mutex::new(部屋たち::new(鍵))),
            口たち: Arc::new(Mutex::new(HashMap::new())),
            届いた文字: Arc::new(Mutex::new(Vec::new())),
        }
    }

    async fn 名簿(&self, 部屋: MeetingId) -> Vec<PublicKey> {
        self.部屋
            .lock()
            .await
            .棚()
            .get(&部屋)
            .map(|c| c.members().to_vec())
            .unwrap_or_default()
    }

    fn 名簿の長さ(&self, 部屋: MeetingId) -> usize {
        self.部屋
            .try_lock()
            .map(|m| m.棚().get(&部屋).map_or(0, |c| c.members().len()))
            .unwrap_or(0)
    }

    fn 文字の数(&self) -> usize {
        self.届いた文字.try_lock().map(|v| v.len()).unwrap_or(0)
    }

    /// 主催が配る名簿（画面の `call::招く` の役）。
    async fn 招待(&self, 部屋: MeetingId) -> Notice {
        let 棚 = self.部屋.lock().await;
        let c = &棚.棚()[&部屋];
        let mut r = Roster::with_capacity(self.鍵, c.capacity()).unwrap();
        for m in c.members() {
            if *m != self.鍵 {
                r.add(*m).unwrap();
            }
        }
        Notice::Invite {
            meeting: 部屋,
            roster: r,
        }
    }

    /// 手配を果たす。**送るのは相手の口へ、文字は控えへ。**
    async fn 果たす(&self, 手配たち: Vec<手配>) {
        for h in 手配たち {
            match h {
                手配::送る { 宛先, 知らせ } => {
                    let 口 = self.口たち.lock().await.get(&宛先).cloned();
                    if let Some(口) = 口 {
                        let _ = 口.send(知らせ).await;
                    }
                }
                手配::文字が届いた {
                    差出人, 本文, ..
                } => {
                    self.届いた文字.lock().await.push((差出人, 本文));
                }
                _ => {}
            }
        }
    }

    /// 線を持つ。**画面の `汲む` の役** —— 送るものは口から、届いたものは `受ける` へ。
    ///
    /// 知らない部屋の名簿（招待）だけは皮が `入った` にする（画面の `招待を待つ`）。
    fn 線を持つ(&self, mut channel: Channel) -> tokio::task::JoinHandle<()> {
        let 相手 = channel.peer();
        let (tx, mut rx) = mpsc::channel::<Notice>(32);
        let 席 = self.clone();
        tokio::spawn(async move {
            席.口たち.lock().await.insert(相手, tx);
            let 手配 = 席.部屋.lock().await.受ける(入り口::線がついた { 相手 });
            席.果たす(手配).await;
            loop {
                tokio::select! {
                    出す = rx.recv() => {
                        // 口が捨てられた＝この席が線を閉じた。**挨拶して閉じる**（相手は「落ちた」ではなく「帰った」と知る）
                        let Some(notice) = 出す else {
                            let _ = channel.finish().await;
                            break;
                        };
                        let Ok(intent) = notice.to_intent() else { continue };
                        if channel.send(&intent).await.is_err() { break; }
                    }
                    来た = channel.recv() => {
                        let Ok(intent) = 来た else { break };
                        let Ok(notice) = Notice::from_intent(&intent) else { continue };
                        let 手配 = {
                            let mut 部屋 = 席.部屋.lock().await;
                            match &notice {
                                Notice::Invite { meeting, roster } if !部屋.棚().contains_key(meeting) => {
                                    部屋.受ける(入り口::入った {
                                        部屋: *meeting,
                                        主催: 相手,
                                        名簿: roster.members().to_vec(),
                                        定員: roster.capacity(),
                                        見る: true,
                                    })
                                }
                                _ => 部屋.受ける(入り口::届いた { 線の相手: 相手, 知らせ: notice }),
                            }
                        };
                        席.果たす(手配).await;
                    }
                }
            }
            席.口たち.lock().await.remove(&相手);
            let 手配 = 席.部屋.lock().await.受ける(入り口::線が切れた { 相手 });
            席.果たす(手配).await;
        })
    }

    async fn 言う(&self, 部屋: MeetingId, 本文: &str) {
        let 手配 = self.部屋.lock().await.受ける(入り口::言う {
            部屋,
            本文: 本文.to_owned(),
            話し手: None,
        });
        self.果たす(手配).await;
    }
}

/// 主催が建て、客が繋いで入る。客どうしは繋がない（星形）。
struct 星形 {
    主催: 席,
    客a: 席,
    客b: 席,
    部屋: MeetingId,
    /// 客 B の結び目と線。落とすときに捨てる
    bの結び目: Option<(Node, tokio::task::JoinHandle<()>)>,
    _aの結び目: (Node, tokio::task::JoinHandle<()>),
    _主催の待受: tokio::task::JoinHandle<()>,
}

async fn 星形を建てる() -> 星形 {
    let (主催の端末, aの端末, bの端末) = (端末(1, "主催"), 端末(2, "A"), 端末(3, "B"));
    let 主催 = 席::new(&主催の端末);
    let 客a = 席::new(&aの端末);
    let 客b = 席::new(&bの端末);
    let 部屋 = {
        let mut m = 主催.部屋.lock().await;
        m.受ける(入り口::建てた {
            id: None,
            定員: DEFAULT_CAPACITY,
        });
        m.見ている().unwrap()
    };

    let 主催の結び目 = Arc::new(Node::bind_without_relay(&主催の端末).await.unwrap());
    let 宛先 = 主催の結び目.address().await.unwrap();
    // **待受：繋がった相手に線を持たせ、名簿を渡して招く**（画面の `listen` の役・戸口は省く）
    let 主催の待受 = {
        let 主催 = 主催.clone();
        let 結び目 = Arc::clone(&主催の結び目);
        tokio::spawn(async move {
            loop {
                let Ok(session) = 結び目.accept(&Revocations::new()).await else {
                    break;
                };
                let 相手 = session.peer();
                主催.線を持つ(Channel::new(session));
                // 送り口が置かれるのを待ってから招く
                for _ in 0..100 {
                    if 主催.口たち.lock().await.contains_key(&相手) {
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
                let 招待 = 主催.招待(部屋).await;
                主催
                    .果たす(vec![手配::送る {
                        宛先: 相手,
                        知らせ: 招待,
                    }])
                    .await;
            }
        })
    };

    let aの結び目 = Node::bind_without_relay(&aの端末).await.unwrap();
    let a線 = 時間を切る(aの結び目.connect(&宛先, &Revocations::new()))
        .await
        .expect("A が繋がらない");
    let a輪 = 客a.線を持つ(Channel::new(a線));
    満つまで(
        || 主催.名簿の長さ(部屋) == 2 && 客a.名簿の長さ(部屋) == 2,
        "A が入る",
    )
    .await;

    let bの結び目 = Node::bind_without_relay(&bの端末).await.unwrap();
    let b線 = 時間を切る(bの結び目.connect(&宛先, &Revocations::new()))
        .await
        .expect("B が繋がらない");
    let b輪 = 客b.線を持つ(Channel::new(b線));
    満つまで(
        || 主催.名簿の長さ(部屋) == 3 && 客a.名簿の長さ(部屋) == 3 && 客b.名簿の長さ(部屋) == 3,
        "3 台の名簿がそろう",
    )
    .await;

    星形 {
        主催,
        客a,
        客b,
        部屋,
        bの結び目: Some((bの結び目, b輪)),
        _aの結び目: (aの結び目, a輪),
        _主催の待受: 主催の待受,
    }
}

#[tokio::test]
async fn 三台の名簿がそろい_文字は主催ごしに全員へ_1_回ずつ届く() {
    let 網 = 星形を建てる().await;
    let 部屋 = 網.部屋;
    let (h, a, b) = (
        網.主催.名簿(部屋).await,
        網.客a.名簿(部屋).await,
        網.客b.名簿(部屋).await,
    );
    let そろえる = |mut v: Vec<PublicKey>| {
        v.sort_by_key(|k| k.to_bytes());
        v
    };
    assert_eq!(そろえる(h.clone()), そろえる(a), "A の名簿は主催と同じ");
    assert_eq!(そろえる(h), そろえる(b), "B の名簿は主催と同じ");

    網.客a.言う(部屋, "やあ").await;
    満つまで(
        || 網.主催.文字の数() == 1 && 網.客b.文字の数() == 1,
        "A の文字が主催と B へ",
    )
    .await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(網.客a.文字の数(), 0, "自分には届かない");
    assert_eq!(網.客b.文字の数(), 1, "B には 1 回だけ");
    let 届いた = 網.客b.届いた文字.lock().await.clone();
    assert_eq!(
        届いた[0],
        (網.客a.鍵, "やあ".to_owned()),
        "差出人は A のまま（主催が書き換えない）"
    );

    網.主催.言う(部屋, "主催から").await;
    満つまで(
        || 網.客a.文字の数() == 1 && 網.客b.文字の数() == 2,
        "主催の文字が両方へ",
    )
    .await;
}

#[tokio::test]
async fn 客が落ちたら_主催は外して名簿を配り直し_残りの客の名簿も揃う() {
    let mut 網 = 星形を建てる().await;
    let 部屋 = 網.部屋;
    // B が線を閉じる（口を捨てると、線を持つ輪が挨拶して閉じる）。結び目はそのあと捨てる
    網.客b.口たち.lock().await.clear();
    let (b結び目, b輪) = 網.bの結び目.take().unwrap();
    let _ = 時間を切る(b輪).await;
    drop(b結び目);
    満つまで(|| 網.主催.名簿の長さ(部屋) == 2, "主催が B を外す").await;
    満つまで(|| 網.客a.名簿の長さ(部屋) == 2, "A の名簿も主催に揃う").await;
    let 主催の名簿 = 網.主催.名簿(部屋).await;
    assert!(!主催の名簿.contains(&網.客b.鍵));
    assert!(
        網.客a.名簿(部屋).await.contains(&網.主催.鍵),
        "A は主催を外さない"
    );
    // 落ちたあとの文字も、残りへ届く
    網.客a.言う(部屋, "まだ居る").await;
    満つまで(|| 網.主催.文字の数() == 1, "主催へ届く").await;
}
