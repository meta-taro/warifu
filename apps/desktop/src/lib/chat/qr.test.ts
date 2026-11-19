import { describe, expect, it } from 'vitest';
import { encode } from 'uqr';

import { QRにする, 余白, 道にする } from './qr';

describe('QRにする（0.1.53）', () => {
  it('余白 4 升を足し、塗る升は余白の内側にだけある', () => {
    const 升目 = QRにする('https://example.com/a?b=1&c=2');
    const 中 = encode('https://example.com/a?b=1&c=2', { ecc: 'M', border: 0 });
    expect(升目.大きさ).toBe(中.size + 余白 * 2);
    for (const [x, y] of 升目.黒) {
      expect(x).toBeGreaterThanOrEqual(余白);
      expect(y).toBeGreaterThanOrEqual(余白);
      expect(x).toBeLessThan(升目.大きさ - 余白);
      expect(y).toBeLessThan(升目.大きさ - 余白);
    }
    const 数 = 中.data.flat().filter(Boolean).length;
    expect(升目.黒).toHaveLength(数);
  });

  it('左上の位置合わせの角は塗られている（余白のすぐ内側）', () => {
    const 升目 = QRにする('https://example.com');
    expect(升目.黒).toContainEqual([余白, 余白]);
  });

  it('道は升ごとに 1 つの四角', () => {
    const 道 = 道にする({ 大きさ: 10, 黒: [[4, 4], [5, 4]] });
    expect(道).toBe('M4 4h1v1h-1zM5 4h1v1h-1z');
  });
});
