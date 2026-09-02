// カメラとマイク（M5-c2）。
//
// **ここに `getUserMedia` の呼び出しそのものは置かない。**
// 求め方と、断られた理由の読み方だけを持つ。実際に叩く所は `session.ts`。
// こうしてあるので、**カメラの無い機械でも規則を確かめられる。**

/** 何を求めるか。 */
export function mediaConstraints(options?: { video?: boolean }): MediaStreamConstraints {
  return { audio: true, video: options?.video ?? true };
}

/**
 * 経路の当て（STUN / TURN）。**既定は空。**
 *
 * `issues/005` の満たすこと 4 は「シグナリングを外部インフラに頼らない」であり、
 * SDP は warifu の E2EE チャネルで運んでいる。**経路の当ても同じ構えにする** —
 * 既定で他人のサーバへ自分の存在を知らせない。
 *
 * 同じ網の中（LAN・同じ Wi-Fi）なら、これで直接つながる。
 * **NAT を越えられないときは繋がらない** — それは D13 が「中継が要ると分かった時点で
 * 自前 Relay かどうかを D3 と一緒に決める」と書いた所であり、**ここで勝手に足さない。**
 */
export const ICE_SERVERS: readonly RTCIceServer[] = [];

/** カメラ・マイクを取れなかった理由。**画面の文言の鍵になる。** */
export type MediaFailure = 'camera-denied' | 'camera-missing' | 'camera-busy' | 'camera-unknown';

/**
 * 断られた理由を読む。
 *
 * **分からない失敗を、分かったことにしない。**
 * 「許可されていない」と「機器が無い」を混ぜると、人は設定画面を探し続ける。
 */
export function describeMediaFailure(error: unknown): MediaFailure {
  const name = error instanceof DOMException ? error.name : '';
  if (name === 'NotAllowedError' || name === 'SecurityError') return 'camera-denied';
  if (name === 'NotFoundError' || name === 'OverconstrainedError') return 'camera-missing';
  if (name === 'NotReadableError' || name === 'AbortError') return 'camera-busy';
  return 'camera-unknown';
}
