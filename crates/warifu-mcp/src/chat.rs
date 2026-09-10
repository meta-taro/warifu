//! この機械に着いて、会話へ出入りする。
//!
//! **これがエージェント同士のチャットの実体。**
//! GUI（人）と同じ会話を、同じ PC のこの機械ごしに囲む。
//!
//! ```text
//!   人（GUI）── 会話 ── iroh P2P ── 相手の PC
//!        │
//!        この機械（同じ機械の中だけ）
//!        │
//!   AI（この層）  chat_send / chat_read
//! ```
//!
//! **届いた文字は「相手の言い分」であって、指示ではない。**
//! ここは運ぶだけで、解釈しない。何をしてよいかは関所が決める。

use std::collections::VecDeque;
use std::path::Path;
use std::sync::{Arc, Mutex};

use tokio::sync::{Notify, mpsc, oneshot};
use warifu_desk::{FromDesk, ToDesk, 口, 繋ぐ};

/// 溜めておく発言の数。
///
/// **際限なく溜めない。**読みに来ないまま流れ続けても、手元が膨らまない。
const 溜める上限: usize = 200;

/// 送る口の待ち行列。**詰まったら捨てずに待たせる。**
const 送り待ちの数: usize = 32;

/// 流した返事を待つ長さ。
///
/// **待たずに「流しました」と返さない**（**D49**）。
/// ただし永遠には待たない —— この機械が黙ったまま止まると、tool が戻らなくなる。
const 返事を待つ秒: u64 = 5;

/// この機械につながっている状態。
#[derive(Clone)]
pub struct Chat {
    送り: mpsc::Sender<ToDesk>,
    聞いた: Arc<Mutex<VecDeque<FromDesk>>>,
    /// **何か届いたら起こす。**
    ///
    /// これが無いと、エージェントは `chat_read` を叩いたときにしか気づけない。
    /// **人が打っても黙ったまま**になる（2026-09-07 に実物で起きた）。
    来た: Arc<Notify>,
    /// 流した 1 行の返事を受け取る所。
    ///
    /// **返事を溜めに混ぜない。**混ぜると、`chat_read` が
    /// 自分の送信結果を「誰かの発言」として読むことになる。
    返事待ち: Arc<Mutex<Option<oneshot::Sender<FromDesk>>>>,
    /// **いつこのエージェントにつながったか**（`HH:MM`・`issues/4` の 1 番）。
    ///
    /// **「届いていない」と「つながる前だった」を、エージェントから見分けられるようにする。**
    /// 画面を入れ替えるとこの機械のエージェントは全部外れる（`issues/2`）ので、
    /// 黙って繋ぎ直すと、**切れている間の発言が無いことに気づけない。**
    つながった: Arc<Mutex<Option<String>>>,
}

