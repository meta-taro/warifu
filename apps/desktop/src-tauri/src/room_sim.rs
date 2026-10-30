//! **机上の網 —— 部屋の食い違いを、席の実機より先に見つける**（0.1.38）。
//!
//! 部屋のバグは、ほぼすべて「主催と客が、それぞれ別に覚えている名簿の食い違い」だった。
//! 立ち上げ直しや線の切れ方で食い違い、2〜3 台を実際につながないと出てこない。
//! 1 台の中の試験では捕まえられず、毎回、席の実機で見つかっていた（0.1.32〜0.1.37）。
//!
//! ここでは**線を使わずに、機械を何台か机の上に並べる。**名簿は本物（`warifu_app::Conference`）、
//! 戻るか・抜けるか・見切るか・居ないと告げるか・どこへ招くかの判断も本物の関数を呼ぶ。
//! 線の上の受け渡し（叩く・招く・入る・抜ける・切れる）だけを、`lib.rs` / `call.rs` /
//! `come_back.rs` の手順どおりに写してある。**写した手順を変えたら、ここも直す。**
//!
//! 見るのは 1 つだけ —— **落ち着いたあと、客が「居る」と思っている部屋に、主催の名簿でも居るか。**

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::{BTreeMap, HashMap, HashSet};

use warifu_app::Conference;
use warifu_core::PublicKey;
use warifu_meeting::{MeetingId, Notice, Roster, Signal, Step};

use crate::RoomRow;
use crate::call::{招く部屋を選ぶ, 自分を名簿に足す};
use crate::come_back::{
    居ないと告げるか, 戻るべきか, 見切るか, 頼んでいない部屋
};

const 定員: usize = warifu_app::DEFAULT_CAPACITY;

fn 鍵(n: u8) -> PublicKey {
    warifu_core::Seed::from_bytes([n; 32])
        .profile("Personal")
        .device("PC")
        .public_key()
}

/// 机の上の 1 台。**起動をまたいで残るもの**（置き場所）と、**消えるもの**（覚えているだけのもの）を分ける。
struct 機械 {
    鍵: PublicKey,
    起きている: bool,
    // ── 起動で消える ──
    棚: BTreeMap<MeetingId, Conference>,
    主催たち: HashMap<MeetingId, PublicKey>,
    見ている: Option<MeetingId>,
    /// 戻る先ごとの、主催に繋がったのに別の部屋を教えられた回数（続けて）
    違った: HashMap<MeetingId, u32>,
    // ── 置き場所（起動をまたぐ）──
    /// `room.tsv`。主催する部屋は 1 つだけ覚える
    自分の部屋: Option<MeetingId>,
    /// `rejoin.tsv`。戻る先の部屋と、その主催（住所の代わりに鍵）
    帰り道: Vec<(MeetingId, PublicKey)>,
    /// 戸口の名簿。一度通した相手は、次から鍵なしで通す（D31）
    戸口: HashSet<PublicKey>,
}

impl 機械 {
    fn new(n: u8) -> Self {
        Self {
            鍵: 鍵(n),
            起きている: false,
            棚: BTreeMap::new(),
            主催たち: HashMap::new(),
            見ている: None,
            違った: HashMap::new(),
            自分の部屋: None,
            帰り道: Vec::new(),
            戸口: HashSet::new(),
        }
    }

    /// `ルームの並び`（lib.rs）と同じ形。覚えていない部屋は自分が建てたもの
    fn 部屋の並び(&self) -> Vec<RoomRow> {
        self.棚
            .values()
            .map(|c| RoomRow {
                id: c.id().to_string(),
                members: c.members().len(),
                host: self
                    .主催たち
                    .get(&c.id())
                    .is_none_or(|主催| *主催 == self.鍵),
                host_key: crate::key_to_string(
                    self.主催たち.get(&c.id()).copied().unwrap_or(self.鍵),
                ),
                member_keys: c
                    .members()
                    .iter()
                    .map(|k| crate::key_to_string(*k))
                    .collect(),
            })
            .collect()
    }

