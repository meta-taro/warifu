import { describe, expect, it } from 'vitest';

import { 連絡帳を組む, 机の印, type 素材 } from './list';

const 自分 = 'AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA';
const 相手 = 'BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB';
const もう一人 = 'CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC';

const 素: 素材 = { 自分, 机の人数: 0, 会議の相手: [], 覚えた: [] };

describe('連絡帳の並び', () => {
  it('いちばん上は「この PC」（自分と、この PC の AI）', () => {
    // `issues/012`「この PC で会議するとき、私とあなたはセットでしょっていう」
    const [先頭] = 連絡帳を組む(素);
    expect(先頭.title).toBe('contacts.this');
    expect(先頭.行たち.map((r) => r.種類)).toEqual(['自分', 'AI']);
  });

  it('机に誰も着いていなくても、この PC の AI の行は消さない', () => {
    // **消すと「そういう仕組みが無い」と読まれる。**居ないなら居ないと出す
    const [先頭] = 連絡帳を組む({ ...素, 机の人数: 0 });
    const ai = 先頭.行たち.find((r) => r.種類 === 'AI');
    expect(ai?.key).toBe(机の印);
    expect(ai?.いま会議に居る).toBe(false);
  });

  it('机に着いていれば、この PC の AI は繋がっている扱いになる', () => {
    const [先頭] = 連絡帳を組む({ ...素, 机の人数: 2 });
    expect(先頭.行たち.find((r) => r.種類 === 'AI')?.いま会議に居る).toBe(true);
  });

  it('会議に誰も居なければ、その区画そのものを出さない', () => {
    // **見出しだけが並ぶ画面にしない**
    const 区画 = 連絡帳を組む(素);
    expect(区画.map((s) => s.title)).toEqual(['contacts.this', 'contacts.saved']);
  });

  it('いま会議に居る人を、覚えている相手の中に二度出さない', () => {
    const 区画 = 連絡帳を組む({
      ...素,
      会議の相手: [相手],
      覚えた: [{ key: 相手, label: 'air', has_address: true }],
    });
    const 覚えた = 区画.find((s) => s.title === 'contacts.saved');
    expect(覚えた?.行たち).toHaveLength(0);
    expect(区画.find((s) => s.title === 'contacts.inmeeting')?.行たち).toHaveLength(1);
  });

  it('覚えていない相手は鍵の頭で出す（知っているように見せない）', () => {
    const 区画 = 連絡帳を組む({ ...素, 会議の相手: [相手] });
    const 行 = 区画.find((s) => s.title === 'contacts.inmeeting')?.行たち[0];
    expect(行?.name).toBe('BBBBBBBBBBBB…');
  });

  it('覚えている相手は呼び名の順に並ぶ', () => {
    // **読み込むたびに並びが変わらない**（毎回探させない）
    const 区画 = 連絡帳を組む({
      ...素,
      覚えた: [
        { key: もう一人, label: 'zzz', has_address: false },
        { key: 相手, label: 'aaa', has_address: true },
      ],
    });
    const 覚えた = 区画.find((s) => s.title === 'contacts.saved');
    expect(覚えた?.行たち.map((r) => r.name)).toEqual(['aaa', 'zzz']);
  });

  it('住所を覚えているかを、行が持つ', () => {
    // **これで「呼ぶ」が押せるかが決まる**
    const 区画 = 連絡帳を組む({
      ...素,
      覚えた: [{ key: 相手, label: 'air', has_address: true }],
    });
    expect(区画.find((s) => s.title === 'contacts.saved')?.行たち[0].住所を覚えている).toBe(true);
  });

  it('自分は「この PC」にだけ出て、覚えている相手には出ない', () => {
    const 区画 = 連絡帳を組む({ ...素, 覚えた: [{ key: 相手, label: 'air', has_address: false }] });
    const 覚えた = 区画.find((s) => s.title === 'contacts.saved');
    expect(覚えた?.行たち.some((r) => r.key === 自分)).toBe(false);
  });
});
