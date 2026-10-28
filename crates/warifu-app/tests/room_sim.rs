//! **机上の網 —— 本物の [`部屋たち`] を動かす**（計画 1-B・0.1.41）。
//!
//! 画面側（`apps/desktop/src-tauri/src/room_sim.rs`）の網は、受信の手順を**手で写して**いた。
//! 写しがずれても緑のままで、0.1.39 の名簿の直しが受け取る側で死んでいたのに気づけなかった。
//!
//! ここでは**皮だけ**を書く。線（誰と誰がつながっているか）・郵便（線を流れる知らせ）・戸口（誰を通すか）・
//! 帰り道。**規則は全部 `部屋たち::受ける` のもの**で、ここには写さない。
//!
//! 手で書くのは、`部屋たち` の外にある 3 つだけ：
//! - **知らない部屋の `Invite` を受けたら `入った` を起こす**（画面の `招待を待つ` の役）
//! - **主催がどの部屋へ招くか**（[`warifu_app::room::招く先を決める`]・[`warifu_app::room::招く部屋を選ぶ`]。
//!   画面も同じ関数を使う）
//! - **帰り道で戻るか**（主催と線が無い・主催が名簿に居ない）

use std::collections::{HashSet, VecDeque};

use warifu_app::DEFAULT_CAPACITY;
use warifu_app::room::{
    入り口, 手配, 招く先, 招く先を決める, 招く部屋を選ぶ, 捨てた訳, 部屋たち
};
use warifu_core::{PublicKey, Seed};
use warifu_meeting::{MeetingId, Notice, Roster};

fn 鍵(n: u8) -> PublicKey {
    Seed::from_bytes([n; 32])
        .profile("Personal")
        .device("PC")
        .public_key()
}

/// 1 台の機械。**部屋の状態は本物（`部屋たち`）。**それ以外は皮が持つもの。
struct 機械 {
    起きている: bool,
    鍵: PublicKey,
    部屋: 部屋たち,
    /// 自分が建てた部屋の id（`room.tsv`。上がり直しても同じ id で建てる）。
    自分の部屋: Option<MeetingId>,
    /// 帰り道（`rejoin.tsv`）：部屋と、その主催。
    帰り道: Vec<(MeetingId, PublicKey)>,
    /// 戸口が通す相手。
    戸口: HashSet<PublicKey>,
    届いた文字: Vec<(PublicKey, String)>,
    捨てた: Vec<(&'static str, 捨てた訳)>,
    食い違い: Vec<String>,
    /// `届いた` を受けたのに手配が空だった回数（**黙って捨てた**）。
    黙って捨てた: usize,
}

impl 機械 {
    fn new(n: u8) -> Self {
        let 鍵 = 鍵(n);
        Self {
            起きている: false,
            鍵,
            部屋: 部屋たち::new(鍵),
            自分の部屋: None,
            帰り道: Vec::new(),
            戸口: HashSet::new(),
            届いた文字: Vec::new(),
            捨てた: Vec::new(),
            食い違い: Vec::new(),
            黙って捨てた: 0,
        }
    }

    fn 主催している部屋(&self) -> Vec<MeetingId> {
        let mut 並び: Vec<MeetingId> = self
            .部屋
            .棚()
            .keys()
            .copied()
            .filter(|id| self.部屋.主催か(*id))
            .collect();
        並び.sort_by_key(ToString::to_string);
        並び
    }

    /// その部屋の名簿（自分を含む）。
    fn 名簿(&self, 部屋: MeetingId) -> Vec<PublicKey> {
        self.部屋
            .棚()
            .get(&部屋)
            .map(|c| c.members().to_vec())
            .unwrap_or_default()
    }

