import { describe, expect, it } from 'vitest';
import { グラフの囲みか, 図を読む, 数にする, 目盛り, 円の切れ, 名を書く番, 線は動きに合わせるか } from './chart';
import { 塊に分ける } from './markdown';

const 読めた = (情報: string, 中: string) => {
  const r = 図を読む(情報, 中);
  if (!('図' in r)) throw new Error(`読めなかった: ${r.訳}`);
  return r.図;
};

describe('グラフの囲み', () => {
  it('chart で始まる囲みだけをグラフにする', () => {
    expect(グラフの囲みか('chart')).toBe(true);
    expect(グラフの囲みか('chart line')).toBe(true);
    expect(グラフの囲みか('ts')).toBe(false);
    expect(グラフの囲みか('charts')).toBe(false);
    expect(グラフの囲みか('')).toBe(false);
  });

  it('発言の中の ```chart は図の塊になり、ほかの囲みは符のまま', () => {
    const 塊 = 塊に分ける('前の文\n```chart\n| SNS | 人 |\n|---|---|\n| X | 6,623 |\n```\n```\nそのまま\n```');
    expect(塊.map((b) => b.種)).toEqual(['段', '図', '符']);
  });
});

describe('図を読む', () => {
  it('Markdown の表を読み、名前の並びなら棒にする', () => {
    const 図 = 読めた('chart', '| SNS | 7 日以内 | 90 日以上 |\n|---|---|---|\n| X | 2 | **6,614** |\n| Bluesky | 55 | 835 |');
    expect(図.形).toBe('棒');
    expect(図.横).toBe('SNS');
    expect(図.系列).toEqual(['7 日以内', '90 日以上']);
    expect(図.行たち).toEqual([
      { 名: 'X', 値: [2, 6614] },
      { 名: 'Bluesky', 値: [55, 835] },
    ]);
  });

  it('1 列目が日付なら線にする', () => {
    expect(読めた('chart', '日,店\n11-17,4310\n11-18,4302').形).toBe('線');
    expect(読めた('chart', '日\t店\n2026/11/17\t4310').形).toBe('線');
  });

  it('囲みの頭の形が勝つ', () => {
    expect(読めた('chart stack', '日,a,b\n11-17,1,2').形).toBe('積み上げ');
    expect(読めた('chart bar', '日,a\n11-17,1').形).toBe('棒');
  });

  it('表の前の 1 行を題にする', () => {
    expect(読めた('chart', '# セラピの鮮度\n名,数\nX,2').題).toBe('セラピの鮮度');
    expect(読めた('chart', 'title: 店の数\n名,数\nX,2').題).toBe('店の数');
  });

  it('空の升は描かない（null）', () => {
    expect(読めた('chart', '名,a,b\nX,1,').行たち[0].値).toEqual([1, null]);
  });

  it('描けない書き方は、理由を返す（黙って字に戻さない）', () => {
    expect(図を読む('chart radar', '名,a\nX,1')).toEqual({ 訳: expect.stringContaining('radar') });
    expect(図を読む('chart', '名,a')).toEqual({ 訳: expect.stringContaining('表がありません') });
    expect(図を読む('chart', '名\nX')).toEqual({ 訳: expect.stringContaining('列が足りません') });
    expect(図を読む('chart', '名,a\nX,たくさん')).toEqual({ 訳: expect.stringContaining('たくさん') });
    expect(図を読む('chart', '名,a\nX,')).toEqual({ 訳: '数が 1 つもありません' });
    expect(図を読む('chart stack', '名,a\nX,-1')).toEqual({ 訳: expect.stringContaining('負の数') });
  });

  it('行と列の上限を越えたら断る', () => {
    const 多い行 = ['名,a', ...Array.from({ length: 61 }, (_, i) => `r${i},${i}`)].join('\n');
    expect(図を読む('chart', 多い行)).toEqual({ 訳: expect.stringContaining('60 行まで') });
    expect(図を読む('chart', '名,a,b,c,d,e,f,g\nX,1,1,1,1,1,1,1')).toEqual({ 訳: expect.stringContaining('6 つまで') });
  });
});

