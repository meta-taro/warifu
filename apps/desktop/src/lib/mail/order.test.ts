import { describe, expect, it } from 'vitest';

import { 札の並び, 畳む札か, 並べる, type メールの行 } from './order';

function 行(uid: number, 札: string, 届いた: number | null = uid): メールの行 {
  return { user: 'a@example.com', uid, 差出人: `s${uid}@example.com`, 名: null, 件名: `件 ${uid}`, 届いた, 既読: false, 札, 理由: [] };
}

describe('メールの並び', () => {
  it('人が見るべき札が先：なりすまし疑い → 受注 → 問い合わせ → 未分類 → 請求 → 知らせ → 広告 → 迷惑', () => {
    expect(札の並び).toEqual(['なりすまし疑い', '受注', '問い合わせ', '未分類', '請求', '知らせ', '広告', '迷惑']);
  });

  it('札ごとにまとめ、札の中は新しい順。無い札の組は作らない', () => {
    const 組 = 並べる([行(1, '知らせ'), 行(2, '受注'), 行(3, '受注'), 行(4, 'なりすまし疑い')]);
    expect(組.map((g) => g.札)).toEqual(['なりすまし疑い', '受注', '知らせ']);
    expect(組[1].行たち.map((r) => r.uid)).toEqual([3, 2]);
  });

  it('知らない札は未分類の組へ入れる（黙って落とさない）', () => {
    const 組 = 並べる([行(1, '新しい札')]);
    expect(組.map((g) => g.札)).toEqual(['未分類']);
    expect(組[0].行たち).toHaveLength(1);
  });

  it('届いた時刻が無い通は、組の最後', () => {
    const 組 = 並べる([行(1, '請求', null), 行(2, '請求', 5)]);
    expect(組[0].行たち.map((r) => r.uid)).toEqual([2, 1]);
  });

  it('知らせ・広告・迷惑は畳んで出す', () => {
    expect(['知らせ', '広告', '迷惑'].every(畳む札か)).toBe(true);
    expect(['なりすまし疑い', '受注', '問い合わせ', '未分類', '請求'].some(畳む札か)).toBe(false);
  });
});
