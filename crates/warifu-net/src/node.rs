//! 結び目と、その上に立つ 1 本の経路。

use core::time::Duration;

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
        let mut raw = device.secret_key_bytes();
        let secret = SecretKey::from_bytes(&raw);
        raw.zeroize();

        let 中継の設定 = match 中継 {
            中継の使い方::使わない => RelayMode::Disabled,
            中継の使い方::使う => RelayMode::Default,
        };

        let endpoint = Endpoint::builder(presets::Minimal)
            .secret_key(secret)
            .alpns(vec![ALPN.to_vec()])
            .relay_mode(中継の設定)
            .bind()
            .await
            .map_err(Error::network("結ぶ"))?;

        Ok(Self {
            endpoint,
            key: device.public_key(),
            中継,
        })
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
            let ips: Vec<_> = addr.ip_addrs().copied().collect();
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
                cause: "宛先に届きませんでした".into(),
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
