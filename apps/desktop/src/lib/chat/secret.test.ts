import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { 秘密らしいもの } from './secret';

/**
 * 送る前に見る決まりが、本体（`crates/warifu-desk/src/secret.rs`）と同じか。
 * **同じ例文の表で試す**ので、片方だけ変えればここが落ちる。
 */
const 表 = readFileSync(
  new URL('../../../../../crates/warifu-desk/tests/secret-cases.tsv', import.meta.url),
  'utf-8',
);

/** `{hex64}` などを展開する（本体の試験と同じ字の並び）。 */
function 展開(本文: string): string {
  const 字: Record<string, string> = {
    hex: '0f1e2d3c4b5a6978',
    alnum: 'aZ3kQ9mX7pL2',
    upper: 'QZ7XK2MW9RT4',
    b32: 'KQ2ZT7MXR4WP',
  };
  return 本文.replace(/\{(hex|alnum|upper|b32)_?(\d+)\}/g, (_, 種, 長さ: string) =>
    字[種].repeat(Math.ceil(Number(長さ) / 字[種].length)).slice(0, Number(長さ)),
  );
}

const 行たち = 表
  .split('\n')
  .filter((l) => l.trim() !== '' && !l.startsWith('#'))
  .map((l) => {
    const [判定, 種類, ...残り] = l.split('\t');
    return { 判定, 種類, 本文: 展開(残り.join('\t')) };
  });

describe('秘密らしいもの', () => {
  it('例文の表を読めている（読めなければ空回りする）', () => {
    expect(行たち.filter((r) => r.判定 === '止める').length).toBeGreaterThanOrEqual(10);
    expect(行たち.filter((r) => r.判定 === '通す').length).toBeGreaterThanOrEqual(8);
  });

  it.each(行たち)('$判定 $種類: $本文', ({ 判定, 種類, 本文 }) => {
    const 見つけた = 秘密らしいもの(本文);
    if (判定 === '止める') expect(見つけた).toContain(種類);
    else expect(見つけた).toEqual([]);
  });
});
