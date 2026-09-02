// 自作タイトルバーの純ロジック（DESIGN.md §8 / D34）。
//
// ここには副作用を置かない。Tauri の窓 API を叩く層は別に置き、
// **ブラウザで開いたときは no-op** になるようにする（この層はブラウザでも動く）。

import type { MessageKey } from '../i18n/messages';

/** 帯の高さ（px）。DESIGN.md §8 の `--topbar-h`。 */
export const TITLEBAR_HEIGHT = 44;

export type ControlId = 'minimize' | 'maximize' | 'close';

export interface WindowControl {
  id: ControlId;
  /** 帯に出す字。アイコンフォントを足さずに済ませる。 */
  glyph: string;
  /** 読み上げとツールチップ用。ラベルが消えてもここは残る（DESIGN.md §9）。 */
  labelKey: MessageKey;
  /** 危険な操作。閉じるだけがホバーで `--danger-fg` になる。 */
  danger?: true;
}

/**
 * 窓の操作を、今の最大化状態から組み立てる。
 *
 * **3 つだけ**で、順は 最小化 → 最大化/復元 → 閉じる。
 * 最大化ボタンは状態で字と読み上げが入れ替わる（押した結果を名乗る）。
 */
export function controlsFor(maximized: boolean): WindowControl[] {
  return [
    { id: 'minimize', glyph: '─', labelKey: 'window.minimize' },
    maximized
      ? { id: 'maximize', glyph: '❐', labelKey: 'window.restore' }
      : { id: 'maximize', glyph: '▢', labelKey: 'window.maximize' },
    { id: 'close', glyph: '✕', labelKey: 'window.close', danger: true },
  ];
}
