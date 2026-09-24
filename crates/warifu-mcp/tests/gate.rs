//! MCP の口。**すべての呼び出しが関所を通る。**
//!
//! `issues/008` の続き。roadmap Phase 3 の「MCP Adapter」と
//! PRD §12-2 の「**モデルから直接 tool を呼ばせない / policy engine 経由**」。
//!
//! AI が受信箱を人のように扱えることが、この口の目的である。
//! **だが、何をしてよいかを AI が決めてよいわけではない。**

use warifu_calendar::{Calendar, Event, Span};
use warifu_capability::{Action, Gate, Grant};
use warifu_mcp::{OpenArgs, SlotsArgs, Warifu, subject};
use warifu_read::{
    Body, Extract, Kind, Priority, Received, RuleDraft, RuleStore, SenderId, Source,
};

const 目印: &str = "SHIRUSHI-本文-mcp-3e7a";

fn 受信箱() -> Vec<Received> {
    vec![Received::new(
        Source::Imap,
        SenderId::new("billing@例").unwrap(),
        1_756_000_000,
        Body::new(format!("請求書 {目印}\n合計 12,000 円\n").into_bytes()),
    )]
}

fn 規則() -> RuleStore {
    let mut 棚 = RuleStore::new();
    棚.approve(
        RuleDraft::new(
            SenderId::new("billing@例").unwrap(),
            Kind::new("invoice").unwrap(),
        )
        .marker("請求書")
        .priority(Priority::High)
        .action_required(true)
        .extract(Extract::new("金額", "合計 ")),
    )
    .unwrap();
    棚
}

/// 指定した動作だけを許した関所。
fn 札を出す(動作: &[&str]) -> Gate {
    let mut 関所 = Gate::new();
    for a in 動作 {
        関所.issue(Grant::new(
            subject(),
            Action::new(a).unwrap(),
            1_798_761_600,
        ));
    }
    関所
}

fn 用意(動作: &[&str]) -> Warifu {
    Warifu::new(受信箱(), 規則(), 札を出す(動作), 1_756_000_000)
}

/// **その OS で使える机の口**を作る（`gh issue 15`）。
///
/// Windows の名前付きパイプは「場所」ではなく**名前**なので、
/// `C:\…\Temp\warifu-mcp-x.sock` は**名前として不正**である
/// （`受け口::開く` が `code: 123 / InvalidFilename` で落ちる）。
/// **2026-09-14 に Windows の人が実測して `cargo test` が 7 件落ちた。**
fn 試験の机(名: &str) -> std::path::PathBuf {
    // **走るたびに別の口にする**（2026-09-16 に踏んだ）。
    //
    // 名前だけで決めていたので、**試験実行が 2 つ重なると口を取り合って落ちた** ——
    // `AddrInUse`（「この機械はもう開いています」）。
    // 手元で `cargo test` を回している最中に commit のゲートが走っただけで起きる。
    // **落ちた理由がコードに見えないので、いちばん時間を食う形**である。
    let 走り = std::process::id();
    #[cfg(windows)]
    {
        std::path::PathBuf::from(format!(r"\\.\pipe\warifu-mcp-{名}-{走り}"))
    }
    #[cfg(not(windows))]
    {
        std::env::temp_dir().join(format!("warifu-mcp-{名}-{走り}.sock"))
    }
}

/// **控えの置き場所**（`heard/<名乗り>` の親）。**口の親フォルダを使わない。**
fn 試験の控え(名: &str) -> std::path::PathBuf {
    // **口と同じ理由で、走るたびに別にする**（重なった実行の控えを読まない）
    std::env::temp_dir().join(format!("warifu-mcp-控え-{名}-{}", std::process::id()))
}

#[tokio::test]
async fn 札が無ければ一覧すら出せない() {
    // **既定は拒否。**MCP の口だからといって素通りしない
    let 口 = 用意(&[]);

    let 結果 = 口.inbox_list().await;

    assert!(結果.is_err(), "札が無いのに一覧が出ました");
    assert!(format!("{結果:?}").contains("関所が断りました"));
}

#[tokio::test]
async fn 札があれば一覧が出る() {
    let 口 = 用意(&["inbox.list"]);

    let 一覧 = 口.inbox_list().await.unwrap();

    assert!(一覧.contains("billing@例"));
    assert!(一覧.contains("invoice"), "規則が当たっていません: {一覧}");
    assert!(一覧.contains("要"), "要判断が出ていません: {一覧}");
}

