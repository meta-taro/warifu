//! `warifu mcp` —— **MCP の口を、標準入出力で出す。**
//!
//! ここまで `warifu-mcp` はライブラリだけで、**どこからも起動されていなかった。**
//! 「チャットやメールを MCP を通じて行う」ことがこの製品の中心なのに、
//! その口を走らせる経路が 1 本も無かった（2026-09-07 に気づいた）。
//!
//! ```text
//!   Claude Code など ── stdio ── warifu mcp ── この機械 ── 割符の画面（人）
//!                                   ↑ 関所
//! ```
//!
//! **札はここでは作らない。**`--allow` は、この命令を書いた**人**が
//! 何を許すかを書く場所である（`.mcp.json` に人が書く）。
//! 書かなければ何も通らない。**既定は拒否。**

use std::path::PathBuf;

use warifu_capability::{Action, Gate, Grant};
use warifu_mcp::{Warifu, subject};
use warifu_read::RuleStore;

/// 札の効き目（秒）。**24 時間。**
///
/// 短くすると、離エージェントのたびに人が出し直すことになる（D54 と同じ理由）。
const 札の効き目: u64 = 60 * 60 * 24;

/// `--allow` に書ける動作。**知らない名前は、黙って無視せず断る。**
///
/// 綴り違いを黙って通すと、「許したのに動かない」を人が延々と探すことになる。
const 許せる動作: &[&str] = &[
    "chat.send",
    "chat.read",
    // **自分のエージェントのプロフィールだけ**を書ける（**D75**）。名乗り（どこで動いているか）は変えられない
    "profile.write",
    "inbox.list",
    "inbox.open.summary",
    "inbox.open.structured",
    "inbox.open.raw",
    "inbox.open.attachments",
    "calendar.freebusy",
    "rules.list",
];

/// `warifu mcp` の設定。
pub struct 設定 {
    /// 人が許した動作。
    pub 許す: Vec<String>,
    /// この機械の場所。既定は [`warifu_desk::この機械の場所`]。
    pub この機械: PathBuf,
    /// **どこで動いているか。**既定は起動した場所のフォルダ名。
    ///
    /// 1 台の PC で複数のエージェントが同じこの機械につながるので、
    /// 名乗らないと**どれが喋ったのか人に分からない**（2026-09-08）。
    pub 名乗り: Option<String>,
}

/// 今の時刻（秒）。
#[must_use]
pub fn いま() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// 名乗りとして置ける形か。**画面の 1 行に収まる長さに切る。**
///
/// # Errors
/// 空・長すぎるとき。
pub fn 名乗りを検める(名: &str) -> Result<String, String> {
    let 名 = 名.trim().to_owned();
    if 名.is_empty() || 名.chars().count() > warifu_desk::名乗りの上限 {
        return Err(format!(
            "名乗りは 1〜{} 文字にしてください",
            warifu_desk::名乗りの上限
        ));
    }
    Ok(名)
}

/// 起動した場所から名乗る。**まず仕事場（リポジトリ）の名前、無ければフォルダ名。**
///
/// **人が書かなくても、どこで動いているかは分かる。**
/// 取れなければ名乗らない（この機械が既定の呼び方をする）。
///
/// # なぜフォルダ名だけでは足りなかったか（2026-09-20）
///
/// **オーナーが、ASUS の再起動のあと「どのエージェントが動いていたか分からない」**と言った。
/// 記録に残っていたのはこれだけである ——
///
/// ```text
/// この機械: scratchpad から着きました
/// 札: 人に聞きます（chat.send・scratchpad のエージェント）
/// ```
///
/// **`scratchpad` は、その場のフォルダ名**である。
/// **どのプロジェクトの席かを言っていない** ——
/// 探し当てたあとで分かったのは「**別のプロダクトを開発していたエージェント**」だった。
///
/// **仕事場の名前なら、それだけで分かる**（`rendou-kun` / `warifu`）。
/// **`.git` を上へ辿るだけ**にしてある —— `git` を起こさない（速さと、入っていない機械のため）。
pub fn 居場所から名乗る() -> Option<String> {
    let ここ = std::env::current_dir().ok()?;
    let 名 =
        仕事場の名(&ここ).or_else(|| ここ.file_name().map(|n| n.to_string_lossy().into_owned()))?;
    let 名 = 名.trim().to_owned();
    if 名.is_empty() || 名.chars().count() > warifu_desk::名乗りの上限 {
        return None;
    }
    Some(名)
}