impl Chat {
    /// この機械へ繋いで、会話を聞き始める。
    ///
    /// **繋がらなければ、繋がったふりをしない。**
    /// この機械が開いていない（＝人の画面が立っていない）ことは、失敗として返す。
    pub async fn つながる(場所: &Path, 名乗り: Option<String>) -> std::io::Result<Self> {
        let mut 口 = 口::新しく(繋ぐ(場所).await?);
        // まず「聞く」と言う。**これまでの会話を先にもらう**。
        // **どこで動いているかを一緒に名乗る** —— 1 台の PC で
        // 複数のエージェントが同じこの機械につながるので、名乗らないと
        // どれが喋ったのか人に分からない（2026-09-08）
        口.送る(&ToDesk::Listen { 場所: 名乗り }.書く()).await?;

        let (送り, mut 受け) = mpsc::channel::<ToDesk>(送り待ちの数);
        let 聞いた = Arc::new(Mutex::new(VecDeque::new()));
        let 溜め先 = Arc::clone(&聞いた);
        let 返事待ち: Arc<Mutex<Option<oneshot::Sender<FromDesk>>>> = Arc::new(Mutex::new(None));
        let 返し先 = Arc::clone(&返事待ち);
        let 来た = Arc::new(Notify::new());
        let 起こす = Arc::clone(&来た);
        // **いつつながったかをこの機械が返す**（`issues/4` の 1 番）
        let つながった: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let つながった写し = Arc::clone(&つながった);

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    出す = 受け.recv() => {
                        let Some(出す) = 出す else { break };
                        if 口.送る(&出す.書く()).await.is_err() {
                            break;
                        }
                    }
                    来た = 口.受ける() => {
                        match 来た {
                            Ok(Some(行)) => 仕分ける(&溜め先, &返し先, &起こす, &つながった写し, &行),
                            // 相手が閉じた・読めない。**黙って繋がっているふりをしない**
                            Ok(None) | Err(_) => break,
                        }
                    }
                }
            }
            積む_直に(
                &溜め先,
                FromDesk::Denied {
                    why: "この機械が閉じました".to_owned(),
                },
            );
        });

        Ok(Self {
            つながった,
            送り,
            聞いた,
            来た,
            返事待ち,
        })
    }

    /// 会話へ 1 行流す。**届いた人数を返す。**
    ///
    /// **返事を待つ。**待たずに「流しました」と返すのは、
    /// 押しても何も起きないボタンと同じである（**D49**）。
    pub async fn 言う(&self, body: &str) -> Result<(usize, u64, Vec<String>), crate::ToolError> {
        let 行 = ToDesk::say(body).map_err(|e| crate::ToolError::BadArgs(e.to_string()))?;
        let (返す, 待つ) = oneshot::channel();
        *self.返事待ち.lock().expect("毒されていない") = Some(返す);

        self.送り
            .send(行)
            .await
            .map_err(|_| crate::ToolError::Unavailable("この機械が閉じています".to_owned()))?;

        let 返事 = tokio::time::timeout(std::time::Duration::from_secs(返事を待つ秒), 待つ)
            .await
            .map_err(|_| crate::ToolError::Unavailable("この機械が返事をしません".to_owned()))?
            .map_err(|_| crate::ToolError::Unavailable("この機械が閉じました".to_owned()))?;

        match 返事 {
            // **通し番号と届いたエージェントも返す。**あとで「誰が読んだか」を尋ねられる（**D76**）
            FromDesk::Sent { to, id, 届いた } => Ok((to, id, 届いた)),
            // **画面には出ている。**同じエージェントの人は読んでいるので、そこまで言う。
            // 「届かなかった」だけだと、言い直しを促すことになる
            FromDesk::Nobody => Err(crate::ToolError::Unavailable(
                "この PC の画面には出ましたが、会議には誰も居ないので誰にも届いていません"
                    .to_owned(),
            )),
            FromDesk::Denied { why } => Err(crate::ToolError::Unavailable(why)),
            // 発言や入退室は返事ではない。**ここへ来た時点で仕分けが壊れている**
            他 => Err(crate::ToolError::Unavailable(format!(
                "この機械が想定しない返事をしました: {他:?}"
            ))),
        }
    }

    /// **いつこのエージェントにつながったか**（`HH:MM`）。まだ返ってきていなければ `None`。
    ///
    /// **これより前の発言は取れない。**「届いていない」と「つながる前だった」は別である
    /// （`issues/4` の 1 番）。
    #[must_use]
    pub fn つながった時刻(&self) -> Option<String> {
        self.つながった.lock().expect("毒されていない").clone()
    }

    /// **そこまで読んだ**とこの機械へ告げる（**D76**）。
    ///
    /// **返事は待たない。**数えてもらうだけで、こちらの手は止めない。
    async fn 読んだと告げる(&self, まで: u64) {
        if まで == 0 {
            return;
        }
        let _ = self.送り.send(ToDesk::Read { まで }).await;
    }

    /// **その発言の届き方**を尋ねる（**D76**）。届いたエージェントと、読んだエージェントを返す。
    ///
    /// # Errors
    /// この機械が閉じているとき、返事が来ないとき。
    pub async fn 届き方(&self, id: u64) -> Result<(Vec<String>, Vec<String>), crate::ToolError> {
        let (返す, 待つ) = oneshot::channel();
        *self.返事待ち.lock().expect("毒されていない") = Some(返す);

        self.送り
            .send(ToDesk::Status { id })
            .await
            .map_err(|_| crate::ToolError::Unavailable("この機械が閉じています".to_owned()))?;

        let 返事 = tokio::time::timeout(std::time::Duration::from_secs(返事を待つ秒), 待つ)
            .await
            .map_err(|_| crate::ToolError::Unavailable("この機械が返事をしません".to_owned()))?
            .map_err(|_| crate::ToolError::Unavailable("この機械が閉じました".to_owned()))?;

        match 返事 {
            FromDesk::Status {
                届いた, 読んだ,
            ..
            } => Ok((届いた, 読んだ)),
            他 => Err(crate::ToolError::Unavailable(format!(
                "この機械が想定しない返事をしました: {他:?}"
            ))),
        }
    }

    /// **自分のエージェントのプロフィールを書く。**書けたら、誰として書いたかを返す。
    ///
    /// **どのエージェントかは口で決まる。**引数に「誰の」は無い ——
    /// 有ると、**同じ機械の別のエージェントに化けられる。**
    ///
    /// # Errors
    /// 長すぎるとき、名乗っていないとき、この機械が返事をしないとき。
    pub async fn 名乗る(&self, 名前: &str, 紹介: &str) -> Result<String, crate::ToolError> {
        let 行 = ToDesk::Profile {
            名前: 名前.to_owned(),
            紹介: 紹介.to_owned(),
        };
        let (返す, 待つ) = oneshot::channel();
        *self.返事待ち.lock().expect("毒されていない") = Some(返す);

        self.送り
            .send(行)
            .await
            .map_err(|_| crate::ToolError::Unavailable("この機械が閉じています".to_owned()))?;

        let 返事 = tokio::time::timeout(std::time::Duration::from_secs(返事を待つ秒), 待つ)
            .await
            .map_err(|_| crate::ToolError::Unavailable("この機械が返事をしません".to_owned()))?
            .map_err(|_| crate::ToolError::Unavailable("この機械が閉じました".to_owned()))?;

        match 返事 {
            FromDesk::Wrote { who } => Ok(who),
            FromDesk::Denied { why } => Err(crate::ToolError::Unavailable(why)),
            他 => Err(crate::ToolError::Unavailable(format!(
                "この機械が想定しない返事をしました: {他:?}"
            ))),
        }
    }

    /// まだこの機械と繋がっているか。
    ///
    /// **切れたまま送り続けない。**切れていれば、繋ぎ直す側が判断できる。
    #[must_use]
    pub fn 生きているか(&self) -> bool {
        !self.送り.is_closed()
    }

    /// **何か届くまで待つ。**届いたらその分を返す。
    ///
    /// `chat_read` は「いま溜まっているか」を覗きに行くだけなので、
    /// **エージェントは自分から気づけない。**人が打っても黙ったままになる。
    /// **待てる口があれば、エージェントは待つ。**
    ///
    /// **永遠には待たない。**待ち続けると、その間そのエージェントは何もできない。
    pub async fn 待つ(&self, 秒: u64) -> Vec<FromDesk> {
        let 期限 = std::time::Duration::from_secs(秒);
        let 待ち受け = self.来た.notified();
        // **先に見る。**待ち受けを構えてから見ないと、
        // 構える直前に届いたものを取りこぼす
        let いま = self.汲む();
        if !いま.is_empty() {
            return いま;
        }
        let _ = tokio::time::timeout(期限, 待ち受け).await;
        self.汲む()
    }

    /// 溜まっている発言を取り出す。**取り出したら消える。**
    ///
    /// 消さないと、読むたびに同じ発言を新着として見ることになる。
    pub fn 汲む(&self) -> Vec<FromDesk> {
        let mut 箱 = self.聞いた.lock().expect("毒されていない");
        箱.drain(..).collect()
    }

    /// 取り出したものを、**読んだとこの機械へ告げる**（**D76**）。
    ///
    /// **渡した時点が「読んだ」である。**中身を理解したかは誰にも分からないので、
    /// そこは名乗らない。
    /// 何も無かったときに添える一言（`issues/4` の 1 番）。
    ///
    /// **「届いていない」と「つながる前だった」を、エージェントから見分けられるようにする。**
    #[must_use]
    pub fn つながってからの一言(&self) -> String {
        match self.つながった時刻() {
            Some(at) => {
                format!(
                    "（このエージェントは {at} からつながっています。それより前の発言は取れません）"
                )
            }
            None => String::new(),
        }
    }

    /// 取り出したものを、**読んだとこの機械へ告げる**（**D76**）。
    ///
    /// **渡した時点が「読んだ」である。**中身を理解したかは誰にも分からないので、
    /// そこは名乗らない。
    pub async fn 汲んで告げる(&self) -> Vec<FromDesk> {
        let 出た = self.汲む();
        if let Some(まで) = 最後の番号(&出た) {
            self.読んだと告げる(まで).await;
        }
        出た
    }

    /// 待って取り出し、**読んだとこの機械へ告げる**（**D76**）。
    pub async fn 待って告げる(&self, 秒: u64) -> Vec<FromDesk> {
        let 出た = self.待つ(秒).await;
        if let Some(まで) = 最後の番号(&出た) {
            self.読んだと告げる(まで).await;
        }
        出た
    }
}