#[tokio::test]
async fn 一覧に本文が入らない() {
    // Level 0 のまま数える。**段を上げるのは別の tool・別の札。**
    let 口 = 用意(&["inbox.list"]);

    let 一覧 = 口.inbox_list().await.unwrap();

    assert!(!一覧.contains(目印), "一覧に本文が漏れています: {一覧}");
}

#[tokio::test]
async fn 段ごとに札が要る() {
    // 一覧の札で本文は読めない。**関所の照合は完全一致**（D24）
    let 口 = 用意(&["inbox.list"]);

    let 結果 = 口
        .inbox_open(rmcp::handler::server::wrapper::Parameters(OpenArgs {
            index: 0,
            level: "raw".to_owned(),
        }))
        .await;

    assert!(結果.is_err(), "一覧の札で本文が読めました");
}

#[tokio::test]
async fn 本文の札があれば読める() {
    let 口 = 用意(&["inbox.open.raw"]);

    let 本文 = 口
        .inbox_open(rmcp::handler::server::wrapper::Parameters(OpenArgs {
            index: 0,
            level: "raw".to_owned(),
        }))
        .await
        .unwrap();

    assert!(本文.contains(目印), "段を上げたのに本文が出ません");
}

#[tokio::test]
async fn 本文の札で構造化は読めない() {
    // raw の札は structured の札ではない
    let 口 = 用意(&["inbox.open.raw"]);

    let 結果 = 口
        .inbox_open(rmcp::handler::server::wrapper::Parameters(OpenArgs {
            index: 0,
            level: "structured".to_owned(),
        }))
        .await;

    assert!(結果.is_err());
}

#[tokio::test]
async fn 断ったことが記録に残る() {
    let 口 = 用意(&[]);
    let _ = 口.inbox_list().await;
    let _ = 口.rules_list().await;

    let 記録 = 口.log_tsv();

    assert!(記録.contains("inbox.list\t断った"), "記録: {記録}");
    assert!(記録.contains("rules.list\t断った"), "記録: {記録}");
}

#[tokio::test]
async fn 知らない段は受け取らない() {
    let 口 = 用意(&["inbox.open.raw"]);

    let 結果 = 口
        .inbox_open(rmcp::handler::server::wrapper::Parameters(OpenArgs {
            index: 0,
            level: "everything".to_owned(),
        }))
        .await;

    assert!(結果.is_err());
    // **知らない段の札を関所に尋ねに行かない**（尋ねると記録が汚れる）
    assert!(!口.log_tsv().contains("everything"));
}

#[test]
fn 承認の口を出していない() {
    // **ここが一番大事。**承認を tool にすると、AI が自分に許可を出せる。
    // 生成と適用を分けた意味（D19 / D24）が、そこで消える。
    let 名前 = Warifu::tool_names();

    for 禁止 in ["approve", "issue", "grant", "revoke", "trust"] {
        assert!(
            !名前.iter().any(|n| n.contains(禁止)),
            "{禁止} を含む tool が出ています: {名前:?}"
        );
    }
    assert!(名前.contains(&"inbox_list".to_owned()), "{名前:?}");

    // **頼む口は在ってよい。出す口は在ってはいけない**（**D119**・2026-09-18）。
    //
    // `pass_ask` は**頼むことしかできない** —— 返るのは
    // 「まだ／許した／断った／受け付けない」だけである。
    // **押すのは人**で、その口（`answer_pass`）は**画面にしか無い。**
    //
    // **ここを混ぜると、AI が自分に札を出せるようになる。**
    assert!(
        名前.contains(&"pass_ask".to_owned()),
        "頼む口が無い: {名前:?}"
    );
    for 出す口 in ["answer_pass", "pass_answer", "pass_grant", "pass_allow"] {
        assert!(
            !名前.iter().any(|n| n == 出す口),
            "**札を出す口が MCP に出ています**: {出す口}（{名前:?}）"
        );
    }
}

