//! **ルームの手順を、経路も画面も知らない形で持つ**（0.1.40・計画 1-A）。
//!
//! 画面（Tauri）の受信の本体は 500 行を超え、名簿・主催・線の状態を 5 つの錠に分けて持ち、
//! **直接の試験が 1 本も無かった。**机上の網（`room_sim`）はその手順を手で写していて、
//! 写しがずれても気づけなかった（0.1.39 の名簿の直しが、受け取る側で死んでいた）。
//!
//! ここでは **「何が起きたか」を受けて「何をするか」を返す**だけにする。
//!
//! ```text
//!   入り口（届いた・線がついた・線が切れた・建てた・入った・畳む・言う）
//!      │
//!      ▼  受ける(&mut self, 入り口) -> Vec<手配>
//!   手配（送る・画面へ・文字が届いた・食い違い・捨てた …）
//! ```
//!
//! 送るのも画面へ出すのも**皮**（Tauri の受信の輪・試験の網）がやる。
//! ここは状態と規則だけなので、**机上の網も、同一プロセスの 3 結び目の試験も、同じ本物を動かせる。**
//!
//! **決まり**（手で写していた所で、ずれていた所をここで 1 つにする）：
//! - 主催かは**覚えた主催が自分か**で決める（覚えていないルームは自分が建てたもの）。名簿の並びでは決めない
//! - 配るのは**その部屋の、線のある相手だけ**。ほかの部屋の線には配らない
//! - 主催は、名簿が動いたら（入った・抜けた・線が切れて外した）名簿を配る
//! - `届いた` に対して空を返さない。状態が動くか、`捨てた` を返す（**黙って捨てない**）

use std::collections::HashMap;

use warifu_core::PublicKey;
use warifu_meeting::{MeetingId, Notice, Roster};

use crate::{Conference, DEFAULT_CAPACITY, Error, Event};

/// 起きたこと。皮がこれを作って `受ける` に渡す。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum 入り口 {
    /// 線の向こうから知らせが届いた。
    届いた {
        線の相手: PublicKey, 知らせ: Notice
    },
    /// 線がついた（叩かれた・呼びに行って繋がった）。
    線がついた { 相手: PublicKey },
    /// 線が切れた。
    線が切れた { 相手: PublicKey },
    /// 自分で部屋を建てた。`id` が無ければ新しく作る。
    建てた {
        id: Option<MeetingId>, 定員: usize
    },
    /// 招かれて部屋に入った。名簿は主催がくれたもの（自分を先頭に組み直す）。
    入った {
        部屋: MeetingId,
        主催: PublicKey,
        名簿: Vec<PublicKey>,
        定員: usize,
        見る: bool,
    },
    /// 部屋を畳む（抜ける）。
    畳む { 部屋: MeetingId },
    /// この機械が文字を言った。
    言う {
        部屋: MeetingId,
        本文: String,
        話し手: Option<String>,
    },
}

/// 捨てた訳。**記録に残すためのもの**で、相手には返さない（D31）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum 捨てた訳 {
    知らないルーム,
    差出人が線の相手と違う,
    主催でない相手からの名簿,
    名簿に居ない相手からの下ごしらえ,
    主催への線が無い,
    名簿が受け取らなかった,
}

/// やること。皮が順に果たす。**順序に意味がある**（画面へ出してから配る、など）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum 手配 {
    /// その相手の線へ知らせを送る。線が無ければ皮が記録して落とす。
    送る {
        宛先: PublicKey, 知らせ: Notice
    },
    /// 画面へ出来事を渡す。
    画面へ { 部屋: MeetingId, 出来事: Event },
    /// 名簿に居る相手が入り直した。画面は通話を作り直す。
    画面へ入り直し {
        部屋: MeetingId, 相手: PublicKey
    },
    /// 文字が届いた。
    文字が届いた {
        部屋: MeetingId,
        差出人: PublicKey,
        話し手: Option<String>,
        本文: String,
    },
    /// 画像が届いた。絵はエージェントに渡さない（皮が札にする）。
    画像が届いた {
        部屋: MeetingId,
        差出人: PublicKey,
        話し手: Option<String>,
        型: String,
        中身: Vec<u8>,
    },
    /// 主催が「居る」と言った相手を、戸口で通す。
    戸口で迎える { 相手: PublicKey },
    /// 線が切れた相手を名簿に残したまま、画面へ「切れた」と言う。
    閉じた { 相手: PublicKey, 居た: bool },
    /// 画面と本体、または名簿と届いたものが食い違った。机へ 1 行・記録へ。
    食い違い { 文: String },
    /// 捨てた。記録へ。
    捨てた {
        知らせ: &'static str, 訳: 捨てた訳
    },
    /// この層では扱わない知らせ（名乗り・紹介・鍵・部屋の名前・合言葉・回線）。皮がいままでどおり扱う。
    皮に任せる {
        線の相手: PublicKey, 知らせ: Notice
    },
}

