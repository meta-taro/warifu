import { describe, expect, it } from 'vitest';
import { controlsFor, TITLEBAR_HEIGHT } from './titlebar';

describe('自作タイトルバー（DESIGN.md §8 / D34）', () => {
  it('操作は最小化・最大化・閉じるの 3 つだけ。この順で並ぶ', () => {
    expect(controlsFor(false).map((c) => c.id)).toEqual(['minimize', 'maximize', 'close']);
  });

  it('最大化していないときは ▢、しているときは ❐（復元）', () => {
    const notMax = controlsFor(false).find((c) => c.id === 'maximize');
    const max = controlsFor(true).find((c) => c.id === 'maximize');
    expect(notMax?.glyph).toBe('▢');
    expect(max?.glyph).toBe('❐');
  });

  it('読み上げ用の鍵も状態で変わる', () => {
    expect(controlsFor(false).find((c) => c.id === 'maximize')?.labelKey).toBe('window.maximize');
    expect(controlsFor(true).find((c) => c.id === 'maximize')?.labelKey).toBe('window.restore');
  });

  it('最小化と閉じるは状態で変わらない', () => {
    for (const maximized of [false, true]) {
      const cs = controlsFor(maximized);
      expect(cs.find((c) => c.id === 'minimize')?.glyph).toBe('─');
      expect(cs.find((c) => c.id === 'close')?.glyph).toBe('✕');
    }
  });

  it('閉じるだけが危険な操作として印を持つ', () => {
    const cs = controlsFor(false);
    expect(cs.filter((c) => c.danger).map((c) => c.id)).toEqual(['close']);
  });

  it('帯の高さは DESIGN.md §8 の 44px', () => {
    expect(TITLEBAR_HEIGHT).toBe(44);
  });
});
