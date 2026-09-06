import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { LOCALES } from './locales';
import { MESSAGES } from './messages';

/**
 * 訳文レビューのシート（`docs/i18n-review.tsv`）が、辞書とずれていないか。
 *
 * **鍵を足してシートを作り直さないと、レビューの穴が見えないまま残る。**
 * 訳文レビューは人にしかできない工程なので（baseline §19 / §29）、
 * **穴が見えないのが一番まずい。**
 *
 * 作り直し方: `python3 scripts/i18n-review.py`
 */
const SHEET = new URL('../../../../../docs/i18n-review.tsv', import.meta.url);

describe('訳文レビューのシート', () => {
  const 行 = readFileSync(SHEET, 'utf-8').trimEnd().split('\n');
  const 頭 = 行[0].split('\t');
  const 鍵 = 行.slice(1).map((l) => l.split('\t')[0]);

  it('辞書と同じ鍵を、過不足なく持つ', () => {
    // **足りない = レビューされない。余る = 消した鍵をレビューさせる**
    expect(鍵.slice().sort()).toEqual(Object.keys(MESSAGES.en).sort());
  });

  it('4 言語ぶんの欄がある', () => {
    for (const locale of LOCALES) {
      expect(頭, `${locale} の欄が無い`).toContain(locale);
    }
  });

  it('判定の欄を、AI が埋めていない', () => {
    // **実物を見た人が記入する**（baseline §19）。
    // 空のまま出す —— `—` や `N/A` や `TBD` で埋めない
    const 判定 = 頭.indexOf('判定');
    expect(判定).toBeGreaterThan(0);
    for (const l of 行.slice(1)) {
      expect(l.split('\t')[判定] ?? '', `${l.split('\t')[0]} の判定が埋まっている`).toBe('');
    }
  });
});
