import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { LOCALES } from './locales';
import { CRITICAL_KEYS, MESSAGES, TRANSLATOR_NOTES, type MessageKey } from './messages';

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

  it('4 言語の訳文は、辞書の値そのまま', () => {
    // 作り直しの道具が `\'` を戻し忘れると、シートにだけ `This PC\'s` が出ていた（0.1.42 で直した）
    for (const locale of LOCALES) {
      const 欄 = 頭.indexOf(locale);
      const 違う = 行.slice(1).filter((l) => {
        const 欄たち = l.split('\t');
        return 欄たち[欄] !== MESSAGES[locale][欄たち[0] as MessageKey].replace(/\t/g, ' ');
      });
      expect(違う.map((l) => `${locale}.${l.split('\t')[0]}`)).toEqual([]);
    }
  });

  it('事故の印と翻訳者への注記は、辞書（CRITICAL_KEYS・TRANSLATOR_NOTES）と同じ', () => {
    // **元は辞書。**シートにだけ書いた注記は、作り直すと消える（0.1.42 で 13 件・★ 4 件を辞書へ移した）
    const 印 = 頭.indexOf('事故になるか');
    const 注 = 頭.indexOf('翻訳者への注記');
    const 違う: string[] = [];
    for (const l of 行.slice(1)) {
      const 欄たち = l.split('\t');
      const 鍵 = 欄たち[0] as MessageKey;
      const 期待の印 = CRITICAL_KEYS.includes(鍵) ? '★' : '';
      const 期待の注 = (TRANSLATOR_NOTES[鍵] ?? '').replace(/\t/g, ' ');
      if ((欄たち[印] ?? '') !== 期待の印) 違う.push(`${鍵} の印`);
      if ((欄たち[注] ?? '') !== 期待の注) 違う.push(`${鍵} の注記`);
    }
    expect(違う).toEqual([]);
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
