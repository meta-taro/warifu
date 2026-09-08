import { describe, expect, it } from 'vitest';

import { マス, 顔を描く } from './avatar';

const 鍵A = 'I3ILQUUJQHVSP6NQ2IDL3BPRVKSOXYK7WCEIUADWZVPZWMV3TMBQ';
const 鍵B = '4VKCVUBF4RNLZ7DG2XFNZVIWHQNXVYNGWTYTAOALSPKNGWTZA3WA';

describe('顔を描く', () => {
  it('同じ鍵なら、いつでも同じ顔', () => {
    // **相手の画面でも同じ絵になる**（送らずに済むのはここが成り立つため）
    expect(顔を描く(鍵A)).toEqual(顔を描く(鍵A));
  });

  it('別の鍵なら、別の顔', () => {
    const a = 顔を描く(鍵A);
    const b = 顔を描く(鍵B);
    expect([a.色, JSON.stringify(a.ます)]).not.toEqual([b.色, JSON.stringify(b.ます)]);
  });

  it('左右対称にする', () => {
    // 非対称だと、小さいときに「模様」ではなく「汚れ」に見える
    const { ます } = 顔を描く(鍵A);
    for (const 行 of ます) {
      expect(行.length).toBe(マス);
      for (let x = 0; x < マス; x += 1) expect(行[x]).toBe(行[マス - 1 - x]);
    }
  });

  it('ます目は 5 × 5', () => {
    const { ます } = 顔を描く(鍵B);
    expect(ます.length).toBe(マス);
  });

  it('空の鍵でも落ちない', () => {
    // **知らない相手・まだ鍵を読めていない行**でも顔は要る
    expect(() => 顔を描く('')).not.toThrow();
  });

  it('頭が同じでも、顔が分かれる', () => {
    // 鍵の頭だけを見ていると、似た鍵で同じ顔になる
    const a = 顔を描く('AAAAAAAAAAAA1');
    const b = 顔を描く('AAAAAAAAAAAA2');
    expect(JSON.stringify(a.ます)).not.toBe(JSON.stringify(b.ます));
  });
});
