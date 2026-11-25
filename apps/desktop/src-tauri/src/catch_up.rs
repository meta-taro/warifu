//! **主催が立ち上げ直しても、戻った客へ抜けた分を渡す**（0.1.60）。
//!
//! 主催の控え（部屋ごとの直近の文字）はメモリーだけにあったので、主催が更新などで立ち上げ直すと、
//! 留守だった客が戻っても、そのあいだの発言を渡せなかった。
//! 画面は会話を `chat.seal` に残している（D127）。立ち上げたときにそこから控えを組み直し、
//! 客が「ここまで持っている」と言った印のあとを渡す（`warifu_app::room::部屋たち::会話を戻す`）。
//!
//! 残した行は画面の形（呼び名・顔の種・本文・行の印）なので、差出人と札は**候補を作って印で確かめる。**
//! 印が合わない行は組まない（違う発言を渡すより、渡さないほうがよい）。

use serde_json::{Map, Value};
use warifu_core::PublicKey;
use warifu_meeting::{MeetingId, Notice};

/// 1 部屋で組み直す行の上限（控えの上限と同じ）。古い行は要らない。
const 組む行の上限: usize = warifu_app::room::控える上限;

/// 残した会話から、部屋ごとの文字（古い順）を組む。返りは（部屋, 文字たち, 組めなかった行の数）。
pub(crate) fn 残した会話から(
    会話: &Map<String, Value>,
    自分: PublicKey,
) -> Vec<(MeetingId, Vec<Notice>, usize)> {
    let 知っている鍵 = 鍵を集める(会話, 自分);
    let mut 出 = Vec::new();
    for (部屋の字, 行たち) in 会話 {
        let Ok(部屋) = 部屋の字.parse::<MeetingId>() else {
            continue;
        };
        let Some(行たち) = 行たち.as_array() else {
            continue;
        };
        let 飛ばす = 行たち.len().saturating_sub(組む行の上限);
        let mut 文字たち = Vec::new();
        let mut 組めない = 0usize;
        for 行 in 行たち.iter().skip(飛ばす) {
            // 印の無い行（知らせ・古い行）は数えない。渡す対象ではない
            if 行.get("印").and_then(Value::as_str).is_none() || 行.get("system").is_some() {
                continue;
            }
            match 行から文字を組む(部屋, 行, 自分, &知っている鍵) {
                Some(n) => 文字たち.push(n),
                None => 組めない += 1,
            }
        }
        if !文字たち.is_empty() || 組めない > 0 {
            出.push((部屋, 文字たち, 組めない));
        }
    }
    出
}

/// 残した行の顔の種のうち、公開鍵の形のもの（と自分）。
fn 鍵を集める(会話: &Map<String, Value>, 自分: PublicKey) -> Vec<PublicKey> {
    let mut 鍵たち = vec![自分];
    for 行 in 会話.values().filter_map(Value::as_array).flatten() {
        if let Some(k) = 行
            .get("顔の種")
            .and_then(Value::as_str)
            .and_then(|s| s.parse::<PublicKey>().ok())
            && !鍵たち.contains(&k)
        {
            鍵たち.push(k);
        }
    }
    鍵たち
}

/// 画面の 1 行を、網に流れた文字に組み直す。**印が合う組だけ**を返す。
pub(crate) fn 行から文字を組む(
    部屋: MeetingId,
    行: &Value,
    自分: PublicKey,
    知っている鍵: &[PublicKey],
) -> Option<Notice> {
    let 印 = crate::ack::字を印に(行.get("印")?.as_str()?)?;
    let body = 行.get("body")?.as_str()?;
    if body.is_empty() {
        return None;
    }
    let who = 行.get("who").and_then(Value::as_str).unwrap_or_default();
    let 種 = 行.get("顔の種").and_then(Value::as_str).unwrap_or_default();
    let 自分の行 = 行.get("mine").and_then(Value::as_bool).unwrap_or(false)
        || 行.get("agent").and_then(Value::as_bool).unwrap_or(false);
    let 差出人たち: Vec<PublicKey> = if 自分の行 {
        vec![自分]
    } else if let Ok(k) = 種.parse::<PublicKey>() {
        vec![k]
    } else {
        // 札の付いた相手の行は、顔の種が「鍵の頭…（札）」。頭の合う鍵を候補にする
        let 頭 = 種
            .split('（')
            .next()
            .unwrap_or_default()
            .trim_end_matches('…');
        if 頭.is_empty() {
            return None;
        }
        知っている鍵
            .iter()
            .copied()
            .filter(|k| k.to_string().starts_with(頭))
            .collect()
    };
    let 札たち = 札の候補(who, 種);
    for from in 差出人たち {
        for 札 in &札たち {
            let n = Notice::Text {
                meeting: 部屋,
                from,
                話し手: 札.clone(),
                body: body.to_owned(),
            };
            if n.文字の印() == Some(印) {
                return Some(n);
            }
        }
    }
    None
}