    fn 主催している部屋(&self) -> Vec<MeetingId> {
        let mut 並び: Vec<MeetingId> = self
            .棚
            .keys()
            .filter(|id| self.主催たち.get(id).is_none_or(|主催| *主催 == self.鍵))
            .copied()
            .collect();
        並び.sort_by_key(ToString::to_string);
        並び
    }

    fn 居ると思っている(&self, 部屋: MeetingId, 相手: PublicKey) -> bool {
        self.棚
            .get(&部屋)
            .is_some_and(|c| c.members().contains(&相手))
    }
}

/// 机の上の網。線は向きを持たない（どちらかが落ちれば切れる）。
struct 網 {
    機械たち: Vec<機械>,
    線: HashSet<(usize, usize)>,
}

fn 組(a: usize, b: usize) -> (usize, usize) {
    if a < b { (a, b) } else { (b, a) }
}

impl 網 {
    fn new(台数: u8) -> Self {
        Self {
            機械たち: (1..=台数).map(機械::new).collect(),
            線: HashSet::new(),
        }
    }

    fn 繋がっている(&self, a: usize, b: usize) -> bool {
        self.線.contains(&組(a, b))
    }

    fn 番号(&self, 鍵: PublicKey) -> usize {
        self.機械たち.iter().position(|m| m.鍵 == 鍵).unwrap()
    }

    /// **起動する**（`host_meeting`）。前に主催していた部屋を同じ id で建て直す。
    fn 起動(&mut self, i: usize) {
        let m = &mut self.機械たち[i];
        m.起きている = true;
        let c = match m.自分の部屋 {
            Some(id) => Conference::host_with_id(m.鍵, 定員, id).unwrap(),
            None => Conference::host(m.鍵, 定員).unwrap(),
        };
        let id = c.id();
        m.棚.insert(id, c);
        m.主催たち.insert(id, m.鍵);
        m.見ている.get_or_insert(id);
        m.自分の部屋 = Some(id);
    }

    /// **落とす。**`気づく` が偽なら、相手は線が切れたことに気づかない ——
    /// 本番で、落ちた側が**すぐ別の部屋の用事で繋ぎ直す**と、古い線の後始末が
    /// 「入り直している」として何も触らない（lib.rs の `私のままだった`）。
    fn 落とす(&mut self, i: usize, 気づく: bool) {
        let 落ちた鍵 = self.機械たち[i].鍵;
        let m = &mut self.機械たち[i];
        m.起きている = false;
        m.棚.clear();
        m.主催たち.clear();
        m.見ている = None;
        m.違った.clear();
        let 相手たち: Vec<usize> = (0..self.機械たち.len())
            .filter(|&j| j != i && self.繋がっている(i, j))
            .collect();
        for j in 相手たち {
            self.線.remove(&組(i, j));
            if 気づく {
                self.線が切れた(j, 落ちた鍵);
            }
        }
    }

    /// **線が切れた**（lib.rs の後始末）。その相手を、すべての部屋の名簿から外す。
    fn 線が切れた(&mut self, j: usize, 相手: PublicKey) {
        for c in self.機械たち[j].棚.values_mut() {
            let ルーム = c.id();
            let _ = c.on_notice(相手, &Notice::Leave { meeting: ルーム });
        }
    }

