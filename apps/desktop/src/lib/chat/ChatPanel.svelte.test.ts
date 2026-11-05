// @vitest-environment jsdom
//
// **会話欄の部品の試験**（計画 1-D・0.1.42）。
// 判断は純粋な部品（`./bubble`・`./name`・`./draft`・`./mention`）が持ち、そちらにも試験がある。
// ここで見るのは、**組んだ画面に、その判断がそのまま出るか**（spec 00 S00-12・02 S02-2／S02-8・D107）。
import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import { afterEach, beforeAll, describe, expect, it, vi } from 'vitest';

import type { 会話行 } from '$lib/meeting/announce';
import { MESSAGES } from '$lib/i18n/messages';
import ChatPanel from './ChatPanel.svelte';

const ja = MESSAGES.ja;
const 鍵 = (c: string) => c.repeat(52);

const 素 = {
  locale: 'ja' as const,
  会話: [] as 会話行[],
  届く先がある: true,
  会議中: true,
  この機械の人数: 0,
  送る: () => {},
  届く先: ['山田さん'],
};

// jsdom は要素の scrollTo を持たない（欄を底へ送るのに使う）
beforeAll(() => {
  Element.prototype.scrollTo = () => {};
});
afterEach(cleanup);

const 打つ欄 = () => screen.getByRole('textbox') as HTMLTextAreaElement;
const 打つ = async (文: string) => {
  await fireEvent.input(打つ欄(), { target: { value: 文 } });
};