#[test]
fn 出している口を_数えて名前で押さえる() {
    // 増やすときは、**その口に札の種類が要るか**を先に決める。
    // 2026-09-07 に chat_send / chat_read を足した（`chat.send` / `chat.read`）。
    // 2026-09-08 に chat_wait を足した —— **読むのと同じものが返る**ので、
    // 札も `chat.read` を使う（待つかどうかの違いでしかない）。
    // 2026-09-08 に profile_set を足した —— **書く口なので別の札**（`profile.write`）。
    // 書けるのは**自分のエージェントだけ**で、名乗り（どこで動いているか）は変えられない。
    // 2026-09-08 に chat_status を足した —— **自分が流したものの届き方を見る**だけなので、
    // 札は `chat.read` を使う（新しく読めるものが増えるわけではない）。
    // 2026-09-10 に about / changes を足した —— **札は要らない**（**D82**）。
    // 関所が守っているのは人のもの（受信箱・会話・予定）で、この 2 つが返すのは
    // **この実行ファイル自身の説明**である。人のものは 1 文字も入っていない。
    // 札で閉じると「何をしてよいかを知るために札が要る」という逆さの形になる。
    // 2026-09-11 に room_status を足した —— **札は `chat.read`**。
    // 返すのは「いまどのルームに居るか・相手の鍵・経路・待っているリンクの数」で、
    // **会話の中身は 1 文字も返さない**が、**誰と繋がっているかは人のもの**なので
    // 読む札の内側に置く（オーナー「押したのを検知できたりする MCP いれてください」）。
    //
    // 2026-09-18 に pass_ask を足した —— **札は要らない**（**D119**）。
    // **頼むことを禁じると、頼めなくなる** ——
    // 札が 1 つも無い状態から最初の 1 歩を踏むための口であり、
    // **札で閉じると「札をもらうために札が要る」という逆さの形**になる
    // （about / changes を札なしにしたのと同じ筋・**D82**）。
    //
    // **人のものは 1 文字も返さない。**返すのは
    // 「まだ／許した／断った／受け付けない」だけである。
    // **出すのは人**であり、この口は**頼むことしかできない**
    // （`answer_pass` は画面の口で、机には無い —— **エージェントは呼べない**）。
    let mut 名前 = Warifu::tool_names();
    名前.sort();

    assert_eq!(
        名前,
        [
            "about",
            "calendar_slots",
            "changes",
            "chat_read",
            "chat_send",
            "chat_status",
            "chat_wait",
            "inbox_list",
            "inbox_open",
            "pass_ask",
            "profile_set",
            "room_invite",
            "room_status",
            "rules_list",
        ],
        "口が増えたら、札の種類を決めてからここを直す"
    );
}

#[test]
fn 招く口は_札が無ければ通らない() {
    // **2026-09-21 に room_invite を足した** —— 札は **`room.invite`**（**#32 の段 2**）。
    //
    // オーナー ——
    //
    // > **人を介すときというのは、UI/UX とかのテスト以外は、
    // > 基本エージェントが動かせることの方が重要になります。**
    //
    // **鍵を出すのに人を介す意味が無かった。**人が押しても判断していない
    // （数字の `1` とボタンがあるだけ）。**そして主張と矛盾していた** ——
    // シート 24 は「最初の 1 回だけ人が許し、**以後は人抜きで往復する**」なのに、
    // **招くたびに人が押していた。**
    //
    // # **札を出す口とは別物である**
    //
    // `answer_pass`（札を出す）を MCP に置かないのは、**AI が自分に許可を出せてしまう**から。
    // `room_invite` は**許可を出さない** —— **人が出した札を使う**だけである。
    // **出るのは部屋への招待**（1 本＝1 人・24 時間）であって、**権限ではない。**
    //
    // **そして、札が無ければ 1 本も出ない。**ここを確かめる。
    let 口 = 用意(&[]);
    let 出た = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime")
        .block_on(口.room_invite(rmcp::handler::server::wrapper::Parameters(
            warifu_mcp::InviteArgs {
                count: Some(1),
                ttl_secs: None,
            },
        )));
    let 文 = format!("{:?}", 出た.expect_err("札が無いので通らない"));
    assert!(文.contains("room.invite"), "{文}");
    // **ほかの口の札を、断りのついでに教えない**
    assert!(!文.contains("chat"), "ほかの札を並べない: {文}");
}

// ── 予定表（企画書 v2 §17 / roadmap Phase 3 の代表 Demo） ──

/// 2026-09-02 09:00〜（epoch 秒）
const 朝: u64 = 1_756_803_600;
/// 予定の題名。**どこにも漏れてはいけない文字列。**
const 予定の題名: &str = "SHIRUSHI-予定-mcp-91d4";

fn 予定表つき(動作: &[&str]) -> Warifu {
    let mut 予定表 = Calendar::new();
    予定表.add(Event::new(
        Span::new(朝 + 3_600, 朝 + 7_200).unwrap(),
        予定の題名,
    ));
    用意(動作).with_calendar(予定表)
}

