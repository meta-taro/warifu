import { describe, expect, it } from 'vitest';
import { 新しいか, 版を読む, 版をそろえる, 進み具合 } from './notice';

describe('版を比べる', () => {
  it('頭の v を落とす', () => {
    expect(版をそろえる('v0.1.0')).toBe('0.1.0');
    expect(版をそろえる('  0.1.0 ')).toBe('0.1.0');
  });

  it('足りない所は 0 で埋める', () => {
    expect(版を読む('1')).toEqual({ 大: 1, 中: 0, 小: 0 });
    expect(版を読む('0.2')).toEqual({ 大: 0, 中: 2, 小: 0 });
  });

  it('alpha などの後ろは落とす', () => {
    expect(版を読む('0.1.0-alpha.15')).toEqual({ 大: 0, 中: 1, 小: 0 });
  });

  it('数字として読めないものは null', () => {
    // **既定に落とさない。**落とすと壊れた札が「新しい版」に化ける
    expect(版を読む('')).toBeNull();
    expect(版を読む('latest')).toBeNull();
    expect(版を読む('0.x.1')).toBeNull();
  });

  it('新しいときだけ true', () => {
    expect(新しいか('0.1.0', '0.1.1')).toBe(true);
    expect(新しいか('0.1.0', '0.2.0')).toBe(true);
    expect(新しいか('0.1.0', '1.0.0')).toBe(true);
  });

  it('同じ版では出さない', () => {
    // **「更新があります」と出して、押しても何も起きない**を作らない
    expect(新しいか('0.1.0', '0.1.0')).toBe(false);
    expect(新しいか('0.1.0', 'v0.1.0')).toBe(false);
  });

  it('古い版では出さない', () => {
    expect(新しいか('0.2.0', '0.1.9')).toBe(false);
  });

  it('読めない版では出さない', () => {
    expect(新しいか('0.1.0', 'latest')).toBe(false);
    expect(新しいか('こわれた', '0.1.1')).toBe(false);
  });

  it('alpha どうしは数で比べる（後ろは見ない）', () => {
    // **タグの後ろ（alpha.15）では区別しない。**版は 0.1.0 のままなので
    // 「更新なし」になる —— それが正しい（配布物の版が上がっていない）
    expect(新しいか('0.1.0-alpha.15', '0.1.0-alpha.16')).toBe(false);
    expect(新しいか('0.1.0-alpha.15', '0.1.1-alpha.1')).toBe(true);
  });
});

describe('進み具合', () => {
  it('総量が分かれば割合にする', () => {
    expect(進み具合(50, 200)).toBe(25);
  });

  it('総量が分からなければ null', () => {
    // **分からないものを分かるように見せない**（DESIGN §2 原則 7）
    expect(進み具合(50, null)).toBeNull();
    expect(進み具合(50, 0)).toBeNull();
  });

  it('外へはみ出さない', () => {
    expect(進み具合(300, 200)).toBe(100);
    expect(進み具合(-5, 200)).toBe(0);
  });
});
