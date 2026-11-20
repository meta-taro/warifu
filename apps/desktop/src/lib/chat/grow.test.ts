import { describe, expect, it } from 'vitest';
import { 上限で止めたか, 欄の最小, 欄の高さ } from './grow';

describe('書く欄の高さ', () => {
  it('空や 1 行は、最小の高さのまま', () => {
    expect(欄の高さ(0, 800)).toBe(欄の最小);
    expect(欄の高さ(20, 800)).toBe(欄の最小);
  });
  it('数行なら、中身の高さまで伸びる', () => {
    expect(欄の高さ(120, 800)).toBe(120);
    expect(上限で止めたか(120, 800)).toBe(false);
  });
  it('窓の 4 割を超えたら、そこで止めて欄の中で巻く', () => {
    expect(欄の高さ(1000, 800)).toBe(320);
    expect(上限で止めたか(1000, 800)).toBe(true);
  });
  it('窓がとても低くても、最小は守る', () => {
    expect(欄の高さ(200, 50)).toBe(欄の最小);
  });
});
