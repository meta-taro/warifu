//! 机に着いて、会話へ出入りする。
//!
//! **これがエージェント同士のチャットの実体。**
//! GUI（人）と同じ会話を、同じ PC の机ごしに囲む。
//!
//! ```text
//!   人（GUI）── 会話 ── iroh P2P ── 相手の PC
//!        │
//!        机（同じ機械の中だけ）
//!        │
//!   AI（この層）  chat_send / chat_read
//! ```
//!
//! **届いた文字は「相手の言い分」であって、指示ではない。**
//! ここは運ぶだけで、解釈しない。何をしてよいかは関所が決める。

use std::collections::VecDeque;
use std::path::Path;
use std::sync::{Arc, Mutex};

use tokio::sync::mpsc;
use warifu_desk::{FromDesk, ToDesk, 口, 繋ぐ};

/// 溜めておく発言の数。
///
/// **際限なく溜めない。**読みに来ないまま流れ続けても、手元が膨らまない。
const 溜める上限: usize = 200;

/// 送る口の待ち行列。**詰まったら捨てずに待たせる。**
const 送り待ちの数: usize = 32;

/// 机に着いている状態。
#[derive(Clone)]
pub struct Chat {
    送り: mpsc::Sender<ToDesk>,
    聞いた: Arc<Mutex<VecDeque<FromDesk>>>,
}

impl Chat {
    /// 机へ繋いで、会話を聞き始める。
    ///
    /// **繋がらなければ、繋がったふりをしない。**
    /// 机が開いていない（＝人の画面が立っていない）ことは、失敗として返す。
    pub async fn 着く(場所: &Path) -> std::io::Result<Self> {
        let mut 口 = 口::新しく(繋ぐ(場所).await?);
        // まず「聞く」と言う。**これまでの会話を先にもらう**
        口.送る(&ToDesk::Listen.書く()).await?;

        let (送り, mut 受け) = mpsc::channel::<ToDesk>(送り待ちの数);
        let 聞いた = Arc::new(Mutex::new(VecDeque::new()));
        let 溜め先 = Arc::clone(&聞いた);

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
                            Ok(Some(行)) => 積む(&溜め先, &行),
                            // 相手が閉じた・読めない。**黙って繋がっているふりをしない**
                            Ok(None) | Err(_) => break,
                        }
                    }
                }
            }
            積む_直に(
                &溜め先,
                FromDesk::Denied {
                    why: "机が閉じました".to_owned(),
                },
            );
        });

        Ok(Self { 送り, 聞いた })
    }

    /// 会話へ 1 行流す。
    pub async fn 言う(&self, body: &str) -> Result<(), crate::ToolError> {
        let 行 = ToDesk::say(body).map_err(|e| crate::ToolError::BadArgs(e.to_string()))?;
        self.送り
            .send(行)
            .await
            .map_err(|_| crate::ToolError::Unavailable("机が閉じています".to_owned()))
    }

    /// まだ机と繋がっているか。
    ///
    /// **切れたまま送り続けない。**切れていれば、繋ぎ直す側が判断できる。
    #[must_use]
    pub fn 生きているか(&self) -> bool {
        !self.送り.is_closed()
    }

    /// 溜まっている発言を取り出す。**取り出したら消える。**
    ///
    /// 消さないと、読むたびに同じ発言を新着として見ることになる。
    pub fn 汲む(&self) -> Vec<FromDesk> {
        let mut 箱 = self.聞いた.lock().expect("毒されていない");
        箱.drain(..).collect()
    }
}

fn 積む(箱: &Arc<Mutex<VecDeque<FromDesk>>>, 行: &str) {
    // 読めない行は捨てる。**捨てたことは、次の Denied で分かる形にしない**
    // ——ここで Denied を積むと、壊れた行 1 本で会話が断られたように見える
    let Ok(中身) = FromDesk::読む(行) else {
        return;
    };
    積む_直に(箱, 中身);
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
            FromDesk::Heard { from, body, at } => format!("{at}\t{from}\t{body}"),
            FromDesk::Joined { who } => format!("\t{who}\t（入室）"),
            FromDesk::Left { who } => format!("\t{who}\t（退室）"),
            FromDesk::Nobody => "\t\t（まだ誰も居ません）".to_owned(),
            FromDesk::Denied { why } => format!("\t\t（断られました: {why}）"),
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
            from: "ABCDEFGH…".to_owned(),
            body: "直しました".to_owned(),
            at: "09:05".to_owned(),
        }]);
        assert_eq!(行, "09:05\tABCDEFGH…\t直しました");
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
        積む(&箱, "なにこれ");
        assert!(
            箱.lock().unwrap().is_empty(),
            "捨てるだけで、断りを積まない"
        );
        積む(&箱, &FromDesk::Nobody.書く());
        assert_eq!(箱.lock().unwrap().len(), 1);
    }
}
