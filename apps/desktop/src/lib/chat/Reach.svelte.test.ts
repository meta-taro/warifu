// @vitest-environment jsdom
//
// **届く先の部品の試験**（計画 1-D・0.1.41）。
// 「いま打つと誰に届くのか」は、渡した一覧がそのまま 1 人ずつ出ること（spec 04 S04-1 の画面側）。
import { cleanup, render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';

import Reach from './Reach.svelte';

const 鍵 = (n: number) => `${'ABCDEFGHIJKLMNOPQRSTUVWXYZ'[n % 26].repeat(52)}`;

afterEach(cleanup);

describe('Reach', () => {
  it('届く先は渡した順に 1 人ずつ出て、全文が title に入る', () => {
    render(Reach, {
      locale: 'ja',
      届く先: ['山田さん', 'dev のエージェント'],
      届く先の顔: [鍵(1), 'dev のエージェント'],
    });
    const 名たち = screen.getAllByText(/山田さん|dev のエージェント/);
    expect(名たち.length).toBe(2);
    // 省略される字には hover で全文（title）が付く（lib/ui/ellipsis の決まり）。外側と字の両方に付くので 1 つ以上
    expect(screen.getAllByTitle('山田さん').length).toBeGreaterThan(0);
    expect(screen.getAllByTitle('dev のエージェント').length).toBeGreaterThan(0);
  });

  it('誰も居なければ名前は 1 つも出ない', () => {
    render(Reach, { locale: 'ja', 届く先: [], 届く先の顔: [] });
    expect(screen.queryAllByTitle(/./).length).toBe(0);
  });
});
