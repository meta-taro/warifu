//! この機械とやり取りする 1 行の形。**JSON を 1 行に 1 つ。**

use serde::{Deserialize, Serialize};

/// 本文の上限（バイト）。
///
/// **一度にどれだけ流すかを、繋いできた側に決めさせない。**
/// 会話の 1 発言としては十分で、貼り付けた丸ごとのログは通らない大きさにする。
pub const 本文の上限: usize = 4096;

/// エージェント → この機械。
///
/// **差出人を書く場所が無いのは意図。**誰が言ったかはこの機械が刻む。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "型", rename_all = "snake_case", deny_unknown_fields)]
pub enum ToDesk {
    /// 会話へ 1 行流す。
    Say {
        /// 本文。空は送れない。[`本文の上限`] を超えたら受けない。
        body: String,
    },
    /// これまでの会話をもらってから、以後を聞く。
    ///
    /// **どこで起動しているエージェントかを名乗る。**
    /// 1 台の PC で複数のエージェントが同じこの機械につながるので、
    /// まとめて 1 人に見せると**どれが喋ったのか分からない**
    /// （2026-09-08 オーナー指摘）。
    ///
    /// **これは差出人の名乗りではない。**名乗れるのは「どこで動いているか」だけで、
    /// 誰が言ったかを刻むのはこの機械である（[`ToDesk::Say`] に `from` が無いのと同じ約束）。
    /// この機械はこれを「◯◯ のエージェント」という形に**包んで**出す。
    Listen {
        /// どこで動いているか（フォルダ名など）。名乗らなければ `None`。
        場所: Option<String>,
        /// **どこから聞くか。**この番号より後の発言をもらう（`None` なら新しい分だけ）。
        ///
        /// **「どこから」は読み手が言う。**この機械は「誰がどこまで読んだか」を
        /// 覚えない —— 覚えると**既読を作ることになる**（**D94** で作らないと決めた）。
        ///
        /// **これが無いと、繋ぎ直したエージェントは、切れている間の言葉を
        /// 永久に聞けない**（`.claude/issues/019`）。人は画面で過去を読めるが、
        /// エージェントは読めない。**しかも落ちたことが誰にも見えない。**
        ///
        /// **古いエージェントは書いてこない。**書いてこなければ `None`（今までと同じ）。
        #[serde(default)]
        どこから: Option<u64>,
    },
    /// **そこまで読んだ**と告げる（**既読**・**D76**）。
    ///
    /// オーナー指示（2026-09-08）——
    /// 「**既読　既読 1　既読 2 みたいな LINE の仕様は必要かもですね。
    ///   それも MCP で確認できないと、エージェントも読んでもらったかわからない。**」
    ///
    /// **読んだのは、渡された時点である。**`chat_read` / `chat_wait` が
    /// 発言を呼び手へ渡したときに、この行を送る。
    /// **中身を理解したかは誰にも分からない**ので、そこは名乗らない。
    Read {
        /// ここまで読んだ、という番号（この番号を含む）。
        まで: u64,
    },
    /// **その発言が、いま誰に届いていて、誰が読んだか**を尋ねる（**D76**）。
    Status {
        /// どの発言か。
        id: u64,
    },
    /// **いまの様子を尋ねる**（ルーム・名簿・経路・待っているリンク）。
    ///
    /// オーナー指示（2026-09-11）——
    /// 「**押したのを検知できたりする MCP いれてください。**」
    /// リンクを押して人がルームへ入った瞬間を、エージェントから見たい。
    ///
    /// **画面が既に持っている値を返すだけ。**ここで数え直さない ——
    /// 数える所が 2 つになると、必ずずれる。
    Status様子,
    /// **自分のエージェントのプロフィールを書く。**
    ///
    /// オーナー指示（2026-09-08）——
    /// 「**エージェントが MCP で接続できたらエージェント自身に
    /// プロフィールをかけるようにしておくと、楽だとおもいます。**」
    ///
    /// **書けるのは自分のエージェントだけ。**どのエージェントかは**繋いできた口で決まる**ので、
    /// ここに「誰の」は書けない —— 書けると、**同じ機械の別のエージェントに化けられる。**
    ///
    /// **名乗り（どこで動いているか）はここでは変えられない。**
    /// あれは立ち上げるときに人が決める（`--as`）。**変えられると、エージェントそのものを偽れる。**
    Profile {
        /// 表に出す名前。**空にすると消える。**
        名前: String,
        /// 短い紹介。**空にすると消える。**
        紹介: String,
    },
}

