// タイトルバーの副作用層（DESIGN.md §8 / D34）。
//
// **ブラウザで開いたときは何もしない。**Tauri の窓 API はブラウザに無く、
// 落ちると「画面が真っ白」という一番読みにくい壊れ方になる。
// 純ロジックは titlebar.ts 側にあり、そちらはブラウザでも動く（テスト対象）。
//
// ファイル名を titlebar.svelte.ts にしない。macOS の大文字小文字を区別しない FS では
// `./titlebar.svelte` が TitleBar.svelte に解決され、自分自身を import する形になる。

import type { ControlId } from './titlebar';

/** Tauri の中で動いているか。ブラウザでは false。 */
export function inTauri(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

/** 窓の操作を実行する。Tauri の外では no-op（例外を投げない）。 */
export async function runControl(id: ControlId): Promise<void> {
  if (!inTauri()) return;
  const { getCurrentWindow } = await import('@tauri-apps/api/window');
  const w = getCurrentWindow();
  if (id === 'minimize') return w.minimize();
  if (id === 'maximize') return w.toggleMaximize();
  return w.close();
}

/** 今 最大化されているか。Tauri の外では常に false。 */
export async function isMaximized(): Promise<boolean> {
  if (!inTauri()) return false;
  const { getCurrentWindow } = await import('@tauri-apps/api/window');
  return getCurrentWindow().isMaximized();
}