/// **仕事場（`.git` のある所）の名前。**無ければ `None`。
///
/// **`git` を起こさない。**`.git` を上へ辿るだけ ——
/// 入っていない機械でも動き、ここで待たされない。
#[must_use]
pub fn 仕事場の名(ここ: &std::path::Path) -> Option<String> {
    let mut 場 = Some(ここ);
    while let Some(p) = 場 {
        if p.join(".git").exists() {
            return p.file_name().map(|n| n.to_string_lossy().into_owned());
        }
        場 = p.parent();
    }
    None
}

/// 引数を読む。
pub fn 読む(args: &mut impl Iterator<Item = String>) -> Result<設定, String> {
    let mut 設 = 設定 {
        許す: Vec::new(),
        この機械: warifu_desk::この機械の場所(),
        名乗り: 居場所から名乗る(),
    };
    while let Some(一つ) = args.next() {
        match 一つ.as_str() {
            "--allow" => {
                let 動作 = args.next().ok_or("--allow のあとに動作がありません")?;
                if !許せる動作.contains(&動作.as_str()) {
                    return Err(format!(
                        "知らない動作です: {動作}\n許せるのは: {}",
                        許せる動作.join(" ")
                    ));
                }
                設.許す.push(動作);
            }
            "--desk" => {
                設.この機械 = PathBuf::from(args.next().ok_or("--desk のあとに場所がありません")?);
            }
            "--as" => {
                let 名 = args.next().ok_or("--as のあとに名前がありません")?;
                設.名乗り = Some(名乗りを検める(&名)?);
            }
            他 => return Err(format!("知らない指定です: {他}")),
        }
    }
    Ok(設)
}

/// **人が画面で押して許した札**を読む（**D119**）。
///
/// 置き場所は**この機械（`desk.sock`）と同じ所**である ——
/// **画面と口は別のプロセス**なので、**同じ置き場所のファイルが 2 つの間の橋になる。**
///
/// **読めなくても止めない。**空で始めれば `--allow` の分だけで動く
/// （**札が 1 つも無ければ、どの口も通らない**・D56）。
///
/// **断ったものは返さない。**返すのは**人が許したものだけ**である。
fn 人が押した札(この機械: &std::path::Path) -> Vec<String> {
    // **この機械の場所から、置き場所を割り出す。**`--desk` で移されていても付いていく
    let Some(置き場所) = この機械.parent() else {
        return Vec::new();
    };
    let vault = warifu_vault::Vault::at(置き場所.to_path_buf());
    match vault.passes() {
        Ok(棚) => 棚
            .into_iter()
            .filter(|(_, 許した)| *許した)
            .map(|(動作, _)| 動作)
            .collect(),
        Err(e) => {
            eprintln!("warifu mcp: 人が押した札を読めません（{e}）");
            Vec::new()
        }
    }
}