/// この機械のルームの状態。**経路も画面も持たない。**
#[derive(Debug, Clone)]
pub struct 部屋たち {
    me: PublicKey,
    棚: HashMap<MeetingId, Conference>,
    /// 部屋ごとの主催。**自分が建てた部屋は載せない**（載せないことが「自分が主催」の印）。
    主催たち: HashMap<MeetingId, PublicKey>,
    見ている: Option<MeetingId>,
    /// 線のある相手。
    線たち: Vec<PublicKey>,
}

impl 部屋たち {
    #[must_use]
    pub fn new(me: PublicKey) -> Self {
        Self {
            me,
            棚: HashMap::new(),
            主催たち: HashMap::new(),
            見ている: None,
            線たち: Vec::new(),
        }
    }

    #[must_use]
    pub fn me(&self) -> PublicKey {
        self.me
    }

    #[must_use]
    pub fn 棚(&self) -> &HashMap<MeetingId, Conference> {
        &self.棚
    }

    #[must_use]
    pub fn 見ている(&self) -> Option<MeetingId> {
        self.見ている
    }

    #[must_use]
    pub fn 主催(&self, 部屋: MeetingId) -> Option<PublicKey> {
        self.主催たち.get(&部屋).copied()
    }

    /// **その部屋の主催は自分か。**覚えていない部屋は自分が建てたもの。名簿の並びでは決めない。
    #[must_use]
    pub fn 主催か(&self, 部屋: MeetingId) -> bool {
        self.主催たち.get(&部屋).is_none_or(|h| *h == self.me)
    }

    #[must_use]
    pub fn 線があるか(&self, 相手: PublicKey) -> bool {
        self.線たち.contains(&相手)
    }

