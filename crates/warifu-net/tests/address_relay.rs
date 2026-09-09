//! 宛先が**中継の場所**を運べること（**D78**）。
//!
//! **既定では入らない。**`--relay` を付けた人の宛先にだけ入る。
//! 入っていない宛先が今までどおりであることを、ここで固定する。

use std::net::SocketAddr;

use warifu_core::Seed;
use warifu_net::Address;

fn 鍵() -> warifu_core::PublicKey {
    Seed::from_bytes([7u8; 32])
        .profile("Personal")
        .device("mini")
        .public_key()
}

fn 番地(s: &str) -> SocketAddr {
    s.parse().expect("番地")
}

#[test]
fn 既定では中継を持たない() {
    let a = Address::from_ip_addrs(鍵(), [番地("192.168.1.2:1234")]);
    assert_eq!(a.relay(), None);
}

#[test]
fn 中継を添えて文字列にして読み戻せる() {
    let a =
        Address::from_ip_addrs(鍵(), [番地("192.168.1.2:1234")]).with_relay("https://例.invalid/");
    let 戻り: Address = a.to_string().parse().expect("読める");
    assert_eq!(戻り.relay(), Some("https://例.invalid/"));
    assert_eq!(戻り, a);
}

#[test]
fn 中継のない宛先は今までどおり読み戻せる() {
    // **既定を壊していないこと。**ここが落ちたら、いま動いている試験が全部止まる
    let a = Address::from_ip_addrs(鍵(), [番地("192.168.1.2:1234"), 番地("[::1]:9")]);
    let 戻り: Address = a.to_string().parse().expect("読める");
    assert_eq!(戻り, a);
    assert_eq!(戻り.relay(), None);
}

#[test]
fn 中継が入っていれば外から届きうる() {
    // 番地は内側だけ。**それでも中継が橋になる**
    let a = Address::from_ip_addrs(鍵(), [番地("192.168.1.2:1234")]);
    assert!(!a.外から届きうる());
    assert!(a.with_relay("https://例.invalid/").外から届きうる());
}

#[test]
fn 空の中継は持たない() {
    let a = Address::from_ip_addrs(鍵(), [番地("192.168.1.2:1234")]).with_relay("");
    assert_eq!(a.relay(), None);
}

#[test]
fn 長すぎる中継は持たない() {
    let 長い = "h".repeat(warifu_net::RELAY_MAX + 1);
    let a = Address::from_ip_addrs(鍵(), [番地("192.168.1.2:1234")]).with_relay(長い);
    assert_eq!(a.relay(), None);
}

#[test]
fn 番地が無くても中継だけで宛先になる() {
    // **中継を使う相手は、外向きの番地を 1 つも持たないことがある**（CGNAT）
    let a = Address::from_ip_addrs(鍵(), []).with_relay("https://例.invalid/");
    let 戻り: Address = a.to_string().parse().expect("読める");
    assert_eq!(戻り.relay(), Some("https://例.invalid/"));
    assert_eq!(戻り.ip_addrs().count(), 0);
}