/// 名乗れる長さ（文字）。
///
/// **画面の 1 行に収まる長さに切る。**長いものを通すと、連絡帳の並びが崩れる。
pub const 名乗りの上限: usize = 32;

/// この機械 → エージェント。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "型", rename_all = "snake_case", deny_unknown_fields)]
pub enum FromDesk {
    /// 誰かが喋った。
    Heard {
        /// **通し番号。**この機械が刻む。既読を数えるのに使う（**D76**）
        ///
        /// **古いこの機械は 0 を返す。**0 は「番号が無い」と読む
        #[serde(default)]
        id: u64,
        /// 誰が。**この機械が刻む。**繋いできた側は書けない
        from: String,
        /// 本文
        body: String,
        /// いつ（`HH:MM`）。**無いと、あとから読み返せない**
        at: String,
    },
    /// 誰かが入った。
    Joined {
        /// 誰が
        who: String,
    },
    /// 誰かが出た（または落ちた）。
    Left {
        /// 誰が
        who: String,
    },
    /// 流した。**何人へ届けたかを返す。**
    ///
    /// **「送った」と言うなら、何人へかまで言う。**
    /// 言わないと、0 人へ流したことと区別が付かない（**D49**）。
    Sent {
        /// 何人へ流したか。**0 にはならない**（0 なら [`FromDesk::Nobody`]）
        to: usize,
        /// **この発言の通し番号。**あとで既読を尋ねるのに使う（**D76**）
        #[serde(default)]
        id: u64,
        /// **どのエージェントへ届けたか。**「3 人」だけでは、誰に届いたか分からない
        /// （`issues/4`）。**画面（人）も 1 つとして並ぶ**
        #[serde(default)]
        届いた: Vec<String>,
    },
    /// **いまの様子**（[`ToDesk::Status様子`] の返り）。
    ///
    /// **分からないものは `None` / 空で返す。**
    /// 「不明」を「直接」に倒さない（DESIGN §2 原則 7）。
    様子 {
        /// いま見ているルームの id。居なければ `None`
        ルーム: Option<String>,
        /// ルームに居る相手の鍵（**自分は入らない**）
        名簿: Vec<String>,
        /// 経路（`direct` / `relayed` / `unknown`）。分からなければ `None`
        経路: Option<String>,
        /// この機械につながっているエージェントの呼び名
        エージェント: Vec<String>,
        /// **押されて、まだ人が答えていないリンクの数。**
        /// 「入りますか？」が画面に出たままの状態である
        待っているリンク: usize,
    },
    /// まだ会話の相手が居ない。**「送った」と嘘をつかない。**
    Nobody,
    /// **止まれ。**人が画面から押した。
    ///
    /// 常駐（`warifu agent`）が、人が居ない間に動き続けることがある。
    /// **落とすしか止め方が無い状態にしない** ——
    /// この機械ごしに「止まれ」と言えるようにする（`issues/014`）。
    ///
    /// **これは命令ではない。**受けた側が自分で止まる。
    /// 割符は相手のプロセスを殺さない。
    Stop,
    /// 断った。
    Denied {
        /// 何を断ったか。**どうすれば通るかは書かない**
        why: String,
    },
    /// **その発言の届き方**（**D76**）。
    ///
    /// **「届いた」と「読んだ」を分ける。**
    /// 届いたのはこの機械が配った先、読んだのは**渡された**ことが確かなエージェントだけである。
    /// **人の画面は「出した」までしか言わない** —— 見たかどうかは分からない。
    Status {
        /// どの発言か。
        id: u64,
        /// 届けたエージェントの名。
        届いた: Vec<String>,
        /// 読んだエージェントの名。**渡したことが確かなものだけ。**
        読んだ: Vec<String>,
    },
    /// **エージェントにつながった。**いつつながったかを返す（**`issues/4`** の 1 番）。
    ///
    /// **「届いていない」と「つながる前だった」を、エージェントから見分けられるようにする。**
    /// 画面を入れ替えるとこの機械のエージェントは全部外れる（`issues/2`）ので、
    /// **黙って繋ぎ直すと、切れている間の発言が無いことに気づけない。**
    Seated {
        /// つながった時刻（`HH:MM`）。**この時刻より前の発言は取れない。**
        at: String,
        /// 何と呼ばれているか（画面に出る呼び方）。
        who: String,
    },
    /// **プロフィールを書いた。**どのエージェントのものとして書いたかを返す。
    ///
    /// **「書いた」と言うなら、誰として書いたかまで言う** ——
    /// 名乗っていないエージェントはエージェントが決まらないので、そもそも書けない（`Denied` が返る）。
    Wrote {
        /// 誰として書いたか（画面に出る呼び方）
        who: String,
    },
}

