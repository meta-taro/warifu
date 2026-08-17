//! 結び目と、その上に立つ 1 本の経路。

use core::time::Duration;

use iroh::endpoint::{Connection, RecvStream, SendStream, presets};
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

/// 経路の結び目。1 台に 1 つ。
#[derive(Debug, Clone)]
pub struct Node {
    endpoint: Endpoint,
    key: PublicKey,
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
        let mut raw = device.secret_key_bytes();
        let secret = SecretKey::from_bytes(&raw);
        raw.zeroize();

        let endpoint = Endpoint::builder(presets::Minimal)
            .secret_key(secret)
            .alpns(vec![ALPN.to_vec()])
            .relay_mode(RelayMode::Disabled)
            .bind()
            .await
            .map_err(Error::network("結ぶ"))?;

        Ok(Self {
            endpoint,
            key: device.public_key(),
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
        let mut watcher = self.endpoint.watch_addr();
        loop {
            let addr = watcher.get();
            let ips: Vec<_> = addr.ip_addrs().copied().collect();
            if !ips.is_empty() {
                return Ok(Address::from_parts(self.key, ips));
            }
            watcher
                .updated()
                .await
                .map_err(Error::network("宛先を待つ"))?;
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
        let addr = EndpointAddr::from_parts(id, to.ip_addrs().map(TransportAddr::Ip));

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
    /// 宣言された長さが [`MAX_MESSAGE`] を超えたら [`Error::TooLarge`]。
    /// 経路が切れたら [`Error::Network`]。
    pub async fn recv(&mut self) -> Result<Vec<u8>, Error> {
        let mut len = [0u8; 4];
        self.recv
            .read_exact(&mut len)
            .await
            .map_err(Error::network("受け取る"))?;

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