    /// **鍵なしで叩く**（`call.rs` → 主催の戸口 → `招く部屋を用意する` → `Notice::Invite` → `Join`）。
    /// 招かれた部屋を返す。戸口が知らなければ断られる（`None`）。
    fn 叩く(&mut self, 客: usize, 主催: usize) -> Option<MeetingId> {
        if !self.機械たち[客].起きている || !self.機械たち[主催].起きている {
            return None;
        }
        let 客の鍵 = self.機械たち[客].鍵;
        let 主催の鍵 = self.機械たち[主催].鍵;
        if !self.機械たち[主催].戸口.contains(&客の鍵) {
            return None;
        }
        self.線.insert(組(客, 主催));
        // 主催：招く部屋を選ぶ。主催している部屋が無ければ建てる（見る部屋は奪わない）
        let 部屋 = {
            let h = &mut self.機械たち[主催];
            match 招く部屋を選ぶ(h.見ている, &h.主催している部屋()) {
                Some(id) => id,
                None => {
                    let c = Conference::host(h.鍵, 定員).unwrap();
                    let id = c.id();
                    h.棚.insert(id, c);
                    h.主催たち.insert(id, h.鍵);
                    h.見ている.get_or_insert(id);
                    id
                }
            }
        };
        // 主催：`call::招く` —— 自分を先頭に、ほかの面々を載せた名簿を渡す
        let 名簿 = {
            let c = &self.機械たち[主催].棚[&部屋];
            let mut r = Roster::with_capacity(c.me(), c.capacity()).unwrap();
            for k in c.members() {
                if *k != c.me() {
                    let _ = r.add(*k);
                }
            }
            r
        };
        // 客：招かれた部屋に入る。主催を覚え、帰り道を書き置く（0.1.35）
        {
            let g = &mut self.機械たち[客];
            let 名簿 = 自分を名簿に足す(名簿, g.鍵);
            g.棚.insert(部屋, Conference::joined(g.鍵, 部屋, 名簿));
            g.主催たち.insert(部屋, 主催の鍵);
            g.見ている.get_or_insert(部屋);
            if !g.帰り道.iter().any(|(id, _)| *id == 部屋) {
                g.帰り道.push((部屋, 主催の鍵));
            }
            // 主催の戸口が通したのと同じく、客の戸口も主催を覚える（相手からも呼べる）
            g.戸口.insert(主催の鍵);
        }
        // 客 → 主催：Join
        let events = self.機械たち[主催]
            .棚
            .get_mut(&部屋)
            .unwrap()
            .on_notice(客の鍵, &Notice::Join { meeting: 部屋 })
            .unwrap_or_default();
        // 主催：名簿が動いたら、ほかの客へ配る（0.1.39）
        if crate::roster_sync::名簿を配るか(true, &events) {
            self.名簿を配る(主催, 部屋, Some(客の鍵));
        }
        Some(部屋)
    }

    /// **主催がいまの名簿をゲストへ配り、ゲストが合わせる**（`roster_sync`・lib.rs の `名簿を配る`）。
    /// 線がつながっている相手にだけ届く。`除く` は入った本人（入るときに名簿を渡している）。
    fn 名簿を配る(&mut self, 主催: usize, 部屋: MeetingId, 除く: Option<PublicKey>) {
        let 主催の鍵 = self.機械たち[主催].鍵;
        let 名簿: Vec<PublicKey> = self.機械たち[主催].棚[&部屋].members().to_vec();
        for k in &名簿 {
            if *k == 主催の鍵 || Some(*k) == 除く {
                continue;
            }
            let g = self.番号(*k);
            if !self.繋がっている(主催, g) || !self.機械たち[g].起きている {
                continue;
            }
            let gm = &mut self.機械たち[g];
            // ゲスト：主催から来た名簿だけを信じる。自分が主催なら自分の名簿が正（lib.rs の Invite の受けと同じ判定）
            if gm.主催たち.get(&部屋) != Some(&主催の鍵)
                || crate::roster_sync::主催か(gm.主催たち.get(&部屋), &gm.鍵)
            {
                continue;
            }
            let Some(c) = gm.棚.get_mut(&部屋) else {
                continue;
            };
            let (足す, 外す) =
                crate::roster_sync::名簿の差(c.members(), &名簿, &gm.鍵, &主催の鍵);
            for 相手 in 足す {
                let _ = c.admit(相手);
                gm.戸口.insert(相手);
            }
            for 相手 in 外す {
                let _ = c.on_notice(相手, &Notice::Leave { meeting: 部屋 });
            }
        }
    }

