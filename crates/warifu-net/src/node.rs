//! 結び目と、その上に立つ 1 本の経路。

use core::time::Duration;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};

use iroh::endpoint::{Connection, ReadExactError, RecvStream, SendStream, presets};
use iroh::{Endpoint, EndpointAddr, EndpointId, RelayMode, SecretKey, TransportAddr, Watcher as _};
use warifu_core::{Device, PublicKey, Revocations};
use zeroize::Zeroize as _;

use crate::{Address, Error};

/// この層が名乗る通信規約。**版を上げたらここを変える。**
const ALPN: &[u8] = b"warifu/1";

/// 経路が開いた合図。相手が warifu であることと、版が合うことを最初に確かめる。
const HELLO: &[u8; 4] = b"WRF1";

/// 一度に受け取る上限。**長さだけ大きく宣言して確保させる攻撃を止める。**
///
/// 上に載る層（`warifu-intent` など）は、自分の header を足した合計がここを超えないよう
/// **自分で**上限を決める必要がある。だから公開している。
pub const MAX_MESSAGE: usize = 16 * 1024 * 1024;

/// 呼びに行くのを諦めるまで。
///
/// 宛先の公開鍵が経路の持ち主と違う場合、**下の層は「届かない」としか分からず、
/// 繋がるまで試し続ける。**上限を切らないと、呼んだ側は永久に待つ。
const CONNECT_LIMIT: Duration = Duration::from_secs(10);

/// 中継の場所が出るまで待つ限度。
///
/// **中継は結んだ直後には決まっていない。**外へ出て、どこが近いかを測ってから決まる。
/// 待たずに宛先を出すと、**中継を頼んだのに中継の入っていない宛先**が配られる
/// （2026-09-09 に実物で踏んだ）。
///
/// **待ちきれなくても止めない。**出なかったという事実を、そのまま宛先に出す
/// （`doctor` が「付けましたが出ていません」と言う）。
const RELAY_WAIT: Duration = Duration::from_secs(5);

/// 経路の結び目。1 台に 1 つ。
#[derive(Debug, Clone)]
pub struct Node {
    endpoint: Endpoint,
    key: PublicKey,
    /// 中継を頼まれたか（**D78**）。**宛先を出すときに待つかどうかを決める。**
    中継: 中継の使い方,
    /// 口がどう決まったか（**#38**）。**呼んだ側が人へ言うために持つ。**
    口: 口の様子,
}

/// **待つ口の決め方**（**#38** の残り半分・2026-09-17）。
///
/// **鍵は「出したときの口」を焼き込む。**だから、立ち上げ直して口が変わると、
/// **配った鍵が全部死ぬ** —— 割符（誰を通すか）を控えても、
/// **待っている場所が変わっていれば、相手はそこへ来られない。**
///
/// ASUS の実測（2026-09-17）——
///
/// ```text
/// 鍵が指す口     51728     ← 09-16 10:32 に出した
/// いま待つ口     57155     ← 入れ替えて立ち上げ直したあと
/// ```
///
/// **51728 では誰も待っていない。**戸口の話ではなく、その手前である。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum 口の決め方 {
    /// **その場の空きに任せる。**毎回変わる（**前の鍵は死ぬ**）。
    #[default]
    まかせる,
    /// **前と同じ口を取りに行く。**取れなければ空きに落ちる（[`口の様子::取れなかった`]）。
    同じ口(u16),
}

/// **口がどう決まったか。****呼んだ側が人へ言うために要る。**
///
/// **黙って空きに落ちない。**落ちたなら「配った鍵は使えません」と言えなければ、
/// **#38 を直した意味が無くなる**（案 A だけでは嘘になる・案 C の文言が要る）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum 口の様子 {
    /// 任せた。**この口は次の起動で変わる。**
    まかせた(u16),
    /// **前と同じ口が取れた。****前に配った鍵がそのまま生きる。**
    取り直せた(u16),
    /// **取れなかった。****前に配った鍵は、もう使えない。**
    取れなかった {
        /// 取りに行った口。
        望んだ: u16,
        /// 代わりに取れた口。
        代わり: u16,
    },
}

