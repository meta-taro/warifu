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
/**
 * 経路が終わった。`[公開鍵, 訳]` で届く。
 *
 * **訳を混ぜない。**人はこの 2 つで次の手が変わる —— `left` なら会議は終わり、
 * `lost` なら**会議キーを作り直して渡し直す**（割符は一度きり・**D12**）。
 */
export const EVENT_CLOSED = 'warifu://closed';

/** 経路が終わった訳。Rust 側（`lib.rs`）と同じ文字列を持つ。 */
export type ClosedReason = 'left' | 'lost';
/** 誰かの住所を教わった（**D41**）。`[公開鍵, 住所]` で届く。 */
export const EVENT_INTRODUCED = 'warifu://introduced';
/** 文字が届いた。`[誰から, 中身]` で届く。 */
export const EVENT_TEXT = 'warifu://text';
/**
 * **この PC の机から出た発言**（`[公開鍵, 中身, 時刻]`）。
 *
 * 同じ席の AI（`warifu mcp` で繋いだエージェント）が言ったもの。
 * **相手から届いた文字（{@link EVENT_TEXT}）と分ける** ——
 * 混ぜると、誰が言ったのか画面から読めなくなる。
 */
export const EVENT_DESK = 'warifu://desk';
/**
 * **机に着いている顔ぶれ**が変わった（呼び方の並びが届く）。
 *
 * 会議に人が居なくても、**同じ席の AI が居るなら人は話しかけられる。**
 * これが無いと、AI が居るのに「入ってきたら送れます」と出たままになる。
 */
export const EVENT_DESK_SEATS = 'warifu://desk-seats';

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
/**
 * 文字を送る。
 *
 * `to` に**同じ PC の AI の呼び方**を渡すと、**その席にだけ**届く。
 * 渡さなければ、部屋に居る全員と机の AI 全員へ。
 */
export const sendText = (body: string, to?: string | null) =>
  invoke<void>('send_text', { body, to: to ?? null });

/**
 * いま机に何人着いているか。
 *
 * **「相手が居ない」と「話し相手が 1 人も居ない」は違う。**
 * 会議に人が居なくても、同じ席の AI が居るなら送れる。
 */
export const deskSeats = () => invoke<string[]>('desk_seats');

/**
 * **覚えた相手を、割符なしで呼ぶ。**会議キーを手で渡さない。
 *
 * 住所を覚えていなければ、繋ぎに行かずにすぐ断る（押した人を待たせない・D49）。
 * 繋がらなかった理由は分けない —— 「居ない」も「断られた」も同じ言い分が返る
 * （分けると、断る理由を相手に返さない D31 が画面越しに崩れる）。
 */
export const callContact = (key: string) => invoke<void>('call_contact', { key });

/**
 * **相手を戸口から降ろす。**次からは割符が要る。
 *
 * 知り合いを保存した以上、取り消す口が要る ——
 * 保存する前は、間違って開けた相手もアプリを閉じれば切れていた。
 */
export const stopKnowing = (key: string) => invoke<boolean>('stop_knowing', { key });

/**
 * **いま会議キーなしで入れる相手**の公開鍵を並べる。
 *
 * **覚えている相手とは別の集まりである。**呼び名を付けただけでは、
 * 鍵なしでは入れない（一度通した相手だけが入れる）。
 */
export const knownKeys = () => invoke<string[]>('known_keys');

/**
 * **同じ PC の AI に「止まれ」と言う。**
 *
 * 常駐（`warifu agent`）は人が居ない間も動く。
 * **落とすしか止め方が無い状態にしない。**
 * 相手のプロセスを殺すのではなく、**受けた側が自分で降りる。**
 */
export const stopAgent = (name: string) => invoke<boolean>('stop_agent', { name });

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

// **`introduce` の口はここに置かない。**
//
// 2026-09-07 まで `invoke('introduce', …)` を export していたが、
// **Rust 側にその命令は無く、呼び出し元も 1 つも無かった。**
// 押せるのに効かない口を残さない（**D49**）。
//
// 住所の名乗り（D41）は Rust の中で済ませる —— `connect` と `call_contact` が
// 繋がった直後に `Notice::Introduce` を送る。**画面に押させると、
// 押し忘れで住所が入らない経路ができる。**

/** 覚えている相手 1 人。**呼び名と鍵の組だけ。** */
export interface ContactRow {
  /** 公開鍵（base32・全桁）。 */
  key: string;
  /** 人が付けた呼び名。 */
  label: string;
  /**
   * **居場所を覚えているか。**覚えていれば、名前を押して呼べる。
   *
   * 住所そのものは渡ってこない —— 画面に要るのは「押せるかどうか」だけで、
   * 中身を出しても人には読めない。
   */
  has_address: boolean;
}

/**
 * 覚えている相手を並べる。
 *
 * **CLI（`warifu contacts`）と同じ置き場所を読む。**別の機械のエージェントに
 * 名前を付けておけば、画面のチャットにもその名前で出る。
 */
export const contacts = () => invoke<ContactRow[]>('contacts');

/** 相手を覚える。**呼び名を空にすると忘れる。** */
export const remember = (key: string, label: string) =>
  invoke<void>('remember', { key, label });

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