    /// **部屋を畳む**（`部屋を畳む`）。その部屋の相手へ Leave を告げ、部屋と帰り道を落とす。
    fn 畳む(&mut self, i: usize, 部屋: MeetingId) {
        let 自分 = self.機械たち[i].鍵;
        let 相手たち: Vec<PublicKey> = self.機械たち[i]
            .棚
            .get(&部屋)
            .map(|c| c.members().iter().copied().filter(|k| *k != 自分).collect())
            .unwrap_or_default();
        for k in 相手たち {
            let j = self.番号(k);
            if !self.繋がっている(i, j) {
                continue;
            }
            // 主催かは覚えた主催で決める（0.1.40・lib.rs と同じ読み）。棚を触る前に読む
            let 主催か = crate::roster_sync::主催か(
                self.機械たち[j].主催たち.get(&部屋),
                &self.機械たち[j].鍵,
            );
            if let Some(c) = self.機械たち[j].棚.get_mut(&部屋) {
                let events = c
                    .on_notice(自分, &Notice::Leave { meeting: 部屋 })
                    .unwrap_or_default();
                // 主催なら、抜けたあとの名簿をほかの客へ配る（0.1.39）
                if crate::roster_sync::名簿を配るか(主催か, &events) {
                    self.名簿を配る(j, 部屋, Some(自分));
                }
            }
        }
        let m = &mut self.機械たち[i];
        m.棚.remove(&部屋);
        m.帰り道.retain(|(id, _)| *id != 部屋);
        if m.見ている == Some(部屋) {
            m.見ている = m.棚.keys().next().copied();
        }
    }

    /// **見張りを 1 回まわす**（`come_back::見張る` の 1 周。待ち時間は飛ばす）。
    fn 見張り(&mut self, i: usize) {
        if !self.機械たち[i].起きている {
            return;
        }
        let 帰り道 = self.機械たち[i].帰り道.clone();
        for (部屋, 主催の鍵) in 帰り道 {
            // 見切ったあとは帰り道ごと消えている
            if !self.戻るか(i, 部屋) || !self.機械たち[i].帰り道.iter().any(|(id, _)| *id == 部屋)
            {
                continue;
            }
            let 前: Vec<MeetingId> = self.機械たち[i].棚.keys().copied().collect();
            let 主催 = self.番号(主催の鍵);
            let 繋がった = self.叩く(i, 主催).is_some();
            let 後: Vec<MeetingId> = self.機械たち[i].棚.keys().copied().collect();
            let 戻る先 = 後.iter().copied().find(|m| *m == 部屋);
            for 余計 in 頼んでいない部屋(&前, &後, 戻る先) {
                self.畳む(i, 余計);
            }
            // 戻れたかは、本番と同じく `戻るべきか` を見直して決める（`いま戻る先`）
            if !self.戻るか(i, 部屋) {
                self.機械たち[i].違った.remove(&部屋);
                continue;
            }
            let 違った = if 繋がった {
                self.機械たち[i].違った.get(&部屋).copied().unwrap_or(0) + 1
            } else {
                0
            };
            self.機械たち[i].違った.insert(部屋, 違った);
            if 見切るか(違った) {
                self.見切る(i, 部屋);
            }
        }
    }

    /// `come_back::戻るべきか` に、その機械から見た部屋の並びと線を渡す。
    fn 戻るか(&self, i: usize, 部屋: MeetingId) -> bool {
        let 並び = self.機械たち[i].部屋の並び();
        let 繋がっている鍵: Vec<String> = (0..self.機械たち.len())
            .filter(|&j| self.繋がっている(i, j))
            .map(|j| crate::key_to_string(self.機械たち[j].鍵))
            .collect();
        戻るべきか(Some(&部屋.to_string()), &並び, |k| {
            繋がっている鍵.iter().any(|c| c == k)
        })
    }

