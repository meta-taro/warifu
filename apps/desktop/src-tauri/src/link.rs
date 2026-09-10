//! **`warifu://` を受ける**（**D79**）。
//!
//! オーナー指摘（2026-09-09）——
//! 「**公開鍵をコピーできるのは何の目的か、どんな手順で相手（知り合いの人間）と
//! つながれるかわかりません**」「**たとえば QR とか URL スキーマで相手におくれるとか**」
//!
//! 鍵を貼り付けさせない。**渡すのはリンク 1 本**にする。
//!
//! # 押しただけでは入らない
//!
//! **届いた URL は、他人が作れる。**チャットにも掲示板にも貼れる。
//! だから**受け取った側に必ず尋ねる** —— 押した瞬間にルームへ入ることはない。
//! これは「開いたら実行」を作らないための約束であって、便利さの都合ではない。

use tauri::{AppHandle, Emitter as _};

/// `warifu://` の中で「ルームに入る」を表す道。
const 入る: &str = "join";

/// 画面へ渡す合図。**入るかどうかは画面が人に尋ねる。**
pub const EVENT_LINK: &str = "warifu://link";

/// 受け取った URL から、**ルームの鍵だけ**を取り出す。
///
/// **知らない形は捨てる。**`warifu://` が付いていれば何でも通す作りにすると、
/// 「割符のリンク」を名乗る別のものが画面へ流れ込む。
///
/// 受ける形は 1 つだけ ——
///
/// ```text
/// warifu://join/WARIFU1-XXXXXXXX…
/// ```
#[must_use]
pub fn 鍵を取り出す(url: &str) -> Option<String> {
    let 本体 = url.strip_prefix("warifu://")?;
    // ホスト名の位置に道の名前が来る（`warifu://join/...`）
    let 残り = 本体.strip_prefix(入る)?;
    let 鍵 = 残り.strip_prefix('/')?.trim();
    // **割符の頭が付いていないものは、鍵ではない。**中身の検めは、この先の層が行う
    if !鍵.starts_with("WARIFU1-") || 鍵.len() > 鍵の上限 {
        return None;
    }
    // **`#` が `%23` に化けて届くことがある。**
    //
    // URL は最初の `#` から先を「断片」として扱うので、
    // **その中に出てくる 2 つ目の `#` は `%23` に書き換えられる。**
    // 会議キーは `WARIFU1-<宛先>#<割符>#<ルーム>` の形なので、これに当たる
    // （2026-09-09 に、実物のリンクを押して踏んだ）。
    let 鍵 = 鍵.replace("%23", "#");
    let 鍵 = 鍵.as_str();

    // **使える字だけで出来ていること。**
    //
    // 会議キーは base32（大文字と 2〜7）＋ `-` ＋ 区切りの `#` でできている
    // （`WARIFU1-<宛先>#<割符>#<ルーム>`）。
    // **`#` を弾いてはいけない** —— 2026-09-09 に実物で踏んだ。
    // URL の断片に見えるが、これは鍵の一部である。
    if !鍵
        .chars()
        .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '-' || c == '#')
    {
        return None;
    }
    Some(鍵.to_owned())
}

/// 鍵の長さの上限。**これを超えるものは受け取らない。**
///
/// 会議キーは宛先と割符とルームを含むので長い（実測で 380 字ほど）。
/// 上限を切らないと、URL 1 本で画面へいくらでも流し込める。
const 鍵の上限: usize = 4096;

/// 受け取った URL を画面へ渡す。**入るかどうかは人が決める。**
pub fn 受ける(app: &AppHandle, urls: &[String]) {
    for url in urls {
        let Some(鍵) = 鍵を取り出す(url) else {
            // **黙って捨てない。**なぜ何も起きなかったのかを、手元の記録に残す
            記録!("知らない形のリンクを捨てました: {}", 頭だけ(url));
            continue;
        };
        記録!("リンクを受け取りました（人に尋ねます）");
        let _ = app.emit(EVENT_LINK, 鍵);
    }
}

/// 記録に出す用に、URL の頭だけを取る。**全部は書かない**（鍵が混ざりうる）。
fn 頭だけ(url: &str) -> String {
    url.chars().take(16).collect()
}

