// カメラの背景を隠す流れ（**D126**・**#40**・DESIGN.md §10-C）。
//
//   カメラ → 隠した <video> → 人の塗り分け（模型）→ canvas で重ねる → captureStream → 相手
//
// **両方の画面部品で動く形にする。**MediaStreamTrackProcessor（Insertable Streams）は
// WKWebView に無いので使わない。canvas の `filter` も WKWebView で頼れないので、
// ぼかしは「縮めて広げる」で作る。
//
// **模型と wasm は配布物に同梱する**（`?url` で画面の資材として出す）。外へ読みに行かない。
// 版は 0.10.35 に留めてある —— 1.0 系は使い方の記録を外へ送る処理を内蔵している。
//
// 純ロジック（大きさ・濃さ・fps の数え方・送るかの判断）は `background.ts`（試験あり）。

import type { ImageSegmenter, ImageSegmenterResult } from '@mediapipe/tasks-vision';
import wasmLoaderSimd from '@mediapipe/tasks-vision/vision_wasm_internal.js?url';
import wasmBinarySimd from '@mediapipe/tasks-vision/vision_wasm_internal.wasm?url';
import wasmLoaderNoSimd from '@mediapipe/tasks-vision/vision_wasm_nosimd_internal.js?url';
import wasmBinaryNoSimd from '@mediapipe/tasks-vision/vision_wasm_nosimd_internal.wasm?url';
import modelUrl from './models/selfie_segmenter.tflite?url';
import {
  BandWatcher,
  FpsMeter,
  blurSize,
  coverCrop,
  fitWithin,
  maskAlpha,
  nextTimestamp,
  outputSize,
  type BackgroundMode,
  type BlurStrength,
  type SpeedBand,
} from './background';

/** 何コマで回すか。**30 を狙い、重ければ落ちる**（fps は測って記録へ出す）。 */
const TARGET_FPS = 30;
/** 模型へ渡す絵の幅。模型の中は 256 四方なので、これより大きく渡しても細かくならない。 */
const SEG_WIDTH = 256;
/** 模型を読むのを待つ上限。**待ち続けてカメラが出ないままにしない。** */
const LOAD_TIMEOUT_MS = 20_000;

export interface HideSettings {
  mode: Exclude<BackgroundMode, 'off'>;
  strength: BlurStrength;
  /** `image` のときに敷く画像（data URL）。 */
  image: string | null;
}

export interface HiddenVideo {
  /** 相手と自分の枠へ渡すトラック。**`stop()` で元のカメラと流れも止まる。** */
  track: MediaStreamTrack;
  /** 隠し方を変える（掴み直さない）。 */
  update(settings: HideSettings): void;
  /** いちばん新しい fps（まだ測れていなければ null）。 */
  fps(): number | null;
  /** 模型がどこで動いているか。 */
  delegate: 'GPU' | 'CPU';
}

// ── 模型 ───────────────────────────────────────────────

let loading: Promise<{ segmenter: ImageSegmenter; delegate: 'GPU' | 'CPU' }> | null = null;
let lastTimestamp = 0;

async function createSegmenter(): Promise<{ segmenter: ImageSegmenter; delegate: 'GPU' | 'CPU' }> {
  const { FilesetResolver, ImageSegmenter } = await import('@mediapipe/tasks-vision');
  const simd = await FilesetResolver.isSimdSupported();
  const fileset = simd
    ? { wasmLoaderPath: wasmLoaderSimd, wasmBinaryPath: wasmBinarySimd }
    : { wasmLoaderPath: wasmLoaderNoSimd, wasmBinaryPath: wasmBinaryNoSimd };
  const options = (delegate: 'GPU' | 'CPU') => ({
    baseOptions: { modelAssetPath: modelUrl, delegate },
    runningMode: 'VIDEO' as const,
    outputConfidenceMasks: true,
    outputCategoryMask: false,
  });
  // **GPU（WebGL）を先に試す。**使えない機械では CPU で動かす（遅いが、隠せる）
  try {
    return { segmenter: await ImageSegmenter.createFromOptions(fileset, options('GPU')), delegate: 'GPU' };
  } catch {
    // 握り潰す理由: GPU が無い・WebGL が使えない機械は CPU で続ける。CPU でも落ちれば下で投げる
    return { segmenter: await ImageSegmenter.createFromOptions(fileset, options('CPU')), delegate: 'CPU' };
  }
}

/**
 * 模型を読む（**1 度だけ**。つかみ直しのたびに読み直さない）。
 * **読めなかったら次の呼びで読み直す**（一度の失敗を覚えたままにしない）。
 */
export function loadSegmenter(): Promise<{ segmenter: ImageSegmenter; delegate: 'GPU' | 'CPU' }> {
  if (!loading) {
    const timeout = new Promise<never>((_, reject) =>
      setTimeout(() => reject(new Error(`模型を ${LOAD_TIMEOUT_MS / 1000} 秒で読めませんでした`)), LOAD_TIMEOUT_MS),
    );
    loading = Promise.race([createSegmenter(), timeout]);
    loading.catch(() => {
      loading = null;
    });
  }
  return loading;
}

