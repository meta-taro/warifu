import { describe, expect, it } from 'vitest';
import { そのまま送れるか, 画像の上限, 画像を拾う, 縮めた寸法 } from './image';

describe('縮めた寸法', () => {
  it('長い辺を 1600 に合わせ、比を保つ', () => {
    expect(縮めた寸法(4000, 3000)).toEqual({ 幅: 1600, 高さ: 1200 });
    expect(縮めた寸法(1000, 5000)).toEqual({ 幅: 320, 高さ: 1600 });
  });

  it('小さいものは大きくしない', () => {
    expect(縮めた寸法(800, 600)).toEqual({ 幅: 800, 高さ: 600 });
  });

  it('細長くても 0 にはしない', () => {
    expect(縮めた寸法(10000, 1).高さ).toBe(1);
  });
});

describe('そのまま送れるか', () => {
  it('小さい PNG はそのまま（書き直すと図が滲む）', () => {
    expect(そのまま送れるか('image/png', 100_000, 800, 600)).toBe(true);
  });

  it('大きすぎる・広すぎるものは書き直す', () => {
    expect(そのまま送れるか('image/jpeg', 画像の上限 + 1, 800, 600)).toBe(false);
    expect(そのまま送れるか('image/jpeg', 1000, 4000, 3000)).toBe(false);
  });

  it('SVG や HEIC はそのまま送らない', () => {
    expect(そのまま送れるか('image/svg+xml', 1000, 100, 100)).toBe(false);
    expect(そのまま送れるか('image/heic', 1000, 100, 100)).toBe(false);
  });
});

describe('画像を拾う', () => {
  it('貼り付けの中から画像だけを拾う', () => {
    const 絵 = new File([new Uint8Array([1])], 'a.png', { type: 'image/png' });
    const 物 = [
      { kind: 'string', type: 'text/plain', getAsFile: () => null },
      { kind: 'file', type: 'image/png', getAsFile: () => 絵 },
    ];
    expect(画像を拾う(物)).toBe(絵);
  });

  it('画像が無ければ null（文字の貼り付けを邪魔しない）', () => {
    expect(画像を拾う([{ kind: 'string', type: 'text/plain', getAsFile: () => null }])).toBeNull();
    expect(画像を拾う(null)).toBeNull();
  });
});
