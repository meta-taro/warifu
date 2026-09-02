import { describe, expect, it } from 'vitest';
import type { LinkPath } from './path';
import { initialWatch, observe, PROMOTE_AFTER } from './watch';

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

  it('連続が途切れたら数え直す', () => {
    expect(run(['direct', 'direct', 'unknown', 'direct', 'direct'])).toBe('unknown');
  });

  it('**中継への変化は 1 回で反映する**', () => {
    // 「直接」と言い続けるほうが害が大きい。通信の性質が変わったことを隠さない
    const seen: LinkPath[] = [...Array<LinkPath>(PROMOTE_AFTER).fill('direct'), 'relayed'];
    expect(run(seen)).toBe('relayed');
  });

  it('**不明への変化も 1 回で反映する**', () => {
    const seen: LinkPath[] = [...Array<LinkPath>(PROMOTE_AFTER).fill('direct'), 'unknown'];
    expect(run(seen)).toBe('unknown');
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