/// 取り出したものの中で、いちばん新しい発言の番号。**発言でなければ数えない。**
fn 最後の番号(出た: &[FromDesk]) -> Option<u64> {
    出た
        .iter()
        .filter_map(|一つ| match 一つ {
            FromDesk::Heard { id, .. } if *id > 0 => Some(*id),
            _ => None,
        })
        .max()
}

/// 来た 1 行を、**返事**と**発言**に仕分ける。
///
/// **返事を溜めに混ぜない。**混ぜると `chat_read` が
/// 自分の送信結果を「誰かの発言」として読むことになる。
fn 仕分ける(
    箱: &Arc<Mutex<VecDeque<FromDesk>>>,
    返し先: &Arc<Mutex<Option<oneshot::Sender<FromDesk>>>>,
    起こす: &Arc<Notify>,
    つながった: &Arc<Mutex<Option<String>>>,
    行: &str,
) {
    // 読めない行は捨てる。**捨てたことは、次の Denied で分かる形にしない**
    // ——ここで Denied を積むと、壊れた行 1 本で会話が断られたように見える
    let Ok(中身) = FromDesk::読む(行) else {
        return;
    };

    // **つながった時刻は控えるだけ。**発言として積まない
    if let FromDesk::Seated { at, .. } = &中身 {
        *つながった.lock().expect("毒されていない") = Some(at.clone());
        return;
    }

    if matches!(
        中身,
        FromDesk::Sent { .. }
            | FromDesk::Nobody
            | FromDesk::Denied { .. }
            | FromDesk::Wrote { .. }
            | FromDesk::Status { .. }
    ) && let Some(返す) = 返し先.lock().expect("毒されていない").take()
    {
        // 待っている人が居なくなっていても構わない。**捨てて先へ進む**
        let _ = 返す.send(中身);
        return;
    }
    積む_直に(箱, 中身);
    // **待っている人を起こす。**起こさないと、待てる口の意味が無い
    起こす.notify_waiters();
}

