import { describe, expect, it } from 'vitest';
import { 呼び名, 鍵の頭 } from './names';

const 鍵 = '67R54JO7ND6PXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXA';

describe('呼び名', () => {
  it('覚えている相手は、その呼び名で出す', () => {
    // **誰が言ったのか分からないチャットは使えない**（2026-09-06 に実際に困った）
    expect(呼び名({ [鍵]: 'Mac Air のエージェント' }, 鍵)).toBe('Mac Air のエージェント');
  });

  it('覚えていない相手は、鍵の頭で出す', () => {
    // **知らない相手を、知っているように見せない**
    expect(呼び名({}, 鍵)).toBe('67R54JO7ND6P…');
  });

  it('空の呼び名を名前として使わない', () => {
    expect(呼び名({ [鍵]: '   ' }, 鍵)).toBe('67R54JO7ND6P…');
  });

  it('短い鍵はそのまま出す', () => {
    expect(鍵の頭('ABC')).toBe('ABC');
  });
});