    /// 主催が配る名簿（`call::招く` の役）。
    fn 招待(&self, 部屋: MeetingId) -> Option<Notice> {
        let c = self.部屋.棚().get(&部屋)?;
        let mut r = Roster::with_capacity(self.鍵, c.capacity()).ok()?;
        for m in c.members() {
            if *m != self.鍵 {
                r.add(*m).ok()?;
            }
        }
        Some(Notice::Invite {
            meeting: 部屋,
            roster: r,
        })
    }
}

/// 機械たちと、その間の線・郵便。
struct 網 {
    機械たち: Vec<機械>,
    線: HashSet<(usize, usize)>,
    郵便: VecDeque<(usize, usize, Notice)>,
    /// 線が無い・相手が落ちていて届けられなかったもの。
    落とした: Vec<(usize, usize, Notice)>,
}

fn 組(a: usize, b: usize) -> (usize, usize) {
    if a < b { (a, b) } else { (b, a) }
}

impl 網 {
    fn new(台数: u8) -> Self {
        Self {
            機械たち: (1..=台数).map(機械::new).collect(),
            線: HashSet::new(),
            郵便: VecDeque::new(),
            落とした: Vec::new(),
        }
    }

    fn 繋がっている(&self, a: usize, b: usize) -> bool {
        self.線.contains(&組(a, b))
    }

    fn 番号(&self, 鍵: PublicKey) -> Option<usize> {
        self.機械たち.iter().position(|m| m.鍵 == 鍵)
    }

    /// 立ち上げる。**部屋の状態は空から**（上がり直しは前の状態を持たない）。自分の部屋は同じ id で建てる
    fn 起動(&mut self, i: usize) {
        let m = &mut self.機械たち[i];
        m.起きている = true;
        m.部屋 = 部屋たち::new(m.鍵);
        let 手配 = m.部屋.受ける(入り口::建てた {
            id: m.自分の部屋,
            定員: DEFAULT_CAPACITY,
        });
        m.自分の部屋 = m.部屋.見ている();
        self.果たす(i, 手配);
    }

    /// 落とす。`気づく` なら、線の相手は切れたことに気づく（気づかない形は 0.1.37 の取り残し）
    fn 落とす(&mut self, i: usize, 気づく: bool) {
        self.機械たち[i].起きている = false;
        let 鍵 = self.機械たち[i].鍵;
        let 相手たち: Vec<usize> = (0..self.機械たち.len())
            .filter(|j| *j != i && self.繋がっている(i, *j))
            .collect();
        for j in 相手たち {
            self.線.remove(&組(i, j));
            if 気づく {
                self.線が切れた(j, 鍵);
            }
        }
    }

    fn 線が切れた(&mut self, j: usize, 相手: PublicKey) {
        let 手配 = self.機械たち[j].部屋.受ける(入り口::線が切れた { 相手 });
        self.果たす(j, 手配);
    }

    /// 客が主催を割符なしで叩く。`望み` は戻りたい部屋（帰り道のとき）。
    ///
    /// 戸口が通せば線がつき、主催が部屋を決めて招く。**招かれた部屋を返す**（「もう無い」なら `None`）。
    fn 叩く(
        &mut self,
        客: usize,
        主催: usize,
        望み: Option<MeetingId>,
    ) -> Result<Option<MeetingId>, String> {
        if !self.機械たち[主催].起きている {
            return Err("主催が落ちている".into());
        }
        let (客の鍵, 主催の鍵) = (self.機械たち[客].鍵, self.機械たち[主催].鍵);
        if !self.機械たち[主催].戸口.contains(&客の鍵) {
            return Err("戸口が通さない".into());
        }
        self.線.insert(組(客, 主催));
        let 手配 = self.機械たち[主催]
            .部屋
            .受ける(入り口::線がついた { 相手: 客の鍵 });
        self.果たす(主催, 手配);
        let 手配 = self.機械たち[客].部屋.受ける(入り口::線がついた {
            相手: 主催の鍵
        });
        self.果たす(客, 手配);
        // **主催がどの部屋へ招くか。**画面と同じ関数で決める
        let 部屋 = match 招く先を決める(望み, &self.機械たち[主催].主催している部屋())
        {
            招く先::その部屋(id) => id,
            招く先::もう無い(id) => {
                self.郵便
                    .push_back((主催, 客, Notice::Leave { meeting: id }));
                return Ok(None);
            }
            招く先::選ぶ => {
                let m = &self.機械たち[主催];
                match 招く部屋を選ぶ(m.部屋.見ている(), &m.主催している部屋())
                {
                    Some(id) => id,
                    None => {
                        let 手配 = self.機械たち[主催].部屋.受ける(入り口::建てた {
                            id: None,
                            定員: DEFAULT_CAPACITY,
                        });
                        self.果たす(主催, 手配);
                        *self.機械たち[主催]
                            .主催している部屋()
                            .last()
                            .expect("建てた")
                    }
                }
            }
        };
        // **招待は、叩いた線でそのまま客へ渡る**（画面の `招待を待つ`）。客は部屋を組み直して Join を返し、
        // 帰り道を書き置く（鍵なしで入った部屋にも帰り道がある・0.1.35）
        let 招待 = self.機械たち[主催].招待(部屋).ok_or("満員")?;
        let Notice::Invite { meeting, roster } = 招待 else {
            unreachable!("招待は Invite")
        };
        let 手配 = self.機械たち[客].部屋.受ける(入り口::入った {
            部屋: meeting,
            主催: 主催の鍵,
            名簿: roster.members().to_vec(),
            定員: roster.capacity(),
            見る: false,
        });
        self.果たす(客, 手配);
        if !self.機械たち[客]
            .帰り道
            .iter()
            .any(|(id, _)| *id == meeting)
        {
            self.機械たち[客].帰り道.push((meeting, 主催の鍵));
        }
        Ok(Some(部屋))
    }

