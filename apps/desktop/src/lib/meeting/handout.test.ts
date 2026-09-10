import { describe, expect, it } from 'vitest';

import { その人の鍵, 渡してあるか, 足す, type 出した鍵 } from './handout';

const 本を作る = (鍵: string, 宛先?: { key: string; 名: string }): 出した鍵 => ({
  鍵,
  リンク: `warifu://join/${鍵}`,
  qr: '<svg />',
  宛先,
});

describe('出した鍵と、その宛先（D84）', () => {
  it('足しても、前の鍵を捨てない', () => {
    // **1 本ずつ上書きしていたのが不具合だった** ——
    // 前の鍵は生きているのに画面から消えていた
    const 鍵たち = 足す(足す([], 本を作る('K1')), 本を作る('K2'));
    expect(鍵たち.map((本) => 本.鍵)).toEqual(['K1', 'K2']);
  });

  it('宛先で引ける', () => {
    const 鍵たち = [本を作る('K1', { key: 'AIR', 名: 'mac air' }), 本を作る('K2')];
    expect(その人の鍵(鍵たち, 'AIR').map((本) => 本.鍵)).toEqual(['K1']);
    expect(渡してあるか(鍵たち, 'AIR')).toBe(true);
  });

  it('一括で出した分は、誰の宛先でもない', () => {
    // **宛先の無い鍵を、誰かに渡した扱いにしない**
    const 鍵たち = [本を作る('K2')];
    expect(その人の鍵(鍵たち, 'AIR')).toEqual([]);
    expect(渡してあるか(鍵たち, 'AIR')).toBe(false);
  });

  it('同じ相手に 2 本目を出しても、1 本目を消さない', () => {
    // 渡し損じることがある。**前の 1 本もまだ生きている**
    const 宛先 = { key: 'AIR', 名: 'mac air' };
    const 鍵たち = 足す([本を作る('K1', 宛先)], 本を作る('K2', 宛先));
    expect(その人の鍵(鍵たち, 'AIR').map((本) => 本.鍵)).toEqual(['K1', 'K2']);
  });
});
