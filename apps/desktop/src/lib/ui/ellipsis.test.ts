import { describe, expect, it } from 'vitest';
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';

/**
 * **省略する字には、全文を出す `title` が要る。**
 *
 * 2026-10-26 —— 連絡帳の「この PC」で長い名前が「…」で切れ、
 * **マウスを当てても全文が出ず、どの席か見分けられなかった**（人の指摘・Windows の 0.1.39）。
 *
 * `text-overflow: ellipsis` は、**窓の幅しだいで字を黙って捨てる。**
 * 捨てた字を人が取り戻す手は hover の `title` しかない。
 * **人が気をつける方式は漏れる**（6 か所のうち title が在ったのは 1 か所）ので、ここで機械に見させる。
 *
 * 規則：`.svelte` の `<style>` で `text-overflow: ellipsis` を当てている
 * 選択子の**末尾のクラス**を持つ要素は、同じタグに `title=` を持つ。
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

/** `<style>` の中から、`text-overflow: ellipsis` を含む規則の選択子の末尾のクラス名を拾う。 */
export function 省略するクラス(文: string): string[] {
  const 出: string[] = [];
  const style = 文.match(/<style[^>]*>([\s\S]*?)<\/style>/)?.[1] ?? '';
  for (const m of style.matchAll(/([^{}]+)\{([^{}]*)\}/g)) {
    const [, 選択子, 本体] = m;
    if (!/text-overflow\s*:\s*ellipsis/.test(本体)) continue;
    for (const 一つ of 選択子.split(',')) {
      const 末尾 = 一つ.trim().match(/\.([a-zA-Z0-9_-]+)(?::[a-z-]+(\([^)]*\))?)?$/);
      if (末尾) 出.push(末尾[1]);
    }
  }
  return 出;
}

/** 印（markup）の中で、そのクラスを持つのに `title=` の無いタグを、行番号つきで返す。 */
export function 題の無い要素(文: string, クラス: string[]): string[] {
  const 出: string[] = [];
  const 印 = 文.replace(/<style[^>]*>[\s\S]*?<\/style>/, '').replace(/<!--[\s\S]*?-->/g, ' ');
  for (const m of 印.matchAll(/<([a-zA-Z][\w-]*)\b([^>]*)>/g)) {
    const [全体, タグ, 属性] = m;
    const クラス属性 = 属性.match(/\bclass\s*=\s*(?:"([^"]*)"|'([^']*)'|\{[^}]*\})/);
    if (!クラス属性) continue;
    const 持つ = (クラス属性[1] ?? クラス属性[2] ?? クラス属性[0]).split(/[\s"'{}`:?]+/);
    if (!クラス.some((c) => 持つ.includes(c))) continue;
    if (/\btitle\s*=/.test(属性)) continue;
    const 行 = 文.slice(0, 文.indexOf(全体, m.index ?? 0)).split('\n').length;
    出.push(`${行}: <${タグ} class=…${クラス.filter((c) => 持つ.includes(c)).join(',')}…>`);
  }
  return 出;
}

describe('省略する字', () => {
  it('text-overflow: ellipsis を当てた要素には、全文を出す title がある', () => {
    const 無い: string[] = [];
    for (const 道 of 集める(根)) {
      const 文 = readFileSync(道, 'utf-8');
      const クラス = 省略するクラス(文);
      if (クラス.length === 0) continue;
      for (const 行 of 題の無い要素(文, クラス)) 無い.push(`${道.replace(根, '')}:${行}`);
    }
    expect(無い, `**「…」で切れた字を、人が取り戻せない**（title が無い）:\n${無い.join('\n')}`).toEqual([]);
  });

  it('選択子の末尾のクラスを拾う（擬似クラスつき・複数）', () => {
    expect(省略するクラス('<style>.row .name, .x:hover { text-overflow: ellipsis; } .y { color: red }</style>')).toEqual(['name', 'x']);
  });

  it('title の無いタグだけを数える', () => {
    const 文 = '<span class="name" title={a}>{a}</span>\n<span class="name">{b}</span>\n<style>.name{text-overflow:ellipsis}</style>';
    expect(題の無い要素(文, ['name'])).toEqual(['2: <span class=…name…>']);
  });
});
