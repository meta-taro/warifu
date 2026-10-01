import { describe, expect, it } from 'vitest';

import { 当てる色, 読み取る } from './theme';

describe('テーマの選び', () => {
  it('知らない値は OS に合わせる', () => {
    expect(読み取る(null)).toBe('auto');
    expect(読み取る('')).toBe('auto');
    expect(読み取る('もくもく')).toBe('auto');
  });

  it('覚えた値はそのまま読む', () => {
    expect(読み取る('light')).toBe('light');
    expect(読み取る('dark')).toBe('dark');
  });
});

describe('当てる色', () => {
  it('OS に合わせるなら、OS に従う', () => {
    // **auto をライト決め打ちにしない。**暗い OS で白い画面が出る
    expect(当てる色('auto', true)).toBe('dark');
    expect(当てる色('auto', false)).toBe('light');
  });

  it('選んだなら、OS より選びが勝つ', () => {
    expect(当てる色('light', true)).toBe('light');
    expect(当てる色('dark', false)).toBe('dark');
  });
});
