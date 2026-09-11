import { describe, expect, it } from 'vitest';

import { 未読の数, 見た印を進める, 見た印を置く, 見たことにする } from './unread';
import type { 会話行 } from '$lib/meeting/announce';

const 相手の行 = (body: string): 会話行 => ({ who: 'B役', body, mine: false });
const 自分の行 = (body: string): 会話行 => ({ who: '自分', body, mine: true });
const 知らせ = (body: string): 会話行 => ({ who: '', body, mine: false, system: true });

describe('未読の数（案 A・相手ごと）', () => {
  it('見た所から後ろの、相手の行だけを数える', () => {
    const 会話 = [相手の行('1'), 相手の行('2'), 相手の行('3')];
    expect(未読の数(会話, 0)).toBe(3);
    expect(未読の数(会話, 2)).toBe(1);
    expect(未読の数(会話, 3)).toBe(0);
  });

  it('自分の行は数えない', () => {
    // **自分が打ったものを未読と言わない**
    expect(未読の数([自分の行('あ'), 自分の行('い')], 0)).toBe(0);
    expect(未読の数([相手の行('あ'), 自分の行('い')], 0)).toBe(1);
  });

  it('知らせ（入退室など）は数えない', () => {
    // 「◯◯ が入りました」で赤い数字が付くと、**話しかけられたと勘違いする**
    expect(未読の数([知らせ('B役 が入りました')], 0)).toBe(0);
  });

  it('見た印が会話より大きくても、負にしない', () => {
    // 会話は閉じると消える（残していない）。**印だけが残ると数が壊れる**
    expect(未読の数([相手の行('あ')], 5)).toBe(0);
  });

  it('見たことにすると 0 になる', () => {
    const 会話 = [相手の行('1'), 相手の行('2')];
    const 印 = 見たことにする(会話);
    expect(未読の数(会話, 印)).toBe(0);
  });
});

describe('見た印を進める', () => {
  it('いま見ている会話だけを進める', () => {
    // **見ていない会話を、勝手に見たことにしない**
    const 印 = { 'desk:zumen': 1 };
    const 次 = 見た印を進める(印, 'room:ABC', [相手の行('1'), 相手の行('2')]);
    expect(次).toEqual({ 'desk:zumen': 1, 'room:ABC': 2 });
  });

  it('何も選んでいなければ、何も進めない', () => {
    const 印 = { 'desk:zumen': 1 };
    expect(見た印を進める(印, null, [相手の行('1')])).toEqual(印);
  });

  it('元の印を壊さない（新しい物を返す）', () => {
    const 印 = { a: 1 };
    const 次 = 見た印を進める(印, 'b', [相手の行('1')]);
    expect(印).toEqual({ a: 1 });
    expect(次).not.toBe(印);
  });
});

describe('行数を直に渡して印を置く', () => {
  it('その会話だけを置き換える', () => {
    expect(見た印を置く({ a: 1 }, 'b', 3)).toEqual({ a: 1, b: 3 });
  });

  it('負の数は 0 にする', () => {
    expect(見た印を置く({}, 'a', -5)).toEqual({ a: 0 });
  });

  it('元の印を壊さない', () => {
    const 印 = { a: 1 };
    見た印を置く(印, 'a', 9);
    expect(印).toEqual({ a: 1 });
  });
});