// ── 拍子 ───────────────────────────────────────────────

/** worker で刻む。作れなければ画面の setTimeout で刻む（窓が隠れると間引かれる）。 */
function startTicker(ms: number, onTick: () => void): () => void {
  try {
    const worker = new Worker(new URL('./backgroundTick.worker.ts', import.meta.url), {
      type: 'module',
    });
    worker.onmessage = onTick;
    worker.postMessage(ms);
    return () => {
      worker.postMessage(0);
      worker.terminate();
    };
  } catch {
    // 握り潰す理由: worker が作れない環境でも、画面の時計で回せば隠して送れる
    const id = setInterval(onTick, ms);
    return () => clearInterval(id);
  }
}

// ── 絵を重ねる ─────────────────────────────────────────

function canvas2d(w: number, h: number): [HTMLCanvasElement, CanvasRenderingContext2D] {
  const c = document.createElement('canvas');
  c.width = w;
  c.height = h;
  const ctx = c.getContext('2d');
  if (!ctx) throw new Error('canvas の 2D が使えません');
  return [c, ctx];
}

function loadImage(src: string): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const img = new Image();
    img.onload = () => resolve(img);
    img.onerror = () => reject(new Error('背景の画像を読めませんでした'));
    img.src = src;
  });
}

/** 人の塗り分け（0〜1）を、濃さの絵へ書き写す。 */
function writeMask(mask: Float32Array, out: ImageData): void {
  const px = out.data;
  for (let i = 0; i < mask.length; i++) px[i * 4 + 3] = maskAlpha(mask[i]);
}

/**
 * **カメラのトラックから、背景を隠したトラックを作る。**
 *
 * 返るのは、模型を読み終えてから。**隠していないコマを 1 枚も出さない**
 * （塗り分けが出るまで canvas には何も描かない）。
 *
 * 読めなければ投げる。**呼んだ側は素のカメラへ落とさないこと**（`videoPlan`）。
 */
export async function hideBackground(
  source: MediaStreamTrack,
  initial: HideSettings,
  onSpeed: (band: SpeedBand, fps: number) => void,
  onError: (reason: string) => void,
): Promise<HiddenVideo> {
  const { segmenter, delegate } = await loadSegmenter();

  // 隠した <video>。**DOM に置く**（WKWebView は DOM に無い video のコマを進めないことがある）
  const video = document.createElement('video');
  video.muted = true;
  video.playsInline = true;
  video.setAttribute('aria-hidden', 'true');
  video.style.cssText =
    'position:fixed;left:-10px;top:-10px;width:1px;height:1px;opacity:0;pointer-events:none;';
  video.srcObject = new MediaStream([source]);
  document.body.appendChild(video);
  try {
    return await build(source, video, segmenter, delegate, initial, onSpeed, onError);
  } catch (e) {
    // **途中で落ちたら、作りかけを残さない**（隠した video が DOM に残り続ける）
    video.srcObject = null;
    video.remove();
    throw e;
  }
}

/** 映像の大きさが分かるまで待つ（分からないまま大きさを決めると 0×0 になる）。 */
function metadataReady(video: HTMLVideoElement): Promise<void> {
  if (video.readyState >= 1 && video.videoWidth > 0) return Promise.resolve();
  return new Promise((resolve) => {
    const done = () => resolve();
    video.addEventListener('loadedmetadata', done, { once: true });
    setTimeout(done, 3000);
  });
}