impl 口の様子 {
    /// いま待っている口。
    #[must_use]
    pub const fn 口(self) -> u16 {
        match self {
            Self::まかせた(口) | Self::取り直せた(口) => 口,
            Self::取れなかった { 代わり, .. } => 代わり,
        }
    }

    /// **前に配った鍵が、まだ使えるか。**
    ///
    /// `取り直せた` のときだけ true。**`まかせた` は「前が無い」ので false** ——
    /// 「分からない」を「使える」と言わない。
    #[must_use]
    pub const fn 前の鍵が生きているか(self) -> bool {
        matches!(self, Self::取り直せた(_))
    }
}

/// 中継を使うかどうか（**D78**・2026-09-09 オーナー判断）。
///
/// **既定は [`使わない`](中継の使い方::使わない)。**呼ぶ側が明示したときだけ使う。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum 中継の使い方 {
    /// 相手と直接つながる経路しか使わない。**同じ網の相手にしか届かない。**
    #[default]
    使わない,
    /// n0 の中継を使う。**網を越えて届く代わりに、繋いだことが中継の運用者に見える。**
    使う,
}

impl Node {
    /// 中継を使わずに結び目を作る。
    ///
    /// **相手と直接つながる経路しか使わない。**
    /// 中継を挟むと「誰がいつ誰に繋いだか」が中継の運用者に出る（`decisions.md` **D10**）。
    ///
    /// # Errors
    /// 結べなければ [`Error::Network`]。
    pub async fn bind_without_relay(device: &Device) -> Result<Self, Error> {
        Self::bind(device, 中継の使い方::使わない).await
    }

    /// 中継を使うかどうかを決めて結び目を作る（**D78**）。
    ///
    /// **`使う` を選んだときだけ、外の中継が経路に入る。**
    /// D13 の既定（中継なし）はそのままで、**選んだ人にだけ効く。**
    ///
    /// # 中継を使っても、名前解決は使わない
    ///
    /// iroh には `presets::N0`（中継 ＋ n0 の DNS）が用意されているが、**採らない。**
    /// あれは**自分の居場所を n0 の DNS へ公開する** —— 公開鍵さえ知っていれば
    /// 誰でも居場所を引ける形になる。割符は宛先を**人が手で渡す**約束なので、
    /// ここで公開すると、その約束のほうが崩れる。
    ///
    /// **足すのは中継だけ**（`presets::Minimal` ＋ [`RelayMode::Default`]）。
    ///
    /// # Errors
    /// 結べなければ [`Error::Network`]。
    pub async fn bind(device: &Device, 中継: 中継の使い方) -> Result<Self, Error> {
        Self::bind_at(device, 中継, 口の決め方::まかせる).await
    }

    /// **待つ口まで決めて結び目を作る**（**#38** の残り半分）。
    ///
    /// `口の決め方::同じ口(n)` を渡すと、**前と同じ口を取りに行く。**
    /// **取れなければ空きに落ちる** —— そのとき [`口の様子`] が `取れなかった` になるので、
    /// **呼んだ側は「配った鍵は使えません」と言える。**
    ///
    /// # なぜ黙って落ちないのか
    ///
    /// **口が変わると、配った鍵は全部死ぬ。**黙って落ちると、
    /// **人は鍵が死んだことを知らないまま待つことになる**（2026-09-16 に Mac Air が
    /// 実際に 6 時間待った）。**「取れなかった」は、必ず外へ出す。**
    ///
    /// # Errors
    /// 結べなければ [`Error::Network`]。
    pub async fn bind_at(
        device: &Device,
        中継: 中継の使い方,
        決め方: 口の決め方,
    ) -> Result<Self, Error> {
        let 望み = match 決め方 {
            口の決め方::まかせる => None,
            口の決め方::同じ口(口) => Some(口),
        };

        // **まず望んだ口で試す。**駄目なら空きで結び直す（**ここで諦めない**）
        let (endpoint, 口) = match 望み {
            Some(望んだ) => match Self::結ぶ(device, 中継, 望んだ).await {
                Ok(endpoint) => (endpoint, 口の様子::取り直せた(望んだ)),
                Err(_) => {
                    let endpoint = Self::結ぶ(device, 中継, 0).await?;
                    let 代わり = Self::いまの口(&endpoint);
                    (
                        endpoint,
                        口の様子::取れなかった {
                            望んだ, 代わり
                        },
                    )
                }
            },
            None => {
                let endpoint = Self::結ぶ(device, 中継, 0).await?;
                let 口 = Self::いまの口(&endpoint);
                (endpoint, 口の様子::まかせた(口))
            }
        };

        Ok(Self {
            endpoint,
            key: device.public_key(),
            中継,
            口,
        })
    }

