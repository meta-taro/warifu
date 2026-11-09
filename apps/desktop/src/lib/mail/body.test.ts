import { describe, expect, it } from 'vitest';

import { 本文を見やすく } from './body';

describe('本文を見やすく（見せ方だけ・メールは変えない）', () => {
  it('よく使う文字の参照を字に戻す', () => {
    expect(本文を見やすく('a=1&amp;b=2 &lt;x&gt; &quot;q&quot; &#39;s&#39; &#x41;')).toBe(`a=1&b=2 <x> "q" 's' A`);
  });
  it('戻した字はタグにならない（ただの字のまま）', () => {
    expect(本文を見やすく('&lt;script&gt;')).toBe('<script>');
  });
  it('知らない参照はそのまま', () => {
    expect(本文を見やすく('&unknown; &#xZZ;')).toBe('&unknown; &#xZZ;');
  });
  it('2 つ以上続く空行は 1 つにまとめ、頭と尻の空行は落とす', () => {
    expect(本文を見やすく('\n\nA\n\n\n\nB\n \n\t\nC\n\n')).toBe('A\n\nB\n\nC');
  });
  it('改行の形（CRLF）を揃える', () => {
    expect(本文を見やすく('A\r\n\r\n\r\nB')).toBe('A\n\nB');
  });
});