    /// 手配を果たす。**送るのは郵便へ、文字は控えへ、戸口は戸口へ。**
    fn 果たす(&mut self, i: usize, 手配たち: Vec<手配>) {
        for h in 手配たち {
            match h {
                手配::送る { 宛先, 知らせ } => match self.番号(宛先) {
                    Some(j) if self.繋がっている(i, j) && self.機械たち[j].起きている =>
                    {
                        self.郵便.push_back((i, j, 知らせ));
                    }
                    Some(j) => self.落とした.push((i, j, 知らせ)),
                    None => self.落とした.push((i, usize::MAX, 知らせ)),
                },
                手配::文字が届いた {
                    差出人, 本文, ..
                } => {
                    self.機械たち[i].届いた文字.push((差出人, 本文));
                }
                手配::戸口で迎える { 相手 } => {
                    self.機械たち[i].戸口.insert(相手);
                }
                手配::食い違い { 文 } => self.機械たち[i].食い違い.push(文),
                手配::捨てた { 知らせ, 訳 } => self.機械たち[i].捨てた.push((知らせ, 訳)),
                手配::画面へ { .. }
                | 手配::画面へ入り直し { .. }
                | 手配::画像が届いた { .. }
                | 手配::閉じた { .. }
                | 手配::皮に任せる { .. } => {}
            }
        }
    }

    /// 郵便を 1 通届ける。線が無い・相手が落ちていれば落とす。
    ///
    /// 届いたものは**そのまま `受ける`** —— 皮は読み替えない（知らない部屋の名簿は本物が捨てる）。
    fn 一通(&mut self) -> bool {
        let Some((from, to, 知らせ)) = self.郵便.pop_front() else {
            return false;
        };
        if !self.繋がっている(from, to) || !self.機械たち[to].起きている {
            self.落とした.push((from, to, 知らせ));
            return true;
        }
        let 線の相手 = self.機械たち[from].鍵;
        // 主催が配った名簿が手元と同じなら、手配は空でよい（動かなかったことも答え）
        let 名簿だった = matches!(知らせ, Notice::Invite { .. });
        let 手配 = self.機械たち[to].部屋.受ける(入り口::届いた {
            線の相手, 知らせ
        });
        if 手配.is_empty() && !名簿だった {
            self.機械たち[to].黙って捨てた += 1;
        }
        self.果たす(to, 手配);
        true
    }

    /// 郵便が空になるまで届ける。
    fn 流す(&mut self) {
        while self.一通() {}
    }

