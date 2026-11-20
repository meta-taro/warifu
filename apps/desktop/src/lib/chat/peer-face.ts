// **相手の機械の顔**（0.1.53）。相手が置いた人と席の顔を、画面の「顔の画」に並べる。
//
// 顔の画は `who` → 画面に出せる URL の帳で、この端末の分は `me` / `desk:<呼び方>`、
// 相手の人は機械の鍵（全桁）で引く。相手の席は「鍵 ＋ 席の名乗り（場所）」で引く —— 名札は
// 名乗り直すと変わるが、場所は変わらない（`speaker-place.svelte.ts` と同じ考え）。

import type { 場所の帳 } from './name';

/** 顔の画の帳での、相手の人か席の鍵。席が空ならその機械の人（鍵そのまま）。 */
export function 相手の顔の鍵(機械の鍵: string, 席: string): string {
  return 席 === '' ? 機械の鍵 : `${機械の鍵}\n${席}`;
}

/**
 * **その行の差出人の顔の画**（無ければ `null`）。
 *
 * - この端末の人・席（`me` / `desk:…`）と相手の人（鍵の全桁）は、そのまま引く
 * - 相手の機械の席は `鍵の頭…（札）` で届くので、話し手の場所の帳で場所を引いてから引く。
 *   場所がまだ届いていない（古い版の相手など）・頭が 2 台に当たるときは引かない（取り違えない）
 */
export function 行の顔の画(
  種: string,
  顔の画: Readonly<Record<string, string>>,
  帳: 場所の帳,
): string | null {
  if (顔の画[種]) return 顔の画[種];
  const 位置 = 種.indexOf('（');
  if (位置 <= 0 || !種.endsWith('）')) return null;
  const 頭 = 種.slice(0, 位置).replace(/…$/, '');
  const 札 = 種.slice(位置 + 1, -1);
  if (頭 === '') return null;
  const 当たり = Object.entries(帳)
    .map(([k, 場所]) => {
      const [鍵, 帳の札] = k.split('\n');
      return { 鍵, 帳の札, 場所 };
    })
    .filter((e) => e.鍵.startsWith(頭) && e.帳の札 === 札);
  const 機械 = new Set(当たり.map((e) => e.鍵));
  if (機械.size !== 1) return null;
  const { 鍵, 場所 } = 当たり[当たり.length - 1];
  return 顔の画[相手の顔の鍵(鍵, 場所)] ?? null;
}