    /// 口を 1 つ指して結ぶ。`0` は「空きに任せる」。
    async fn 結ぶ(
        device: &Device, 中継: 中継の使い方, 口: u16
    ) -> Result<Endpoint, Error> {
        let mut raw = device.secret_key_bytes();
        let secret = SecretKey::from_bytes(&raw);
        raw.zeroize();

        let 中継の設定 = match 中継 {
            中継の使い方::使わない => RelayMode::Disabled,
            中継の使い方::使う => RelayMode::Default,
        };

        let mut builder = Endpoint::builder(presets::Minimal)
            .secret_key(secret)
            .alpns(vec![ALPN.to_vec()])
            .relay_mode(中継の設定);

        // **口を指すときだけ、既定の口を外して置き直す。**
        // iroh の builder は 0.0.0.0:0 と [::]:0 を最初から持っており、
        // **同じ族に二度指すと経路の選び方が定まらない**（iroh の注意書き）
        if 口 != 0 {
            builder = builder
                .clear_ip_transports()
                .bind_addr(SocketAddr::from((Ipv4Addr::UNSPECIFIED, 口)))
                .map_err(|_| Error::network("口を指す")(std::io::Error::other("口が悪い")))?
                .bind_addr(SocketAddr::from((Ipv6Addr::UNSPECIFIED, 口)))
                .map_err(|_| Error::network("口を指す")(std::io::Error::other("口が悪い")))?;
        }

        builder.bind().await.map_err(Error::network("結ぶ"))
    }

    /// いま待っている口。**v4 を先に見る**（鍵に載るのも v4 が先）。
    fn いまの口(endpoint: &Endpoint) -> u16 {
        let 口たち = endpoint.bound_sockets();
        口たち
            .iter()
            .find(|a| a.is_ipv4())
            .or_else(|| 口たち.first())
            .map_or(0, SocketAddr::port)
    }

    /// **口がどう決まったか**（**#38**）。
    ///
    /// **`取れなかった` を握り潰さない。**呼んだ側は、これを人へ見せる責任がある。
    #[must_use]
    pub const fn 口の様子(&self) -> 口の様子 {
        self.口
    }

    /// 自分の公開鍵。
    #[must_use]
    pub fn public_key(&self) -> PublicKey {
        self.key
    }

    /// 相手に渡す宛先。**経路が 1 本も見つかるまで待つ。**
    ///
    /// # Errors
    /// 経路が出てこないまま結び目が閉じたら [`Error::Network`]。
    pub async fn address(&self) -> Result<Address, Error> {
        // **中継を頼んだなら、その場所が出るまで少しだけ待つ**（**D78**）。
        // 中継は結んだ直後には決まっていないので、待たないと
        // **中継の入っていない宛先**を配ることになる
        if self.中継 == 中継の使い方::使う {
            let _ = tokio::time::timeout(RELAY_WAIT, self.中継を待つ()).await;
        }
        let mut watcher = self.endpoint.watch_addr();
        loop {
            let addr = watcher.get();
            // **届きそうな順に並べて、本数を切る**（**#34**）——
            // 仮想 NIC（WSL / Docker）の番地が**鍵の半分を占めていた。**
            // **落とすのはループバックだけ**で、あとは後ろへ回す
            let ips: Vec<_> = crate::宛先に載せる(addr.ip_addrs().copied());
            // **中継を使っていれば、その場所も渡す**（**D78**）。
            // 渡さないと、相手は中継の在り処を知らないまま呼ぶことになる
            let 中継 = addr.relay_urls().next().map(ToString::to_string);
            // **中継だけでも宛先になる。**外向きの番地が 1 つも無い回線（CGNAT）では、
            // 番地を待っていると永久に返らない
            if !ips.is_empty() || 中継.is_some() {
                return Ok(Address::新しく(self.key, ips, 中継));
            }
            watcher
                .updated()
                .await
                .map_err(Error::network("宛先を待つ"))?;
        }
    }

