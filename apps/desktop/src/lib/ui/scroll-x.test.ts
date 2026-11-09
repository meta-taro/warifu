import { describe, expect, it } from 'vitest';
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';

/**
 * **縦に流す欄は、横の振る舞いを書く**（0.1.46）。
 *
 * 2026-11-09 —— 会話欄の下に、謎の横スクロールが出た（人の指摘）。`overflow-y: auto` だけを書くと、
 * `overflow-x` も auto になる（CSS の決まり：片方が visible 以外なら、もう片方の visible は auto に読み替わる）。
 * 中の 1 行（呼んだ相手の札を 1 行に固定していた）が欄より広くなると、黙って横に流れた。
 *
 * 規則：`.svelte` の `<style>` で `overflow-y: auto|scroll` を当てた規則は、同じ規則に `overflow-x` を書く
 * （横に流したいなら `overflow-x: auto` と明示する。書いていなければ、それは意図していない）。
 */
const 根 = new URL('../../', import.meta.url).pathname;

function 集める(場所: string, 出: string[] = []): string[] {
  for (const 名 of readdirSync(場所)) {
    const 道 = join(場所, 名);
    if (statSync(道).isDirectory()) 集める(道, 出);
    else if (名.endsWith('.svelte')) 出.push(道);
  }
  return 出;
}

/** `<style>` の中で、縦に流すのに横を書いていない規則の選択子。 */
export function 横を書いていない規則(文: string): string[] {
  const style = 文.match(/<style[^>]*>([\s\S]*?)<\/style>/)?.[1] ?? '';
  // コメントを外す（説明の中の `overflow-y: auto` を規則と取り違えない）
  const 素 = style.replace(/\/\*[\s\S]*?\*\//g, '');
  const 出: string[] = [];
  for (const m of 素.matchAll(/([^{}]+)\{([^{}]*)\}/g)) {
    const [, 選択子, 本体] = m;
    if (!/overflow-y\s*:\s*(auto|scroll)/.test(本体)) continue;
    if (/overflow-x\s*:/.test(本体)) continue;
    出.push(選択子.trim());
  }
  return 出;
}

describe('縦に流す欄は、横の振る舞いを書く', () => {
  it('規則を読める（対照）', () => {
    expect(横を書いていない規則('<style>.a { overflow-y: auto; }</style>')).toEqual(['.a']);
    expect(横を書いていない規則('<style>.a { overflow-y: auto; overflow-x: hidden; }</style>')).toEqual([]);
    expect(横を書いていない規則('<style>/* overflow-y: auto */ .a { color: red; }</style>')).toEqual([]);
  });

  it('画面の全部の .svelte で守られている', () => {
    const 外れ = 集める(根).flatMap((道) =>
      横を書いていない規則(readFileSync(道, 'utf-8')).map((s) => `${道.replace(根, '')}: ${s}`),
    );
    expect(外れ).toEqual([]);
  });
});
