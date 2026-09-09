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
}

#[test]
fn 出している口を_数えて名前で押さえる() {
    // 増やすときは、**その口に札の種類が要るか**を先に決める。
    // 2026-09-07 に chat_send / chat_read を足した（`chat.send` / `chat.read`）。
    // 2026-09-08 に chat_wait を足した —— **読むのと同じものが返る**ので、
    // 札も `chat.read` を使う（待つかどうかの違いでしかない）。
    // 2026-09-08 に profile_set を足した —— **書く口なので別の札**（`profile.write`）。
    // 書けるのは**自分の席だけ**で、名乗り（どこで動いているか）は変えられない。
    // 2026-09-08 に chat_status を足した —— **自分が流したものの届き方を見る**だけなので、
    // 札は `chat.read` を使う（新しく読めるものが増えるわけではない）
    let mut 名前 = Warifu::tool_names();
    名前.sort();

    assert_eq!(
        名前,
        [
            "calendar_slots",
            "chat_read",
            "chat_send",
            "chat_status",
            "chat_wait",
            "inbox_list",
            "inbox_open",
            "profile_set",
            "rules_list",
        ],
        "口が増えたら、札の種類を決めてからここを直す"
    );
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

// ── 会話（机） ──

#[tokio::test]
async fn 札が無ければ_会話へ流せない() {
    // **机に着く前に断られること。**札の判定が机の有無より後だと、
    // 机が無いだけで通ったように見える
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
    assert!(!文.contains("机"), "札の話に机の話を混ぜない: {文}");
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
    assert!(!文.contains("机"), "札の話に机の話を混ぜない: {文}");
}

#[tokio::test]
async fn 札があっても_机が無ければ流せない() {
    // **札の問題と、机が開いていない問題を混ぜない。**
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
    assert!(文.contains("机が開いていません"), "{文}");
}

#[tokio::test]
async fn 札が無ければ_会話を読めない() {
    let 口 = 用意(&[]);
    let 文 = format!("{:?}", 口.chat_read().await.unwrap_err());
    assert!(文.contains("関所"), "{文}");
}

#[tokio::test]
async fn 机に着けば_流した行が机に届く() {
    // **ここが「エージェントが喋ると人の画面に出る」の実体。**
    use warifu_desk::{FromDesk, ToDesk, 受け口, 口 as 行の口};

    let 場所 = std::env::temp_dir().join("warifu-mcp-chat-test.sock");
    let mut 待ち = 受け口::開く(&場所).await.expect("机が開くこと");
    let 机 = tokio::spawn(async move {
        let mut 行の口 = 行の口::新しく(待ち.受ける().await.unwrap());
        // 1 本目は「聞く」の挨拶
        let 挨拶 = 行の口.受ける().await.unwrap().unwrap();
        // **どこで動いているかを名乗る**（2026-09-08）。名乗らない形も通る
        assert_eq!(ToDesk::読む(&挨拶).unwrap(), ToDesk::Listen { 場所: None });
        let 行 = 行の口.受ける().await.unwrap().unwrap();
        // **机は必ず返事をする。**返さないと、送った側は待ち続ける（D49）
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
        .机に着く(&場所)
        .await
        .expect("着けること");
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

    let 届いた = 机.await.unwrap();
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

    let 場所 = std::env::temp_dir().join("warifu-mcp-late-desk.sock");
    let _ = std::fs::remove_file(&場所);

    // まだ机は開いていない
    let 口 = 用意(&["chat.send"]).机を覚える(&場所);
    let 早すぎた = 口
        .chat_send(rmcp::handler::server::wrapper::Parameters(
            warifu_mcp::SayArgs {
                body: "まだ誰も居ない".to_owned(),
            },
        ))
        .await;
    assert!(早すぎた.is_err(), "机が無いのに流れました");

    // ここで人が画面を開いた
    let mut 待ち = 受け口::開く(&場所).await.expect("机が開くこと");
    let 机 = tokio::spawn(async move {
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
        机.await.unwrap(),
        ToDesk::Say {
            body: "いま繋がりました".to_owned()
        }
    );
}

#[tokio::test]
async fn 誰も居ないとき_流せたことにしない() {
    // **2026-09-07、実物で再発した。**画面を建てて机に着き、
    // 会議に人が 1 人も居ない状態で chat_send を叩いたら
    // 「流しました。」と返った。**実際は誰にも届いていない**（D49）
    use warifu_desk::{FromDesk, 受け口, 口 as 行の口};

    let 場所 = std::env::temp_dir().join("warifu-mcp-nobody.sock");
    let mut 待ち = 受け口::開く(&場所).await.expect("机が開くこと");
    tokio::spawn(async move {
        let mut 行の口 = 行の口::新しく(待ち.受ける().await.unwrap());
        let _挨拶 = 行の口.受ける().await.unwrap().unwrap();
        let _言った = 行の口.受ける().await.unwrap().unwrap();
        行の口.送る(&FromDesk::Nobody.書く()).await.unwrap();
    });

    let 口 = 用意(&["chat.send"])
        .机に着く(&場所)
        .await
        .expect("着けること");
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

    let 場所 = std::env::temp_dir().join("warifu-mcp-wait.sock");
    let mut 待ち = 受け口::開く(&場所).await.expect("机が開くこと");
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
        .机に着く(&場所)
        .await
        .expect("着けること");
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

    let 場所 = std::env::temp_dir().join("warifu-mcp-wait-none.sock");
    let mut 待ち = 受け口::開く(&場所).await.expect("机が開くこと");
    tokio::spawn(async move {
        let mut 行の口 = 行の口::新しく(待ち.受ける().await.unwrap());
        let _挨拶 = 行の口.受ける().await.unwrap().unwrap();
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    });

    let 口 = 用意(&["chat.read"])
        .机に着く(&場所)
        .await
        .expect("着けること");
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