describe('数にする', () => {
  it('桁区切り・%・太字を読む', () => {
    expect(数にする('6,614')).toBe(6614);
    expect(数にする('12%')).toBe(12);
    expect(数にする('**835**')).toBe(835);
    expect(数にする('')).toBeNull();
    expect(数にする('—')).toBeNull();
    expect(数にする('abc')).toBeUndefined();
  });
});

describe('目盛り', () => {
  it('0 から、きりのよい刻みで最大を覆う', () => {
    expect(目盛り(0, 6614)).toEqual([0, 2000, 4000, 6000, 8000]);
    expect(目盛り(2, 55)).toEqual([0, 20, 40, 60]);
  });
  it('負の数も覆う', () => {
    const 刻み = 目盛り(-5, 10);
    expect(刻み[0]).toBeLessThanOrEqual(-5);
    expect(刻み.at(-1)).toBeGreaterThanOrEqual(10);
  });
  it('全部 0 でも割り算で壊れない', () => {
    expect(目盛り(0, 0)).toEqual([0, 1]);
  });
});

describe('縦の棒と円', () => {
  it('column は縦の棒、pie は円', () => {
    expect(読めた('chart column', '日,投稿\n11-17,267\n11-18,301').形).toBe('縦棒');
    expect(読めた('chart pie', 'SNS,人\nX,6623\nBluesky,959').形).toBe('円');
  });

  it('円は値の列 1 つ・6 切れまで・負の数なし。断るときは理由を言う', () => {
    expect(図を読む('chart pie', '名,a,b\nX,1,2')).toEqual({ 訳: expect.stringContaining('1 つだけ') });
    const 多い = ['名,a', ...Array.from({ length: 7 }, (_, i) => `r${i},1`)].join('\n');
    expect(図を読む('chart pie', 多い)).toEqual({ 訳: expect.stringContaining('6 切れまで') });
    expect(図を読む('chart pie', '名,a\nX,-1\nY,2')).toEqual({ 訳: expect.stringContaining('負の数') });
  });

  it('円の切れは 1 周を割合で分ける', () => {
    const 切れ = 円の切れ([
      { 名: 'X', 値: [3] },
      { 名: 'Y', 値: [1] },
    ]);
    expect(切れ).toEqual([
      { 始め: 0, 終わり: 0.75, 割合: 0.75 },
      { 始め: 0.75, 終わり: 1, 割合: 0.25 },
    ]);
  });
});

describe('横軸に名前を書く番', () => {
  it('8 つまでは全部書く', () => {
    expect([...名を書く番(7)]).toEqual([0, 1, 2, 3, 4, 5, 6]);
  });
  it('最後とその手前を重ねない（30 日なら 0・5・10・15・20・29）', () => {
    expect([...名を書く番(30)].sort((a, b) => a - b)).toEqual([0, 5, 10, 15, 20, 29]);
  });
  it('毎日 30 日分なら、間引いても最初と最後は書き、6 つ前後に収める', () => {
    const 番 = 名を書く番(30);
    expect(番.has(0)).toBe(true);
    expect(番.has(29)).toBe(true);
    expect(番.size).toBeLessThanOrEqual(7);
  });
});

describe('線の目盛り', () => {
  it('毎日 4,300 前後なら、0 からではなく動きの幅に合わせる', () => {
    expect(線は動きに合わせるか([4290, 4310, 4330])).toBe(true);
    const 刻み = 目盛り(4290, 4330, 4, false);
    expect(刻み[0]).toBeGreaterThan(4000);
  });
  it('0 に近い値や負の数があれば 0 から', () => {
    expect(線は動きに合わせるか([2, 6614])).toBe(false);
    expect(線は動きに合わせるか([-1, 5])).toBe(false);
  });
});