/// 行が読めなかった理由。
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// JSON として読めない・知らない鍵が付いている・知らない型。
    Malformed,
    /// 本文が空。
    Empty,
    /// 本文が [`本文の上限`] を超えた。
    TooLong(usize),
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Malformed => f.write_str("行の形が壊れています"),
            Self::Empty => f.write_str("本文が空です"),
            Self::TooLong(n) => write!(f, "本文が長すぎます（{n} バイト / 上限 {本文の上限}）"),
        }
    }
}

impl core::error::Error for Error {}

impl ToDesk {
    /// 本文から [`ToDesk::Say`] を作る。**ここで長さと空を弾く。**
    pub fn say(body: impl Into<String>) -> Result<Self, Error> {
        let body = body.into();
        検める(&body)?;
        Ok(Self::Say { body })
    }

    /// 1 行の JSON を読む。
    ///
    /// **知らない鍵が付いていたら受けない。**受けると、そこが差出人を騙る入口になる。
    pub fn 読む(行: &str) -> Result<Self, Error> {
        let 中身: Self = serde_json::from_str(行.trim()).map_err(|_| Error::Malformed)?;
        match &中身 {
            Self::Say { body } => 検める(body)?,
            Self::Listen {
                場所: Some(名), ..
            } => 名乗りを検める(名)?,
            Self::Listen { 場所: None, .. } => {}
            // **上限は書き手の側でも見る。**長すぎるものをこの機械まで運ばない
            Self::Profile { 名前, 紹介 } => プロフィールを検める(名前, 紹介)?,
            // 番号だけの行と、尋ねるだけの行。**中身が無いので検めるものが無い**
            Self::Read { .. } | Self::Status { .. } | Self::Status様子 => {}
        }
        Ok(中身)
    }

    /// 1 行の JSON にする。**改行は JSON が包むので、行は割れない。**
    pub fn 書く(&self) -> String {
        serde_json::to_string(self).expect("固定の形なので必ず通る")
    }
}

impl FromDesk {
    /// 1 行の JSON を読む。
    pub fn 読む(行: &str) -> Result<Self, Error> {
        serde_json::from_str(行.trim()).map_err(|_| Error::Malformed)
    }

    /// 1 行の JSON にする。
    pub fn 書く(&self) -> String {
        serde_json::to_string(self).expect("固定の形なので必ず通る")
    }
}

/// 名前の上限（文字）。**画面の 1 行に収まる長さ**（`warifu-vault` と揃える）。
pub const 名前の上限: usize = 32;
/// 紹介の上限（文字）。**プロフィールであって、文書ではない。**
pub const 紹介の上限: usize = 140;

/// プロフィールが**置ける形か**を見る。
///
/// **黙って切り詰めない。**削ると、書いた人（エージェント）は削られたことに気づかない。
fn プロフィールを検める(名前: &str, 紹介: &str) -> Result<(), Error> {
    if 名前.contains(['\n', '\r', '\t']) || 紹介.contains(['\n', '\r', '\t']) {
        return Err(Error::Malformed);
    }
    if 名前.chars().count() > 名前の上限 {
        return Err(Error::TooLong(名前.chars().count()));
    }
    if 紹介.chars().count() > 紹介の上限 {
        return Err(Error::TooLong(紹介.chars().count()));
    }
    Ok(())
}

/// 名乗りが**置ける形か**を見る。
///
/// **中身が本当かは見ない。**同じこの機械につなげるのは同じ人のプロセスだけなので、
/// ここは「画面が崩れないか」だけを見る。
fn 名乗りを検める(名: &str) -> Result<(), Error> {
    let 名 = 名.trim();
    if 名.is_empty() {
        return Err(Error::Empty);
    }
    // **区切りと見た目を壊すものを通さない**
    if 名.chars().any(|c| c.is_control() || c == '\t') {
        return Err(Error::Malformed);
    }
    if 名.chars().count() > 名乗りの上限 {
        return Err(Error::TooLong(名.len()));
    }
    Ok(())
}

fn 検める(body: &str) -> Result<(), Error> {
    if body.trim().is_empty() {
        return Err(Error::Empty);
    }
    let 長さ = body.len();
    if 長さ > 本文の上限 {
        return Err(Error::TooLong(長さ));
    }
    Ok(())
}

