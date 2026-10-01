import { describe, expect, it } from 'vitest';
import {
  BandWatcher,
  FpsMeter,
  blurSize,
  coverCrop,
  effectiveMode,
  fitWithin,
  maskAlpha,
  nextTimestamp,
  outputSize,
  readBackgroundMode,
  readBlurStrength,
  speedBand,
  videoPlan,
} from './background';

describe('背景の保存値を読む', () => {
  it('明示の off と image は残す', () => {
    expect(readBackgroundMode('off')).toBe('off');
    expect(readBackgroundMode('image')).toBe('image');
  });

  it('**それ以外は隠す側（blur）**。前の版の none・欠け・壊れも', () => {
    for (const v of ['none', 'blur', undefined, null, 3, 'OFF']) {
      expect(readBackgroundMode(v), String(v)).toBe('blur');
    }
  });

  it('度合いは 3 段。知らない値は中くらい', () => {
    expect(readBlurStrength('weak')).toBe('weak');
    expect(readBlurStrength('strong')).toBe('strong');
    expect(readBlurStrength('medium')).toBe('medium');
    expect(readBlurStrength('max')).toBe('medium');
  });
});

describe('いま掛ける隠し方', () => {
  it('画像を選んでいても、画像が無ければぼかす（**見せる側へ倒さない**）', () => {
    expect(effectiveMode('image', false)).toBe('blur');
    expect(effectiveMode('image', true)).toBe('image');
  });

  it('off は off、blur は blur', () => {
    expect(effectiveMode('off', false)).toBe('off');
    expect(effectiveMode('blur', true)).toBe('blur');
  });
});

describe('模型が読めなかったら、素のカメラを送らない', () => {
  it('隠すと決めていて模型が読めなければ、**送らない**', () => {
    expect(videoPlan('blur', 'failed')).toBe('withhold');
    expect(videoPlan('image', 'failed')).toBe('withhold');
  });

  it('読めていれば隠して送る', () => {
    expect(videoPlan('blur', 'ready')).toBe('hidden');
    expect(videoPlan('image', 'ready')).toBe('hidden');
  });

  it('**素のまま送るのは、人が off を選んだときだけ**', () => {
    expect(videoPlan('off', 'failed')).toBe('raw');
    expect(videoPlan('off', 'ready')).toBe('raw');
  });
});

describe('大きさ', () => {
  it('送る大きさは長い辺 640 に収める。縦横比は保つ', () => {
    expect(outputSize(1280, 720)).toEqual({ width: 640, height: 360 });
    expect(outputSize(640, 480)).toEqual({ width: 640, height: 480 });
    expect(outputSize(320, 240)).toEqual({ width: 320, height: 240 });
  });

  it('まだ大きさが分からなければ 640×480', () => {
    expect(outputSize(0, 0)).toEqual({ width: 640, height: 480 });
  });

  it('強いほど小さく縮める（強くぼける）', () => {
    const w = blurSize(640, 480, 'weak').width;
    const m = blurSize(640, 480, 'medium').width;
    const s = blurSize(640, 480, 'strong').width;
    expect(w).toBeGreaterThan(m);
    expect(m).toBeGreaterThan(s);
    expect(s).toBeGreaterThanOrEqual(4);
  });

  it('画像は縦横比を崩さず、はみ出しを切って敷く', () => {
    // 横長の画像を 4:3 の枠へ → 左右を切る
    const c = coverCrop(1600, 900, 640, 480);
    expect(c.sh).toBe(900);
    expect(c.sw).toBeCloseTo(1200);
    expect(c.sx).toBeCloseTo(200);
    // 縦長の画像 → 上下を切る
    const d = coverCrop(900, 1600, 640, 480);
    expect(d.sw).toBe(900);
    expect(d.sh).toBeCloseTo(675);
    expect(d.sy).toBeCloseTo(462.5);
  });

  it('保存する画像は 1280×720 に収める（大きい画像で保存場所を食わない）', () => {
    expect(fitWithin(4000, 3000)).toEqual({ width: 960, height: 720 });
    expect(fitWithin(800, 600)).toEqual({ width: 800, height: 600 });
  });
});

describe('縁の濃さ', () => {
  it('背景らしい所は消し、人らしい所は残す', () => {
    expect(maskAlpha(0)).toBe(0);
    expect(maskAlpha(0.2)).toBe(0);
    expect(maskAlpha(0.8)).toBe(255);
    expect(maskAlpha(1)).toBe(255);
  });

  it('あいだは滑らかに増える', () => {
    const a = maskAlpha(0.4);
    const b = maskAlpha(0.5);
    const c = maskAlpha(0.6);
    expect(a).toBeLessThan(b);
    expect(b).toBeLessThan(c);
  });
});

describe('模型へ渡す時刻', () => {
  it('必ず前より進める（同じ時刻を 2 度渡さない）', () => {
    expect(nextTimestamp(10, 5)).toBe(10);
    expect(nextTimestamp(5, 5)).toBeGreaterThan(5);
    expect(nextTimestamp(4, 5)).toBeGreaterThan(5);
  });
});

describe('コマ数を数える', () => {
  it('窓が閉じるまでは値を出さない', () => {
    const m = new FpsMeter(1000);
    expect(m.tick(0)).toBeNull();
    expect(m.tick(500)).toBeNull();
    expect(m.fps).toBeNull();
  });

  it('30 fps で出していれば 30 前後を返す', () => {
    const m = new FpsMeter(1000);
    let out: number | null = null;
    for (let i = 0; i <= 30; i++) out = m.tick((i * 1000) / 30) ?? out;
    expect(out).not.toBeNull();
    expect(out!).toBeGreaterThan(29);
    expect(out!).toBeLessThan(32);
    expect(m.fps).toBe(out);
  });

  it('**窓ごとに 1 度だけ**値を出す（毎コマは出さない）', () => {
    const m = new FpsMeter(1000);
    let 出た = 0;
    for (let i = 0; i < 300; i++) if (m.tick((i * 1000) / 30) !== null) 出た++;
    // 300 コマ ≒ 10 秒 → 9〜10 回
    expect(出た).toBeGreaterThanOrEqual(9);
    expect(出た).toBeLessThanOrEqual(10);
  });
});

describe('速さの段（記録は段が変わったときだけ）', () => {
  it('15 fps を境に分ける', () => {
    expect(speedBand(15)).toBe('ok');
    expect(speedBand(14.9)).toBe('slow');
  });

  it('初めて測れたときに 1 度、そのあとは変わったときだけ知らせる', () => {
    const w = new BandWatcher();
    expect(w.observe(28)).toBe('ok');
    expect(w.observe(25)).toBeNull();
    expect(w.observe(30)).toBeNull();
    expect(w.observe(10)).toBe('slow');
    expect(w.observe(12)).toBeNull();
    expect(w.observe(20)).toBe('ok');
  });
});
