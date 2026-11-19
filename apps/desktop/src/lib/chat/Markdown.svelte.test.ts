// @vitest-environment jsdom
//
// **発言の中の繋ぎ**（11-11）。http(s) は押すと外のブラウザで開き、画面の中では移らない。
import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';

const 開いた: string[] = [];
vi.mock('$lib/bridge', () => ({
  openLink: (url: string) => {
    開いた.push(url);
    return Promise.resolve();
  },
}));

import Markdown from './Markdown.svelte';

afterEach(() => {
  cleanup();
  開いた.length = 0;
});

describe('Markdown の繋ぎ', () => {
  it('むき出しの https:// を押すと、外のブラウザで開き、画面の中では移らない', async () => {
    render(Markdown, { 本文: '先は https://example.com/a です', 見分ける名: [], 自分の呼ばれる名: [] });
    const 繋ぎ = screen.getByRole('link', { name: 'https://example.com/a' });
    expect(繋ぎ.getAttribute('href')).toBe('https://example.com/a');
    const 押した = await fireEvent.click(繋ぎ);
    expect(押した, '既定の移動は止める').toBe(false);
    expect(開いた).toEqual(['https://example.com/a']);
  });

  it('http(s) でない繋ぎは押せない字のまま', () => {
    render(Markdown, { 本文: '[ここ](javascript:alert(1))', 見分ける名: [], 自分の呼ばれる名: [] });
    expect(screen.queryByRole('link')).toBeNull();
  });
});

describe('Markdown の QR（0.1.53）', () => {
  const 説明 = 'QR コードで出す（スマホで開く）';

  it('http(s) の繋ぎの横に QR の押し口があり、押すとその URL の QR が出て、閉じると消える', async () => {
    render(Markdown, { 本文: '先は https://example.com/a?b=1&c=2 です', 見分ける名: [], 自分の呼ばれる名: [] });
    expect(screen.queryByRole('dialog')).toBeNull();
    await fireEvent.click(screen.getByRole('button', { name: 説明 }));
    const 窓 = screen.getByRole('dialog');
    expect(screen.getByRole('img', { name: 'https://example.com/a?b=1&c=2' })).toBeTruthy();
    expect(窓.textContent).toContain('https://example.com/a?b=1&c=2');
    expect(開いた, 'QR を出すだけで、ブラウザは開かない').toEqual([]);
    await fireEvent.click(screen.getByRole('button', { name: '閉じる' }));
    expect(screen.queryByRole('dialog')).toBeNull();
  });

  it('Esc でも閉じる', async () => {
    render(Markdown, { 本文: 'https://example.com', 見分ける名: [], 自分の呼ばれる名: [] });
    await fireEvent.click(screen.getByRole('button', { name: 説明 }));
    await fireEvent.keyDown(screen.getByRole('dialog'), { key: 'Escape' });
    expect(screen.queryByRole('dialog')).toBeNull();
  });

  it('押せない繋ぎ（http(s) でない）には QR を付けない', () => {
    render(Markdown, { 本文: '[ここ](javascript:alert(1))', 見分ける名: [], 自分の呼ばれる名: [] });
    expect(screen.queryByRole('button', { name: 説明 })).toBeNull();
  });
});