describe('ChatPanel', () => {
  it('見出しは「文字で話す」ではなく、宛先を名乗る', () => {
    render(ChatPanel, { ...素, 宛先: { 鍵: 'chat.to.room', 値: { name: '開発', n: 3 } } });
    expect(screen.getByRole('heading').textContent).toContain('開発（3 人）へ');
    expect(screen.queryByText(ja['chat.title'])).toBeNull();
  });

  it('名簿に名前のある相手は、行の鍵の頭ではなく名前で出る（生の鍵を見せない）', () => {
    const 相手 = 鍵('K');
    render(ChatPanel, {
      ...素,
      会話: [{ who: 相手.slice(0, 8), 顔の種: 相手, body: 'こんにちは', mine: false }],
      画面での名簿: { [相手]: '山田さん' },
    });
    expect(screen.getByText('山田さん', { selector: 'b' })).toBeTruthy();
    expect(document.body.textContent).not.toContain(相手.slice(0, 8));
  });

  it('自分の発言は右・ほかの機械からは左・知らせは吹き出しにせず、エージェントには字で印を付ける', () => {
    const { container } = render(ChatPanel, {
      ...素,
      会話: [
        { who: 'わたし', body: '送った', mine: true },
        { who: '山田さん', body: '届いた', mine: false },
        { who: '', body: '山田さんが入りました', mine: false, system: true },
        { who: 'dev', body: '調べます', mine: true, agent: true },
      ],
    });
    const 行 = (文: string) => screen.getByText(文).closest('.line') as HTMLElement;
    expect(行('送った').classList.contains('right')).toBe(true);
    expect(行('届いた').classList.contains('right')).toBe(false);
    expect(行('山田さんが入りました').classList.contains('system')).toBe(true);
    expect(container.querySelectorAll('.bubble').length).toBe(3);
    expect(行('調べます').querySelector('.kind')?.textContent).toBe(ja['avatar.owner.agent']);
  });

  it('発言のマークダウンは表や見出しとして見せ、タグは字のまま出す（0.1.44）', () => {
    const { container } = render(ChatPanel, {
      ...素,
      会話: [
        {
          who: '山田さん',
          body: '## 題\n\n| 問い | 場所 |\n|---|---|\n| 仕様 | **試験書** |\n\n<img src=x onerror=alert(1)>',
          mine: false,
        },
      ],
    });
    const 吹き出し = container.querySelector('.bubble') as HTMLElement;
    expect(吹き出し.querySelector('table td strong')?.textContent).toBe('試験書');
    expect(吹き出し.textContent).not.toContain('|---|');
    // 届いた文で要素を作らない
    expect(吹き出し.querySelector('img')).toBeNull();
    expect(吹き出し.textContent).toContain('<img src=x onerror=alert(1)>');
  });

  it('Enter で前後の空白を落として送り、欄を空ける。届く先が無ければ送る押し所は押せない', async () => {
    const 送る = vi.fn();
    const { rerender } = render(ChatPanel, { ...素, 送る });
    await 打つ('  こんにちは  ');
    await fireEvent.keyDown(打つ欄(), { key: 'Enter' });
    expect(送る).toHaveBeenCalledWith('こんにちは', false);
    expect(打つ欄().value).toBe('');

    await rerender({ ...素, 送る, 届く先がある: false });
    await 打つ('まだ送らない');
    const 押し所 = screen.getAllByRole('button').find((b) => b.closest('.say') && !b.classList.contains('attach'));
    expect((押し所 as HTMLButtonElement).disabled).toBe(true);
  });

  it('部屋を替えても打ちかけの文は混ざらず、戻れば前の文が出る', async () => {
    const { rerender } = render(ChatPanel, { ...素, 部屋: 'A' });
    await 打つ('A に書きかけ');
    await rerender({ ...素, 部屋: 'B' });
    expect(打つ欄().value).toBe('');
    await 打つ('B に書きかけ');
    await rerender({ ...素, 部屋: 'A' });
    expect(打つ欄().value).toBe('A に書きかけ');
  });

  it('@ で呼んでも届く先は変わらないと言い、呼ばれた行にだけ「あなた宛て」が付く', async () => {
    render(ChatPanel, {
      ...素,
      自分の呼ばれる名: ['わたし'],
      会話: [
        { who: '山田さん', body: '@わたし 見てください', mine: false },
        { who: '山田さん', body: 'みなさんへ', mine: false },
      ],
    });
    expect(screen.getAllByText(ja['chat.mention.me']).length).toBe(1);
    expect(screen.getByText('みなさんへ').closest('.line')?.classList.contains('to-me')).toBe(false);

    await 打つ('@山田さん お願いします');
    expect(screen.getByText(/@山田さん を呼びます（ルームの全員に見えます）/)).toBeTruthy();
  });

  it('呼ばれた行にだけ［受けました］を出して押せば返し、自分が呼んだ行には受けた／まだを出す（D130・0.1.44）', async () => {
    const 受ける = vi.fn();
    const 分 = 60_000;
    const 今 = Date.now();
    render(ChatPanel, {
      ...素,
      受ける,
      自分の呼ばれる名: ['わたし'],
      会話: [
        { who: '山田さん', body: '@わたし 見積を見てください', mine: false, 印: 'aa' },
        { who: '山田さん', body: 'みなさんへ', mine: false, 印: 'bb' },
        { who: 'わたし', body: '@山田さん 受注です', mine: true, 印: 'cc', 時: 今 - 40 * 分 },
        {
          who: 'わたし',
          body: '@山田さん 請求です',
          mine: true,
          印: 'dd',
          時: 今,
          受け: [{ 誰: '山田さん', 方: '押した', 時: 今 }],
        },
      ],
    });
    const 押し所 = screen.getAllByRole('button', { name: ja['chat.ack.press'] });
    expect(押し所).toHaveLength(1);
    await fireEvent.click(押し所[0]);
    expect(受ける).toHaveBeenCalledWith('aa', undefined);
    // 30 分を過ぎた「まだ」は注意の色（字でも言う）
    const まだ = screen.getByText('まだ受けていない（40 分）');
    expect(まだ.classList.contains('none')).toBe(true);
    expect(screen.getByText('山田さん が受けた')).toBeTruthy();
  });

  it('秘密らしいものは、すぐ送らずに確かめる。直すなら欄に残り、「それでも送る」で初めて送る（0.1.42）', async () => {
    // 割符の会話は送ったら消せない（P2P）。止められるのは送る前だけ
    const 送る = vi.fn();
    render(ChatPanel, { ...素, 送る });
    const 値 = '0f1e2d3c4b5a6978'.repeat(4);
    await 打つ(`TOKEN=${値}`);
    await fireEvent.keyDown(打つ欄(), { key: 'Enter' });

    expect(送る).not.toHaveBeenCalled();
    expect(打つ欄().value).toBe(`TOKEN=${値}`);
    const 確かめ = screen.getByRole('alert');
    expect(確かめ.textContent).toContain('取り消せません');
    // 値は確かめの文に繰り返さない
    expect(確かめ.textContent).not.toContain(値);

    await fireEvent.click(screen.getByRole('button', { name: ja['chat.secret.send'] }));
    expect(送る).toHaveBeenCalledWith(`TOKEN=${値}`, true);
    expect(screen.queryByRole('alert')).toBeNull();
  });

  it('秘密らしいものを見つけても「送らずに直す」なら送らず、欄に残す', async () => {
    const 送る = vi.fn();
    render(ChatPanel, { ...素, 送る });
    await 打つ('ghp_' + 'aZ3kQ9mX7pL2'.repeat(3));
    await fireEvent.keyDown(打つ欄(), { key: 'Enter' });
    await fireEvent.click(screen.getByRole('button', { name: ja['chat.secret.cancel'] }));
    expect(送る).not.toHaveBeenCalled();
    expect(screen.queryByRole('alert')).toBeNull();
    expect(打つ欄().value).toContain('ghp_');
  });

  it('秘密らしいものが無ければ、確かめずにそのまま送る', async () => {
    const 送る = vi.fn();
    render(ChatPanel, { ...素, 送る });
    await 打つ('commit 9085f6a を push しました');
    await fireEvent.keyDown(打つ欄(), { key: 'Enter' });
    expect(送る).toHaveBeenCalledWith('commit 9085f6a を push しました', false);
    expect(screen.queryByRole('alert')).toBeNull();
  });

  it('長い発言だけ畳み、［もっと見る］で開いて［たたむ］で戻る（0.1.42）', async () => {
    const { container } = render(ChatPanel, {
      ...素,
      会話: [
        { who: '山田さん', body: 'あ'.repeat(500), mine: false },
        { who: '山田さん', body: '短い', mine: false },
      ],
    });
    const 吹き出し = container.querySelectorAll('.bubble');
    expect(吹き出し[0].classList.contains('folded')).toBe(true);
    expect(吹き出し[1].classList.contains('folded')).toBe(false);
    const 開く = screen.getByRole('button', { name: ja['chat.more'] });
    expect(screen.getAllByRole('button', { name: ja['chat.more'] })).toHaveLength(1);
    expect(開く.getAttribute('aria-expanded')).toBe('false');

    await fireEvent.click(開く);
    expect(container.querySelectorAll('.bubble')[0].classList.contains('folded')).toBe(false);
    const 閉じる = screen.getByRole('button', { name: ja['chat.less'] });
    expect(閉じる.getAttribute('aria-expanded')).toBe('true');

    await fireEvent.click(閉じる);
    expect(container.querySelectorAll('.bubble')[0].classList.contains('folded')).toBe(true);
  });
});
