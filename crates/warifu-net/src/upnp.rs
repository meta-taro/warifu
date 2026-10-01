//! **自前の UPnP**（2026-09-29）。
//!
//! # なぜ portmapper だけでは足りないか
//!
//! portmapper（→ igd-next）はルーターを探すとき、**`0.0.0.0` に結んで**問い合わせを投げる。
//! Windows の機械（Wi-Fi・WSL あり）で実測した ——
//!
//! ```text
//! 送り元 0.0.0.0         → 返事なし（3 回とも）
//! 送り元 192.168.24.11   → 返事あり 3〜6 ms（3 回とも）
//! ```
//!
//! **ルーターは応じているのに、割符が聞けていなかった。**
//! 同じルーターの下の macOS の機械は通っていた。
//!
//! そこで、**外へ出る口の番地を決めて**探す。見つかったら、その番地へ向けて口を借りる。

use std::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4};
use std::time::Duration;

use igd_next::aio::tokio::{Tokio, search_gateway};
use igd_next::{PortMappingProtocol, SearchOptions};

type 係 = igd_next::aio::Gateway<Tokio>;

/// 探す締め切り。試験に使ったルーターは数 ms で返したので、2 秒あれば十分である。
const 探す締め切り: Duration = Duration::from_secs(2);

/// 借りる長さ（秒）。**落ちても、この長さで口は閉じる。**
pub(crate) const 借りる秒: u32 = 20 * 60;

/// ルーターの口の一覧に出る名札。
const 名札: &str = "warifu";

/// ルーターから借りた口。
pub(crate) struct 借りた口 {
    係: 係,
    内: SocketAddrV4,
    /// ルーターの外側の番地と口。**鍵に載せるのはこれ。**
    pub(crate) 外: SocketAddrV4,
}

/// **外へ出る口の番地**（既定の経路が乗っている口）。
pub(crate) fn 外へ出る番地() -> Option<Ipv4Addr> {
    let 家 = netwatch::interfaces::HomeRouter::new()?;
    match 家.my_ip {
        Some(IpAddr::V4(ip)) if !ip.is_unspecified() && !ip.is_loopback() => Some(ip),
        _ => None,
    }
}

/// **その番地に結んでルーターを探し、`口` を外へ向けて借りる。**
///
/// 見つからない・断られたら `None`（**頼んだが無い**）。
pub(crate) async fn 借りる(内の番地: Ipv4Addr, 口: u16) -> Option<借りた口> {
    let 探し = SearchOptions {
        bind_addr: SocketAddr::new(IpAddr::V4(内の番地), 0),
        timeout: Some(探す締め切り),
        single_search_timeout: Some(探す締め切り),
        ..SearchOptions::default()
    };
    let 係 = tokio::time::timeout(探す締め切り, search_gateway(探し))
        .await
        .ok()?
        .ok()?;
    let IpAddr::V4(外の番地) = 係.get_external_ip().await.ok()? else {
        return None;
    };
    let 内 = SocketAddrV4::new(内の番地, 口);
    // **まず内と同じ番号で**（覚えやすい・前と同じになりやすい）。駄目なら空いている番号
    let 外の口 = if 係
        .add_port(PortMappingProtocol::UDP, 口, 内.into(), 借りる秒, 名札)
        .await
        .is_ok()
    {
        口
    } else {
        係.add_any_port(PortMappingProtocol::UDP, 内.into(), 借りる秒, 名札)
            .await
            .ok()?
    };
    Some(借りた口 {
        係,
        内,
        外: SocketAddrV4::new(外の番地, 外の口),
    })
}

impl 借りた口 {
    /// **期限を延ばす。**同じ外の口をもう一度頼む。断られたら `false`。
    pub(crate) async fn 延ばす(&self) -> bool {
        self.係
            .add_port(
                PortMappingProtocol::UDP,
                self.外.port(),
                self.内.into(),
                借りる秒,
                名札,
            )
            .await
            .is_ok()
    }
}

#[cfg(test)]
mod 試験 {
    /// **実物のルーターに頼む**ので、既定では走らせない。
    /// `cargo test -p warifu-net -- --ignored 実物のルーターから借りる` で、その網のルーターを試す。
    #[tokio::test]
    #[ignore = "実物のルーターへ頼む"]
    async fn 実物のルーターから借りる() {
        let 内 = super::外へ出る番地().expect("外へ出る口がある");
        let 借りた = super::借りる(内, 45_999).await.expect("ルーターが口を貸す");
        assert_eq!(借りた.内.ip(), &内);
        assert!(借りた.延ばす().await, "同じ口をもう一度頼める");
        println!("外の口 {}", 借りた.外.port());
    }
}
