import { describe, expect, it } from 'vitest';
import { 溜める, 取り出す, 忘れる, 溜める上限 } from './pending';

import type { SignalPayload } from '$lib/bridge';

// `blob` に番号を入れて、**着いた順が変わっていないか**を見る
const 玉 = (step: SignalPayload['step'], n = 0): SignalPayload => ({
  from: 'AAA',
  step,
  blob: String(n),
});

describe('通話ができる前に来た下ごしらえ', () => {
  // gh issue 9 の後半（ASUS・2026-09-12）——
  //
  // > offer / candidate が「通話を作る」より先に着いて捨てられています。
  // > 経路が落ちていなくても、ここで取りこぼす見込みがあります。
  //
  // **握手の最初の玉を捨てると、経路は永久に unknown のまま**になり、映像も乗らない。

  it('**着いた順のまま取り出せる**（順番が変わると握手が壊れる）', () => {
    const 箱 = new Map();
    溜める(箱, 'AAA', 玉('offer', 1));
    溜める(箱, 'AAA', 玉('candidate', 2));
    溜める(箱, 'AAA', 玉('candidate', 3));
    expect(取り出す(箱, 'AAA').map((x) => x.blob)).toEqual(['1', '2', '3']);
  });

  it('取り出したら空になる（二度食わせない）', () => {
    const 箱 = new Map();
    溜める(箱, 'AAA', 玉('offer'));
    取り出す(箱, 'AAA');
    expect(取り出す(箱, 'AAA')).toEqual([]);
  });

  it('相手ごとに分ける（別の組の玉を混ぜない）', () => {
    const 箱 = new Map();
    溜める(箱, 'AAA', 玉('offer', 1));
    溜める(箱, 'BBB', 玉('offer', 2));
    expect(取り出す(箱, 'AAA')).toHaveLength(1);
    expect(取り出す(箱, 'BBB')).toHaveLength(1);
  });

  it('**際限なく溜めない。**上限を超えたら古いほうから捨てる', () => {
    const 箱 = new Map();
    for (let i = 0; i < 溜める上限 + 5; i += 1) 溜める(箱, 'AAA', 玉('candidate', i));
    const 出た = 取り出す(箱, 'AAA');
    expect(出た).toHaveLength(溜める上限);
    // **捨てたのは古いほう**（新しい候補のほうが通る見込みが高い）
    expect(出た[0].blob).toBe('5');
  });

  it('相手が抜けたら忘れる（次に来た人へ渡さない）', () => {
    const 箱 = new Map();
    溜める(箱, 'AAA', 玉('offer'));
    忘れる(箱, 'AAA');
    expect(取り出す(箱, 'AAA')).toEqual([]);
  });

  it('溜まっていない相手からは、空が返る', () => {
    expect(取り出す(new Map(), 'ZZZ')).toEqual([]);
  });
});
