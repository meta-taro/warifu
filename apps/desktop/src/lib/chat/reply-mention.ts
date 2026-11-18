// **返信したら、返信先の書き手を呼ぶ**（0.1.52）。
//
// 返信（引用）で返しても、返信先の発言を書いた人には知らせが行かなかった。
// `@` を書き足すのを忘れると、返事が届いたことに相手が気づけない。
// **返信を始めたら、打つ欄の頭に `@書き手` を入れる。**人が消せば外れる（届く先は変わらない）。

import type { 会話行 } from '$lib/meeting/announce';
import { この機械の印 } from '$lib/contacts/list';
import { エージェントの札 } from './name';
import { 呼べる名に広げる, 宛ての名 } from './mention';

/** **返信先の行の書き手を、`@` で呼ぶ名。**呼べない（自分・知らせ・名前の無い人）なら `null`。 */
export function 返信で呼ぶ名(
  行: 会話行,
  名簿: Readonly<Record<string, string>>,
  名無し: string,
): string | null {
  if (行.mine || 行.system) return null;
  const 種 = 行.顔の種;
  if (!種) return null;
  // **この機械のエージェント**は、種がフォルダの名（`desk:◯◯ のエージェント`）なので、行の名札で呼ぶ
  if (種.startsWith(この機械の印)) return 行.who.trim() || null;
  const 札 = エージェントの札(行);
  if (札) return 札;
  const 引いた = 名簿[種];
  // **名前の無い人は呼ばない。**鍵の頭や「名無しさん」では、相手の機械が自分への呼びと読めない
  return 引いた && 引いた !== 名無し ? 引いた : null;
}

/** 下書きの頭に `@名 ` を足す。**もう呼んでいれば足さない**（`足した` は外すときに使う）。 */
export function 返信の呼びを足す(下書き: string, 名: string): { 本文: string; 足した: string | null } {
  if (宛ての名(下書き, 呼べる名に広げる([名])).length > 0) return { 本文: 下書き, 足した: null };
  const 足した = `@${名} `;
  return { 本文: `${足した}${下書き}`, 足した };
}

/** 返信をやめたら、**足したまま触っていない頭だけ**を外す。人が直した頭は残す。 */
export function 返信の呼びを外す(下書き: string, 足した: string | null): string {
  if (!足した || !下書き.startsWith(足した)) return 下書き;
  return 下書き.slice(足した.length);
}
