import { describe, expect, it } from 'vitest';
import { isLocale, LOCALE_LABELS, LOCALES, resolveLocale } from './locales';

describe('対応ロケール（DESIGN.md §9 / D35）', () => {
  it('en / ja / zh / ko の 4 つで、並びが固定されている', () => {
    expect(LOCALES).toEqual(['en', 'ja', 'zh', 'ko']);
  });

  it('ラベルは自言語表記', () => {
    expect(LOCALE_LABELS).toEqual({
      en: 'English',
      ja: '日本語',
      zh: '中文',
      ko: '한국어',
    });
  });

  it('対応していない値を受け取らない', () => {
    expect(isLocale('ja')).toBe(true);
    expect(isLocale('fr')).toBe(false);
    expect(isLocale('')).toBe(false);
    expect(isLocale(null)).toBe(false);
    expect(isLocale(['ja'])).toBe(false);
  });

  it('OS の言語から選ぶ。地域付きも拾う', () => {
    expect(resolveLocale(['ja-JP', 'en-US'])).toBe('ja');
    expect(resolveLocale(['zh-Hans-CN'])).toBe('zh');
    expect(resolveLocale(['ko'])).toBe('ko');
  });

  it('対応していない言語しか無ければ en へ落とす', () => {
    expect(resolveLocale(['fr-FR', 'de'])).toBe('en');
    expect(resolveLocale([])).toBe('en');
  });

  it('前から順に見る。最初に一致したものを採る', () => {
    expect(resolveLocale(['fr', 'ko', 'ja'])).toBe('ko');
  });
});
