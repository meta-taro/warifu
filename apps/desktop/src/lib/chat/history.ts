// **会話を手元に残す**（D127・0.1.44）。
//
// 0.1.43 まで、会話は画面の記憶の中にしか無く、**立ち上げ直す（版の入れ替えを含む）たびに消えていた。**
// 残すのは Rust 側（`chat_keep`・機械の鍵で封をしてファイルへ）。ここは、その前後の形だけを決める。

import type { 会話行 } from '$lib/meeting/announce';
import type { ルームの会話 } from './rooms';

/**
 * **残す形にする。**
 *
 * 画像（`blob:` の URL）は残さない —— 立ち上げ直すと指す先が無い。
 * 代わりに「画像があった」ことを残す（何も無かったように見せない）。
 */
export function 残す形(会話: ルームの会話): Record<string, 会話行[]> {
  const 残す: Record<string, 会話行[]> = {};
  for (const [ルーム, 行たち] of Object.entries(会話)) {
    残す[ルーム] = 行たち.map(画像を外す);
  }
  return 残す;
}

function 画像を外す(行: 会話行): 会話行 {
  if (行.画像 === undefined) return 行;
  const { 画像: _外す, ...残り } = 行;
  return { ...残り, 画像は残していない: true };
}

/**
 * **残した会話を、いまの会話の前に戻す。**
 *
 * 読み終わる前に届いた発言は、残した分の後ろに並ぶ（潰さない）。
 * 形の合わない行は捨てる —— 壊れた控えで画面ごと止めない。
 */
export function 戻す(残した: unknown, いま: ルームの会話): ルームの会話 {
  if (typeof 残した !== 'object' || 残した === null) return いま;
  const 戻した: Record<string, readonly 会話行[]> = { ...いま };
  for (const [ルーム, 行たち] of Object.entries(残した)) {
    if (!Array.isArray(行たち)) continue;
    const 合う = 行たち.filter(会話行の形か);
    if (合う.length === 0) continue;
    戻した[ルーム] = [...合う, ...(いま[ルーム] ?? [])];
  }
  return 戻した;
}

function 会話行の形か(行: unknown): 行 is 会話行 {
  if (typeof 行 !== 'object' || 行 === null) return false;
  const r = 行 as Record<string, unknown>;
  return typeof r.who === 'string' && typeof r.body === 'string' && typeof r.mine === 'boolean';
}

/**
 * **行に出す時刻。**今日の行は `HH:MM`、別の日の行は `M/D HH:MM`。
 *
 * 残した会話を読み返すと、昨日の 20:15 と今日の 20:15 が見分けられない。
 */
export function 時刻の札(行: 会話行, 今: Date = new Date()): string | undefined {
  if (行.時 === undefined || 行.at === undefined) return 行.at;
  const その日 = new Date(行.時);
  const 同じ日 =
    その日.getFullYear() === 今.getFullYear() &&
    その日.getMonth() === 今.getMonth() &&
    その日.getDate() === 今.getDate();
  return 同じ日 ? 行.at : `${その日.getMonth() + 1}/${その日.getDate()} ${行.at}`;
}
