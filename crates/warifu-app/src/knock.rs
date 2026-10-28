//! **割符を持たずに叩く。**
//!
//! 戸口は前から「**一度開けた相手は、次から割符なしで開ける**」と決めていた（**D31**）。
//! だが握手の手順（**D39**）は「繋がった直後、**生のバイト列で片割れ（`Acceptance`）を
//! 1 通だけ**やり取りする」しか定めておらず、**割符を持たない相手が何を送るかが
//! 決まっていなかった。**
//!
//! 何も送らなければ、迎える側は 10 秒待ってから諦める。**待たせない。**
//! 壊れた塊を送るのも採らない —— 記録に残ったとき、
//! **目印なのか通信が壊れたのかが読めなくなる。**
//!
//! だから「**割符は持っていない**」と名乗る目印を 1 つ決める。
//! これは D39 の手順に**足す 1 行**であって、置き換えではない。

/// 割符を持たずに叩くときに送る目印。
///
/// `WRFK` は綴りの族（割符は `WRF1`、宛先は `WRFA`）。
/// 末尾の 1 byte は版で、**版が違えば受け取らない**
/// （黙って新しい版として扱わない —— D39 の「旧版の鍵は長さで落ちる」と同じ構え）。
///
/// **これは鍵ではない。**通してよいかを決めるのは戸口であって、この目印ではない。
/// 目印は「割符を出さない」という申告にすぎず、**知り合いでなければ断られる。**
pub const KNOCK_WITHOUT_TALLY: &[u8] = b"WRFK\x01";

/// 受け取った塊が、割符なしの目印か。
///
/// **`Acceptance` として読めなかったものを、片端から目印と見なさない。**
/// 見なすと、切れた通信の残りが「割符なしで来た」ことになる。
#[must_use]
pub fn is_knock_without_tally(bytes: &[u8]) -> bool {
    bytes == KNOCK_WITHOUT_TALLY
}

/// **戻りたい部屋を添えて叩くときの目印**（0.1.41）。`WRFK\x02` のあとに部屋 id の文字が続く。
///
/// 帰り道で主催を叩くとき、**どの部屋へ戻りたいか**を一緒に言う。
/// 言わないと主催は「主催している部屋のどれか」へ招き、無ければ建てる ——
/// 戻りたい部屋でなければゲストは抜け、叩くたびに主催の棚に一時の部屋が増えた（10-27 に 3 台で見た）。
///
/// **古い主催はこの塊を今までどおり「割符なし」として扱う**（塊の中身は見ず、戸口で決める）ので、互換は切れない。
pub const KNOCK_FOR_ROOM: &[u8] = b"WRFK\x02";

/// 戻りたい部屋を添えた叩き。
#[must_use]
pub fn knock_for_room(room: &str) -> Vec<u8> {
    let mut 塊 = KNOCK_FOR_ROOM.to_vec();
    塊.extend_from_slice(room.as_bytes());
    塊
}

/// 叩きに添えられた部屋。**部屋なしの叩き・ほかの塊なら `None`。**
#[must_use]
pub fn room_of_knock(bytes: &[u8]) -> Option<&str> {
    let 残り = bytes.strip_prefix(KNOCK_FOR_ROOM)?;
    let 部屋 = std::str::from_utf8(残り).ok()?;
    (!部屋.is_empty() && 部屋.len() <= 64 && 部屋.bytes().all(|b| b.is_ascii_alphanumeric()))
        .then_some(部屋)
}

#[cfg(test)]
mod 試験 {
    use super::*;

    #[test]
    fn 部屋を添えた叩きは_部屋が読める_部屋なしや他の塊は読めない() {
        let 塊 = knock_for_room("NBIW3PA2TUQ52DTJNVXSQOL3G4");
        assert_eq!(room_of_knock(&塊), Some("NBIW3PA2TUQ52DTJNVXSQOL3G4"));
        assert!(!is_knock_without_tally(&塊));
        assert_eq!(room_of_knock(KNOCK_WITHOUT_TALLY), None);
        assert_eq!(room_of_knock(b"WRFK\x02"), None);
        assert_eq!(room_of_knock("WRFK\x02部屋".as_bytes()), None);
        assert_eq!(room_of_knock(b"WRF1xxxx"), None);
    }
}