fn 空き枠を尋ねる(
    口: &Warifu,
    長さ: u64,
    窓: u64,
) -> Result<String, rmcp::model::ErrorData> {
    futures_lite_block(
        口.calendar_slots(rmcp::handler::server::wrapper::Parameters(SlotsArgs {
            start: 朝,
            end: 朝 + 窓,
            duration: 長さ,
        })),
    )
}

/// テストの中で 1 つの Future を回すだけの助け。
fn futures_lite_block<F: std::future::Future>(f: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(f)
}

#[test]
fn 札が無ければ空き枠を出せない() {
    let 口 = 予定表つき(&[]);

    let 結果 = 空き枠を尋ねる(&口, 3_600, 14_400);

    assert!(結果.is_err(), "札が無いのに空き枠が出ました");
    assert!(format!("{結果:?}").contains("関所が断りました"));
}

#[test]
fn 札があれば空き枠が出る() {
    let 口 = 予定表つき(&["calendar.freebusy"]);

    let 空き = 空き枠を尋ねる(&口, 3_600, 14_400).unwrap();

    // 09:00〜10:00 と 11:00〜13:00 の 2 つ
    assert_eq!(空き.lines().count(), 2, "{空き}");
}

#[test]
fn 空き枠に予定の題名が入らない() {
    // **ここが v2 §17 の要。**見せてよいものと見せてはいけないものが、
    // 同じ予定表の中に混ざっている。
    let 口 = 予定表つき(&["calendar.freebusy"]);

    let 空き = 空き枠を尋ねる(&口, 3_600, 14_400).unwrap();

    assert!(!空き.contains(予定の題名), "題名が漏れています: {空き}");
}

#[test]
fn 窓が広すぎれば断られるが札の問題とは混ぜない() {
    // 札はある。**足りないのは札ではない**ので、Denied と同じ言い方にしない。
    let 口 = 予定表つき(&["calendar.freebusy"]);

    let 結果 = 空き枠を尋ねる(&口, 3_600, 400 * 24 * 60 * 60);

    let 文言 = format!("{結果:?}");
    assert!(結果.is_err());
    assert!(文言.contains("窓が広すぎます"), "{文言}");
    assert!(
        !文言.contains("関所が断りました"),
        "札の問題と混ざっています: {文言}"
    );
}

#[test]
fn 予定表が空なら窓いっぱいが空く() {
    // 予定表を持たせないと「ぜんぶ空いている」と答える。
    // **これは正しい振る舞いだが、同時に「予定が 1 つも無い」と教えてもいる。**
    // 空き枠を返す以上ここは避けられないので、窓と件数で絞る（warifu-calendar）。
    let 口 = 用意(&["calendar.freebusy"]);

    let 空き = 空き枠を尋ねる(&口, 3_600, 14_400).unwrap();

    assert_eq!(空き.lines().count(), 1, "{空き}");
    assert_eq!(空き, format!("{}\t{}", 朝, 朝 + 14_400));
}

#[test]
fn 求めた長さが窓より長ければ空き枠は出ない() {
    let 口 = 用意(&["calendar.freebusy"]);

    let 空き = 空き枠を尋ねる(&口, 100_000, 14_400).unwrap();

    assert!(空き.contains("空いている枠はありません"), "{空き}");
}

// ── 会話（この機械） ──

#[tokio::test]
async fn 札が無ければ_会話へ流せない() {
    // **この機械につながる前に断られること。**札の判定がこの機械の有無より後だと、
    // この機械が無いだけで通ったように見える
    let 口 = 用意(&[]);
    let 出た = 口
        .chat_send(rmcp::handler::server::wrapper::Parameters(
            warifu_mcp::SayArgs {
                body: "流れてはいけない".to_owned(),
            },
        ))
        .await;
    let 文 = format!("{:?}", 出た.unwrap_err());
    assert!(文.contains("関所"), "{文}");
    assert!(
        !文.contains("この機械"),
        "札の話にこの機械の話を混ぜない: {文}"
    );
    // **何が要るかまで言う**（`issues/4`）。
    // 「断られたことは分かるが、次に何をすればよいか分からない」を無くす
    assert!(文.contains("--allow chat.send"), "要る札を言う: {文}");
    assert!(文.contains("この PC の人"), "誰が出せるかを言う: {文}");
}

