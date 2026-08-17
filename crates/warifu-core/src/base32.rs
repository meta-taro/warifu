//! RFC 4648 の base32（大文字・パディングなし）。
//!
//! 16 進より短く、Base64 と違って**紛らわしい記号が入らない**。
//! 割符は QR で撮るか、最悪は読み上げて渡すものなので、ここは見た目が効く。
//!
//! **同じバイト列に複数の表記を許さない**（余りビットが 0 でなければ受け取らない）。
//! 許すと、同じ割符から別の文字列を無限に作れてしまう。
//!
//! **warifu が外に出す文字列は、すべてここを通す。**
//! 割符と宛先で表記が違うと、受け取った人が「どちらの形式か」を当てる羽目になる。

const ALPHABET: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

/// バイト列を base32 の文字列にする。
pub fn encode(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(5) * 8);
    let mut buf: u32 = 0;
    let mut bits: u32 = 0;

    for &byte in data {
        buf = (buf << 8) | u32::from(byte);
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            out.push(ALPHABET[((buf >> bits) & 0x1f) as usize] as char);
        }
    }
    if bits > 0 {
        out.push(ALPHABET[((buf << (5 - bits)) & 0x1f) as usize] as char);
    }
    out
}

/// base32 の文字列をバイト列に戻す。読めなければ [`None`]。
pub fn decode(text: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(text.len() * 5 / 8);
    let mut buf: u32 = 0;
    let mut bits: u32 = 0;

    for c in text.bytes() {
        let value = match c {
            b'A'..=b'Z' => c - b'A',
            b'2'..=b'7' => c - b'2' + 26,
            _ => return None,
        };
        buf = (buf << 5) | u32::from(value);
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            out.push(((buf >> bits) & 0xff) as u8);
        }
    }

    // 余りが 5 ビット以上あるのは、1 文字ぶん多い（バイト列に対応しない）
    if bits >= 5 {
        return None;
    }
    // 余りビットが 0 でない表記は受け取らない
    if bits > 0 && (buf & ((1 << bits) - 1)) != 0 {
        return None;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 往復する() {
        for len in 0..40usize {
            let data: Vec<u8> = (0..len).map(|i| (i * 37 + 11) as u8).collect();
            assert_eq!(decode(&encode(&data)).as_deref(), Some(data.as_slice()));
        }
    }

    #[test]
    fn rfc4648_の例と一致する() {
        assert_eq!(encode(b""), "");
        assert_eq!(encode(b"f"), "MY");
        assert_eq!(encode(b"fo"), "MZXQ");
        assert_eq!(encode(b"foo"), "MZXW6");
        assert_eq!(encode(b"foob"), "MZXW6YQ");
        assert_eq!(encode(b"fooba"), "MZXW6YTB");
        assert_eq!(encode(b"foobar"), "MZXW6YTBOI");
    }

    #[test]
    fn 出力は常にascii() {
        assert!(encode(&[0xff; 32]).is_ascii());
    }

    #[test]
    fn 読めない文字を受け取らない() {
        assert_eq!(decode("my"), None, "小文字は受け取らない");
        assert_eq!(decode("M="), None, "パディングは受け取らない");
        assert_eq!(decode("M1"), None, "0 1 8 9 は英字と紛れるので入っていない");
        assert_eq!(decode("あ"), None);
    }

    #[test]
    fn 余りビットがゼロでない表記を受け取らない() {
        // "MZ" は 1 バイト分 + 余り 2 ビット。余りが 0 のものだけが正しい表記
        assert_eq!(decode("MY"), Some(b"f".to_vec()));
        assert_eq!(decode("MZ"), None);
    }

    #[test]
    fn 端数だけの文字を受け取らない() {
        assert_eq!(decode("A"), None, "5 ビットではどのバイト列にもならない");
    }
}