/// 札の候補：札なし・呼び名そのもの・`desk:` のあと・「（」のあとから最後の「）」まで（どの「（」からも）。
fn 札の候補(who: &str, 種: &str) -> Vec<Option<String>> {
    let mut 候補: Vec<Option<String>> = vec![None];
    let mut 足す = |s: &str| {
        let s = s.trim();
        if !s.is_empty() && !候補.iter().any(|c| c.as_deref() == Some(s)) {
            候補.push(Some(s.to_owned()));
        }
    };
    足す(who);
    if let Some(名) = 種.strip_prefix("desk:") {
        足す(名);
    }
    for 字 in [who, 種] {
        let Some(中) = 字.strip_suffix('）') else {
            continue;
        };
        for (i, _) in 中.match_indices('（') {
            足す(&中[i + '（'.len_utf8()..]);
        }
    }
    候補
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use warifu_core::Seed;

    fn 鍵(n: u8) -> PublicKey {
        Seed::from_bytes([n; 32])
            .profile("Personal")
            .device("PC")
            .public_key()
    }

    fn 印(n: &Notice) -> String {
        crate::ack::印を字に(&n.文字の印().unwrap())
    }

    fn 文字(部屋: MeetingId, from: PublicKey, 札: Option<&str>, 本文: &str) -> Notice {
        Notice::Text {
            meeting: 部屋,
            from,
            話し手: 札.map(str::to_owned),
            body: 本文.to_owned(),
        }
    }

    #[test]
    fn 人の行と_札つきの相手の行と_自分の行を組み直し_印が合わない行は組まない() {
        let 部屋 = MeetingId::generate();
        let (自分, 相手) = (鍵(1), 鍵(2));
        let 人 = 文字(部屋, 相手, None, "こんにちは");
        let 席 = 文字(部屋, 相手, Some("席（PC）"), "席の発言");
        let 私 = 文字(部屋, 自分, None, "主催から");
        let 私の席 = 文字(部屋, 自分, Some("主催の席（PC）"), "主催の席から");
        let 頭 = crate::短く(&相手.to_string());
        let 会話 = json!({
            部屋.to_string(): [
                { "who": "相手", "顔の種": 相手.to_string(), "body": "こんにちは", "mine": false, "印": 印(&人) },
                { "who": format!("{頭}（席（PC））"), "顔の種": format!("{頭}（席（PC））"),
                  "body": "席の発言", "mine": false, "印": 印(&席) },
                { "who": "あなた", "body": "主催から", "mine": true, "印": 印(&私) },
                { "who": "主催の席（PC）", "顔の種": "desk:主催の席（PC）", "body": "主催の席から",
                  "mine": false, "agent": true, "印": 印(&私の席) },
                { "who": "誰か", "顔の種": 相手.to_string(), "body": "書き換えた", "mine": false, "印": 印(&人) },
                { "who": "知らせ", "body": "入室", "mine": false, "system": true },
            ]
        });
        let Value::Object(会話) = 会話 else {
            unreachable!()
        };
        let 出 = 残した会話から(&会話, 自分);
        assert_eq!(出.len(), 1);
        let (戻した部屋, 文字たち, 組めない) = &出[0];
        assert_eq!(*戻した部屋, 部屋);
        assert_eq!(文字たち, &vec![人, 席, 私, 私の席]);
        assert_eq!(*組めない, 1, "本文が印と合わない行は組まない");
    }

    #[test]
    fn 部屋の形でない鍵と_空の部屋は飛ばす() {
        let 会話 = json!({ "room-x": [], "desk:": [{ "who": "a", "body": "b", "mine": false }] });
        let Value::Object(会話) = 会話 else {
            unreachable!()
        };
        assert!(残した会話から(&会話, 鍵(1)).is_empty());
    }
}
