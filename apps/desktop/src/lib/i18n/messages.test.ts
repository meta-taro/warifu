import { describe, expect, it } from 'vitest';
import { LOCALES } from './locales';
import { CRITICAL_KEYS, MESSAGES, TRANSLATOR_NOTES } from './messages';

const keysOf = (locale: (typeof LOCALES)[number]) => Object.keys(MESSAGES[locale]).sort();

describe('文言辞書（DESIGN.md §9 / D35）', () => {
  it('4 言語すべてが同じ鍵を持つ。欠けた言語を出さない', () => {
    const base = keysOf('en');
    expect(base.length).toBeGreaterThan(0);
    for (const locale of LOCALES) {
      expect(keysOf(locale)).toEqual(base);
    }
  });

  it('空の訳文を持たない', () => {
    for (const locale of LOCALES) {
      for (const [key, value] of Object.entries(MESSAGES[locale])) {
        expect(value.trim(), `${locale}.${key} が空`).not.toBe('');
      }
    }
  });

  it('割符・宛先の生値を文言に埋め込まない（D35）', () => {
    // base32 は同じバイト列に複数の表記を許さない。言語ごとに見た目が変わると
    // 紙・口頭で伝える経路が壊れる。生値は差し込み口（{tally}）で渡す。
    for (const locale of LOCALES) {
      for (const [key, value] of Object.entries(MESSAGES[locale])) {
        expect(value, `${locale}.${key} に生値が埋まっている`).not.toContain('WARIFU1-');
      }
    }
  });

  it('誤訳が事故になる文言には、翻訳者への注記が必ず付く', () => {
    expect(CRITICAL_KEYS.length).toBeGreaterThan(0);
    for (const key of CRITICAL_KEYS) {
      expect(TRANSLATOR_NOTES[key], `${key} に注記が無い`).toBeTruthy();
    }
  });

  it('失効と戸口の文言が、事故になる側に入っている', () => {
    expect(CRITICAL_KEYS).toContain('revoke.irreversible');
    expect(CRITICAL_KEYS).toContain('door.refused');
  });

  it('相手が帰ったのと落ちたのを、同じ文言にしない', () => {
    // 人はこの 2 つで次の手が変わる。帰ったなら会議は終わり、
    // 落ちたなら**会議キーを作り直して渡し直す**（割符は一度きり・D12）。
    // CLI 側は 2026-09-04 に分けた（`相手が帰りました` / `相手が落ちました`）。
    for (const locale of LOCALES) {
      expect(MESSAGES[locale]['link.closed'], `${locale} で分かれていない`).not.toBe(
        MESSAGES[locale]['link.lost'],
      );
      expect(MESSAGES[locale]['link.lost'].trim(), `${locale}.link.lost が空`).not.toBe('');
    }
  });

  it('注記は辞書にある鍵にしか付けられない', () => {
    const base = keysOf('en');
    for (const key of Object.keys(TRANSLATOR_NOTES)) {
      expect(base, `${key} は辞書に無い`).toContain(key);
    }
  });
});
