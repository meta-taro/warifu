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
      名: (key: string) => (key === 鍵(1) ? '使われない' : '主催の機械'),
      自分の名: '自分の機械',
    });
    expect(screen.getByText('自分の機械')).toBeTruthy();
    expect(screen.getByText('主催の機械')).toBeTruthy();
    expect(screen.getAllByText('あなた').length).toBe(1);
    expect(screen.getAllByText('主催').length).toBe(1);
    expect(screen.getByText('dev のエージェント')).toBeTruthy();
  });

  it('名乗りの無い自分は、鍵の頭ではなく連絡帳と同じ「あなた（この PC の人）」で出て、札を重ねない', () => {
    render(Participants, {
      locale: 'ja',
      並び: [
        { key: 鍵(1), 自分: true, 主催: true, エージェントたち: [] },
        { key: 鍵(2), 自分: false, 主催: false, エージェントたち: [] },
      ],
      // 呼び名を知らない相手は鍵の頭で返ってくる（`画面での名` の既定）
      名: (key: string) => `${key.slice(0, 12)}…`,
    });
    expect(screen.getByText('あなた（この PC の人）')).toBeTruthy();
    expect(document.body.textContent).not.toContain(鍵(1).slice(0, 12));
    expect(screen.queryByText('あなた')).toBeNull();
    expect(screen.getByText('主催')).toBeTruthy();
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
