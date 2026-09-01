//! MCP サーバ本体。

use std::sync::{Arc, Mutex};

use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{ErrorData, Implementation, ServerCapabilities, ServerInfo};
use rmcp::{ServerHandler, tool, tool_handler, tool_router};
use warifu_capability::{Action, Decision, Gate, Request, Subject};
use warifu_read::{Level, Reader, Received, RuleStore, View};

use crate::{OpenArgs, ToolError};

/// この口を叩いている相手の名前。
///
/// MCP は stdio で繋がるだけで、相手が誰かを名乗る仕組みを持たない。
/// **だから「手元の AI」という 1 つの相手として札を出す。**
/// 名乗れないものに、名乗れたことにした名前を付けない。
pub fn subject() -> Subject {
    Subject::new("mcp:local-agent").expect("固定の名前なので必ず通る")
}

/// 受信箱を MCP の口として出すサーバ。
#[derive(Clone)]
pub struct Warifu {
    inner: Arc<Mutex<Inner>>,
}

struct Inner {
    messages: Vec<Received>,
    reader: Reader,
    gate: Gate,
    now: u64,
}

impl Warifu {
    /// 受信箱と規則と関所を渡してサーバを作る。
    ///
    /// `now` は**こちらの時計**。札の期限判定に使う。
    pub fn new(messages: Vec<Received>, rules: RuleStore, gate: Gate, now: u64) -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner {
                messages,
                reader: Reader::with_rules(rules),
                gate,
                now,
            })),
        }
    }

    /// 出している口の名前。
    ///
    /// **増やすときは、その口に札の種類が要るかを先に決める。**
    /// 札の要らない口を 1 つ足した時点で、関所を迂回する経路ができる。
    pub fn tool_names() -> Vec<String> {
        Self::tool_router()
            .list_all()
            .into_iter()
            .map(|t| t.name.to_string())
            .collect()
    }

    /// 関所の記録を TSV で取り出す。**何を断ったかを人が見るため。**
    pub fn log_tsv(&self) -> String {
        self.inner
            .lock()
            .expect("毒されていない")
            .gate
            .log()
            .to_tsv()
    }

    /// 関所に尋ねる。**この関数を通らない tool は無い。**
    fn 通るか(&self, action: &str) -> Result<(), ToolError> {
        let mut inner = self.inner.lock().expect("毒されていない");
        let 動作 = Action::new(action).map_err(|e| ToolError::BadArgs(e.to_string()))?;
        let 今 = inner.now;
        match inner.gate.decide(&Request::new(subject(), 動作), 今) {
            Decision::Allow => Ok(()),
            // **断った理由に「どうすれば通るか」を書かない。**
            // 書くと、断られた側が総当たりで札の形を探れる
            Decision::Deny => Err(ToolError::Denied(action.to_owned())),
        }
    }
}

#[tool_router]
impl Warifu {
    /// 受信箱を metadata だけで並べる。**本文は 1 文字も返らない。**
    #[tool(description = "受信箱を metadata だけで並べる。本文は返らない。")]
    pub async fn inbox_list(&self) -> Result<String, ErrorData> {
        self.通るか("inbox.list")?;

        let inner = self.inner.lock().expect("毒されていない");
        let mut 行 = vec!["番号\t送信元\t種別\t優先度\t要判断".to_owned()];
        for (i, 一通) in inner.messages.iter().enumerate() {
            // **段は上げない。**View::Metadata には本文が入る場所が無い
            let m = inner.reader.read(一通);
            let m = m.metadata();
            行.push(format!(
                "{i}\t{}\t{}\t{:?}\t{}",
                m.sender().as_str(),
                m.kind(),
                m.priority(),
                if m.action_required() { "要" } else { "" }
            ));
        }
        Ok(行.join("\n"))
    }

    /// 1 通の段を上げて読む。**段ごとに別の札が要る。**
    #[tool(description = "1 通の段を上げて読む。段ごとに別の許可が要る。")]
    pub async fn inbox_open(
        &self,
        Parameters(args): Parameters<OpenArgs>,
    ) -> Result<String, ErrorData> {
        let 段 = match args.level.as_str() {
            "summary" => Level::Summary,
            "structured" => Level::Structured,
            "raw" => Level::Raw,
            "attachments" => Level::Attachments,
            // metadata は inbox_list の役目。ここで受けると札の粒度が崩れる
            other => return Err(ToolError::BadArgs(format!("知らない段です: {other}")).into()),
        };
        self.通るか(&format!("inbox.open.{}", args.level))?;

        let inner = self.inner.lock().expect("毒されていない");
        let 一通 = inner
            .messages
            .get(args.index)
            .ok_or_else(|| ToolError::BadArgs(format!("{} 通目はありません", args.index)))?;

        match inner
            .reader
            .open_at(一通, 段)
            .map_err(|e| ToolError::Unavailable(e.to_string()))?
        {
            View::Structured { fields, .. } => Ok(fields
                .iter()
                .map(|f| format!("{}\t{}", f.name(), f.value()))
                .collect::<Vec<_>>()
                .join("\n")),
            View::Raw { body, .. } => Ok(String::from_utf8_lossy(body.as_bytes()).into_owned()),
            View::Attachments { attachments, .. } => Ok(attachments
                .iter()
                .map(|a| format!("{}\t{} バイト", a.name(), a.bytes().len()))
                .collect::<Vec<_>>()
                .join("\n")),
            View::Summary { summary, .. } => Ok(summary),
            View::Metadata(_) => {
                Err(ToolError::Unavailable("段が上がりませんでした".to_owned()).into())
            }
            // View は non_exhaustive。**知らない段を勝手に文字列にしない**
            _ => Err(ToolError::Unavailable("知らない段が返りました".to_owned()).into()),
        }
    }

    /// 承認済みの規則を、人が読める形で出す。
    #[tool(description = "承認済みの読み取り規則を人が読める形で出す。")]
    pub async fn rules_list(&self) -> Result<String, ErrorData> {
        self.通るか("rules.list")?;

        let inner = self.inner.lock().expect("毒されていない");
        let 棚 = inner.reader.rules();
        if 棚.is_empty() {
            return Ok("承認済みの規則はありません。".to_owned());
        }
        Ok(棚
            .rules()
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n"))
    }
}

#[tool_handler]
impl ServerHandler for Warifu {
    fn get_info(&self) -> ServerInfo {
        let mut info = ServerInfo::default();
        info.capabilities = ServerCapabilities::builder().enable_tools().build();
        // 既定は SDK の名前（rmcp）が出る。**何に繋がっているかは繋いだ人が見るところ**なので、
        // ここは名乗り直す
        let mut 名乗り = Implementation::default();
        名乗り.name = "warifu".to_owned();
        名乗り.version = env!("CARGO_PKG_VERSION").to_owned();
        info.server_info = 名乗り;
        info.instructions = Some(
            "受信箱を読む口。既定では本文を返さない。\
             段を上げるには、その段の許可（札）が要る。\
             規則の承認と札の発行は、この口には無い（人が行う）。"
                .to_owned(),
        );
        info
    }
}

impl From<ToolError> for ErrorData {
    fn from(e: ToolError) -> Self {
        ErrorData::invalid_request(e.to_string(), None)
    }
}