    /// **使わない線が閉じる。**両側が気づき、互いを名簿から外す（外せる分だけ）。
    fn 線を閉じる(&mut self, a: usize, b: usize) {
        if self.線.remove(&組(a, b)) {
            let (ka, kb) = (self.機械たち[a].鍵, self.機械たち[b].鍵);
            self.線が切れた(a, kb);
            self.線が切れた(b, ka);
        }
    }

    /// `come_back::見切る`。自分のほかに誰も居なければ畳み、居れば帰り道だけ忘れる。
    fn 見切る(&mut self, i: usize, 部屋: MeetingId) {
        self.機械たち[i].違った.remove(&部屋);
        let 誰も居ない = self.機械たち[i]
            .棚
            .get(&部屋)
            .is_none_or(|c| c.members().len() <= 1);
        if 誰も居ない && self.機械たち[i].棚.contains_key(&部屋) {
            self.畳む(i, 部屋);
        } else {
            self.機械たち[i].帰り道.retain(|(id, _)| *id != 部屋);
        }
    }

    /// **下ごしらえを送る**（画面が通話を作るたび）。その部屋の名簿の相手へ 1 通ずつ。
    /// 受けた側が名簿に居ないとして断り、自分が主催なら「居ない」と告げる（0.1.37）。
    fn 下ごしらえ(&mut self, i: usize, 部屋: MeetingId) {
        let 自分 = self.機械たち[i].鍵;
        let Some(c) = self.機械たち[i].棚.get(&部屋) else {
            return;
        };
        let 相手たち: Vec<PublicKey> = c.members().iter().copied().filter(|k| *k != 自分).collect();
        for k in 相手たち {
            let j = self.番号(k);
            if !self.繋がっている(i, j) {
                continue;
            }
            let 合図 = Notice::Signal(Signal::new(部屋, Step::Offer, Vec::new()));
            let 断った = match self.機械たち[j].棚.get_mut(&部屋) {
                Some(c) => matches!(c.on_notice(自分, &合図), Err(warifu_app::Error::NotAMember)),
                None => false,
            };
            let 告げる = {
                let h = &self.機械たち[j];
                居ないと告げるか(断った, h.主催たち.get(&部屋), &h.鍵, true)
            };
            if 告げる && let Some(c) = self.機械たち[i].棚.get_mut(&部屋) {
                let _ = c.on_notice(k, &Notice::Leave { meeting: 部屋 });
            }
        }
    }

    /// **落ち着くまで回す。**見張り → 下ごしらえ（客の部屋だけ）を、何周か。
    fn 落ち着くまで(&mut self, 周: usize) {
        for _ in 0..周 {
            for i in 0..self.機械たち.len() {
                self.見張り(i);
            }
            for i in 0..self.機械たち.len() {
                let 客の部屋: Vec<MeetingId> = self.機械たち[i]
                    .棚
                    .keys()
                    .copied()
                    .filter(|id| {
                        self.機械たち[i]
                            .主催たち
                            .get(id)
                            .is_some_and(|h| *h != self.機械たち[i].鍵)
                    })
                    .collect();
                for 部屋 in 客の部屋 {
                    self.下ごしらえ(i, 部屋);
                }
            }
        }
    }

