import { describe, expect, it } from 'vitest';
import type { 会話行 } from '$lib/meeting/announce';
import { 画像をまとめる, まとめる間 } from './pic-run';

const 分 = 60 * 1000;
const 画像 = (who: string, 時: number, extra: Partial<会話行> = {}): 会話行 => ({
  who,
  body: '',
  画像: `blob:${who}-${時}`,
  mine: false,
  時,
  at: '10:00',
  ...extra,
});
const 文字 = (who: string, 時: number, body = 'こんにちは', extra: Partial<会話行> = {}): 会話行 => ({
  who,
  body,
  mine: false,
  時,
  at: '10:00',
  ...extra,
});

describe('画像をまとめる', () => {
  it('同じ人の画像の連投を 1 つの塊にする', () => {
    const r = 画像をまとめる([画像('A', 0), 画像('A', 分), 画像('A', 2 * 分)]);
    expect([...r.頭]).toEqual([[0, [1, 2]]]);
    expect([...r.続き]).toEqual([1, 2]);
  });

  it('文字の行の直後の画像も、その文字の塊に入れる', () => {
    const r = 画像をまとめる([文字('A', 0), 画像('A', 0), 画像('A', 0), 画像('A', 0)]);
    expect(r.頭.get(0)).toEqual([1, 2, 3]);
  });

  it('1 枚だけなら塊にしない（今の見た目のまま）', () => {
    const r = 画像をまとめる([文字('B', 0), 画像('A', 0), 文字('B', 0)]);
    expect(r.頭.size).toBe(0);
    expect(r.続き.size).toBe(0);
  });

  it('別の人の行が挟まったら切る', () => {
    const r = 画像をまとめる([画像('A', 0), 文字('B', 0), 画像('A', 0)]);
    expect(r.頭.size).toBe(0);
  });

  it('別の人の画像は、隣でも混ぜない', () => {
    const r = 画像をまとめる([画像('A', 0), 画像('B', 0), 画像('B', 0)]);
    expect([...r.頭]).toEqual([[1, [2]]]);
  });

  it('会議からの知らせが挟まったら切る', () => {
    const r = 画像をまとめる([画像('A', 0), { who: '', body: '入室', mine: false, system: true }, 画像('A', 0)]);
    expect(r.頭.size).toBe(0);
  });

  it('文字の行が来たら塊は終わる（文字のあとの画像は新しい塊）', () => {
    const r = 画像をまとめる([画像('A', 0), 画像('A', 0), 文字('A', 0), 画像('A', 0)]);
    expect([...r.頭]).toEqual([
      [0, [1]],
      [2, [3]],
    ]);
  });

  it('間が空いたら切る', () => {
    const r = 画像をまとめる([画像('A', 0), 画像('A', まとめる間 + 1)]);
    expect(r.頭.size).toBe(0);
  });

  it('前の行からの間で見る（塊の頭からではない）', () => {
    const r = 画像をまとめる([画像('A', 0), 画像('A', 2 * 分), 画像('A', 4 * 分)]);
    expect(r.頭.get(0)).toEqual([1, 2]);
  });

  it('時刻のミリ秒が無ければ、HH:MM が同じときだけまとめる', () => {
    const 無し = (at: string): 会話行 => ({ who: 'A', body: '', 画像: 'blob:x', mine: false, at });
    expect(画像をまとめる([無し('10:00'), 無し('10:00')]).頭.size).toBe(1);
    expect(画像をまとめる([無し('10:00'), 無し('10:01')]).頭.size).toBe(0);
  });

  it('返信の画像は続きにしない（元の発言を添えて 1 枚で出す）', () => {
    const r = 画像をまとめる([画像('A', 0), 画像('A', 0, { 返信先: 'x' })]);
    expect(r.頭.size).toBe(0);
  });

  it('自分の行と、同じ名の相手の行は混ぜない', () => {
    const r = 画像をまとめる([画像('A', 0, { mine: true }), 画像('A', 0)]);
    expect(r.頭.size).toBe(0);
  });

  it('残していない画像の行は頭にしない', () => {
    const 残していない: 会話行 = { who: 'A', body: '', mine: false, 時: 0, 画像は残していない: true };
    const r = 画像をまとめる([残していない, 画像('A', 0)]);
    expect(r.頭.size).toBe(0);
  });
});