#[cfg(test)]
mod tests {
    use super::鍵を取り出す;

    #[test]
    fn ルームに入るリンクから鍵を取り出す() {
        assert_eq!(
            鍵を取り出す("warifu://join/WARIFU1-ABCDEF"),
            Some("WARIFU1-ABCDEF".to_owned())
        );
    }

    #[test]
    fn 割符の頭が無いものは鍵にしない() {
        assert_eq!(鍵を取り出す("warifu://join/ABCDEF"), None);
    }

    #[test]
    fn 知らない道は捨てる() {
        // **「warifu:// が付いていれば通す」にしない**
        assert_eq!(鍵を取り出す("warifu://run/WARIFU1-ABCDEF"), None);
        assert_eq!(鍵を取り出す("warifu://WARIFU1-ABCDEF"), None);
    }

    #[test]
    fn 別のスキームは受けない() {
        assert_eq!(鍵を取り出す("https://join/WARIFU1-ABCDEF"), None);
    }

    /// **`#` は鍵の一部である。**弾いてはいけない（2026-09-09 に実物で踏んだ）。
    ///
    /// 会議キーは `WARIFU1-<宛先>#<割符>#<ルーム>` の形をしている。
    /// URL の断片に見えるが、これを落とすと**鍵が壊れる。**
    #[test]
    fn 区切りの井桁は鍵の一部として通す() {
        let 鍵 = "WARIFU1-AAAA#BBBB#CCCC";
        assert_eq!(
            鍵を取り出す(&format!("warifu://join/{鍵}")),
            Some(鍵.to_owned())
        );
    }

    /// **`%23` は `#` に戻す。**戻さないと、実物のリンクが 1 本も通らない
    /// （2026-09-09 に、押して踏んだ）。
    #[test]
    fn 井桁が化けていても戻す() {
        assert_eq!(
            鍵を取り出す("warifu://join/WARIFU1-AAAA#BBBB%23CCCC"),
            Some("WARIFU1-AAAA#BBBB#CCCC".to_owned())
        );
    }

    #[test]
    fn 使えない字が混じっていたら捨てる() {
        // 小文字・記号・道の区切りは、会議キーには出てこない
        assert_eq!(鍵を取り出す("warifu://join/WARIFU1-abc"), None);
        assert_eq!(鍵を取り出す("warifu://join/WARIFU1-ABC?x=1"), None);
        assert_eq!(鍵を取り出す("warifu://join/WARIFU1-ABC/z"), None);
        assert_eq!(鍵を取り出す("warifu://join/WARIFU1-ABC%20D"), None);
    }

    #[test]
    fn 長すぎる鍵は捨てる() {
        let 長い = format!("WARIFU1-{}", "A".repeat(5000));
        assert_eq!(鍵を取り出す(&format!("warifu://join/{長い}")), None);
    }

    #[test]
    fn 作ったリンクは自分で読み戻せる() {
        // **出す形と読む形が別々に育たないようにする**
        let 鍵 = "WARIFU1-ABCDEFGH";
        assert_eq!(
            super::鍵を取り出す(&super::ルームのリンク(鍵)),
            Some(鍵.to_owned())
        );
    }

    #[test]
    fn qrにできる() {
        let svg = super::qrにする("warifu://join/WARIFU1-ABCDEFGH").expect("QR");
        assert!(svg.starts_with("<svg"), "{}", &svg[..40.min(svg.len())]);
    }

    /// **目印は塗りで描く。**
    ///
    /// 2026-09-09 に、角の目印を `stroke` の枠で描いて**半升ずれ**、
    /// **読み取り機が QR そのものを見つけられなくなった**（実測）。
    /// 見た目を変えるときに、同じ踏み方をしないための杭である。
    #[test]
    fn 角の目印を線で描かない() {
        let svg = super::qrにする("warifu://join/WARIFU1-ABCDEFGH").expect("QR");
        assert!(!svg.contains("stroke=\"#6b4c33\""), "目印を線で描いている");
    }