async function build(
  source: MediaStreamTrack,
  video: HTMLVideoElement,
  segmenter: ImageSegmenter,
  delegate: 'GPU' | 'CPU',
  initial: HideSettings,
  onSpeed: (band: SpeedBand, fps: number) => void,
  onError: (reason: string) => void,
): Promise<HiddenVideo> {
  await video.play().catch(() => {
    // 握り潰す理由: 自動再生が断られても、コマが来れば readyState が上がって回る
  });
  await metadataReady(video);
  const s = source.getSettings();
  const size = outputSize(video.videoWidth || s.width || 0, video.videoHeight || s.height || 0);
  const [out, ctx] = canvas2d(size.width, size.height);
  const segSize = { width: SEG_WIDTH, height: Math.round((SEG_WIDTH * size.height) / size.width) };
  const [segIn, segCtx] = canvas2d(segSize.width, segSize.height);
  const [maskCanvas, maskCtx] = canvas2d(segSize.width, segSize.height);
  const maskData = maskCtx.createImageData(segSize.width, segSize.height);

  let settings = initial;
  let bgImage: HTMLImageElement | null = null;
  let blur = canvas2d(4, 3);
  const mid = canvas2d(Math.max(8, Math.round(size.width / 4)), Math.max(6, Math.round(size.height / 4)));

  const applySettings = (next: HideSettings) => {
    settings = next;
    const b = blurSize(size.width, size.height, next.strength);
    blur = canvas2d(b.width, b.height);
    bgImage = null;
    if (next.mode === 'image' && next.image) {
      void loadImage(next.image).then(
        (img) => {
          if (settings === next) bgImage = img;
        },
        () => {
          // 画像が読めなければ、ぼかしのまま隠す（見せる側へは倒さない）
        },
      );
    }
  };
  applySettings(initial);

  const drawBackground = () => {
    if (settings.mode === 'image' && bgImage) {
      const c = coverCrop(bgImage.naturalWidth, bgImage.naturalHeight, size.width, size.height);
      ctx.drawImage(bgImage, c.sx, c.sy, c.sw, c.sh, 0, 0, size.width, size.height);
      return;
    }
    // **縮めて広げる。**2 段で縮めると、1 段より縞が出にくい
    const [midC, midCtx] = mid;
    const [blurC, blurCtx] = blur;
    midCtx.drawImage(video, 0, 0, midC.width, midC.height);
    blurCtx.drawImage(midC, 0, 0, blurC.width, blurC.height);
    ctx.imageSmoothingEnabled = true;
    ctx.imageSmoothingQuality = 'high';
    ctx.drawImage(blurC, 0, 0, size.width, size.height);
  };

  const composite = (result: ImageSegmenterResult) => {
    const mask = result.confidenceMasks?.[0];
    if (!mask) return;
    writeMask(mask.getAsFloat32Array(), maskData);
    maskCtx.putImageData(maskData, 0, 0);
    // 人の形 → そこへ映像 → その下に背景
    ctx.globalCompositeOperation = 'copy';
    ctx.drawImage(maskCanvas, 0, 0, size.width, size.height);
    ctx.globalCompositeOperation = 'source-in';
    ctx.drawImage(video, 0, 0, size.width, size.height);
    ctx.globalCompositeOperation = 'destination-over';
    drawBackground();
    ctx.globalCompositeOperation = 'source-over';
  };

  const stream = out.captureStream(TARGET_FPS);
  const track = stream.getVideoTracks()[0];
  if (!track) throw new Error('canvas からトラックを取れませんでした');

  const meter = new FpsMeter();
  const watcher = new BandWatcher();
  let lastVideoTime = -1;
  let failed = false;

  const frame = () => {
    // **切ってあるときは回さない**（黒を送っている間に重さを使わない）
    if (failed || !track.enabled || track.readyState !== 'live') return;
    if (video.readyState < 2 || video.videoWidth === 0) return;
    // 同じコマを 2 度塗らない
    if (video.currentTime === lastVideoTime) return;
    lastVideoTime = video.currentTime;
    try {
      segCtx.drawImage(video, 0, 0, segSize.width, segSize.height);
      lastTimestamp = nextTimestamp(performance.now(), lastTimestamp);
      segmenter.segmentForVideo(segIn, lastTimestamp, composite);
    } catch (e) {
      // **塗り分けが落ちたら、描くのをやめる**（素の映像を描き始めない）。
      // canvas には最後に隠したコマが残る。理由は 1 度だけ言う
      failed = true;
      onError(e instanceof Error ? e.message : String(e));
      return;
    }
    const fps = meter.tick(performance.now());
    if (fps !== null) {
      const band = watcher.observe(fps);
      if (band) onSpeed(band, fps);
    }
  };

  const stopTicker = startTicker(Math.round(1000 / TARGET_FPS), frame);

  // **止めると、元のカメラと流れも止まる。**
  // 画面の側は送っているトラックを `stop()` するだけで放せる（60 秒で放す・やめる・抜ける）
  let stopped = false;
  const nativeStop = track.stop.bind(track);
  const stopAll = () => {
    if (stopped) return;
    stopped = true;
    stopTicker();
    nativeStop();
    source.stop();
    video.srcObject = null;
    video.remove();
  };
  Object.defineProperty(track, 'stop', { value: stopAll, configurable: true });
  // カメラが抜かれたら、こちらも止める
  source.addEventListener('ended', stopAll);

  return {
    track,
    update: applySettings,
    fps: () => meter.fps,
    delegate,
  };
}

/**
 * **手元の画像を、背景に敷ける形へ整える**（縮めて JPEG の data URL にする）。
 * 画像は外へ出さない。この機械の保存場所に置くだけ。
 */
export async function prepareBackgroundImage(file: Blob): Promise<string> {
  const url = URL.createObjectURL(file);
  try {
    const img = await loadImage(url);
    const size = fitWithin(img.naturalWidth, img.naturalHeight);
    const [c, ctx] = canvas2d(size.width, size.height);
    ctx.drawImage(img, 0, 0, size.width, size.height);
    return c.toDataURL('image/jpeg', 0.85);
  } finally {
    URL.revokeObjectURL(url);
  }
}