    /// 中継の場所が出るまで待つ。**出るまで返らない**（呼ぶ側が時間を切る）。
    async fn 中継を待つ(&self) {
        let mut watcher = self.endpoint.watch_addr();
        loop {
            if watcher.get().relay_urls().next().is_some() {
                return;
            }
            if watcher.updated().await.is_err() {
                return;
            }
        }
    }

    /// 呼ばれるのを待って、1 本の経路にする。
    ///
    /// # Errors
    /// 相手が名簿に載っていれば [`Error::Revoked`]。繋がらなければ [`Error::Network`]。
    pub async fn accept(&self, revocations: &Revocations) -> Result<Session, Error> {
        let incoming = self.endpoint.accept().await.ok_or_else(|| Error::Network {
            doing: "受ける",
            cause: "結び目が閉じています".into(),
        })?;

        let connection = incoming
            .accept()
            .map_err(Error::network("受ける"))?
            .await
            .map_err(Error::network("受ける"))?;

        let peer = to_public_key(connection.remote_id())?;
        // **呼ばれた側でも名簿を見る。**呼ぶ側の名簿は相手の手元にあり、当てにできない
        if revocations.is_revoked_device(&peer) {
            connection.close(0u32.into(), b"revoked");
            return Err(Error::Revoked);
        }

        let (send, mut recv) = connection
            .accept_bi()
            .await
            .map_err(Error::network("受ける"))?;

        let mut hello = [0u8; 4];
        recv.read_exact(&mut hello)
            .await
            .map_err(Error::network("受ける"))?;
        if &hello != HELLO {
            return Err(Error::Malformed);
        }

        Ok(Session {
            peer,
            connection,
            send,
            recv,
            _endpoint: self.endpoint.clone(),
        })
    }

    /// 宛先を呼びに行って、1 本の経路にする。[`CONNECT_LIMIT`] で諦める。
    ///
    /// # Errors
    /// 相手が名簿に載っていれば [`Error::Revoked`]。繋がらなければ [`Error::Network`]。
    pub async fn connect(&self, to: &Address, revocations: &Revocations) -> Result<Session, Error> {
        self.connect_within(to, revocations, CONNECT_LIMIT).await
    }

    /// 諦めるまでの時間を自分で決めて呼びに行く。
    ///
    /// # Errors
    /// 相手が名簿に載っていれば [`Error::Revoked`]。繋がらなければ [`Error::Network`]。
    pub async fn connect_within(
        &self,
        to: &Address,
        revocations: &Revocations,
        limit: Duration,
    ) -> Result<Session, Error> {
        // 名簿は各自が持つ。**呼ぶ側で止めないと、失くした端末を自分から呼びに行く**
        if revocations.is_revoked_device(&to.public_key()) {
            return Err(Error::Revoked);
        }

        let id =
            EndpointId::from_bytes(&to.public_key().to_bytes()).map_err(|_| Error::Malformed)?;
        let mut addr = EndpointAddr::from_parts(id, to.ip_addrs().map(TransportAddr::Ip));
        // **相手が中継を名乗っていれば、そこも経路の候補に入れる**（**D78**）。
        // 読めない中継の場所は**黙って捨てる** —— 番地だけで繋がることはある
        if let Some(relay) = to.relay() {
            if let Ok(url) = relay.parse() {
                addr = addr.with_relay_url(url);
            }
        }

        let connection = tokio::time::timeout(limit, self.endpoint.connect(addr, ALPN))
            .await
            .map_err(|_| Error::Network {
                doing: "呼ぶ",
                // **どこへ当てにいったかを言う**（2026-09-24・ASUS と Mac Air の実測から）。
                //
                // **「届きませんでした」だけでは、3 つが区別できない** ——
                // 鍵が古い／相手が落ちている／別の網に居る。
                // ASUS の席は、この 3 つを切り分けるために **ping を打った**
                // （warifu は UDP なので、ping も TCP も決め手にならない）。
                //
                // **番地を出せば、その場で分かる** ——
                // 相手のいまの番地と見比べれば、「鍵が古い」がすぐ見える。
                // **秘密ではない**（宛先は鍵の中に平文で入っている）。
                cause: 届かなかった言い方(to).into(),
            })?
            .map_err(Error::network("呼ぶ"))?;

        let peer = to_public_key(connection.remote_id())?;
        // iroh 側でも照合されるが、**この層の約束として自分でも確かめる**
        if peer != to.public_key() {
            connection.close(0u32.into(), b"wrong peer");
            return Err(Error::Malformed);
        }

        let (mut send, recv) = connection.open_bi().await.map_err(Error::network("呼ぶ"))?;
        send.write_all(HELLO)
            .await
            .map_err(Error::network("呼ぶ"))?;

        Ok(Session {
            peer,
            connection,
            send,
            recv,
            _endpoint: self.endpoint.clone(),
        })
    }
}