    /// 目印は **7 / 5 / 3** の入れ子。**大きさと位置は規格どおり。**
    #[test]
    fn 目印は規格どおりの入れ子にする() {
        let 印 = super::目印(4, 4);
        assert!(印.contains(r#"x="4" y="4" width="7" height="7""#), "{印}");
        assert!(印.contains(r#"x="5" y="5" width="5" height="5""#), "{印}");
        assert!(印.contains(r#"x="6" y="6" width="3" height="3""#), "{印}");
    }

    /// **地は白のまま。**暗い地に置くと読み取り機が拾えないことがある
    #[test]
    fn 地は白のままにする() {
        let svg = super::qrにする("warifu://join/WARIFU1-ABCDEFGH").expect("QR");
        assert!(svg.contains(r##"fill="#ffffff""##), "地が白でない");
    }

    /// **中央の印は一辺の 2 割まで。**それ以上隠すと誤り訂正でも足りない
    #[test]
    fn 中央の印は大きくしすぎない() {
        let 印 = super::中央の印(100);
        // 白抜きも含めて、一辺の 1/4 を超えない
        assert!(印.contains(r#"width="20.00""#), "{印}");
        assert!(印.contains(r#"width="23.60""#), "{印}");
    }

    #[test]
    fn 空の鍵は捨てる() {
        assert_eq!(鍵を取り出す("warifu://join/"), None);
        assert_eq!(鍵を取り出す("warifu://join"), None);
    }
}

/// **リンクの形**（**D79**）。渡すのはこれ 1 本。
#[must_use]
pub fn ルームのリンク(鍵: &str) -> String {
    format!("warifu://{入る}/{鍵}")
}

/// 目の前の相手に読ませる **QR**（**D79**）。
///
/// オーナー指摘（2026-09-09）——
/// 「**たとえば QR とか URL スキーマで相手におくれるとか**」
/// 「**QR 割符感だせます？カラーとかまんなかにアイコンとか**」
///
/// **画像ではなく SVG を返す。**画像にすると、画面の大きさに合わせられない。
///
/// # 読めることが先
///
/// 見た目のために読めなくなったら本末転倒である。だから ——
///
/// - **地は白のまま。**暗い地に置くと、読み取り機が拾えないことがある
/// - **中央を隠すぶんは、誤り訂正の一番強いもの（[`ECL::H`]・30%）で埋め合わせる**
/// - 隠すのは**一辺の 2 割まで**（面積で 4%）。それ以上は削らない
///
/// # Errors
/// 長すぎて QR に入らないとき。
pub fn qrにする(中身: &str) -> Result<String, String> {
    use fast_qr::ECL;

    let qr = fast_qr::QRBuilder::new(中身)
        // **中央に印を置くので、誤り訂正を一番強くする**
        .ecl(ECL::H)
        .build()
        .map_err(|e| format!("QR にできません: {e}"))?;

    let 一辺 = qr.size;
    // 静穏帯（まわりの余白）。**これを削ると読めなくなる**
    let 余白 = 4usize;
    let 全体 = 一辺 + 余白 * 2;

    let mut 出力 = String::with_capacity(全体 * 全体 * 8);
    出力.push_str(&format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {全体} {全体}" shape-rendering="geometricPrecision">"#
    ));
    // **地は白。**読み取り機のために固定する（テーマでは変えない）
    出力.push_str(&format!(
        r#"<rect width="{全体}" height="{全体}" fill="{紙}" rx="1.5"/>"#
    ));

    // 目印（3 つの角）は**枠として描く**。ここが割符の顔になる
    出力.push_str(&format!(r#"<g fill="{濃}">"#));
    for y in 0..一辺 {
        for x in 0..一辺 {
            let 升 = qr[y][x];
            if !升.value() || 角の中(x, y, 一辺) {
                continue;
            }
            let (cx, cy) = (x + 余白, y + 余白);
            // **わずかに丸める。**尖った点の集まりより、木の板の質感に近い
            出力.push_str(&format!(
                r#"<rect x="{cx}" y="{cy}" width="1" height="1" rx="0.28"/>"#
            ));
        }
    }
    for (x, y) in [(0, 0), (一辺 - 7, 0), (0, 一辺 - 7)] {
        出力.push_str(&目印(x + 余白, y + 余白));
    }
    出力.push_str("</g>");

    出力.push_str(&中央の印(全体));
    出力.push_str("</svg>");
    Ok(出力)
}

/// 紙の色。**白のままにする**（読み取りのため）。
const 紙: &str = "#ffffff";
/// 板の色。アイコンの濃いほう。
const 濃: &str = "#6b4c33";
/// 割れ目の色。アイコンの淡いほう。
const 淡: &str = "#c9a98a";

/// その升が、3 つの角（目印）の中か。**中は別に描く。**
const fn 角の中(x: usize, y: usize, 一辺: usize) -> bool {
    let 端 = 一辺 - 7;
    (x < 7 && y < 7) || (x >= 端 && y < 7) || (x < 7 && y >= 端)
}

/// 角の目印を描く。
///
/// **形は規格どおりに置く**（外枠 7×7 の 1 升ぶん・内側 5×5 の白・芯 3×3）。
/// 2026-09-09 に、枠を `stroke` で描いて**半升ずれ**、
/// **読み取り機が QR そのものを見つけられなくなった**（実測で確かめた）。
/// **丸めるのは角だけ。**大きさと位置は動かさない。
fn 目印(x: usize, y: usize) -> String {
    format!(r#"<rect x="{x}" y="{y}" width="7" height="7" rx="1.6" fill="{濃}"/>"#)
        + &format!(
            r#"<rect x="{}" y="{}" width="5" height="5" rx="1.1" fill="{紙}"/>"#,
            x + 1,
            y + 1
        )
        + &format!(
            r#"<rect x="{}" y="{}" width="3" height="3" rx="0.8" fill="{濃}"/>"#,
            x + 2,
            y + 2
        )
}

/// 中央の **割符の印**（アイコンと同じ形）。
///
/// **一辺の 2 割まで。**それ以上隠すと、誤り訂正でも足りなくなる。
fn 中央の印(全体: usize) -> String {
    let 全体 = 全体 as f32;
    let 一辺 = 全体 * 0.20;
    let 元 = (全体 - 一辺) / 2.0;
    // 白く抜いてから置く。**升の上に直に載せると、境目が読めない**
    let 抜き = 一辺 * 1.18;
    let 抜き元 = (全体 - 抜き) / 2.0;
    let 割れ目 = |t: f32| 元 + 一辺 * t;
    format!(
        r#"<rect x="{抜き元:.2}" y="{抜き元:.2}" width="{抜き:.2}" height="{抜き:.2}" rx="{r:.2}" fill="{紙}"/>"#,
        r = 抜き * 0.22
    ) + &format!(
        r#"<rect x="{元:.2}" y="{元:.2}" width="{一辺:.2}" height="{一辺:.2}" rx="{r:.2}" fill="{濃}"/>"#,
        r = 一辺 * 0.22
    ) + &format!(
        // **割れ目。**割符は 2 つに割った札で、この形が名前そのものである
        r#"<path d="M{a:.2} {t:.2} L{b:.2} {c:.2} L{d:.2} {e:.2} L{b:.2} {f:.2} L{a:.2} {g:.2}" fill="none" stroke="{淡}" stroke-width="{w:.2}" stroke-linejoin="round" stroke-linecap="round"/>"#,
        a = 割れ目(0.62),
        b = 割れ目(0.30),
        d = 割れ目(0.68),
        t = 割れ目(0.10),
        c = 割れ目(0.32),
        e = 割れ目(0.50),
        f = 割れ目(0.68),
        g = 割れ目(0.90),
        w = 一辺 * 0.13,
    )
}

#[cfg(test)]
mod 目で見る {
    /// **見た目を変えたときに、目で確かめるためのもの。**
    ///
    /// `cargo test -p warifu-desktop -- --ignored 書き出す` で SVG を書き出す。
    /// **読めるかどうかは、書き出したものを読み取り機に掛けて確かめる**
    /// （2026-09-09 に、見た目を変えて**読めなくなった**ことがある）。
    #[test]
    #[ignore = "見た目を確かめるときだけ"]
    fn 書き出す() {
        let 先 = std::env::temp_dir().join("warifu-qr.svg");
        let svg = super::qrにする("warifu://join/WARIFU1-K5JEMQIDRFSI54VBGE56D23J3AY").unwrap();
        std::fs::write(&先, svg).unwrap();
        println!("{}", 先.display());
    }
}
