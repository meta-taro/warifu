// @vitest-environment jsdom
//
// **ルームに居る人の部品の試験**（計画 1-D・0.1.41）。
// 自分の札・主催の札が正しい行に付き、その機械のエージェントが名札で並ぶこと。
import { cleanup, render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';

import Participants from './Participants.svelte';

const 鍵 = (n: number) => `${'ABCDEFGHIJKLMNOPQRSTUVWXYZ'[n % 26].repeat(52)}`;

afterEach(cleanup);

describe('Participants', () => {
  it('自分の行に「あなた」、主催の行に「主催」が付き、エージェントは名札で並ぶ', () => {
    render(Participants, {
      locale: 'ja',
      並び: [
        { key: 鍵(1), 自分: true, 主催: false, エージェントたち: ['dev のエージェント'] },
        { key: 鍵(2), 自分: false, 主催: true, エージェントたち: [] },
      ],
      名: (key: string) => (key === 鍵(1) ? '自分の機械' : '主催の機械'),
    });
    expect(screen.getByText('自分の機械')).toBeTruthy();
    expect(screen.getByText('主催の機械')).toBeTruthy();
    expect(screen.getAllByText('あなた').length).toBe(1);
    expect(screen.getAllByText('主催').length).toBe(1);
    expect(screen.getByText('dev のエージェント')).toBeTruthy();
  });

  it('主催が居ない並びには主催の札が出ない', () => {
    render(Participants, {
      locale: 'ja',
      並び: [{ key: 鍵(1), 自分: true, 主催: false, エージェントたち: [] }],
      名: () => '自分の機械',
    });
    expect(screen.queryByText('主催')).toBeNull();
  });
});