    /// 見張り（`come_back`）：帰り道の部屋で主催と繋がっていなければ、戻りたい部屋を添えて叩く。
    fn 見張り(&mut self, i: usize) {
        if !self.機械たち[i].起きている {
            return;
        }
        let 帰り道 = self.機械たち[i].帰り道.clone();
        for (部屋, 主催) in 帰り道 {
            let m = &self.機械たち[i];
            let 戻るべき = match m.部屋.棚().get(&部屋) {
                None => true,
                Some(c) => {
                    !m.部屋.主催か(部屋)
                        && (!m.部屋.線があるか(主催) || !c.members().contains(&主催))
                }
            };
            if !戻るべき {
                continue;
            }
            let Some(j) = self.番号(主催) else {
                continue;
            };
            match self.叩く(i, j, Some(部屋)) {
                // 主催が「もう無い」と言った。**1 回で帰り道を消し、部屋も畳む**
                Ok(None) => {
                    self.流す();
                    self.機械たち[i].帰り道.retain(|(id, _)| *id != 部屋);
                    if self.機械たち[i].部屋.棚().contains_key(&部屋) {
                        let 手配 = self.機械たち[i].部屋.受ける(入り口::畳む { 部屋 });
                        self.果たす(i, 手配);
                    }
                }
                Ok(Some(_)) => self.流す(),
                Err(_) => {}
            }
        }
    }

    /// 全員の見張りを回して、郵便が空になるまで。
    fn 落ち着くまで(&mut self, 周: usize) {
        for _ in 0..周 {
            for i in 0..self.機械たち.len() {
                self.見張り(i);
            }
            self.流す();
        }
    }

    /// 言う。主催ごしに全員へ届く。
    fn 言う(&mut self, i: usize, 部屋: MeetingId, 本文: &str) {
        let 手配 = self.機械たち[i].部屋.受ける(入り口::言う {
            部屋,
            本文: 本文.to_owned(),
            話し手: None,
        });
        self.果たす(i, 手配);
        self.流す();
    }

    /// **食い違い**：起きている機械どうしで、同じ部屋の名簿が主催の名簿と同じか。
    /// 黙って捨てた・食い違いの行・落とした郵便も数える。
    fn 食い違い(&self) -> Vec<String> {
        let mut 出 = Vec::new();
        for (i, m) in self.機械たち.iter().enumerate() {
            if !m.起きている {
                continue;
            }
            for (id, c) in m.部屋.棚() {
                let Some(主催) = m.部屋.主催(*id) else {
                    continue;
                };
                let Some(h) = self.番号(主催) else {
                    continue;
                };
                let 主催側 = &self.機械たち[h];
                if !主催側.起きている {
                    continue;
                }
                let 主催の名簿: HashSet<PublicKey> = 主催側.名簿(*id).into_iter().collect();
                let 自分の名簿: HashSet<PublicKey> = c.members().iter().copied().collect();
                if 主催の名簿 != 自分の名簿 {
                    出.push(format!(
                        "機械 {i} の部屋 {} の名簿（{} 人）が主催（機械 {h}・{} 人）と違う",
                        頭(*id),
                        自分の名簿.len(),
                        主催の名簿.len()
                    ));
                }
            }
            if m.黙って捨てた > 0 {
                出.push(format!(
                    "機械 {i} が届いたものを {} 回黙って捨てた",
                    m.黙って捨てた
                ));
            }
            for 文 in &m.食い違い {
                出.push(format!("機械 {i}: {文}"));
            }
        }
        for (from, to, 知らせ) in &self.落とした {
            出.push(format!("{from} → {to} の {} を落とした", 名(知らせ)));
        }
        出
    }

    fn 部屋の数(&self) -> usize {
        self.機械たち
            .iter()
            .filter(|m| m.起きている)
            .map(|m| m.部屋.棚().len())
            .sum()
    }
}

fn 頭(id: MeetingId) -> String {
    id.to_string().chars().take(8).collect()
}

fn 名(n: &Notice) -> &'static str {
    match n {
        Notice::Invite { .. } => "名簿",
        Notice::Join { .. } => "参加",
        Notice::Leave { .. } => "退出",
        Notice::Text { .. } => "文字",
        _ => "ほか",
    }
}