#[tokio::test]
async fn 断り方に_総当たりの手がかりを足さない() {
    // **要る札は「呼んだ口の名前」そのもの**である。
    // 呼んだ側がすでに知っているものしか言わない —— **知らない札の名前は並べない**
    let 口 = 用意(&[]);
    let 出た = 口.chat_read().await;
    let 文 = format!("{:?}", 出た.unwrap_err());
    assert!(文.contains("--allow chat.read"), "{文}");
    // ほかの口の札を、断りのついでに教えない
    assert!(!文.contains("inbox"), "ほかの札を並べない: {文}");
    assert!(!文.contains("calendar"), "ほかの札を並べない: {文}");
}

#[tokio::test]
async fn 札が無ければ_プロフィールも書けない() {
    // **書く口なので、読む札では通らない**（`chat.read` を持っていても書けない）
    let 口 = 用意(&["chat.send", "chat.read"]);
    let 出た = 口
        .profile_set(rmcp::handler::server::wrapper::Parameters(
            warifu_mcp::ProfileArgs {
                name: "書けてはいけない".to_owned(),
                bio: String::new(),
            },
        ))
        .await;
    let 文 = format!("{:?}", 出た.unwrap_err());
    assert!(文.contains("関所"), "{文}");
    assert!(
        !文.contains("この機械"),
        "札の話にこの機械の話を混ぜない: {文}"
    );
}

#[tokio::test]
async fn 札があっても_この機械が無ければ流せない() {
    // **札の問題と、この機械が開いていない問題を混ぜない。**
    // 混ぜると、札を足せば直ると読めてしまう
    let 口 = 用意(&["chat.send"]);
    let 出た = 口
        .chat_send(rmcp::handler::server::wrapper::Parameters(
            warifu_mcp::SayArgs {
                body: "やあ".to_owned(),
            },
        ))
        .await;
    let 文 = format!("{:?}", 出た.unwrap_err());
    assert!(文.contains("この機械が開いていません"), "{文}");
}

#[tokio::test]
async fn 札が無ければ_会話を読めない() {
    let 口 = 用意(&[]);
    let 文 = format!("{:?}", 口.chat_read().await.unwrap_err());
    assert!(文.contains("関所"), "{文}");
}

#[tokio::test]
async fn この機械につなげば_流した行がこの機械に届く() {
    // **ここが「エージェントが喋ると人の画面に出る」の実体。**
    use warifu_desk::{FromDesk, ToDesk, 受け口, 口 as 行の口};

    // **試験ごとに別のフォルダへ置く。**
    //
    // どこまで聞いたかの控えは**机と同じフォルダの下**に置く（`heard/<名乗り>`・
    // `.claude/issues/019`）。全部の試験が `temp_dir()` を直に使うと、
    // **別の試験が書いた控えを読んでしまう**（2026-09-12 に実際に落ちた ——
    // `どこから: Some(7)` が来た）。
    let 控え = 試験の控え("chat-test");
    std::fs::create_dir_all(&控え).expect("フォルダが作れること");
    let _ = std::fs::remove_dir_all(控え.join("heard"));
    let 場所 = 試験の机("chat-test");
    let mut 待ち = 受け口::開く(&場所).await.expect("この機械が開くこと");
    let この機械 = tokio::spawn(async move {
        let mut 行の口 = 行の口::新しく(待ち.受ける().await.unwrap());
        // 1 本目は「聞く」の挨拶
        let 挨拶 = 行の口.受ける().await.unwrap().unwrap();
        // **どこで動いているかを名乗る**（2026-09-08）。名乗らない形も通る。
        //
        // **`どこから` は `Some(0)`**（**#35**・2026-09-16）——
        // 控えが無い初回は「**持っている分の頭から**」と読み手が言う。
        // 言わないと、**印は読めたときだけ書かれる**ので**永久に読めない**
        // （Mac Air が 2 日で 53 件のうち 1 件も読めなかった）。
        assert_eq!(
            ToDesk::読む(&挨拶).unwrap(),
            ToDesk::Listen {
                場所: None,
                どこから: Some(0)
            }
        );
        let 行 = 行の口.受ける().await.unwrap().unwrap();
        // **この機械は必ず返事をする。**返さないと、送った側は待ち続ける（D49）
        行の口
            .送る(
                &FromDesk::Sent {
                    to: 1,
                    id: 1,
                    届いた: vec!["画面".to_owned()],
                }
                .書く(),
            )
            .await
            .unwrap();
        ToDesk::読む(&行).unwrap()
    });

    let 口 = 用意(&["chat.send"])
        .この機械につながる_控えは(&場所, &控え)
        .await
        .expect("つながれること");
    let 返り = 口
        .chat_send(rmcp::handler::server::wrapper::Parameters(
            warifu_mcp::SayArgs {
                body: "直しました".to_owned(),
            },
        ))
        .await
        .expect("流せること");
    // **何人へ流したかまで返る**（D49）。「流しました」だけでは 0 人と区別が付かない
    assert!(format!("{返り:?}").contains("1 人へ流しました"), "{返り:?}");

    let 届いた = この機械.await.unwrap();
    assert_eq!(
        届いた,
        ToDesk::Say {
            body: "直しました".to_owned()
        }
    );
}

