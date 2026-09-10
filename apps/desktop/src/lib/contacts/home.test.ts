import { describe, expect, it } from 'vitest';

import { いまの様子, できることの案内, はじめの一歩を出すか } from './home';
import type { 素材 } from './list';

const 空: 素材 = {
  自分: 'ME',
  机のAIたち: [],
  会議の相手: [],
  覚えた: [],
};

describe('初期画面のいまの様子', () => {
  it('何も無ければ、ぜんぶ 0', () => {
    expect(いまの様子(空, false)).toEqual({
      席: 0,
      ルーム: 0,
      ルームの人: 0,
      覚えた: 0,
      預かり所: false,
    });
  });

  it('机の席・ルーム・覚えた相手を数える', () => {
    const 様子 = いまの様子(
      {
        ...空,
        机のAIたち: ['zumen', 'git-qa'],
        部屋たち: [
          { id: 'A', members: 2, host: true },
          { id: 'B', members: 3, host: false },
        ],
        覚えた: [{ key: 'K1', label: 'mac air', has_address: true }],
      },
      true,
    );
    expect(様子).toEqual({ 席: 2, ルーム: 2, ルームの人: 5, 覚えた: 1, 預かり所: true });
  });

  it('預かり所は、置いているかをそのまま持つ', () => {
    // **ここで判断しない。**置いてあるかどうかは、置いた人が決めたことである
    expect(いまの様子(空, true).預かり所).toBe(true);
  });
});

describe('できることの案内（相手に依らない一般形）', () => {
  it('4 つ、相手を選んだときと同じ並び', () => {
    expect(できることの案内().map((k) => k.種類)).toEqual(['chat', 'group', 'call', 'calendar']);
  });

  it('ビデオ会議は「条件つき」、予定は「できる」（2026-09-10 から）', () => {
    // **網越えの実測は 0 件。**できると書かない（D88）
    // 予定は自分のぶんを置けるようになった（相手の予定は見えない）
    const 引く = (種類: string) => できることの案内().find((k) => k.種類 === 種類);
    expect(引く('call')?.状態).toBe('条件つき');
    expect(引く('calendar')?.状態).toBe('できる');
    expect(引く('chat')?.状態).toBe('できる');
    expect(引く('group')?.状態).toBe('できる');
  });

  it('どの案内にも 1 行が付く', () => {
    for (const 案内 of できることの案内()) expect(案内.訳).toBeTruthy();
  });
});

describe('はじめの一歩', () => {
  it('誰も居らず、何も覚えていないときだけ出す', () => {
    expect(はじめの一歩を出すか(いまの様子(空, false))).toBe(true);
  });

  it('机に誰か着いていれば出さない', () => {
    expect(はじめの一歩を出すか(いまの様子({ ...空, 机のAIたち: ['zumen'] }, false))).toBe(false);
  });

  it('相手を覚えていれば出さない', () => {
    const 覚えた = { ...空, 覚えた: [{ key: 'K', label: 'air', has_address: true }] };
    expect(はじめの一歩を出すか(いまの様子(覚えた, false))).toBe(false);
  });
});
