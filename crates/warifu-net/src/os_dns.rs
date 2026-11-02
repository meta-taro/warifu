//! **名前は OS に引かせる**（0.1.42）。
//!
//! iroh の既定の引き方（hickory）は、`/etc/resolv.conf` のサーバへ EDNS つきで直に問い合わせる。
//! 家庭用ルーターの中には、これに **Form Error** を返して答えないものがある（実測・どの名前も引けない）。
//! そうなると中継の場所が引けず、**中継を使う設定でも一度も中継に繋がらない**（黙って直接だけになる）。
//! OS の引き方（dig・curl・ブラウザと同じ道）なら同じ網で引ける。
//!
//! だから **A / AAAA は OS に先に聞き、引けなければ iroh の既定に任せる。**TXT は OS に道が無いので既定のまま。

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::time::Duration;

use iroh::dns::{BoxIter, DnsError, DnsResolver, Resolver, TxtRecordData};

type BoxFuture<T> = std::pin::Pin<Box<dyn std::future::Future<Output = T> + Send + 'static>>;

/// 予備（iroh の既定）に任せるときの待ち。
const 予備の待ち: Duration = Duration::from_secs(3);

/// OS に先に聞く引き方。`予備` は OS で引けなかったときに使う。
#[derive(Debug, Clone)]
pub(crate) struct OSに先に聞く {
    予備: DnsResolver,
}

impl OSに先に聞く {
    /// 予備を iroh の既定（システムの設定を読む hickory）にして作る。
    pub(crate) fn 既定() -> DnsResolver {
        Self::予備つき(DnsResolver::new())
    }

    pub(crate) fn 予備つき(予備: DnsResolver) -> DnsResolver {
        DnsResolver::custom(Self { 予備 })
    }
}

/// OS に引かせる（`getaddrinfo`）。引けなければ空。
async fn os_で引く(host: &str) -> Vec<IpAddr> {
    // 引けないのは「無い」と同じに扱う（予備に回す）。理由は予備の失敗が返す
    tokio::net::lookup_host((host, 0))
        .await
        .map(|並び| 並び.map(|a| a.ip()).collect())
        .unwrap_or_default()
}

fn v4だけ(並び: impl IntoIterator<Item = IpAddr>) -> Vec<Ipv4Addr> {
    並び
        .into_iter()
        .filter_map(|a| match a {
            IpAddr::V4(v) => Some(v),
            IpAddr::V6(_) => None,
        })
        .collect()
}

fn v6だけ(並び: impl IntoIterator<Item = IpAddr>) -> Vec<Ipv6Addr> {
    並び
        .into_iter()
        .filter_map(|a| match a {
            IpAddr::V6(v) => Some(v),
            IpAddr::V4(_) => None,
        })
        .collect()
}

impl Resolver for OSに先に聞く {
    fn lookup_ipv4(&self, host: String) -> BoxFuture<Result<BoxIter<Ipv4Addr>, DnsError>> {
        let 予備 = self.予備.clone();
        Box::pin(async move {
            let 引けた = v4だけ(os_で引く(&host).await);
            if !引けた.is_empty() {
                return Ok(Box::new(引けた.into_iter()) as BoxIter<Ipv4Addr>);
            }
            let 予備の答え = v4だけ(予備.lookup_ipv4(host, 予備の待ち).await?);
            Ok(Box::new(予備の答え.into_iter()) as BoxIter<Ipv4Addr>)
        })
    }

    fn lookup_ipv6(&self, host: String) -> BoxFuture<Result<BoxIter<Ipv6Addr>, DnsError>> {
        let 予備 = self.予備.clone();
        Box::pin(async move {
            let 引けた = v6だけ(os_で引く(&host).await);
            if !引けた.is_empty() {
                return Ok(Box::new(引けた.into_iter()) as BoxIter<Ipv6Addr>);
            }
            let 予備の答え = v6だけ(予備.lookup_ipv6(host, 予備の待ち).await?);
            Ok(Box::new(予備の答え.into_iter()) as BoxIter<Ipv6Addr>)
        })
    }

    fn lookup_txt(&self, host: String) -> BoxFuture<Result<BoxIter<TxtRecordData>, DnsError>> {
        let 予備 = self.予備.clone();
        Box::pin(async move {
            let 答え: Vec<_> = 予備.lookup_txt(host, 予備の待ち).await?.collect();
            Ok(Box::new(答え.into_iter()) as BoxIter<TxtRecordData>)
        })
    }

    fn clear_cache(&self) {
        self.予備.clear_cache();
    }

    fn reset(&self) -> Box<dyn Resolver> {
        // 網が変わったら予備を作り直す（ネームサーバを読み直す）。OS の側は毎回聞くので持ち物が無い
        Box::new(Self {
            予備: DnsResolver::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iroh::dns::DnsProtocol;
    use std::net::SocketAddr;

    /// 何も答えない予備（家庭用ルーターが Form Error を返すのと同じく、iroh の既定では引けない形）。
    fn 答えない予備() -> DnsResolver {
        DnsResolver::builder()
            .with_nameserver(SocketAddr::from(([127, 0, 0, 1], 9)), DnsProtocol::Udp)
            .build()
    }

    #[tokio::test]
    async fn 既定の引き方が答えなくても_os_で引ける名前は引ける() {
        // **この機械の名前**は OS なら引ける（hosts・mDNS・NetBIOS）。答えないサーバだけの予備では引けない。
        // `localhost` は hickory が特別に答えるので、OS に聞いたかの確かめにならない
        let 名前 = std::process::Command::new("hostname")
            .output()
            .expect("hostname が在る");
        let 名前 = String::from_utf8_lossy(&名前.stdout).trim().to_string();
        assert!(!名前.is_empty());
        let os_の答え: Vec<_> = tokio::net::lookup_host((名前.as_str(), 0))
            .await
            .map(|i| i.collect())
            .unwrap_or_default();
        if !os_の答え.iter().any(SocketAddr::is_ipv4) {
            // OS でも v4 が引けない機械では、確かめる物が無い（黙って通さず、そう書く）
            eprintln!("この機械の名前 {名前} は OS でも v4 が引けないので見送ります");
            return;
        }
        let 引き方 = OSに先に聞く::予備つき(答えない予備());
        let 答え: Vec<_> = 引き方
            .lookup_ipv4(名前.clone(), Duration::from_secs(5))
            .await
            .expect("OS で引ける")
            .collect();
        assert!(!答え.is_empty(), "{名前}");
    }

    #[tokio::test]
    async fn os_でも予備でも引けない名前は_止まらずに失敗を返す() {
        let 引き方 = OSに先に聞く::予備つき(答えない予備());
        let 返り = tokio::time::timeout(
            Duration::from_secs(20),
            引き方.lookup_ipv4("no-such-host.invalid", Duration::from_secs(5)),
        )
        .await
        .expect("待ち続けない");
        assert!(返り.is_err());
    }
}
