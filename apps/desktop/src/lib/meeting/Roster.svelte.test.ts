// @vitest-environment jsdom
//
// **画面の部品の試験**（計画 1-D・0.1.41）。画面（Svelte）に試験が 1 本も無かった。
// ここでは**見出しの人数・主催の札・自分の行**が、渡した名簿どおりに出ることを固定する
// （spec 04 S04-2 / S04-4 の画面側）。
import { cleanup, render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';

import Roster from './Roster.svelte';

const 鍵 = (n: number) => `${'ABCDEFGHIJKLMNOPQRSTUVWXYZ'[n % 26].repeat(52)}`;

afterEach(cleanup);

describe('Roster', () => {
  it('見出しの人数は名簿の長さで、定員と並ぶ', () => {
    render(Roster, {
      locale: 'ja',
      capacity: 12,
      members: [
        { key: 鍵(1), me: true, host: true, path: 'unknown' },
        { key: 鍵(2), path: 'direct' },
        { key: 鍵(3), path: 'relayed' },
      ],
    });
    expect(screen.getByText('3 / 12')).toBeTruthy();
  });

  it('主催の札は主催の行にだけ付き、自分の行には（自分）が付く', () => {
    render(Roster, {
      locale: 'ja',
      capacity: 12,
      members: [
        { key: 鍵(1), me: true, path: 'unknown' },
        { key: 鍵(2), host: true, path: 'direct' },
      ],
      names: { [鍵(2)]: '主催の人' },
    });
    const 主催の札 = screen.getAllByText('主催');
    expect(主催の札.length).toBe(1);
    expect(screen.getByText(/（自分）/)).toBeTruthy();
    expect(screen.getByText(/主催の人/)).toBeTruthy();
  });

  it('同じ PC のエージェントは名簿に出るが、人数には数えない', () => {
    render(Roster, {
      locale: 'ja',
      capacity: 12,
      members: [{ key: 鍵(1), me: true, host: true, path: 'unknown' }],
      この機械のAIたち: ['dev のエージェント'],
    });
    expect(screen.getByText('1 / 12')).toBeTruthy();
    expect(screen.getByText(/dev のエージェント/)).toBeTruthy();
  });
});
