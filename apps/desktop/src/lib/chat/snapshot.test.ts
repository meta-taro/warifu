import { describe, expect, it } from 'vitest';

import { 写しの上限, 画面の写しを書く, type 写しの素材 } from './snapshot';

const 素材 = (行数: number, 本文 = 'ああああ'): 写しの素材 => ({
  版: '0.1.34',
  題字: 'つながっています',
  知らせ: null,
  選んでいる: null,
  見ている: {
    名前: '開発6台',
    人数: 2,
    見出し: '開発6台（2 人）へ',
    届く先: ['ZenBook'],
    行たち: Array.from({ length: 行数 }, (_, i) => ({ who: 'ZenBook', body: `${本文}${i}`, mine: false, at: '15:56' })),
  },
  ほか: [{ 名前: 'ASUS', 人数: 3, 行たち: [{ who: '自分', body: 'いいいい', mine: true, at: '15:56' }] }],
});

describe('画面の写し', () => {
  it('開いているルーム・届く先・行と、ほかのルームの行を書く', () => {
    const 写し = 画面の写しを書く(素材(2));
    expect(写し).toContain('開いているルーム 開発6台（2 人）');
    expect(写し).toContain('届く先 ZenBook');
    expect(写し).toContain('15:56 ZenBook「ああああ1」');
    expect(写し).toContain('ほかのルーム ASUS（3 人・全 1 行）');
    expect(写し).toContain('15:56 自分「いいいい」');
  });

  it('多くても長くても、上限のバイトに収める', () => {
    const 写し = 画面の写しを書く(素材(200, 'う'.repeat(500)));
    expect(new TextEncoder().encode(写し).length).toBeLessThanOrEqual(写しの上限);
    expect(写し).toContain('全 200 行');
  });

  it('ルームを開いていなければ、そう書く', () => {
    expect(画面の写しを書く({ ...素材(0), 見ている: null })).toContain('開いているルーム なし');
  });
});