    /// **食い違い**：客が「主催と同じ部屋に居る」と思っているのに、主催の名簿に客が居ない、
    /// または主催が起きていて部屋も在るのに、客が戻る先を持ったまま入れていない。
    fn 食い違い(&self) -> Vec<String> {
        let mut 並び = Vec::new();
        for g in self.機械たち.iter().filter(|m| m.起きている) {
            for (部屋, c) in &g.棚 {
                let Some(主催の鍵) = g.主催たち.get(部屋).copied() else {
                    continue;
                };
                if 主催の鍵 == g.鍵 {
                    continue;
                }
                let h = &self.機械たち[self.番号(主催の鍵)];
                let 客は居ると思う = c.members().contains(&主催の鍵);
                let 主催の名簿に居る = h.起きている && h.居ると思っている(*部屋, g.鍵);
                if 客は居ると思う && !主催の名簿に居る {
                    並び.push(format!(
                        "{} は部屋 {} に居るつもりだが、主催の名簿に居ない",
                        名(g),
                        頭(*部屋)
                    ));
                }
                // **客の名簿は、主催の名簿と同じ顔ぶれであること**（0.1.39・`roster_sync`）。
                // 違うと、見出しの人数にも届く先にも出ない相手から発言が届く
                if let Some(hc) = h.棚.get(部屋).filter(|_| h.起きている) {
                    let mut 客の = c.members().to_vec();
                    let mut 主催の = hc.members().to_vec();
                    客の.sort_by_key(|k| k.to_bytes());
                    主催の.sort_by_key(|k| k.to_bytes());
                    if 客の != 主催の {
                        並び.push(format!(
                            "{} の部屋 {} の名簿（{} 人）が、主催の名簿（{} 人）と違う",
                            名(g),
                            頭(*部屋),
                            客の.len(),
                            主催の.len()
                        ));
                    }
                }
            }
            for (部屋, 主催の鍵) in &g.帰り道 {
                let h = &self.機械たち[self.番号(*主催の鍵)];
                if h.起きている
                    && h.棚.contains_key(部屋)
                    && !h.居ると思っている(*部屋, g.鍵)
                {
                    並び.push(format!("{} は部屋 {} に戻れていない", 名(g), 頭(*部屋)));
                }
            }
        }
        並び
    }

    fn 部屋の数(&self) -> usize {
        self.機械たち.iter().map(|m| m.棚.len()).sum()
    }
}

fn 名(m: &機械) -> String {
    crate::短く(&crate::key_to_string(m.鍵))
}

fn 頭(id: MeetingId) -> String {
    crate::短く(&id.to_string())
}

/// 0 番が主催、ほかが鍵なしで入った客（知り合い）の網。
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
        網.叩く(g, 0).unwrap();
    }
    網
}

#[test]
fn 客どうしが直接つながれなくても_名簿は主催と同じになる() {
    // 10-23、網を越えて直接つながれない相手が、発言は主催ごしに届くのに
    // 見出しの人数にも届く先にも出なかった（2 台で報告）
    let 網 = 主催と客(2);
    assert!(!網.繋がっている(1, 2), "客どうしは直接つながっていない前提");
    assert_eq!(網.食い違い(), Vec::<String>::new());
}

#[test]
fn 入ったばかりなら_食い違いは無い() {
    let 網 = 主催と客(2);
    assert_eq!(網.食い違い(), Vec::<String>::new());
}

#[test]
fn 主催が上がり直しても_客は自分で戻る() {
    // 0.1.35〜0.1.36。10-23 に mac mini（主催）と ZenBook（客）で実測した形
    let mut 網 = 主催と客(2);
    let 前の数 = 網.部屋の数();
    網.落とす(0, true);
    網.起動(0);
    網.落ち着くまで(3);
    assert_eq!(網.食い違い(), Vec::<String>::new());
    assert_eq!(網.部屋の数(), 前の数, "部屋が増えも減りもしない");
}

#[test]
fn 主催が上がり直してすぐ別の用事で繋ぎ直しても_取り残されない() {
    // 0.1.37。10-23、mac air は mac mini（主催）の部屋に「居るつもり」のまま取り残された。
    // mac mini が上がり直してすぐ、mac air の部屋の用事で mac air を叩いたので、
    // mac air は線が切れたことに気づかず、名簿から mac mini を外さなかった
    let mut 網 = 主催と客(1);
    // 客（1）も部屋を持ち、主催（0）はそこへ客として入っている
    網.叩く(0, 1).unwrap();
    網.落とす(0, false); // 客は切れたことに気づかない
    網.起動(0);
    網.落ち着くまで(4);
    assert_eq!(網.食い違い(), Vec::<String>::new());
}