fn to_public_key(id: EndpointId) -> Result<PublicKey, Error> {
    PublicKey::from_bytes(*id.as_bytes()).map_err(Error::from)
}

/// 相手と繋がった 1 本の経路。
///
/// 中身は QUIC の双方向ストリーム 1 本。**長さを先に書いて、その長さだけ読む。**
/// 切れ目を決めておかないと、受け取る側は「どこまでが 1 つの塊か」を当てられない。
///
/// # 送ってすぐ落とさない
///
/// [`Session::send`] が返るのは「送る列に積んだ」ところまでで、相手に届いた合図ではない。
/// **積んだまま落とすと、まだ網に出ていない分は消える。**
/// 送り終わりなら [`Session::finish`] を呼ぶ（相手が受け取り切るまで待つ）。
///
/// # 元の [`Node`] は落としてよい
///
/// 繋がった後は、この経路だけを持ち回せる。**結び目の寿命に巻き込まれない。**
#[derive(Debug)]
pub struct Session {
    peer: PublicKey,
    connection: Connection,
    send: SendStream,
    recv: RecvStream,
    /// **読まないが、手放さない。**
    ///
    /// 結び目は最後の持ち手が落ちた時点で閉じ、その上の経路も道連れになる。
    /// そうなると送った側には成功が返り、受ける側は永久に待つ
    /// ――**落ちたことにすら気づけない**ので、経路が自分で生かしておく。
    _endpoint: Endpoint,
}

impl Session {
    /// 繋がっている相手の公開鍵。**割符で確定した相手と一致するはず。**
    #[must_use]
    pub fn peer(&self) -> PublicKey {
        self.peer
    }

    /// 生きているかどうか。
    #[must_use]
    pub fn is_open(&self) -> bool {
        self.connection.close_reason().is_none()
    }

    /// **いま文字が流れている通り道**（直接か、中継か）。
    ///
    /// **2026-09-25、網を越えて初めてつながったとき、これをどこにも出していなかった。**
    /// 画面の「経路」は映像（WebRTC）の経路なので、画面なしの相手だと必ず unknown になる。
    #[must_use]
    pub fn 通り道(&self) -> 通り道 {
        self.connection
            .paths()
            .iter()
            .find(|道| 道.is_selected())
            .map_or(通り道::不明, |道| 通り道::から(道.remote_addr()))
    }

    /// **通り道が変わるたびに知らせる**（穴があいて中継から直接へ移った、など）。
    ///
    /// 経路が閉じたら終わる。知らせは別の仕事の上で呼ばれる。
    pub fn 通り道を見張る(
        &self, 知らせ: impl Fn(通り道の出来事) + Send + 'static
    ) {
        let 結び = self.connection.clone();
        tokio::spawn(async move {
            use futures::StreamExt as _;
            let mut 出来事 = 結び.path_events();
            let mut 前 = None;
            let mut 入れ替え = 0_u32;
            while let Some(一つ) = 出来事.next().await {
                if let iroh::endpoint::PathEvent::Selected { remote_addr, .. } = 一つ {
                    let 今 = 通り道::から(&remote_addr);
                    if 通り道::知らせるか(前.as_ref(), &今) {
                        知らせ(通り道の出来事::変わった(今.clone()));
                    } else if 前.as_ref() != Some(&今) {
                        // **番地の揺れは行にしない。数だけ残す**（sshboard の席の提案）——
                        // 落ち着かない網では、ここが映像の途切れの手がかりになる
                        入れ替え = 入れ替え.saturating_add(1);
                    }
                    前 = Some(今);
                }
            }
            知らせ(通り道の出来事::閉じた {
                番地の入れ替え: 入れ替え,
            });
        });
    }

