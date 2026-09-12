//! **どこまで聞いたか**を控える（`.claude/issues/019`）。
//!
//! **エージェントは落ちる。**`warifu agent` は繋ぎ直せるが（**D93**）、
//! **繋ぎ直した瞬間、溜めは空である**（溜めはこのプロセスの中にある）。
//! つまり**切れている間の言葉は、永久に届かない。しかも落ちたことが誰にも見えない。**
//!
//! 2026-09-12、この形で**33 分前に来ていた返事を見落とした。**
//!
//! **「どこから」は読み手が言う。**この機械（机）は「誰がどこまで読んだか」を
//! 覚えない —— 覚えると**既読を作ることになる**（**D94** で作らないと決めた）。
//! だから**控えるのはこちら側**である。

use std::path::{Path, PathBuf};

/// 名乗らないエージェントの控え先。
///
/// **名乗らない相手は机に着けない**（`desk.rs`）ので、ここは本来使われない。
/// それでも名前を持つのは、**書く先が無いことを理由に黙って落とさない**ため。
const 名無し: &str = "名乗りなし";

/// 控えを置く場所。机（口）と同じフォルダの下に置く。
///
/// **名乗りをそのままファイル名にしない** —— `..` や `/` が来ると、
/// 机の外へ書けてしまう。**英数と、日本語をそのまま通す以外は落とす。**
#[must_use]
pub fn 印の場所(机: &Path, 名乗り: Option<&str>) -> PathBuf {
    let 名 = 名乗り.map_or_else(|| 名無し.to_owned(), 安全な名);
    let 親 = 机.parent().unwrap_or(Path::new("."));
    親.join("heard").join(名)
}

/// ファイル名にしてよい形へ直す。
fn 安全な名(名乗り: &str) -> String {
    let 直した: String = 名乗り
        .chars()
        .map(|c| {
            if c == '/' || c == '\\' || c == '.' || c.is_control() {
                '_'
            } else {
                c
            }
        })
        .collect();
    let 削った = 直した.trim();
    if 削った.is_empty() {
        名無し.to_owned()
    } else {
        削った.to_owned()
    }
}

/// 控えを読む。**読めなければ `None`**（初めて繋ぐときはこれ）。
///
/// **`None` は「新しい分だけ聞く」**（今までと同じ動き）。
/// 初めてのエージェントに 200 行を渡さない —— 頼まれてもいない過去を押し付けない。
#[must_use]
pub fn 印を読む(道: &Path) -> Option<u64> {
    let 文 = std::fs::read_to_string(道).ok()?;
    文.trim().parse().ok()
}

/// 控えを書く。**書けなくても会話は止めない。**
///
/// 控えが書けないのは困るが、**そのために会話を落とすほうが困る。**
/// 書けなかったことは呼び手へ返す（握り潰さない）。
pub fn 印を書く(道: &Path, まで: u64) -> std::io::Result<()> {
    if let Some(親) = 道.parent() {
        std::fs::create_dir_all(親)?;
    }
    std::fs::write(道, まで.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 机と同じ所の下に置く() {
        let 道 = 印の場所(Path::new("/tmp/warifu/desk.sock"), Some("zumen"));
        assert_eq!(道, Path::new("/tmp/warifu/heard/zumen"));
    }

    #[test]
    fn 名乗りに区切りが入っていても_外へ出ない() {
        // **`..` や `/` で机の外へ書かせない**
        let 道 = 印の場所(Path::new("/tmp/warifu/desk.sock"), Some("../../etc/passwd"));
        assert_eq!(道, Path::new("/tmp/warifu/heard/______etc_passwd"));
    }

    #[test]
    fn 名乗らない相手にも_置き場所はある() {
        let 道 = 印の場所(Path::new("/tmp/warifu/desk.sock"), None);
        assert_eq!(道, Path::new("/tmp/warifu/heard/名乗りなし"));
    }

    #[test]
    fn 空の名乗りも_名無しとして扱う() {
        let 道 = 印の場所(Path::new("/tmp/warifu/desk.sock"), Some("   "));
        assert_eq!(道, Path::new("/tmp/warifu/heard/名乗りなし"));
    }

    #[test]
    fn 書いて読み直せる() {
        let 所 = std::env::temp_dir().join(format!("warifu-heard-{}", std::process::id()));
        let 道 = 所.join("heard").join("zumen");
        印を書く(&道, 42).expect("書ける");
        assert_eq!(印を読む(&道), Some(42));
        let _ = std::fs::remove_dir_all(&所);
    }

    #[test]
    fn まだ無いなら_新しい分だけ聞く() {
        // **初めてのエージェントに、頼まれてもいない過去を押し付けない**
        assert_eq!(印を読む(Path::new("/tmp/warifu-無い-控え")), None);
    }

    #[test]
    fn 読めない中身は_無いものとして扱う() {
        let 所 = std::env::temp_dir().join(format!("warifu-heard-x-{}", std::process::id()));
        let 道 = 所.join("heard").join("zumen");
        印を書く(&道, 7).expect("書ける");
        std::fs::write(&道, "こわれている").expect("書ける");
        // **決めつけない。**数でなければ「無い」と同じ
        assert_eq!(印を読む(&道), None);
        let _ = std::fs::remove_dir_all(&所);
    }
}
