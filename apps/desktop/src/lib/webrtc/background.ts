// カメラの背景（**D126**・**#40**・DESIGN.md §10-C）。
//
// **既定は背景を隠す。**背景を見せること自体ができない人が会議に入れるように。
// 掛けるのは支度の時点から（相手に送る前）。**最初のコマにも背景を出さない。**
//
// ここは純ロジックだけ。模型も canvas も触らない（それは `backgroundPipeline.ts`）。
// **カメラの無い機械でも、規則を試験で確かめられる。**

/** 背景の扱い。`off` は人が明示して選んだときだけ（既定ではない）。 */
export type BackgroundMode = 'off' | 'blur' | 'image';

/** ぼかしの度合い。 */
export type BlurStrength = 'weak' | 'medium' | 'strong';

export const BACKGROUND_MODES: readonly BackgroundMode[] = ['blur', 'image', 'off'];
export const BLUR_STRENGTHS: readonly BlurStrength[] = ['weak', 'medium', 'strong'];

export const DEFAULT_BACKGROUND: BackgroundMode = 'blur';
export const DEFAULT_BLUR_STRENGTH: BlurStrength = 'medium';

/**
 * 保存値を背景の扱いに読む。
 *
 * **明示の `off` だけを `off` にする。**それ以外（前の版の `none`・欠け・壊れ）は既定の `blur`。
 * 前の版の `none` は既定値であって、人が「見せる」と選んだものではない（D126 で既定が変わった）。
 */
export function readBackgroundMode(v: unknown): BackgroundMode {
  if (v === 'off' || v === 'image') return v;
  return DEFAULT_BACKGROUND;
}

export function readBlurStrength(v: unknown): BlurStrength {
  return v === 'weak' || v === 'strong' ? v : DEFAULT_BLUR_STRENGTH;
}

/**
 * **いま実際に掛ける隠し方。**
 *
 * `image` を選んでいても画像がまだ無ければ、ぼかして隠す（**見せる側へは倒さない**）。
 */
export function effectiveMode(mode: BackgroundMode, hasImage: boolean): BackgroundMode {
  if (mode === 'image' && !hasImage) return 'blur';
  return mode;
}

/**
 * **カメラの映像をどう送るか。**
 *
 * 模型が読めなかったとき、**素のカメラへ黙って落とさない**（背景が映る）。
 * 送らずに、読めなかったことを人へ言う。素のまま送るのは、人が `off` を選んだときだけ。
 */
export function videoPlan(
  mode: BackgroundMode,
  segmenter: 'ready' | 'failed',
): 'raw' | 'hidden' | 'withhold' {
  if (mode === 'off') return 'raw';
  return segmenter === 'ready' ? 'hidden' : 'withhold';
}

/** 送る大きさ。**長い辺を上限に収める**（重さを 640×480 前後に抑える）。 */
export function outputSize(
  w: number,
  h: number,
  maxLong = 640,
): { width: number; height: number } {
  if (w <= 0 || h <= 0) return { width: 640, height: 480 };
  const scale = Math.min(1, maxLong / Math.max(w, h));
  return { width: Math.max(2, Math.round(w * scale)), height: Math.max(2, Math.round(h * scale)) };
}

/**
 * ぼかしに使う縮めた大きさ。**縮めて広げるとぼける**（canvas の `filter` は WKWebView で頼れない）。
 * 割る数が大きいほど強い。
 */
export function blurSize(
  w: number,
  h: number,
  strength: BlurStrength,
): { width: number; height: number } {
  const div = strength === 'weak' ? 8 : strength === 'medium' ? 16 : 32;
  return { width: Math.max(4, Math.round(w / div)), height: Math.max(3, Math.round(h / div)) };
}

/** 画像を枠いっぱいに敷くときに切り出す範囲（はみ出しは切る・縦横比は崩さない）。 */
export function coverCrop(
  srcW: number,
  srcH: number,
  dstW: number,
  dstH: number,
): { sx: number; sy: number; sw: number; sh: number } {
  const src = srcW / srcH;
  const dst = dstW / dstH;
  if (src > dst) {
    const sw = srcH * dst;
    return { sx: (srcW - sw) / 2, sy: 0, sw, sh: srcH };
  }
  const sh = srcW / dst;
  return { sx: 0, sy: (srcH - sh) / 2, sw: srcW, sh };
}

/** 保存する画像の大きさ。**手元の保存場所を食わない**よう、この大きさへ縮める。 */
export function fitWithin(
  w: number,
  h: number,
  maxW = 1280,
  maxH = 720,
): { width: number; height: number } {
  const scale = Math.min(1, maxW / w, maxH / h);
  return { width: Math.max(1, Math.round(w * scale)), height: Math.max(1, Math.round(h * scale)) };
}

/**
 * 人らしさ（0〜1）を、重ねるときの濃さ（0〜255）へ。
 *
 * **縁を少し締める。**そのままだと背景がうっすら残る所が出る。
 * 0.25 以下は背景として消し、0.75 以上は人として残し、あいだは滑らかに繋ぐ。
 */
export function maskAlpha(confidence: number): number {
  const t = Math.min(1, Math.max(0, (confidence - 0.25) / 0.5));
  return Math.round(t * t * (3 - 2 * t) * 255);
}

/**
 * 模型へ渡す時刻（ミリ秒）。**必ず前より進める**（同じ時刻を 2 度渡すと模型が断る）。
 */
export function nextTimestamp(now: number, last: number): number {
  return now > last ? now : last + 0.001;
}

/** 速さの段。**記録は段が変わったときだけ**書く（毎コマは書かない）。 */
export type SpeedBand = 'ok' | 'slow';

/** 目安は 15 fps。これを下回ると、相手には動きがカクついて見える。 */
export function speedBand(fps: number): SpeedBand {
  return fps >= 15 ? 'ok' : 'slow';
}

/**
 * **コマ数を数える。**窓（既定 2 秒）ごとに 1 度だけ値を出す。
 *
 * 時刻は外から渡す（試験で時間を進められるように）。
 */
export class FpsMeter {
  private start: number | null = null;
  private frames = 0;
  private latest: number | null = null;

  constructor(private readonly windowMs = 2000) {}

  /** 1 コマ出した。窓が閉じたら、その窓の fps を返す（閉じていなければ null）。 */
  tick(now: number): number | null {
    if (this.start === null) {
      this.start = now;
      this.frames = 0;
    }
    this.frames += 1;
    const elapsed = now - this.start;
    if (elapsed < this.windowMs) return null;
    const fps = (this.frames * 1000) / elapsed;
    this.latest = Math.round(fps * 10) / 10;
    this.start = now;
    this.frames = 0;
    return this.latest;
  }

  /** いちばん新しい窓の値（まだ無ければ null）。 */
  get fps(): number | null {
    return this.latest;
  }
}

/**
 * 段が変わったときだけ知らせる。**初めて測れたときも 1 度知らせる。**
 */
export class BandWatcher {
  private band: SpeedBand | null = null;

  /** 変わったら新しい段を返す。変わっていなければ null。 */
  observe(fps: number): SpeedBand | null {
    const next = speedBand(fps);
    if (next === this.band) return null;
    this.band = next;
    return next;
  }
}