#[test]
fn 主催の居ない部屋は_見切って叩き合いをやめ_部屋を増やさない() {
    // 0.1.37。0.1.32 の取り違えで、2 台が互いに「主催は相手」と覚えた部屋が残っていた
    let mut 網 = 網::new(2);
    網.起動(0);
    網.起動(1);
    let (a, b) = (網.機械たち[0].鍵, 網.機械たち[1].鍵);
    網.機械たち[0].戸口.insert(b);
    網.機械たち[1].戸口.insert(a);
    let 幽霊 = Conference::host(a, 定員).unwrap().id();
    // 起動し直したあとの形：部屋は覚えておらず、帰り道（`rejoin.tsv`）にだけ残っている
    for (自分, 相手) in [(0, b), (1, a)] {
        網.機械たち[自分].帰り道.push((幽霊, 相手));
    }
    // 本番で 10 分おきに叩き合ったのは、叩くたびに線が閉じたから。線を閉じながら回す
    for _ in 0..6 {
        網.落ち着くまで(1);
        網.線を閉じる(0, 1);
    }
    assert!(
        網.機械たち
            .iter()
            .all(|m| !m.帰り道.iter().any(|(id, _)| *id == 幽霊)),
        "どちらも見切る"
    );
    assert!(
        網.機械たち.iter().all(|m| !m.棚.contains_key(&幽霊)),
        "幽霊の部屋は畳まれる"
    );
    assert!(
        網.部屋の数() <= 4,
        "叩くたびに部屋が増えない（{} 室）",
        網.部屋の数()
    );
}

#[test]
fn 客として他人の部屋を見ていても_呼ばれたら自分の部屋へ招く() {
    // 0.1.35 / 0.1.37。見ている部屋へ招くと、その部屋の主催の知らない所で部屋が割れた
    let mut 網 = 主催と客(1); // 1 は 0 の部屋の客
    let x = 網.機械たち[1]
        .棚
        .keys()
        .copied()
        .find(|id| 網.機械たち[1].主催たち[id] != 網.機械たち[1].鍵)
        .unwrap();
    網.機械たち[1].見ている = Some(x);
    // 2 台目の知り合い（2）が 1 を鍵なしで呼ぶ
    網.機械たち.push(機械::new(3));
    網.起動(2);
    let (k1, k2) = (網.機械たち[1].鍵, 網.機械たち[2].鍵);
    網.機械たち[1].戸口.insert(k2);
    網.機械たち[2].戸口.insert(k1);
    let 招かれた = 網.叩く(2, 1).unwrap();
    assert_ne!(招かれた, x, "他人の部屋へは招かない");
    assert_eq!(網.食い違い(), Vec::<String>::new());
}

#[test]
#[ignore = "調べるときだけ"]
fn 調べ_取り残し() {
    let mut 網 = 主催と客(1);
    let 見せる = |網: &網, 段: &str| {
        eprintln!("── {段}");
        for m in &網.機械たち {
            for (id, c) in &m.棚 {
                let 主催 = m
                    .主催たち
                    .get(id)
                    .map(|k| crate::短く(&crate::key_to_string(*k)))
                    .unwrap_or_default();
                let 面々: Vec<String> = c
                    .members()
                    .iter()
                    .map(|k| crate::短く(&crate::key_to_string(*k)))
                    .collect();
                eprintln!(
                    "  {} 部屋 {} 主催 {} 名簿 {:?} 見ている {:?}",
                    名(m),
                    頭(*id),
                    主催,
                    面々,
                    m.見ている.map(頭)
                );
            }
            eprintln!(
                "  {} 帰り道 {:?}",
                名(m),
                m.帰り道.iter().map(|(i, _)| 頭(*i)).collect::<Vec<_>>()
            );
        }
        eprintln!("  線 {:?}", 網.線);
    };
    網.叩く(0, 1).unwrap();
    見せる(&網, "叩いたあと");
    網.落とす(0, false);
    網.起動(0);
    見せる(&網, "上がり直したあと");
    for n in 0..4 {
        網.落ち着くまで(1);
        見せる(&網, &format!("{n} 周"));
    }
}
