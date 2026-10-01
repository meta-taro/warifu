import { describe, expect, it } from 'vitest';
import type { LinkPath } from './path';
import { initialWatch, observe, PROMOTE_AFTER, 読めないを許す回数 } from './watch';

/** 観測を順に流して、最後に画面へ出る状態を返す。 */
const run = (seen: readonly LinkPath[]) =>
  seen.reduce((state, s) => observe(state, s), initialWatch()).shown;

describe('経路の表示を振動させない（DESIGN.md §4.1 / D29 と同じ姿勢）', () => {
  it('始まりは「不明」。測る前に何かを名乗らない', () => {
    expect(initialWatch().shown).toBe('unknown');
  });

  it('「直接」は連続して見えるまで名乗らない', () => {
    // ICE の途中で一瞬 host 同士になることがある。そこで「直接」と出すと、
    // 次の瞬間に中継へ変わって表示が跳ねる
    expect(run(['direct'])).toBe('unknown');
    expect(run(['direct', 'direct'])).toBe('unknown');
    expect(run(Array<LinkPath>(PROMOTE_AFTER).fill('direct'))).toBe('direct');
  });

  it('連続が途切れたら数え直す（まだ直接を名乗る前なら、読めない 1 回で戻る）', () => {
    // **名乗る前**なので、取り下げる物がない。数え直しになる
    expect(run(['direct', 'direct', 'unknown', 'direct', 'direct'])).toBe('unknown');
  });

  it('**中継への変化は 1 回で反映する**', () => {
    // 「直接」と言い続けるほうが害が大きい。通信の性質が変わったことを隠さない
    const seen: LinkPath[] = [...Array<LinkPath>(PROMOTE_AFTER).fill('direct'), 'relayed'];
    expect(run(seen)).toBe('relayed');
  });

  // 2026-09-13、3 台で実測して分かった ——
  // **`direct` ⇄ `unknown` が 6 秒ごとに行き来していた。**
  //
  // `unknown` は「**通信の性質が変わった**」ではなく「**今回は読めなかった**」である
  // （`pathFromStats` は、成立している組が拾えないだけで `unknown` を返す）。
  // 1 回で取り下げると、戻すのに PROMOTE_AFTER 回かかるので**揺れ続ける。**
  it('**1 回読めなかっただけでは、直接を取り下げない**', () => {
    const seen: LinkPath[] = [...Array<LinkPath>(PROMOTE_AFTER).fill('direct'), 'unknown'];
    expect(run(seen)).toBe('direct');
  });

  it('**続けて読めなければ、不明にする**（分からないものを分かるように見せない）', () => {
    const seen: LinkPath[] = [
      ...Array<LinkPath>(PROMOTE_AFTER).fill('direct'),
      ...Array<LinkPath>(読めないを許す回数).fill('unknown'),
    ];
    expect(run(seen)).toBe('unknown');
  });

  it('途中で読めたら、数え直す', () => {
    const seen: LinkPath[] = [
      ...Array<LinkPath>(PROMOTE_AFTER).fill('direct'),
      'unknown',
      'unknown',
      'direct',
      'unknown',
      'unknown'
    ];
    // **読めない回数は続けてでないと効かない**
    expect(run(seen)).toBe('direct');
  });

  it('**中継は待たない**（読めなかったのではなく、性質が変わったから）', () => {
    const seen: LinkPath[] = [...Array<LinkPath>(PROMOTE_AFTER).fill('direct'), 'relayed'];
    expect(run(seen)).toBe('relayed');
  });

  it('中継は連続を求めない（名乗るのに待たせるのは「直接」だけ）', () => {
    expect(run(['relayed'])).toBe('relayed');
  });

  it('一度も直接が続かなければ、直接とは名乗らない', () => {
    // 最後の direct は 1 回目なので、まだ名乗らない（表示は直前の relayed のまま）
    expect(run(['relayed', 'direct', 'unknown', 'direct', 'relayed', 'direct'])).toBe('relayed');
  });

  it('直接を名乗った後、続けて直接を見ても変わらない', () => {
    const seen: LinkPath[] = Array<LinkPath>(PROMOTE_AFTER + 5).fill('direct');
    expect(run(seen)).toBe('direct');
  });
});
