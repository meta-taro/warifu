// 入室前の支度（M5-d）。
//
// **入ってから慌てるのが一番困る。**どの会議に入る前でも、
// 自分のカメラとマイクが何で、いま入と切のどちらかを見えるようにする。
//
// ここは純ロジックだけ。`getUserMedia` も `enumerateDevices` も呼ばない
// （呼ぶのは `session.ts` と画面側）。**カメラの無い機械でも規則を確かめられる。**

/** `MediaDeviceInfo` のうち、ここが見る所だけ。 */
export interface DeviceLike {
  kind: string;
  deviceId: string;
  label: string;
  groupId: string;
}

export interface DeviceOption {
  id: string;
  label: string;
}

export interface DeviceOptions {
  cameras: DeviceOption[];
  microphones: DeviceOption[];
}

/** 背景の扱い。**「ぼかす」は環境が持っていれば使う**（下の `canBlurBackground`）。 */
export type Background = 'none' | 'blur';

export interface Prefs {
  micOn: boolean;
  cameraOn: boolean;
  cameraId: string | null;
  micId: string | null;
  background: Background;
}

/**
 * **既定はマイクもカメラも切。**
 *
 * 入った瞬間に映って喋っている状態にしない。
 * 「入る前に確かめられる」ことが目的なのに、既定が入だと**確かめる前に流れる。**
 */
export const DEFAULT_PREFS: Prefs = {
  micOn: false,
  cameraOn: false,
  cameraId: null,
  micId: null,
  background: 'none',
};

const STORAGE_KEY = 'warifu.prefs';

/** 機器の一覧を、選べる形に直す。 */
export function toOptions(devices: readonly DeviceLike[]): DeviceOptions {
  const pick = (kind: string, fallback: string) => {
    const seen = new Set<string>();
    const out: DeviceOption[] = [];
    for (const d of devices) {
      if (d.kind !== kind || seen.has(d.deviceId)) continue;
      seen.add(d.deviceId);
      // **名前が空でも捨てない。**許可を出す前は label が空で返る。
      // 捨てると「機器が無い」に見えて、人は設定画面を探しに行く
      out.push({ id: d.deviceId, label: d.label || fallback });
    }
    return out;
  };
  return {
    cameras: pick('videoinput', 'カメラ'),
    microphones: pick('audioinput', 'マイク'),
  };
}

/**
 * 音の処理。**ハウリング（回り込み）を止める。**
 *
 * WebRTC が持っている機能だが、**明示的に要求しないと環境によって切れる。**
 * 既定に任せない — 1 台で 2 窓を開いた瞬間に鳴き始めるのがこれである。
 *
 * - `echoCancellation` … スピーカーから出た自分の声を、マイク側から差し引く
 * - `noiseSuppression` … 定常的な雑音（空調・ファン）を抑える
 * - `autoGainControl` … 声の大きさを揃える
 *
 * **これだけでは足りない場面がある。**同じ部屋で 2 台を鳴らすと、
 * エコー除去は「自分の出力」しか知らないので、隣の端末の音は消せない。
 * そこはヘッドフォンで解く（画面でそう案内する）。
 */
export const AUDIO_PROCESSING = {
  echoCancellation: true,
  noiseSuppression: true,
  autoGainControl: true,
} as const;

/** 選んだ機器を制約にする。選んでいなければ既定の機器へ任せる。 */
export function constraintsFor(prefs: Prefs): MediaStreamConstraints {
  return {
    video: prefs.cameraId ? { deviceId: { exact: prefs.cameraId } } : true,
    audio: prefs.micId
      ? { deviceId: { exact: prefs.micId }, ...AUDIO_PROCESSING }
      : { ...AUDIO_PROCESSING },
  };
}

/**
 * 保存値を読む。**壊れていても落ちない。**
 *
 * 設定が読めないことは、会議に入れない理由にならない。既定へ戻して先へ進む。
 */
export function loadPrefs(raw: string | null): Prefs {
  if (!raw) return DEFAULT_PREFS;
  let parsed: unknown;
  try {
    parsed = JSON.parse(raw);
  } catch {
    return DEFAULT_PREFS;
  }
  if (typeof parsed !== 'object' || parsed === null) return DEFAULT_PREFS;
  const o = parsed as Record<string, unknown>;
  // 1 つでも型が違えば既定へ戻す。**半分だけ効いている設定を作らない**
  if (typeof o.micOn !== 'boolean' || typeof o.cameraOn !== 'boolean') return DEFAULT_PREFS;
  const id = (v: unknown) => (typeof v === 'string' ? v : null);
  const bg = o.background === 'blur' ? 'blur' : 'none';
  return {
    micOn: o.micOn,
    cameraOn: o.cameraOn,
    cameraId: id(o.cameraId),
    micId: id(o.micId),
    background: bg,
  };
}

/** 読み書きは画面側から。**保存できなくても止めない。** */
export function readStored(): Prefs {
  try {
    return loadPrefs(localStorage.getItem(STORAGE_KEY));
  } catch {
    return DEFAULT_PREFS;
  }
}

export function writeStored(prefs: Prefs): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(prefs));
  } catch {
    // 保存できないことは会議の妨げにならない。**黙って先へ進む**のはここだけ
  }
}

/**
 * **この環境が背景をぼかせるか。**
 *
 * 自前で人と背景を分けるには分割モデル（数 MB）を同梱することになる。
 * **まず環境が持っているかを見る** — 持っていれば、こちらは何も足さずに済む。
 *
 * 持っていない場合、macOS では**コントロールセンターのビデオエフェクト**が
 * アプリの実装なしに効く。画面はそれを案内する（`background.os` の文言）。
 */
export function canBlurBackground(supported: object | undefined): boolean {
  return Boolean(supported && 'backgroundBlur' in supported);
}

/** 背景の希望を制約へ足す。**環境が持っていなければ何も足さない。** */
export function withBackground(
  constraints: MediaStreamConstraints,
  prefs: Prefs,
  blurAvailable: boolean,
): MediaStreamConstraints {
  if (prefs.background !== 'blur' || !blurAvailable) return constraints;
  const video = constraints.video === true ? {} : { ...(constraints.video as object) };
  return { ...constraints, video: { ...video, backgroundBlur: true } as MediaTrackConstraints };
}