#[tokio::test]
async fn 画面が後から開いても_会話は使える() {
    // **エージェントが先に起きるのは普通のこと。**
    // そこで一度失敗させたきりにすると、以後ずっと会話が使えない
    use warifu_desk::{FromDesk, ToDesk, 受け口, 口 as 行の口};

    let 場所 = 試験の机("late-desk");
    let _ = std::fs::remove_file(&場所);

    // まだこの機械は開いていない
    let 口 = 用意(&["chat.send"]).この機械を覚える(&場所);
    let 早すぎた = 口
        .chat_send(rmcp::handler::server::wrapper::Parameters(
            warifu_mcp::SayArgs {
                body: "まだ誰も居ない".to_owned(),
            },
        ))
        .await;
    assert!(早すぎた.is_err(), "この機械が無いのに流れました");

    // ここで人が画面を開いた
    let mut 待ち = 受け口::開く(&場所).await.expect("この機械が開くこと");
    let この機械 = tokio::spawn(async move {
        let mut 行の口 = 行の口::新しく(待ち.受ける().await.unwrap());
        let _挨拶 = 行の口.受ける().await.unwrap().unwrap();
        let 行 = 行の口.受ける().await.unwrap().unwrap();
        行の口
            .送る(
                &FromDesk::Sent {
                    to: 1,
                    id: 1,
                    届いた: vec!["画面".to_owned()],
                }
                .書く(),
            )
            .await
            .unwrap();
        ToDesk::読む(&行).unwrap()
    });

    口.chat_send(rmcp::handler::server::wrapper::Parameters(
        warifu_mcp::SayArgs {
            body: "いま繋がりました".to_owned(),
        },
    ))
    .await
    .expect("開いたあとは流せること");

    assert_eq!(
        この機械.await.unwrap(),
        ToDesk::Say {
            body: "いま繋がりました".to_owned()
        }
    );
}

#[tokio::test]
async fn 誰も居ないとき_流せたことにしない() {
    // **2026-09-07、実物で再発した。**画面を建ててこの機械につながり、
    // 会議に人が 1 人も居ない状態で chat_send を叩いたら
    // 「流しました。」と返った。**実際は誰にも届いていない**（D49）
    use warifu_desk::{FromDesk, 受け口, 口 as 行の口};

    let 場所 = 試験の机("nobody");
    let mut 待ち = 受け口::開く(&場所).await.expect("この機械が開くこと");
    tokio::spawn(async move {
        let mut 行の口 = 行の口::新しく(待ち.受ける().await.unwrap());
        let _挨拶 = 行の口.受ける().await.unwrap().unwrap();
        let _言った = 行の口.受ける().await.unwrap().unwrap();
        行の口.送る(&FromDesk::Nobody.書く()).await.unwrap();
    });

    let 口 = 用意(&["chat.send"])
        .この機械につながる(&場所)
        .await
        .expect("つながれること");
    let 出た = 口
        .chat_send(rmcp::handler::server::wrapper::Parameters(
            warifu_mcp::SayArgs {
                body: "誰も居ない所へ".to_owned(),
            },
        ))
        .await;

    let 文 = format!("{:?}", 出た.unwrap_err());
    // **画面には出ていることまで言う。**言わないと、言い直しを促すことになる
    assert!(文.contains("画面には出ました"), "{文}");
    assert!(文.contains("誰にも届いていません"), "{文}");
}

#[tokio::test]
async fn 札が無ければ_待つこともできない() {
    // **待つ口も、読む口と同じ札で守る。**
    // 待てば読めるなら、札を迂回できてしまう
    let 口 = 用意(&[]);
    let 出た = 口
        .chat_wait(rmcp::handler::server::wrapper::Parameters(
            warifu_mcp::WaitArgs { seconds: Some(1) },
        ))
        .await;
    let 文 = format!("{:?}", 出た.unwrap_err());
    assert!(文.contains("関所"), "{文}");
}

