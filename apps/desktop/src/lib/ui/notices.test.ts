import { describe, expect, it } from 'vitest';
import { 出すか, 記録に足す, 閉じる, 閉じたから外す } from './notices';

describe('知らせの記録', () => {
  it('新しい知らせを後ろへ足す', () => {
    const 記録 = 記録に足す([], '経路が切れました', '15:50');
    expect(記録).toEqual([{ 文: '経路が切れました', 時刻: '15:50' }]);
  });

  it('空の知らせは足さない', () => {
    expect(記録に足す([], '', '15:50')).toEqual([]);
  });

  it('直前と同じ文は重ねない', () => {
    const 一 = 記録に足す([], 'A', '15:50');
    expect(記録に足す(一, 'A', '15:51')).toBe(一);
  });

  it('多すぎたら古いものから捨てる', () => {
    let 記録: ReturnType<typeof 記録に足す> = [];
    for (let i = 0; i < 60; i++) 記録 = 記録に足す(記録, `知らせ${i}`, '00:00');
    expect(記録).toHaveLength(50);
    expect(記録[0].文).toBe('知らせ10');
  });
});

describe('閉じる', () => {
  it('閉じた知らせは帯に出さない', () => {
    const 閉じた = 閉じる(new Set(), 'A');
    expect(出すか('A', 閉じた)).toBe(false);
    expect(出すか('B', 閉じた)).toBe(true);
  });

  it('同じ知らせがもう一度出たら、また帯に出す', () => {
    const 閉じた = 閉じたから外す(閉じる(new Set(), 'A'), 'A');
    expect(出すか('A', 閉じた)).toBe(true);
  });

  it('空の文は出さない', () => {
    expect(出すか('', new Set())).toBe(false);
  });
});
