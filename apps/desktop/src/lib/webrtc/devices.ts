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

import {
  DEFAULT_BACKGROUND,
  DEFAULT_BLUR_STRENGTH,
  readBackgroundMode,
  readBlurStrength,
  type BackgroundMode,
  type BlurStrength,
} from './background';

/** 背景の扱い（**D126**）。中身は `background.ts`。 */
export type Background = BackgroundMode;

export interface Prefs {
  micOn: boolean;
  cameraOn: boolean;
  cameraId: string | null;
  micId: string | null;
  background: Background;
  /** ぼかしの度合い（`background` が `blur` のときに効く）。 */
  blurStrength: BlurStrength;
}

/**
 * **既定はマイクもカメラも切。**
 *
 * 入った瞬間に映って喋っている状態にしない。
 * 「入る前に確かめられる」ことが目的なのに、既定が入だと**確かめる前に流れる。**
 *
 * **背景は既定で隠す**（**D126**・DESIGN.md §10-C）。見せたい人だけが切る。
 */
export const DEFAULT_PREFS: Prefs = {
  micOn: false,
  cameraOn: false,
  cameraId: null,
  micId: null,
  background: DEFAULT_BACKGROUND,
  blurStrength: DEFAULT_BLUR_STRENGTH,
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
 * **カメラを切ったまま（マイクをミュートしたまま）、これだけ経ったら放す**（**#44**）。
 *
 * カメラを一瞬切って、少し動いてからすぐ入れ直す使い方がある。
 * **すぐ入れ直す人を待たせない長さ**にする（放すと、つかみ直すまで映らない）。
 */
export const 機器を放すまでの秒 = 60;

/**
 * **カメラ・マイクをどうするか**（**#44**・2026-09-28）。
 *
 * **マイクも同じ判断を使う**（同日。出す前に実物で測る）。
 *
 * 前は切にしても映像を止めるだけで、**カメラはつかんだまま**だった（38 分つかみっぱなし）。
 * 「切ってあります」と書いてあるのに明かりが点いていると、人はこっそり撮られていると読む。
 * 使っていない機器は放す、という行儀の問題として扱う。
 */
export function 機器をどうするか(場: {
  入: boolean;
  掴んでいる: boolean;
  放してある: boolean;
}): '待って放す' | 'つかみ直す' | 'そのまま' {
  if (!場.入 && 場.掴んでいる) return '待って放す';
  if (場.入 && !場.掴んでいる && 場.放してある) return 'つかみ直す';
  return 'そのまま';
}

/**
 * **「カメラ（マイク）が見つかりません」と言うか。**
 *
 * 機器の一覧は、許可が下りたあとに初めて読み直す。**許可の窓が出ている間は一覧が空**なので、
 * そこで「無い」と言うと嘘になる（2026-09-28・Windows の WebView2 で出た）。
 */
export function 機器が無いと言うか(場: {
  支度した: boolean;
  許可を待っている: boolean;
  本数: number;
}): boolean {
  return 場.支度した && !場.許可を待っている && 場.本数 === 0;
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
 * **これだけでは足りない場面がある。**同じルームで 2 台を鳴らすと、
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
  return {
    micOn: o.micOn,
    cameraOn: o.cameraOn,
    cameraId: id(o.cameraId),
    micId: id(o.micId),
    // 前の版の `none` は既定値だったので、隠す側へ読み替える（`readBackgroundMode`）
    background: readBackgroundMode(o.background),
    blurStrength: readBlurStrength(o.blurStrength),
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
 * **差し替える背景の画像**（**D126**）。data URL のまま、この機械の中にだけ置く。
 *
 * 好み（`warifu.prefs`）とは分けて置く。好みは入切のたびに書き直すので、
 * 画像まで毎回書き直さない。
 */
const IMAGE_KEY = 'warifu.background.image';

export function readBackgroundImage(): string | null {
  try {
    const v = localStorage.getItem(IMAGE_KEY);
    return v && v.startsWith('data:image/') ? v : null;
  } catch {
    return null;
  }
}

/** 置けなかったら false（容量が足りないなど）。**黙って捨てず、画面が言う。** */
export function writeBackgroundImage(dataUrl: string): boolean {
  try {
    localStorage.setItem(IMAGE_KEY, dataUrl);
    return true;
  } catch {
    return false;
  }
}