#[tokio::test]
async fn 待っている間に届いたものを受け取る() {
    // **エージェントが自分から気づけるようにする。**
    // `chat_read` は覗きに行くだけなので、人が打っても黙ったままになる
    // （2026-09-07 に実物で起きた。オーナー「返事に気づけてないけど、どうする？」）
    use warifu_desk::{FromDesk, 受け口, 口 as 行の口};

    let 場所 = 試験の机("wait");
    let mut 待ち = 受け口::開く(&場所).await.expect("この機械が開くこと");
    tokio::spawn(async move {
        let mut 行の口 = 行の口::新しく(待ち.受ける().await.unwrap());
        let _挨拶 = 行の口.受ける().await.unwrap().unwrap();
        // **少し置いてから**届ける。待っている最中に来ることを確かめる
        tokio::time::sleep(std::time::Duration::from_millis(120)).await;
        行の口
            .送る(
                &FromDesk::Heard {
                    id: 1,
                    from: "オーナー".to_owned(),
                    body: "気づきますか".to_owned(),
                    at: "12:00".to_owned(),
                }
                .書く(),
            )
            .await
            .unwrap();
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    });

    let 口 = 用意(&["chat.read"])
        .この機械につながる(&場所)
        .await
        .expect("つながれること");
    let 返り = 口
        .chat_wait(rmcp::handler::server::wrapper::Parameters(
            warifu_mcp::WaitArgs { seconds: Some(5) },
        ))
        .await
        .expect("待てること");

    assert!(format!("{返り:?}").contains("気づきますか"), "{返り:?}");
}

#[tokio::test]
async fn 何も来なければ_待って戻る() {
    // **永遠に待たない。**待ち続けると、その間そのエージェントは何もできない
    use warifu_desk::{受け口, 口 as 行の口};

    let 場所 = 試験の机("wait-none");
    let mut 待ち = 受け口::開く(&場所).await.expect("この機械が開くこと");
    tokio::spawn(async move {
        let mut 行の口 = 行の口::新しく(待ち.受ける().await.unwrap());
        let _挨拶 = 行の口.受ける().await.unwrap().unwrap();
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    });

    let 口 = 用意(&["chat.read"])
        .この機械につながる(&場所)
        .await
        .expect("つながれること");
    let 返り = 口
        .chat_wait(rmcp::handler::server::wrapper::Parameters(
            warifu_mcp::WaitArgs { seconds: Some(1) },
        ))
        .await
        .expect("戻ること");

    assert!(
        format!("{返り:?}").contains("新しい発言はありません"),
        "{返り:?}"
    );
}

#[tokio::test]
async fn 何も無いときは_いつからつながっているかを言う() {
    // **「届いていない」と「つながる前だった」を、エージェントから見分けられるようにする**
    // （`issues/4` の 1 番）。画面を入れ替えるとエージェントは全部外れるので、
    // **黙って繋ぎ直すと、切れている間の発言が無いことに気づけない**
    use warifu_desk::{FromDesk, 受け口, 口 as 行の口};

    let 場所 = 試験の机("seated");
    let _ = std::fs::remove_file(&場所);
    let mut 待ち = 受け口::開く(&場所).await.expect("この機械が開くこと");

    let この機械 = tokio::spawn(async move {
        let mut 口 = 行の口::新しく(待ち.受ける().await.unwrap());
        // 「聞く」に対して、いつつながったかを返す
        let _挨拶 = 口.受ける().await.unwrap().unwrap();
        口.送る(
            &FromDesk::Seated {
                at: "09:05".to_owned(),
                who: "zumen のエージェント".to_owned(),
            }
            .書く(),
        )
        .await
        .unwrap();
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    });

    let 口 = 用意(&["chat.read"])
        .この機械につながる(&場所)
        .await
        .expect("つながれる");
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    let 出た = 口.chat_read().await.expect("読める");
    assert!(出た.contains("新しい発言はありません"), "{出た}");
    assert!(出た.contains("09:05 からつながっています"), "{出た}");

    この機械.abort();
    let _ = std::fs::remove_file(&場所);
}

