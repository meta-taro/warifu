// カメラとマイク（M5-c2）。
//
// **ここに `getUserMedia` の呼び出しそのものは置かない。**
// 求め方と、断られた理由の読み方だけを持つ。実際に叩く所は `session.ts`。
// こうしてあるので、**カメラの無い機械でも規則を確かめられる。**

import type { LinkPath } from '../link/path';

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

/**
 * 映像を流してよいか（**D29「始まりは音声だけ」**）。
 *
 * **測る前に映像を出さない。**「測っていない」を「たぶん速い」にしない、というのが
 * D28 / D29 の構えである（`Meter` が観測ゼロで `0` と答えないのと同じ）。
 *
 * 経路が分かった時点で流す。**中継でも流す** — 中継は異常ではない（DESIGN.md §4.1）。
 * 見ているのは「速いかどうか」ではなく「**測れたかどうか**」である。
 */
export function shouldSendVideo(path: LinkPath): boolean {
  return path !== 'unknown';
}

/** 何を送っている状態か。 */
export type SendMode = 'both' | 'audio' | 'none';

/**
 * **機器が無くても会議に入れるようにする。**
 *
 * カメラもマイクも無い機械はある（画面だけの端末、会場のモニタ、
 * 音を切って見るだけの人）。**`getUserMedia` が失敗した時点で止めると、
 * 「機器が無い＝参加できない」になる。**
 *
 * 段を下げながら試す。`null` を渡すと最初の段、返り値が `null` なら**もう下げない**
 * （＝何も送らずに入る）。
 */
export function nextAttempt(current: MediaStreamConstraints | null): MediaStreamConstraints | null {
  if (current === null) return { audio: true, video: true };
  if (current.video !== false) return { audio: true, video: false };
  return null;
}

/** いまの段が、何を送っている状態か。 */
export function sendModeFor(constraints: MediaStreamConstraints | null): SendMode {
  if (constraints === null) return 'none';
  return constraints.video === false ? 'audio' : 'both';
}

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