    /// バイト列を 1 つ送る。
    ///
    /// # Errors
    /// [`MAX_MESSAGE`] を超えたら [`Error::TooLarge`]。経路が切れたら [`Error::Network`]。
    pub async fn send(&mut self, message: &[u8]) -> Result<(), Error> {
        let len = u32::try_from(message.len()).map_err(|_| Error::TooLarge)?;
        if message.len() > MAX_MESSAGE {
            return Err(Error::TooLarge);
        }
        self.send
            .write_all(&len.to_be_bytes())
            .await
            .map_err(Error::network("送る"))?;
        self.send
            .write_all(message)
            .await
            .map_err(Error::network("送る"))?;
        Ok(())
    }

    /// 送る側を閉じて、**相手が受け取り切るまで待つ。**
    ///
    /// これを呼ばずに落とすと、送る列に残っている分は消える。
    ///
    /// # Errors
    /// 相手が受け取る前に経路が切れたら [`Error::Network`]。
    pub async fn finish(mut self) -> Result<(), Error> {
        self.send.finish().map_err(Error::network("送り終わる"))?;
        self.send
            .stopped()
            .await
            .map_err(Error::network("送り終わる"))?;
        Ok(())
    }

    /// バイト列を 1 つ受け取る。
    ///
    /// # Errors
    /// 相手が挨拶して閉じたら [`Error::Closed`]。
    /// 宣言された長さが [`MAX_MESSAGE`] を超えたら [`Error::TooLarge`]。
    /// 経路が切れたら [`Error::Network`]。
    pub async fn recv(&mut self) -> Result<Vec<u8>, Error> {
        let mut len = [0u8; 4];
        self.recv
            .read_exact(&mut len)
            .await
            // **荷物の切れ目で終わったなら、それは「閉じた」であって「落ちた」ではない。**
            // 途中（`FinishedEarly(1..)`）で終わったのは荷物が千切れているので落ちた扱い
            .map_err(|e| match e {
                ReadExactError::FinishedEarly(0) => Error::Closed,
                other => Error::network("受け取る")(other),
            })?;

        let len = u32::from_be_bytes(len) as usize;
        if len > MAX_MESSAGE {
            return Err(Error::TooLarge);
        }

        let mut message = vec![0u8; len];
        self.recv
            .read_exact(&mut message)
            .await
            .map_err(Error::network("受け取る"))?;
        Ok(message)
    }
}

/// **届かなかったときの言い方**（2026-09-24）。
///
/// **「宛先に届きませんでした」だけでは、3 つが区別できない。**
///
/// | 何が起きたか | 人がやること |
/// |---|---|
/// | 鍵が古い（相手が立ち上げ直して番地が変わった） | **鍵を出し直してもらう** |
/// | 相手が落ちている | 待つ・声をかける |
/// | 別の網に居る | 中継を使う（**D78**） |
///
/// **当てにいった番地を出せば、その場で見分けられる** ——
/// 相手のいまの番地と見比べればよい。
///
/// **番地は秘密ではない**（鍵の中に、そのまま入っている）。
fn 届かなかった言い方(宛先: &Address) -> String {
    let 番地: Vec<String> = 宛先.ip_addrs().map(|a| a.to_string()).collect();
    let 中継 = if 宛先.relay().is_some() {
        "／中継も試しました"
    } else {
        "／中継は使っていません"
    };
    if 番地.is_empty() {
        return format!("宛先に届きませんでした（番地が 1 つも入っていません{中継}）");
    }
    format!(
        "宛先に届きませんでした（当てにいった番地: {}{中継}）。\
         **相手が立ち上げ直して番地が変わった／相手が落ちている／別の網に居る**の\
         どれかです。相手のいまの番地と見比べてください",
        番地.join("・")
    )
}

/// 通り道を見張って知らせること。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum 通り道の出来事 {
    /// 直接・中継・不明の種類が変わった。
    変わった(通り道),
    /// 経路が閉じた。**直接（または中継）の中で番地が入れ替わった回数**を添える。
    閉じた {
        /// 同じ種類のまま、相手の番地が入れ替わった回数。
        番地の入れ替え: u32,
    },
}

