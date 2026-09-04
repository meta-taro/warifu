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
/** 誰かの住所を教わった（**D41**）。`[公開鍵, 住所]` で届く。 */
export const EVENT_INTRODUCED = 'warifu://introduced';
/** 文字が届いた。`[誰から, 中身]` で届く。 */
export const EVENT_TEXT = 'warifu://text';

/** 相手から届いた下ごしらえ 1 通。 */
export interface SignalPayload {
  step: 'offer' | 'answer' | 'candidate' | 'end';
  blob: string;
  /** 誰から（受け取ったときだけ）。 */
  from?: string;
  /** 誰へ（送るときだけ）。**3 人以上では省けない**（M6）。 */
  to?: string;
}

/** Rust 側が返す失敗。`code` があれば**画面が訳す**（文言を 2 か所に持たない）。 */
export interface Failure {
  message: string;
  code?: string;
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
 *
 * `startsAt`（Unix 秒）を渡すと、**その時刻までは誰も入れない**（D43）。
 * 予定に紐づく鍵を前もって配るための口。渡さなければ「いまから」。
 */
export const invite = (ttlSecs: number, startsAt?: number) =>
  invoke<string>('invite', { ttlSecs, startsAt: startsAt ?? null });

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

/**
 * **会議から抜けると告げる。**
 *
 * 告げないと、相手の名簿からは経路が切れたときにしか消えない。
 * 2 人なら経路が切れれば分かるが、**3 人以上では他の人の名簿に残り続ける。**
 */
export const leave = () => invoke<void>('leave');

/**
 * 文字を送る（チャット）。**会議に入っている全員へ。**
 *
 * 下ごしらえ（SDP）と違って、文字は組ごとのものではないので宛先を指定しない。
 * **残らない。**閉じれば消える（保存には身元が続く必要があり、D2 が未決）。
 */
export const sendText = (body: string) => invoke<void>('send_text', { body });

/**
 * 画面の出来事を、Rust と同じログへ流す。
 *
 * WebView のコンソールはターミナルに出ない。**画面側だけで起きたことが見えないと、
 * 切り分けが「Rust までは来ていた」で止まる。**
 *
 * **短い一言だけ**を渡す。中身（SDP・鍵・住所）は渡さない。
 */
export const log = (message: string) => void invoke<void>('log', { message });

/** 待ち受けを始める。**呼ぶ側だけでは 2 台は出会えない。** */
export const listen = () => invoke<void>('listen');

/**
 * **入った人を、既に居る面々へ紹介する**（D41）。
 *
 * 名簿は公開鍵しか運ばないので、3 人目は既存の面々の住所を知る手段が無い。
 * **主催者だけが配る** — 主催者でなければ何も起きない（断りではない）。
 */
export const introduce = (newcomer: string, address: string) =>
  invoke<void>('introduce', { newcomer, address });

/** 相手に対して自分が offer を出す側か（D38）。 */
export const shouldOfferTo = (peer: string) => invoke<boolean>('should_offer_to', { peer });

/** 下ごしらえを 1 通送る。**中身は解釈しない。** */
export const sendSignal = (step: SignalPayload['step'], blob: string, to?: string) =>
  invoke<void>('send_signal', { payload: { step, blob, to } });

/** 出来事を受け取る。Tauri の外では何も起きない（購読解除だけ返す）。 */
export async function onEvent<T>(name: string, handler: (payload: T) => void): Promise<() => void> {
  if (!inTauri()) return () => {};
  const { listen: subscribe } = await import('@tauri-apps/api/event');
  const un = await subscribe<T>(name, (e) => handler(e.payload));
  return un;
}