/// 主催（0）と、戸口どうしで通し合う客たち。客は主催の部屋に入っている。客どうしに線は無い。
fn 主催と客(客の数: u8) -> 網 {
    let mut 網 = 網::new(客の数 + 1);
    for i in 0..網.機械たち.len() {
        網.起動(i);
    }
    let 主催の鍵 = 網.機械たち[0].鍵;
    for g in 1..網.機械たち.len() {
        let 客の鍵 = 網.機械たち[g].鍵;
        網.機械たち[0].戸口.insert(客の鍵);
        網.機械たち[g].戸口.insert(主催の鍵);
        網.叩く(g, 0, None).unwrap().expect("招かれる");
        網.流す();
    }
    網
}

#[test]
fn 入ったばかりなら_名簿は全員そろい_何も捨てていない() {
    let 網 = 主催と客(2);
    assert!(!網.繋がっている(1, 2), "客どうしは直接つながっていない前提");
    assert_eq!(網.食い違い(), Vec::<String>::new());
    let 部屋 = 網.機械たち[0].自分の部屋.unwrap();
    assert_eq!(網.機械たち[0].名簿(部屋).len(), 3);
}

#[test]
fn 文字は主催ごしに_ほかの全員へ_1_回ずつ届き_自分には届かない() {
    let mut 網 = 主催と客(2);
    let 部屋 = 網.機械たち[0].自分の部屋.unwrap();
    網.言う(1, 部屋, "やあ");
    assert_eq!(網.機械たち[0].届いた文字.len(), 1, "主催に 1 回");
    assert_eq!(
        網.機械たち[2].届いた文字.len(),
        1,
        "もう 1 人の客に 1 回（主催ごし）"
    );
    assert_eq!(網.機械たち[1].届いた文字.len(), 0, "自分には届かない");
    網.言う(0, 部屋, "主催から");
    assert_eq!(網.機械たち[1].届いた文字.len(), 1);
    assert_eq!(網.機械たち[2].届いた文字.len(), 2);
    assert_eq!(網.食い違い(), Vec::<String>::new());
}

#[test]
fn 客が落ちたら_主催は外して名簿を配り直し_残りの客の名簿もそろう() {
    let mut 網 = 主催と客(2);
    let 部屋 = 網.機械たち[0].自分の部屋.unwrap();
    網.落とす(2, true);
    網.流す();
    assert_eq!(網.機械たち[0].名簿(部屋).len(), 2, "主催は外す");
    assert_eq!(網.機械たち[1].名簿(部屋).len(), 2, "残りの客も主催と同じ");
    assert_eq!(網.食い違い(), Vec::<String>::new());
}

#[test]
fn 主催が上がり直しても_客は帰り道で戻り_部屋は増えない() {
    let mut 網 = 主催と客(2);
    let 前の数 = 網.部屋の数();
    網.落とす(0, true);
    網.起動(0);
    網.落ち着くまで(3);
    assert_eq!(網.食い違い(), Vec::<String>::new());
    assert_eq!(網.部屋の数(), 前の数, "部屋が増えも減りもしない");
    let 部屋 = 網.機械たち[0].自分の部屋.unwrap();
    assert_eq!(網.機械たち[0].名簿(部屋).len(), 3, "全員戻っている");
}

#[test]
fn 主催が上がり直したあとの文字も_全員へ届く() {
    let mut 網 = 主催と客(2);
    網.落とす(0, true);
    網.起動(0);
    網.落ち着くまで(3);
    let 部屋 = 網.機械たち[0].自分の部屋.unwrap();
    網.言う(1, 部屋, "戻ったよ");
    assert_eq!(網.機械たち[0].届いた文字.len(), 1);
    assert_eq!(網.機械たち[2].届いた文字.len(), 1);
    assert_eq!(網.食い違い(), Vec::<String>::new());
}