    /// その部屋の、**線のある**自分以外の面々。配る先はこれだけ。
    #[must_use]
    pub fn 線のある面々(&self, 部屋: MeetingId) -> Vec<PublicKey> {
        self.棚
            .get(&部屋)
            .map(|c| {
                c.members()
                    .iter()
                    .copied()
                    .filter(|k| *k != self.me && self.線があるか(*k))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// 起きたことを受けて、やることを返す。
    pub fn 受ける(&mut self, 入り: 入り口) -> Vec<手配> {
        match 入り {
            入り口::届いた {
                線の相手, 知らせ
            } => self.届いた(線の相手, 知らせ),
            入り口::線がついた { 相手 } => {
                if !self.線たち.contains(&相手) {
                    self.線たち.push(相手);
                }
                Vec::new()
            }
            入り口::線が切れた { 相手 } => self.線が切れた(相手),
            入り口::建てた { id, 定員 } => self.建てた(id, 定員),
            入り口::入った {
                部屋,
                主催,
                名簿,
                定員,
                見る,
            } => self.入った(部屋, 主催, &名簿, 定員, 見る),
            入り口::畳む { 部屋 } => self.畳む(部屋),
            入り口::言う {
                部屋, 本文, 話し手
            } => self.言う(部屋, 本文, 話し手),
        }
    }

    fn 建てた(&mut self, id: Option<MeetingId>, 定員: usize) -> Vec<手配> {
        let 定員 = if 定員 == 0 {
            DEFAULT_CAPACITY
        } else {
            定員
        };
        let c = match id {
            Some(id) => Conference::host_with_id(self.me, 定員, id),
            None => Conference::host(self.me, 定員),
        };
        let Ok(c) = c else {
            return vec![手配::捨てた {
                知らせ: "建てた",
                訳: 捨てた訳::名簿が受け取らなかった,
            }];
        };
        let id = c.id();
        self.棚.insert(id, c);
        self.主催たち.remove(&id);
        self.見ている.get_or_insert(id);
        Vec::new()
    }

    /// 招かれて入る。**名簿は自分を先頭に組み直す**（先頭は外せないので、主催が消えなくなる）。
    fn 入った(
        &mut self,
        部屋: MeetingId,
        主催: PublicKey,
        名簿: &[PublicKey],
        定員: usize,
        見る: bool,
    ) -> Vec<手配> {
        let 定員 = if 定員 == 0 {
            DEFAULT_CAPACITY
        } else {
            定員
        };
        let Ok(mut r) = Roster::with_capacity(self.me, 定員) else {
            return vec![手配::捨てた {
                知らせ: "入った",
                訳: 捨てた訳::名簿が受け取らなかった,
            }];
        };
        let _ = r.add(主催);
        for k in 名簿 {
            if *k != self.me && *k != 主催 {
                // 満員で足せない分は落とす（定員は主催が決めた）
                let _ = r.add(*k);
            }
        }
        self.棚.insert(部屋, Conference::joined(self.me, 部屋, r));
        self.主催たち.insert(部屋, 主催);
        if 見る {
            self.見ている = Some(部屋);
        } else {
            self.見ている.get_or_insert(部屋);
        }
        vec![手配::送る {
            宛先: 主催,
            知らせ: Notice::Join { meeting: 部屋 },
        }]
    }

    fn 畳む(&mut self, 部屋: MeetingId) -> Vec<手配> {
        let mut 出 = Vec::new();
        for k in self.線のある面々(部屋) {
            出.push(手配::送る {
                宛先: k,
                知らせ: Notice::Leave { meeting: 部屋 },
            });
        }
        self.棚.remove(&部屋);
        self.主催たち.remove(&部屋);
        if self.見ている == Some(部屋) {
            self.見ている = self.棚.keys().next().copied();
        }
        出
    }

    /// 言う。**ゲストは主催へだけ、主催は面々へ。**主催が配るので、ほかの全員に 1 回ずつ届く。
    fn 言う(
        &mut self, 部屋: MeetingId, 本文: String, 話し手: Option<String>
    ) -> Vec<手配> {
        if !self.棚.contains_key(&部屋) {
            return vec![手配::捨てた {
                知らせ: "言う",
                訳: 捨てた訳::知らないルーム,
            }];
        }
        let 知らせ = Notice::Text {
            meeting: 部屋,
            from: self.me,
            話し手,
            body: 本文,
        };
        if self.主催か(部屋) {
            return self
                .線のある面々(部屋)
                .into_iter()
                .map(|k| 手配::送る {
                    宛先: k,
                    知らせ: 知らせ.clone(),
                })
                .collect();
        }
        match self.主催(部屋) {
            Some(h) if self.線があるか(h) => vec![手配::送る {
                宛先: h, 知らせ
            }],
            _ => vec![手配::捨てた {
                知らせ: "言う",
                訳: 捨てた訳::主催への線が無い,
            }],
        }
    }

    fn 線が切れた(&mut self, 相手: PublicKey) -> Vec<手配> {
        self.線たち.retain(|k| *k != 相手);
        let mut 出 = Vec::new();
        let mut 居た = false;
        let 部屋たち: Vec<MeetingId> = self.棚.keys().copied().collect();
        for 部屋 in 部屋たち {
            let 主催か = self.主催か(部屋);
            let Some(c) = self.棚.get_mut(&部屋) else {
                continue;
            };
            // 名簿からは外さない —— 抜けたのではなく、切れた（0.1.28）。
            // ただし主催は、切れた相手を外して名簿を配り直す（残りの客の名簿を揃える）
            if !c.members().contains(&相手) {
                continue;
            }
            居た = true;
            if 主催か {
                let events = c
                    .on_notice(相手, &Notice::Leave { meeting: 部屋 })
                    .unwrap_or_default();
                if roster::名簿を配るか(true, &events) {
                    出.extend(self.名簿を配る(部屋, Some(相手)));
                }
            }
        }
        出.push(手配::閉じた { 相手, 居た });
        出
    }

    /// 主催が、いまの名簿を線のある面々へ配る。`除く` は入った本人（入るときに渡している）。
    fn 名簿を配る(&self, 部屋: MeetingId, 除く: Option<PublicKey>) -> Vec<手配> {
        let Some(c) = self.棚.get(&部屋) else {
            return Vec::new();
        };
        let Ok(mut r) = Roster::with_capacity(self.me, c.capacity()) else {
            return Vec::new();
        };
        for k in c.members() {
            if *k != self.me {
                let _ = r.add(*k);
            }
        }
        self.線のある面々(部屋)
            .into_iter()
            .filter(|k| Some(*k) != 除く)
            .map(|k| 手配::送る {
                宛先: k,
                知らせ: Notice::Invite {
                    meeting: 部屋,
                    roster: r.clone(),
                },
            })
            .collect()
    }

    fn 届いた(&mut self, 線の相手: PublicKey, 知らせ: Notice) -> Vec<手配> {
        let 部屋 = 知らせ.meeting();
        if !self.棚.contains_key(&部屋) {
            return vec![手配::捨てた {
                知らせ: 知らせの名(&知らせ),
                訳: 捨てた訳::知らないルーム,
            }];
        }
        match 知らせ {
            Notice::Invite { meeting, roster } => self.名簿が来た(線の相手, meeting, &roster),
            Notice::Text {
                meeting,
                from,
                話し手,
                body,
            } => self.配られたものが来た(
                線の相手,
                meeting,
                from,
                配られたもの::文字 {
                    話し手, 本文: body
                },
            ),
            Notice::Image {
                meeting,
                from,
                話し手,
                型,
                中身,
            } => self.配られたものが来た(
                線の相手,
                meeting,
                from,
                配られたもの::画像 {
                    話し手, 型, 中身
                },
            ),
            Notice::Join { .. } | Notice::Leave { .. } | Notice::Signal(_) => {
                self.名簿が動くものが来た(線の相手, 知らせ)
            }
            other => vec![手配::皮に任せる {
                線の相手,
                知らせ: other,
            }],
        }
    }

    /// 主催が配った名簿に、自分の名簿を合わせる。**信じるのは主催の言い分だけ。**
    fn 名簿が来た(
        &mut self,
        線の相手: PublicKey,
        部屋: MeetingId,
        名簿: &Roster,
    ) -> Vec<手配> {
        if self.主催か(部屋) || self.主催(部屋) != Some(線の相手) {
            return vec![手配::捨てた {
                知らせ: "名簿",
                訳: 捨てた訳::主催でない相手からの名簿,
            }];
        }
        let me = self.me;
        let Some(c) = self.棚.get_mut(&部屋) else {
            return Vec::new();
        };
        let (足す, 外す) = roster::名簿の差(c.members(), 名簿.members(), &me, &線の相手);
        let mut 出 = Vec::new();
        for k in &足す {
            if c.admit(*k).is_ok() {
                出.push(手配::画面へ {
                    部屋,
                    出来事: Event::Joined(*k),
                });
                出.push(手配::戸口で迎える { 相手: *k });
            }
        }
        for k in &外す {
            if let Ok(events) = c.on_notice(*k, &Notice::Leave { meeting: 部屋 }) {
                出.extend(events.into_iter().map(|e| 手配::画面へ {
                    部屋, 出来事: e
                }));
            }
        }
        // 合っていれば空。**名簿が動かなかったことも 1 つの答え**（捨てたのではない）
        出
    }

    fn 配られたものが来た(
        &mut self,
        線の相手: PublicKey,
        部屋: MeetingId,
        差出人: PublicKey,
        中身: 配られたもの,
    ) -> Vec<手配> {
        let 主催 = self.主催(部屋);
        if !roster::配られた発言を通すか(&差出人, &線の相手, &self.me, 主催.as_ref())
        {
            return vec![手配::捨てた {
                知らせ: 中身.名(),
                訳: 捨てた訳::差出人が線の相手と違う,
            }];
        }
        let mut 出 = Vec::new();
        let 名簿に居る = self
            .棚
            .get(&部屋)
            .is_some_and(|c| c.members().contains(&差出人));
        if !名簿に居る {
            出.push(手配::食い違い {
                文: format!("名簿に居ない相手から部屋への{}が届きました", 中身.名()),
            });
        }
        let 知らせ = 中身.知らせに(部屋, 差出人);
        出.push(中身.手配に(部屋, 差出人));
        // 主催は、その部屋のほかの面々へそのまま配る（差出人は書き換えない・D48）
        if self.主催か(部屋) {
            for k in self.線のある面々(部屋) {
                if k != 線の相手 && k != 差出人 {
                    出.push(手配::送る {
                        宛先: k,
                        知らせ: 知らせ.clone(),
                    });
                }
            }
        }
        出
    }

    fn 名簿が動くものが来た(
        &mut self,
        線の相手: PublicKey,
        知らせ: Notice,
    ) -> Vec<手配> {
        let 部屋 = 知らせ.meeting();
        let 主催か = self.主催か(部屋);
        let Some(c) = self.棚.get_mut(&部屋) else {
            return Vec::new();
        };
        match c.on_notice(線の相手, &知らせ) {
            Ok(events) => {
                let mut 出 = Vec::new();
                if events.is_empty()
                    && matches!(知らせ, Notice::Join { .. })
                    && c.members().contains(&線の相手)
                {
                    // 名簿に居る相手の入り直し。名簿は動かさず、画面だけ通話を作り直す（#23）
                    出.push(手配::画面へ入り直し {
                        部屋,
                        相手: 線の相手,
                    });
                }
                出.extend(events.iter().cloned().map(|e| 手配::画面へ {
                    部屋, 出来事: e
                }));
                // 名簿が動いたら、主催はいまの名簿を配る（画面へ出した後）
                if roster::名簿を配るか(主催か, &events) {
                    出.extend(self.名簿を配る(部屋, Some(線の相手)));
                }
                出
            }
            Err(Error::NotAMember) => vec![手配::捨てた {
                知らせ: 知らせの名(&知らせ),
                訳: 捨てた訳::名簿に居ない相手からの下ごしらえ,
            }],
            Err(_) => vec![手配::捨てた {
                知らせ: 知らせの名(&知らせ),
                訳: 捨てた訳::名簿が受け取らなかった,
            }],
        }
    }
}

/// 文字と画像は同じ道を通る。違いは中身だけ。
enum 配られたもの {
    文字 {
        話し手: Option<String>,
        本文: String,
    },
    画像 {
        話し手: Option<String>,
        型: String,
        中身: Vec<u8>,
    },
}

impl 配られたもの {
    const fn 名(&self) -> &'static str {
        match self {
            Self::文字 { .. } => "文字",
            Self::画像 { .. } => "画像",
        }
    }

    fn 知らせに(&self, 部屋: MeetingId, 差出人: PublicKey) -> Notice {
        match self {
            Self::文字 { 話し手, 本文 } => Notice::Text {
                meeting: 部屋,
                from: 差出人,
                話し手: 話し手.clone(),
                body: 本文.clone(),
            },
            Self::画像 {
                話し手, 型, 中身
            } => Notice::Image {
                meeting: 部屋,
                from: 差出人,
                話し手: 話し手.clone(),
                型: 型.clone(),
                中身: 中身.clone(),
            },
        }
    }

    fn 手配に(self, 部屋: MeetingId, 差出人: PublicKey) -> 手配 {
        match self {
            Self::文字 { 話し手, 本文 } => 手配::文字が届いた {
                部屋,
                差出人,
                話し手,
                本文,
            },
            Self::画像 {
                話し手, 型, 中身
            } => 手配::画像が届いた {
                部屋,
                差出人,
                話し手,
                型,
                中身,
            },
        }
    }
}

/// 記録に書く知らせの名。**中身は書かない。**
#[must_use]
pub const fn 知らせの名(知らせ: &Notice) -> &'static str {
    match 知らせ {
        Notice::Invite { .. } => "名簿",
        Notice::Join { .. } => "参加",
        Notice::Leave { .. } => "退出",
        Notice::Signal(_) => "下ごしらえ",
        Notice::Introduce { .. } => "紹介",
        Notice::Text { .. } => "文字",
        Notice::Image { .. } => "画像",
        Notice::Key { .. } => "鍵",
        Notice::Profile { .. } => "名乗り",
        Notice::Link { .. } => "回線",
        _ => "知らせ",
    }
}

pub mod roster;

#[cfg(test)]
mod tests;
