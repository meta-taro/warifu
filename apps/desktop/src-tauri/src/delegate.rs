//! **任せる相手**（0.1.44・D5 の例外）—— この機械の人が「この相手の機械からの頼みは、私の頼みとして扱う」と
//! 画面で決めた相手。その相手から**直接の線で**届いた発言にだけ、エージェントへの行に「任された」を立てる。
//!
//! - **決めるのは画面だけ。**エージェントの口（机）からは書けない（自分で自分に許しを出せない）
//! - **主催ごしに届いた分には立てない。**主催は差出人を名乗れるので、差出人を確かめられない
//! - 単位は相手の**機械の鍵**。話し手の名前は自己申告なので単位にしない
//! - 期限つき（既定 30 日）。いつでも画面から取り消せる

use warifu_core::PublicKey;
use warifu_vault::{DELEGATE_DEFAULT_SECS, Delegate, Vault, 任せているか};

use crate::{Answer, key_to_string};

/// いま（UNIX 秒）。
fn いま() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// **この発言に「任された」を立てるか。**
///
/// 名乗った差出人と、線をつないできた相手が同じ（直接の線）で、その鍵をいま任せているときだけ。
#[must_use]
pub(crate) fn 任された発言か(
    差出人: &PublicKey,
    線の相手: &PublicKey,
    任せている: &[Delegate],
    いま: u64,
) -> bool {
    差出人 == 線の相手 && 任せているか(任せている, &key_to_string(*差出人), いま)
}

/// 控えから読んで、この発言に「任された」を立てるか。**読めなければ立てない**（記録に残す）。
pub(crate) fn 控えを見て任されたか(
    差出人: &PublicKey, 線の相手: &PublicKey
) -> bool {
    if 差出人 != 線の相手 {
        return false;
    }
    let いま = いま();
    match Vault::default_location().and_then(|v| v.delegates(いま)) {
        Ok(一覧) => 任された発言か(差出人, 線の相手, &一覧, いま),
        Err(e) => {
            記録!("任せる相手: 控えを読めません。任されたとは扱いません（{e}）");
            false
        }
    }
}

/// 画面へ返す 1 件。
#[derive(Debug, serde::Serialize)]
pub(crate) struct 任せている相手 {
    key: String,
    /// いつまで（UNIX 秒）。
    until: u64,
}

/// **いま任せている相手**（画面の連絡帳が読む）。
#[tauri::command]
pub(crate) fn delegates() -> Answer<Vec<任せている相手>> {
    let v = Vault::default_location()?;
    Ok(v.delegates(いま())?
        .into_iter()
        .map(|d| 任せている相手 {
            key: d.key,
            until: d.until,
        })
        .collect())
}

/// **任せる**（画面からだけ）。日数を省けば 30 日。
#[tauri::command]
pub(crate) fn delegate_set(key: String, days: Option<u32>) -> Answer<u64> {
    let v = Vault::default_location()?;
    let いま = いま();
    let 長さ = days.map_or(DELEGATE_DEFAULT_SECS, |d| {
        u64::from(d.clamp(1, 365)) * 24 * 60 * 60
    });
    let until = いま + 長さ;
    v.save_delegate(&key, until, いま)?;
    記録!(
        "任せる相手: {}… を任せました（{} 日）",
        key.chars().take(12).collect::<String>(),
        長さ / (24 * 60 * 60)
    );
    Ok(until)
}

/// **任せるのをやめる**（画面からだけ）。
#[tauri::command]
pub(crate) fn delegate_forget(key: String) -> Answer<()> {
    let v = Vault::default_location()?;
    v.forget_delegate(&key, いま())?;
    記録!(
        "任せる相手: {}… を任せるのをやめました",
        key.chars().take(12).collect::<String>()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use warifu_core::Seed;

    fn 鍵(n: u8) -> PublicKey {
        Seed::from_bytes([n; 32])
            .profile("Personal")
            .device("PC")
            .public_key()
    }

    #[test]
    fn 任せた相手から直接の線で届いた発言だけに立てる() {
        let (任せた, 主催, 他) = (鍵(1), 鍵(2), 鍵(3));
        let 一覧 = vec![Delegate {
            key: key_to_string(任せた),
            until: 100,
        }];
        assert!(任された発言か(&任せた, &任せた, &一覧, 10));
        assert!(
            !任された発言か(&任せた, &主催, &一覧, 10),
            "主催ごしは差出人を確かめられない"
        );
        assert!(!任された発言か(&他, &他, &一覧, 10), "任せていない相手");
        assert!(
            !任された発言か(&任せた, &任せた, &一覧, 100),
            "期限が過ぎた"
        );
    }
}