impl std::fmt::Display for 通り道の出来事 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::変わった(道) => write!(f, "通り道が変わりました: {道}"),
            Self::閉じた {
                番地の入れ替え
            } => {
                write!(
                    f,
                    "通り道が閉じました（番地の入れ替え {番地の入れ替え} 回）"
                )
            }
        }
    }
}

/// 文字が流れている通り道。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum 通り道 {
    /// 相手の番地へ直に（穴あけ・同じ網）。
    直接(std::net::SocketAddr),
    /// 中継を通っている。
    中継(String),
    /// まだ決まっていない・知らない種類。
    不明,
}

impl 通り道 {
    /// **種類（直接・中継・不明）が変わったときだけ知らせる。**
    ///
    /// 2026-09-25、直接のまま LAN 側と外側の番地を行き来して「変わりました」が 7 行続いた。
    /// 人が知りたいのは「中継を通っているか」であって、直接の中の番地の揺れではない。
    fn 知らせるか(前: Option<&Self>, 今: &Self) -> bool {
        前.is_none_or(|前| std::mem::discriminant(前) != std::mem::discriminant(今))
    }

    fn から(宛: &iroh::TransportAddr) -> Self {
        match 宛 {
            iroh::TransportAddr::Ip(番地) => Self::直接(*番地),
            iroh::TransportAddr::Relay(url) => Self::中継(url.to_string()),
            _ => Self::不明,
        }
    }
}

/// **外側の番地（グローバル IP）は出さない。**記録は人が Issue に貼る
/// （2026-09-25・sshboard の席の指摘）。LAN の番地は出す —— 切り分けに要る。
impl std::fmt::Display for 通り道 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::直接(番地) if 内側か(番地.ip()) => write!(f, "直接（{番地}）"),
            Self::直接(_) => write!(f, "直接（外側の番地）"),
            Self::中継(url) => write!(f, "中継（{url}）"),
            Self::不明 => write!(f, "不明"),
        }
    }
}

fn 内側か(ip: std::net::IpAddr) -> bool {
    match ip {
        std::net::IpAddr::V4(v4) => v4.is_private() || v4.is_loopback() || v4.is_link_local(),
        std::net::IpAddr::V6(v6) => {
            v6.is_loopback() || v6.is_unique_local() || v6.is_unicast_link_local()
        }
    }
}

#[cfg(test)]
mod 通り道の試験 {
    use super::通り道;

    #[test]
    fn 外側の番地は_字に出さない() {
        let 外 = 通り道::直接("203.0.113.5:39756".parse().unwrap());
        assert_eq!(外.to_string(), "直接（外側の番地）");
        assert!(!外.to_string().contains("203.0.113.5"));
    }

    #[test]
    fn 直接の中で番地が揺れても_知らせない() {
        let lan = 通り道::直接("192.168.24.15:61706".parse().unwrap());
        let 外 = 通り道::直接("203.0.113.5:61706".parse().unwrap());
        assert!(!通り道::知らせるか(Some(&lan), &外));
        assert!(!通り道::知らせるか(Some(&外), &lan));
    }

    #[test]
    fn 直接と中継が入れ替わったら_知らせる() {
        let lan = 通り道::直接("192.168.24.15:61706".parse().unwrap());
        let 中 = 通り道::中継("https://relay.example/".to_owned());
        assert!(通り道::知らせるか(Some(&中), &lan));
        assert!(通り道::知らせるか(Some(&lan), &中));
        assert!(通り道::知らせるか(None, &lan), "最初の 1 回は知らせる");
    }

    #[test]
    fn 閉じたときに_番地の入れ替えの回数を言う() {
        let 閉 = super::通り道の出来事::閉じた {
            番地の入れ替え: 7
        };
        assert_eq!(閉.to_string(), "通り道が閉じました（番地の入れ替え 7 回）");
    }

    #[test]
    fn 内側の番地は_字に出す() {
        let 内 = 通り道::直接("192.168.24.3:55335".parse().unwrap());
        assert_eq!(内.to_string(), "直接（192.168.24.3:55335）");
    }
}