/// 口を出す。**繋いだ相手が閉じるまで戻らない。**
pub async fn 出す(設: &設定) -> Result<(), Box<dyn std::error::Error>> {
    use rmcp::ServiceExt;

    let 今 = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());

    let mut 関所 = Gate::new();
    for 動作 in &設.許す {
        関所.issue(Grant::new(subject(), Action::new(動作)?, 今 + 札の効き目));
    }

    // **人が画面で押した札も読む**（**D119**）。
    //
    // **`--allow` は立ち上げるときに人が書くもので、そのあと増やせない。**
    // オーナー ——「**許可をください。OK って言えば始まる。**」
    // **目と耳の中に居るなら、JSON は開けない。**
    //
    // **「人が行う」は保つ。**押したのは人であり、ここは**その結果を読むだけ**である。
    // 断ったものは**入れない**（`--allow` に書いてあっても、人が断ったなら通さない）。
    let 押した = 人が押した札(&設.この機械);
    for 動作 in &押した {
        関所.issue(Grant::new(subject(), Action::new(動作)?, 今 + 札の効き目));
    }

    // 受信箱と規則はまだ空。**空であることを、繋がっていることと混ぜない**
    // （`issues/011` が決まるまで、inbox_* は「無い」を返す）
    // **押した札を、断る直前に読み直せるようにする**（2026-09-24）。
    //
    // **押しても何も起きなかった** —— ここで 1 回読むだけだったので、
    // **人が押したのは口を立てたあと**であり、**立て直すまで通らなかった。**
    //
    // オーナー ——「**通すを押しても何も起きません。
    // 何が起きたのか人には意味がわかりません。**」
    let 机の場所 = 設.この機械.clone();
    let mut 口 = Warifu::new(Vec::new(), RuleStore::new(), 関所, 今)
        .この機械を覚える(&設.この機械)
        .札を読み直すには(std::sync::Arc::new(move || 人が押した札(&机の場所)));
    if let Some(名) = &設.名乗り {
        口 = 口.名乗る(名);
    }

    // **標準出力は MCP のもの。**言いたいことは標準エラーへ出す。
    //
    // **`--allow` と「人が押した札」を、1 行にまとめて言う**（2026-09-18）。
    //
    // **分けて出していたら、下の行が嘘になった** ——
    // 人が画面で `inbox.list` を許したのに、
    // **「（なし。--allow を書かないと何も通りません）」と出ていた。**
    // **実際は通るのに、通らないと言っていた。**
    // **`押した` は動作名だけ**（`人が押した札` が `--allow` を付けずに返す）。
    // **2026-09-18、ここを「`--allow` との組」だと思って書いて、空になった** ——
    // **自分の関数を読まずに、記憶で書いた。**
    let 言い方 = match (設.許す.is_empty(), 押した.is_empty()) {
        (true, true) => "（なし。--allow を書くか、pass_ask で頼んでください）".to_owned(),
        (false, true) => 設.許す.join(" "),
        (true, false) => format!("{}（人が画面で押した分）", 押した.join(" ")),
        (false, false) => format!(
            "{}／{}（人が画面で押した分）",
            設.許す.join(" "),
            押した.join(" ")
        ),
    };
    eprintln!(
        "warifu mcp: 名乗り {}／許した動作 {}／この機械 {}",
        設.名乗り.as_deref().unwrap_or("（名乗らない）"),
        言い方,
        設.この機械.display()
    );

    let 務め = 口.serve(rmcp::transport::stdio()).await?;
    務め.waiting().await?;
    Ok(())
}

#[cfg(test)]
mod 名乗りの試験 {
    use super::仕事場の名;

    /// 試験ごとに別の場所。**本物のリポジトリに触らない。**
    fn 仮の場(名: &str) -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!(
            "warifu-cli-{名}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&p);
        p
    }

    #[test]
    fn 仕事場の名を上へ辿って拾う() {
        // **2026-09-20 に踏んだ形。**エージェントは `<仕事場>/scratchpad` で動いていて、
        // **名乗りが `scratchpad` になり、どのプロダクトの席か分からなかった。**
        let 根 = 仮の場("rendou-kun");
        let 中 = 根.join("scratchpad");
        std::fs::create_dir_all(&中).expect("作れる");
        std::fs::create_dir_all(根.join(".git")).expect("作れる");

        assert_eq!(
            仕事場の名(&中).as_deref(),
            根.file_name().and_then(|n| n.to_str()),
            "**フォルダ名ではなく、仕事場の名を返す**"
        );
    }

    #[test]
    fn 仕事場の中なら_その名をそのまま返す() {
        let 根 = 仮の場("warifu");
        std::fs::create_dir_all(根.join(".git")).expect("作れる");
        assert_eq!(
            仕事場の名(&根).as_deref(),
            根.file_name().and_then(|n| n.to_str())
        );
    }

    #[test]
    fn 仕事場の外なら_名乗らない() {
        // **`None` を返す。**呼ぶ側がフォルダ名へ落とす ——
        // **ここで勝手に決めない**（どこまで辿ったかを混ぜない）
        let 外 = 仮の場("nowhere");
        std::fs::create_dir_all(&外).expect("作れる");
        assert_eq!(仕事場の名(&外), None);
    }

    #[test]
    fn 上に一つでも見つかれば_そこで止まる() {
        // **いちばん近い仕事場を返す**（入れ子のとき、外側まで行かない）
        let 外 = 仮の場("outer");
        let 内 = 外.join("inner");
        std::fs::create_dir_all(内.join("src")).expect("作れる");
        std::fs::create_dir_all(外.join(".git")).expect("作れる");
        std::fs::create_dir_all(内.join(".git")).expect("作れる");
        assert_eq!(仕事場の名(&内.join("src")).as_deref(), Some("inner"));
    }
}
