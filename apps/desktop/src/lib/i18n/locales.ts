// 対応ロケールの定義（DOM 非依存の純データ層・DESIGN.md §9 / D35）。
//
// 反応状態と永続化はここに置かない。この層は「どの言語に対応しているか」だけを知る。

/** アプリが対応する UI 言語。英・日・中（簡体）・韓。 */
export type Locale = 'en' | 'ja' | 'zh' | 'ko';

/** 選択肢の並び。**固定**（DESIGN.md §9）。 */
export const LOCALES: readonly Locale[] = ['en', 'ja', 'zh', 'ko'] as const;

/** 各ロケールの自言語表記。選択肢に出すラベル。 */
export const LOCALE_LABELS: Record<Locale, string> = {
  en: 'English',
  ja: '日本語',
  zh: '中文',
  ko: '한국어',
};

/** 対応外の値が入ってこないことを型で保証する。 */
export function isLocale(value: unknown): value is Locale {
  return typeof value === 'string' && (LOCALES as readonly string[]).includes(value);
}

/**
 * OS から渡された言語の候補列から 1 つ選ぶ。
 *
 * 候補は `ja-JP` や `zh-Hans-CN` のように地域や表記体系が付く。
 * **前から順に見て、最初に一致したものを採る** — 並び自体が利用者の優先順である。
 * 対応するものが 1 つも無ければ `en` へ落とす（無言で日本語にしない）。
 */
export function resolveLocale(candidates: readonly string[]): Locale {
  for (const candidate of candidates) {
    const primary = candidate.split('-')[0]?.toLowerCase();
    if (isLocale(primary)) return primary;
  }
  return 'en';
}