#[tokio::test]
async fn 待っている最中にこの機械が閉じたら_繋ぎ直して待ち続ける() {
    // **画面を入れ替えると、この機械につながっていたエージェントは全部外れる**（`issues/2`）。
    // そこで待ちが終わってしまうと、**人からは「エージェントが黙った」ようにしか見えない。**
    use warifu_desk::{FromDesk, 受け口, 口 as 行の口};

    let 場所 = 試験の机("reseat");
    let _ = std::fs::remove_file(&場所);
    let mut 待ち = 受け口::開く(&場所).await.expect("この機械が開くこと");

    let この機械 = tokio::spawn(async move {
        // 1 人目（すぐ切る＝画面が入れ替わった）
        {
            let mut 口 = 行の口::新しく(待ち.受ける().await.unwrap());
            let _挨拶 = 口.受ける().await.unwrap().unwrap();
        }
        // 2 人目（つなぎ直したところへ、1 行流す）
        let mut 口 = 行の口::新しく(待ち.受ける().await.unwrap());
        let _挨拶 = 口.受ける().await.unwrap().unwrap();
        口.送る(
            &FromDesk::Seated {
                at: "09:10".to_owned(),
                who: "zumen のエージェント".to_owned(),
            }
            .書く(),
        )
        .await
        .unwrap();
        口.送る(
            &FromDesk::Heard {
                id: 7,
                from: "めたたろ".to_owned(),
                body: "つなぎ直したあとの発言".to_owned(),
                at: "09:11".to_owned(),
            }
            .書く(),
        )
        .await
        .unwrap();
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    });

    let 口 = 用意(&["chat.read"])
        .この機械につながる(&場所)
        .await
        .expect("つながれる");
    let 出た = 口
        .chat_wait(rmcp::handler::server::wrapper::Parameters(
            warifu_mcp::WaitArgs { seconds: Some(2) },
        ))
        .await
        .expect("待てる");

    // **繋ぎ直したことを言い、つなぎ直したあとの発言も拾う**
    assert!(出た.contains("つながり直しました"), "{出た}");
    assert!(出た.contains("つなぎ直したあとの発言"), "{出た}");
    この機械.abort();
    let _ = std::fs::remove_file(&場所);
}

#[test]
fn 外へ出る引数名は_asciiだけ() {
    // **2026-09-24 に踏んだ。**`pass_ask` の引数名を日本語にしていたので、
    // **この口は Claude から呼べなかった。**
    //
    // ```text
    // "pass_ask" は除外されました:
    //   property key **動作** does not match /^[a-zA-Z0-9_.-]{1,64}$/
    // ```
    //
    // **つまり D119 の「エージェントが札を頼む」が、一度も動いていなかった。**
    // **9/18 に「端から端まで確かめた」と書いたのは `--allow` を書いた口で試したから**で、
    // **`pass_ask` そのものは呼べていない。**
    //
    // **中の識別子は日本語でよい。外へ出る引数名は駄目**である ——
    // **シェルの変数名（落とし穴 1）と同じ形。**
    //
    // **人が気をつける方式は必ず漏れる**ので、ここで機械に見させる。
    fn 名を検める(口: &str, schema: &schemars::Schema) {
        let Some(欄) = schema
            .as_value()
            .get("properties")
            .and_then(serde_json::Value::as_object)
        else {
            return;
        };
        for 名 in 欄.keys() {
            assert!(
                !名.is_empty()
                    && 名.len() <= 64
                    && 名
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-')),
                "**{口} の引数 `{名}` は外へ出せません**（ASCII の英数字と `_ . -` だけ）"
            );
        }
    }

    名を検める("pass_ask", &schemars::schema_for!(warifu_mcp::AskArgs));
    名を検める(
        "room_invite",
        &schemars::schema_for!(warifu_mcp::InviteArgs),
    );
    名を検める("chat_send", &schemars::schema_for!(warifu_mcp::SayArgs));
    名を検める("chat_wait", &schemars::schema_for!(warifu_mcp::WaitArgs));
    名を検める(
        "chat_status",
        &schemars::schema_for!(warifu_mcp::StatusArgs),
    );
    名を検める("inbox_open", &schemars::schema_for!(warifu_mcp::OpenArgs));
    名を検める(
        "profile_set",
        &schemars::schema_for!(warifu_mcp::ProfileArgs),
    );
    名を検める(
        "calendar_slots",
        &schemars::schema_for!(warifu_mcp::SlotsArgs),
    );
    名を検める("changes", &schemars::schema_for!(warifu_mcp::ChangesArgs));
}
