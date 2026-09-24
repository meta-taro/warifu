import { describe, expect, it } from 'vitest';
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';

/**
 * **書いたトークンが、実在するか。**
 *
 * 2026-09-24、オーナー指摘 ——
 *
 * > **あと押した方ホバーじゃないとどっち押したかわかんなくね？**
 *
 * **切り替えの「選んでいる側」が、周りと同じ見た目になっていた。**
 * 原因は `var(--bg)` と `var(--text)` と `var(--text-muted)` ——
 * **どれも `tokens.css` に無い名前**である（在るのは `--bg-app` / `--text-primary` など）。
 *
 * **CSS は、名前を間違えても黙って通る。**型も試験も通り、**画面を見るまで分からない。**
 * **人が気をつける方式は必ず漏れる**ので、ここで機械に見させる。
 */
const 根 = new URL('../../', import.meta.url).pathname;

function 集める(場所: string, 出: string[] = []): string[] {
  for (const 名 of readdirSync(場所)) {
    const 道 = join(場所, 名);
    if (statSync(道).isDirectory()) {
      集める(道, 出);
    } else if (名.endsWith('.svelte') || 名.endsWith('.css')) {
      出.push(道);
    }
  }
  return 出;
}

describe('画面に書いたトークン', () => {
  const ファイルたち = 集める(根);
  // **注記は外す。**説明の中に書いた `var(--bg)` を、使っていると数えない
  const 中身 = ファイルたち.map((道) => ({
    道,
    文: readFileSync(道, 'utf-8')
      .replace(/\/\*[\s\S]*?\*\//g, ' ')
      .replace(/<!--[\s\S]*?-->/g, ' '),
  }));

  // **定義は 1 つの束にする** —— tokens.css だけでなく、
  // 部品の中で定義しているもの（`--なんとか: 値`）も数える
  const 定義 = new Set<string>();
  for (const { 文 } of 中身) {
    for (const m of 文.matchAll(/(--[a-zA-Z0-9-]+)\s*:/g)) 定義.add(m[1]);
  }

  it('var(--…) に書いた名前が、どこかで定義されている', () => {
    const 無い: string[] = [];
    for (const { 道, 文 } of 中身) {
      for (const m of 文.matchAll(/var\(\s*(--[a-zA-Z0-9-]+)\s*([,)])/g)) {
        const [, 名, 次] = m;
        // **控えを書いてあるものは通す**（`var(--x, 既定)` は、無くても当たる）
        if (次 === ',') continue;
        if (!定義.has(名)) 無い.push(`${道.replace(根, '')}: ${名}`);
      }
    }
    expect(無い, `**無い名前を書いている**（CSS は黙って通る）:\n${無い.join('\n')}`).toEqual([]);
  });

  it('見張る先が空になっていない', () => {
    // **数え漏れで「0 件だから ok」にしない**（落とし穴 7 と同じ形）
    expect(ファイルたち.length).toBeGreaterThan(10);
    expect(定義.size).toBeGreaterThan(40);
  });
});