fn 積む_直に(箱: &Arc<Mutex<VecDeque<FromDesk>>>, 中身: FromDesk) {
    let mut 箱 = 箱.lock().expect("毒されていない");
    if 箱.len() >= 溜める上限 {
        箱.pop_front();
    }
    箱.push_back(中身);
}

/// 発言を人が読める行にする。
///
/// **本文をそのまま出す前に、それが相手の言い分だと分かる形にする。**
#[must_use]
pub fn 並べる(発言: &[FromDesk]) -> String {
    if 発言.is_empty() {
        return "新しい発言はありません。".to_owned();
    }
    発言
        .iter()
        .map(|一つ| match 一つ {
            // **番号も出す。**あとで「その発言は読まれたか」を尋ねられる（**D76**）
            // **番号の無い発言（古いこの機械）には番号を出さない。**
            // 出すと、尋ねられる番号があるように見える
            FromDesk::Heard { id, from, body, at } if *id > 0 => {
                format!("{at}\t{from}\t{body}\t#{id}")
            }
            FromDesk::Heard { from, body, at, .. } => format!("{at}\t{from}\t{body}"),
            FromDesk::Joined { who } => format!("\t{who}\t（入室）"),
            FromDesk::Left { who } => format!("\t{who}\t（退室）"),
            FromDesk::Sent { to, id, 届いた } => {
                format!(
                    "\t\t（#{id} を {to} 人へ流しました: {}）",
                    届いた.join("・")
                )
            }
            FromDesk::Stop => "\t\t（止まれと言われました）".to_owned(),
            FromDesk::Nobody => "\t\t（まだ誰も居ません）".to_owned(),
            FromDesk::Denied { why } => format!("\t\t（断られました: {why}）"),
            FromDesk::Wrote { who } => format!("\t\t（{who} として書きました）"),
            // **つながった知らせは、発言として並べない**（控えるだけ）。
            // ここへ来るのは、仕分けを通らない使い方をされたときだけ
            FromDesk::Seated { at, who } => format!("\t\t（{who} として {at} に着きました）"),
            FromDesk::Status {
                id, 届いた, 読んだ
            } => format!(
                "\t\t（#{id} 届いた {} / 読んだ {}）",
                届いた.join("・"),
                読んだ.join("・")
            ),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 発言は_いつ_誰が_なにを_の順で出る() {
        let 行 = 並べる(&[FromDesk::Heard {
            id: 1,
            from: "ABCDEFGH…".to_owned(),
            body: "直しました".to_owned(),
            at: "09:05".to_owned(),
        }]);
        // **番号も出す。**あとで「その発言は読まれたか」を尋ねられる（**D76**）
        assert_eq!(行, "09:05\tABCDEFGH…\t直しました\t#1");
    }

    #[test]
    fn 番号の無い発言には番号を出さない() {
        // **古いこの機械は 0 を返す。**0 を「#0」として出すと、尋ねられる番号に見える
        let 行 = 並べる(&[FromDesk::Heard {
            id: 0,
            from: "ABCDEFGH…".to_owned(),
            body: "むかしのこの機械から".to_owned(),
            at: "09:05".to_owned(),
        }]);
        assert_eq!(行, "09:05\tABCDEFGH…\tむかしのこの機械から");
    }

    #[test]
    fn 何も無いときに_空を返さない() {
        // **空文字を返すと「読めなかった」と区別がつかない**
        assert_eq!(並べる(&[]), "新しい発言はありません。");
    }

    #[test]
    fn 入退室も_発言と同じ並びに出る() {
        let 行 = 並べる(&[FromDesk::Joined {
            who: "X".to_owned(),
        }]);
        assert!(行.contains("入室"), "{行}");
    }

    #[test]
    fn 溜めすぎない() {
        let 箱 = Arc::new(Mutex::new(VecDeque::new()));
        for i in 0..(溜める上限 + 10) {
            積む_直に(&箱, FromDesk::Joined { who: i.to_string() });
        }
        assert_eq!(箱.lock().unwrap().len(), 溜める上限);
        // **古いほうから落ちる。**新しい発言を捨てない
        let 先頭 = 箱.lock().unwrap().front().cloned().unwrap();
        assert_eq!(
            先頭,
            FromDesk::Joined {
                who: "10".to_owned()
            }
        );
    }

    #[test]
    fn 壊れた行は_会話を止めない() {
        let 箱 = Arc::new(Mutex::new(VecDeque::new()));
        let 返し先 = Arc::new(Mutex::new(None));
        仕分ける(
            &箱,
            &返し先,
            &Arc::new(Notify::new()),
            &Arc::new(Mutex::new(None)),
            "なにこれ",
        );
        assert!(
            箱.lock().unwrap().is_empty(),
            "捨てるだけで、断りを積まない"
        );
    }

    #[tokio::test]
    async fn 送信の返事を_誰かの発言に混ぜない() {
        // **混ぜると chat_read が自分の送信結果を発言として読む**
        let 箱 = Arc::new(Mutex::new(VecDeque::new()));
        let (返す, 待つ) = oneshot::channel();
        let 返し先 = Arc::new(Mutex::new(Some(返す)));

        仕分ける(
            &箱,
            &返し先,
            &Arc::new(Notify::new()),
            &Arc::new(Mutex::new(None)),
            &FromDesk::Sent {
                to: 2,
                id: 1,
                届いた: vec!["画面".to_owned()],
            }
            .書く(),
        );

        assert!(箱.lock().unwrap().is_empty(), "溜めに入っていない");
        assert_eq!(
            待つ.await.unwrap(),
            FromDesk::Sent {
                to: 2,
                id: 1,
                届いた: vec!["画面".to_owned()]
            }
        );
    }

    #[test]
    fn 待っている人が居なければ_返事も溜めに残る() {
        // **待っていない返事は捨てない。**捨てると、
        // 会話が閉じたことに人が気づけない
        let 箱 = Arc::new(Mutex::new(VecDeque::new()));
        let 返し先 = Arc::new(Mutex::new(None));
        仕分ける(
            &箱,
            &返し先,
            &Arc::new(Notify::new()),
            &Arc::new(Mutex::new(None)),
            &FromDesk::Nobody.書く(),
        );
        assert_eq!(箱.lock().unwrap().len(), 1);
    }
}