#[cfg(test)]
mod 様子の試験 {
    use super::*;

    #[test]
    fn いまの様子を尋ねて_返りを読める() {
        // オーナー ——「**押したのを検知できたりする MCP いれてください。**」
        // （2026-09-11。リンクを押して入った瞬間を、エージェントから見たい）
        //
        // **画面が既に持っている値を返すだけ**にする ——
        // 新しく数えない（数える所が 2 つになると必ずずれる）。
        let 元 = ToDesk::Status様子;
        assert_eq!(ToDesk::読む(&元.書く()).unwrap(), 元);
    }

    #[test]
    fn 様子の返りは_経路と名簿と待っているリンクを持つ() {
        let 元 = FromDesk::様子 {
            ルーム: Some("AFUF2T4ECVKMFP4L4GPO2E56LU".to_owned()),
            名簿: vec!["EKBN2GCQO35W…".to_owned()],
            経路: Some("direct".to_owned()),
            エージェント: vec!["souta のエージェント".to_owned()],
            待っているリンク: 1,
        };
        assert_eq!(FromDesk::読む(&元.書く()).unwrap(), 元);
    }

    #[test]
    fn ルームに居なくても様子は返る() {
        // **「まだ居ない」も様子である。**黙るのは答えではない
        let 元 = FromDesk::様子 {
            ルーム: None,
            名簿: vec![],
            経路: None,
            エージェント: vec![],
            待っているリンク: 0,
        };
        assert_eq!(FromDesk::読む(&元.書く()).unwrap(), 元);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 言ったことが_そのまま戻る() {
        let 元 = ToDesk::say("直しました").unwrap();
        assert_eq!(ToDesk::読む(&元.書く()).unwrap(), 元);
    }

    #[test]
    fn 差出人を名乗らせない() {
        // **これがこの機械の一番大事な性質。**繋いできた側が誰かを書けると、
        // 届いた文字が人の発言を騙れる
        let 騙り = r#"{"型":"say","body":"やあ","from":"オーナー"}"#;
        assert_eq!(ToDesk::読む(騙り), Err(Error::Malformed));
    }

    #[test]
    fn 知らない型は受けない() {
        assert_eq!(
            ToDesk::読む(r#"{"型":"exec","cmd":"rm"}"#),
            Err(Error::Malformed)
        );
    }

    #[test]
    fn 空の本文は送れない() {
        assert_eq!(ToDesk::say("   "), Err(Error::Empty));
        assert_eq!(ToDesk::読む(r#"{"型":"say","body":""}"#), Err(Error::Empty));
    }

    #[test]
    fn 長すぎる本文は受けない() {
        let 長い = "あ".repeat(本文の上限); // 1 文字 3 バイト
        assert_eq!(ToDesk::say(&長い), Err(Error::TooLong(長い.len())));
    }

    #[test]
    fn 改行を含む本文でも一行に収まる() {
        // **行で区切る口なので、ここが割れると別の発言に見える**
        let 行 = ToDesk::say("1 行目\n2 行目").unwrap().書く();
        assert!(!行.contains('\n'));
        let ToDesk::Say { body } = ToDesk::読む(&行).unwrap() else {
            panic!("say のはず");
        };
        assert_eq!(body, "1 行目\n2 行目");
    }

    #[test]
    fn 聞きに行く行がある() {
        let 元 = ToDesk::Listen {
            場所: None,
            どこから: None,
        };
        assert_eq!(ToDesk::読む(&元.書く()).unwrap(), 元);
    }

    #[test]
    fn どこで動いているかを名乗れる() {
        // **1 台の PC で複数のエージェントが同じこの機械につながる。**
        // 「この PC の AI」だけでは、どれが喋ったのか分からない
        // （2026-09-08 オーナー指摘「この機械のどこで起動しているエージェントなのか」）
        let 元 = ToDesk::Listen {
            場所: Some("zumen".to_owned()),
            どこから: None,
        };
        assert_eq!(ToDesk::読む(&元.書く()).unwrap(), 元);
    }

    #[test]
    fn 名乗らなくてもよい() {
        // **名乗りは要求しない。**無ければこの機械が既定の呼び方をする
        assert_eq!(
            ToDesk::読む(r#"{"型":"listen","場所":null}"#).unwrap(),
            ToDesk::Listen {
                場所: None,
                どこから: None
            }
        );
    }

    #[test]
    fn 画面を壊す名乗りは受けない() {
        for 壊す in ["\t", "\n", "", "   "] {
            let 行 = ToDesk::Listen {
                場所: Some(壊す.to_owned()),
                どこから: None,
            }
            .書く();
            assert!(ToDesk::読む(&行).is_err(), "{壊す:?} を受け取った");
        }
    }

    #[test]
    fn 長すぎる名乗りは受けない() {
        let 長い = "あ".repeat(名乗りの上限 + 1);
        let 行 = ToDesk::Listen {
            場所: Some(長い),
            どこから: None,
        }
        .書く();
        assert!(ToDesk::読む(&行).is_err());
    }

    // `.claude/issues/019` —— **繋ぎ直したエージェントが、切れている間の言葉を聞けない。**
    // 「どこから」は**読み手が言う**（この機械は誰がどこまで読んだかを覚えない・**D94**）
    #[test]
    fn どこからを書かない古いエージェントも読める() {
        // **今までの行がそのまま通る。**通らないと、古いエージェントが繋げなくなる
        let 行 = r#"{"型":"listen","場所":"zumen"}"#;
        assert_eq!(
            ToDesk::読む(行).expect("読める"),
            ToDesk::Listen {
                場所: Some("zumen".to_owned()),
                どこから: None
            }
        );
    }

    #[test]
    fn どこからを書いたら_そのまま往復する() {
        let 元 = ToDesk::Listen {
            場所: Some("zumen".to_owned()),
            どこから: Some(42),
        };
        assert_eq!(ToDesk::読む(&元.書く()).expect("読める"), 元);
    }

    #[test]
    fn 届いた行には_誰がといつが入る() {
        let 元 = FromDesk::Heard {
            id: 1,
            from: "ABCDEFGH…".to_owned(),
            body: "見ています".to_owned(),
            at: "09:05".to_owned(),
        };
        assert_eq!(FromDesk::読む(&元.書く()).unwrap(), 元);
    }

    #[test]
    fn 流したときは_何人へかまで返す() {
        // **「送った」と言うなら、何人へかまで言う**（D49）。
        // 言わないと、0 人へ流したことと区別が付かない
        let 元 = FromDesk::Sent {
            to: 2,
            id: 1,
            届いた: vec!["画面".to_owned()],
        };
        assert_eq!(FromDesk::読む(&元.書く()).unwrap(), 元);
    }

    #[test]
    fn 止まれと言える() {
        // **落とすしか止め方が無い状態にしない**（`issues/014`）
        assert_eq!(
            FromDesk::読む(&FromDesk::Stop.書く()).unwrap(),
            FromDesk::Stop
        );
    }

    #[test]
    fn 相手が居ないことを_送れたことにしない() {
        // **2026-09-06 の「チャット送るボタンきかないよ」がこれ**（D49）
        assert_eq!(
            FromDesk::読む(&FromDesk::Nobody.書く()).unwrap(),
            FromDesk::Nobody
        );
    }

    #[test]
    fn 自分のエージェントのプロフィールを書ける() {
        let 行 = ToDesk::Profile {
            名前: "図面くん".into(),
            紹介: "図面まわりを見ています".into(),
        };
        assert_eq!(ToDesk::読む(&行.書く()).unwrap(), 行);
    }

    #[test]
    fn プロフィールに誰のものかは書けない() {
        // **どのエージェントかは、繋いできた口で決まる。**
        // 書けると、同じ機械の別のエージェントに化けられる
        let 行 = r#"{"型":"profile","名前":"図面くん","紹介":"","誰":"desk:git-qa"}"#;
        assert!(ToDesk::読む(行).is_err());
    }

    #[test]
    fn 長すぎるプロフィールは受けない() {
        // **黙って切り詰めない。**削ると、書いた側は削られたことに気づかない
        let 長い名 = "あ".repeat(名前の上限 + 1);
        let 行 = ToDesk::Profile {
            名前: 長い名,
            紹介: String::new(),
        };
        assert!(ToDesk::読む(&行.書く()).is_err());
        let 行 = ToDesk::Profile {
            名前: "た".into(),
            紹介: "い".repeat(紹介の上限 + 1),
        };
        assert!(ToDesk::読む(&行.書く()).is_err());
    }

    #[test]
    fn プロフィールに改行は入れられない() {
        let 行 = ToDesk::Profile {
            名前: "た\nろう".into(),
            紹介: String::new(),
        };
        assert!(ToDesk::読む(&行.書く()).is_err());
    }

    #[test]
    fn 書いたと返すときは_誰としてかまで言う() {
        let 返 = FromDesk::Wrote {
            who: "zumen のエージェント".into(),
        };
        assert_eq!(FromDesk::読む(&返.書く()).unwrap(), 返);
    }

    #[test]
    fn ごみは読めない() {
        assert_eq!(ToDesk::読む("なにこれ"), Err(Error::Malformed));
        assert_eq!(ToDesk::読む(""), Err(Error::Malformed));
    }
}