#[test]
fn 戻るときは戻りたい部屋を添えるので_主催が別の部屋を見ていてもその部屋へ招かれる() {
    // 10-27、主催が別の用事で見ている部屋へ招かれ、客は抜けては叩き直していた
    let mut 網 = 主催と客(1);
    let 部屋 = 網.機械たち[0].自分の部屋.unwrap();
    // 主催（0）はもう 1 つ部屋を建てて、そちらを見ている
    let 手配 = 網.機械たち[0].部屋.受ける(入り口::建てた {
        id: None,
        定員: DEFAULT_CAPACITY,
    });
    網.果たす(0, 手配);
    let 別の部屋 = *網.機械たち[0]
        .主催している部屋()
        .iter()
        .find(|id| **id != 部屋)
        .unwrap();
    網.機械たち[0].部屋.受ける(入り口::見る {
        部屋: 別の部屋
    });
    assert_eq!(網.機械たち[0].部屋.見ている(), Some(別の部屋));
    // 客（1）が上がり直す（帰り道だけ残る）。見張りが戻りたい部屋を添えて叩く
    網.落とす(1, true);
    網.起動(1);
    網.落ち着くまで(2);
    assert!(
        網.機械たち[1].部屋.棚().contains_key(&部屋),
        "戻りたい部屋に戻っている"
    );
    assert!(
        !網.機械たち[1].部屋.棚().contains_key(&別の部屋),
        "主催が見ている部屋へは招かれない"
    );
    assert_eq!(網.機械たち[1].帰り道.len(), 1, "帰り道は増えも減りもしない");
    assert_eq!(網.食い違い(), Vec::<String>::new());
    // 対照：部屋を添えずに叩くと（古い戻り方）、主催が見ている部屋へ招かれる
    let 招かれた = 網.叩く(1, 0, None).unwrap().unwrap();
    assert_eq!(招かれた, 別の部屋);
}

#[test]
fn 主催がもう主催していない部屋へ戻ろうとしたら_1_回で帰り道が消え_部屋は増えない() {
    // 0.1.41：主催は「もう無い」と返して建てない。客は 3 回叩かずに見切る
    let mut 網 = 主催と客(1);
    let 幽霊 = 網.機械たち[0].自分の部屋.unwrap();
    // 主催が部屋を畳み、覚えも消して上がり直す（room.tsv が無い形）
    網.落とす(0, true);
    網.機械たち[0].自分の部屋 = None;
    網.起動(0);
    網.落ち着くまで(2);
    assert!(
        !網.機械たち[1].帰り道.iter().any(|(id, _)| *id == 幽霊),
        "帰り道は 1 回で消える"
    );
    assert!(
        !網.機械たち[1].部屋.棚().contains_key(&幽霊),
        "幽霊の部屋は畳まれる"
    );
    assert_eq!(
        網.機械たち[0].部屋.棚().len(),
        1,
        "主催に一時の部屋が増えない"
    );
    assert_eq!(網.機械たち[1].部屋.棚().len(), 1, "客は自分の部屋だけ");
    assert_eq!(網.食い違い(), Vec::<String>::new());
}

#[test]
fn 客として他人の部屋を見ていても_呼ばれたら自分の部屋へ招く() {
    // 0.1.35 / 0.1.37。見ている部屋へ招くと、その部屋の主催の知らない所で部屋が割れた
    let mut 網 = 主催と客(1);
    let 主催の部屋 = 網.機械たち[0].自分の部屋.unwrap();
    // 客（1）は主催の部屋を見ている
    let (主催の鍵, 主催の名簿) = (網.機械たち[0].鍵, 網.機械たち[0].名簿(主催の部屋));
    網.機械たち[1].部屋.受ける(入り口::入った {
        部屋: 主催の部屋,
        主催: 主催の鍵,
        名簿: 主催の名簿,
        定員: DEFAULT_CAPACITY,
        見る: true,
    });
    assert_eq!(網.機械たち[1].部屋.見ている(), Some(主催の部屋));
    // 3 台目（2）が 1 を鍵なしで呼ぶ
    網.機械たち.push(機械::new(9));
    網.起動(2);
    let (k1, k2) = (網.機械たち[1].鍵, 網.機械たち[2].鍵);
    網.機械たち[1].戸口.insert(k2);
    網.機械たち[2].戸口.insert(k1);
    let 招かれた = 網.叩く(2, 1, None).unwrap().unwrap();
    網.流す();
    assert_ne!(招かれた, 主催の部屋, "他人の部屋へは招かない");
    assert_eq!(
        網.機械たち[1].自分の部屋,
        Some(招かれた),
        "自分の部屋へ招く"
    );
    assert_eq!(網.食い違い(), Vec::<String>::new());
}
