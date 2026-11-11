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
