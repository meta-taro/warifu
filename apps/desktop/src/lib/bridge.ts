// Tauri の口を画面から呼ぶための薄い層（M5-c2）。
//
// **ブラウザで開いたときは何もしない。**Tauri の API はブラウザに無く、
// 落ちると「画面が真っ白」という一番読みにくい壊れ方になる（`window/controls.ts` と同じ構え）。
//
// 名前は Rust 側（`src-tauri/src/lib.rs`）と揃える。**ずれたら黙って届かなくなる**ので、
// 定数を 1 か所に置いて両側から参照できるようにしてある（Rust 側は同じ文字列を持つ）。

export const EVENT_JOINED = 'warifu://joined';
export const EVENT_LEFT = 'warifu://left';
export const EVENT_SIGNAL = 'warifu://signal';
export const EVENT_CLOSED = 'warifu://closed';

/** 相手から届いた下ごしらえ 1 通。 */
export interface SignalPayload {
  step: 'offer' | 'answer' | 'candidate' | 'end';
  blob: string;
  from?: string;
}

export function inTauri(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

async function invoke<T>(command: string, args?: Record<string, unknown>): Promise<T | null> {
  if (!inTauri()) return null;
  const { invoke: call } = await import('@tauri-apps/api/core');
  return call<T>(command, args);
}

/** 自分の宛先。**これだけでは繋げない**（割符が要る・D31）。 */
export const myAddress = () => invoke<string>('my_address');

/**
 * 招待を出す。**宛先と割符を 1 本にした文字列**が返る。
 *
 * 宛先だけを渡す形にしない。それでは受け取った側が誰でも繋げてしまう（D31）。
 * 出すたびに前の招待は無効になる。
 */
export const invite = (ttlSecs: number) => invoke<string>('invite', { ttlSecs });

/**
 * **OS のメニューを、画面と同じ言語にする**（D35）。
 *
 * 画面の中だけを訳しても足りない。macOS では窓の外にメニューが出る。
 * 言語は画面側が決めた答えを渡す — Rust 側で OS へ聞き直すと、2 か所が別の答えを出しうる。
 */
export const setMenuLocale = (locale: string) => invoke<void>('set_menu_locale', { locale });

/** 自分の公開鍵。画面が「自分かどうか」を見分けるのに使う。 */
export const myKey = () => invoke<string>('my_key');

/** 会議を作る。定員は `2..=16`（D27）。 */
export const hostMeeting = (capacity: number) => invoke<string>('host_meeting', { capacity });

/**
 * 会議キーで入室する。**宛先だけでは通らない。**
 *
 * 自分の会議キーを貼ったときは、Rust 側が `meeting.key.own` を返す。
 * **下の層（iroh）の英語をそのまま出さない** — 画面が辞書から訳す。
 */
export const connect = (invite: string) => invoke<void>('connect', { invite });

/** 待ち受けを始める。**呼ぶ側だけでは 2 台は出会えない。** */
export const listen = () => invoke<void>('listen');

/** 相手に対して自分が offer を出す側か（D38）。 */
export const shouldOfferTo = (peer: string) => invoke<boolean>('should_offer_to', { peer });

/** 下ごしらえを 1 通送る。**中身は解釈しない。** */
export const sendSignal = (step: SignalPayload['step'], blob: string) =>
  invoke<void>('send_signal', { payload: { step, blob } });

/** 出来事を受け取る。Tauri の外では何も起きない（購読解除だけ返す）。 */
export async function onEvent<T>(name: string, handler: (payload: T) => void): Promise<() => void> {
  if (!inTauri()) return () => {};
  const { listen: subscribe } = await import('@tauri-apps/api/event');
  const un = await subscribe<T>(name, (e) => handler(e.payload));
  return un;
}
