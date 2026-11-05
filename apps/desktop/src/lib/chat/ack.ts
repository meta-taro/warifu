// **メンションした行への「受けた」**（D130・0.1.45）。
//
// 既読ではない（D94 はそのまま）。人の画面に出ただけでは返らない ——
// 返るのは、人が［受けました］を押した・返事を書いた・エージェントが読んだ、のどれかだけ。

import type { 会話行 } from '$lib/meeting/announce';
import { 宛てられたか } from './mention';
import type { ルームの会話 } from './rooms';

/** 受けないまま、送り手に知らせるまでの間（分）。 */
export const 知らせるまで = 30;

/**
 * **届いた「受けた」を、自分が呼んだ行に足す。**同じ相手の 2 度目は足さない。
 * その部屋に印の合う自分の行が無ければ、会話はそのまま返す。
 */
export function 受けたを足す(
  会話: ルームの会話,
  部屋: string,
  印: string,
  誰: string,
  方: string,
  今: number,
): ルームの会話 {
  const 行たち = 会話[部屋];
  if (!行たち) return 会話;
  let 変えた = false;
  const 次 = 行たち.map((行) => {
    if (!行.mine || 行.印 !== 印) return 行;
    if (行.受け?.some((u) => u.誰 === 誰)) return 行;
    変えた = true;
    return { ...行, 受け: [...(行.受け ?? []), { 誰, 方, 時: 今 }] };
  });
  return 変えた ? { ...会話, [部屋]: 次 } : 会話;
}

/** 自分が呼ばれた行を、受けた印にする（押した・返事を書いた）。 */
export function 受けたことにする(会話: ルームの会話, 部屋: string, 印: string): ルームの会話 {
  const 行たち = 会話[部屋];
  if (!行たち) return 会話;
  return { ...会話, [部屋]: 行たち.map((行) => (行.印 === 印 ? { ...行, 受けた: true as const } : 行)) };
}

/**
 * **その部屋で返事を書いたので、自分が呼ばれていた行を受けた印にする。**
 * 本体（Rust）が同じ行に「受けた（返事）」を返している。
 */
export function 返事で受けたことにする(
  会話: ルームの会話,
  部屋: string,
  呼ばれる名: readonly string[],
): ルームの会話 {
  const 行たち = 会話[部屋];
  if (!行たち) return 会話;
  return {
    ...会話,
    [部屋]: 行たち.map((行) =>
      !行.mine && !行.受けた && 行.印 !== undefined && 宛てられたか(行.body, 呼ばれる名)
        ? { ...行, 受けた: true as const }
        : 行,
    ),
  };
}

/** ［受けました］を出す行か —— 相手の発言で、自分が呼ばれていて、まだ受けていない。 */
export function 受ける押し所を出すか(行: 会話行, 呼ばれた: boolean): boolean {
  return 呼ばれた && !行.mine && !行.system && !行.受けた && 行.印 !== undefined;
}

/** 呼んだ行の横に出す様子。 */
export type 受けの様子 =
  | { 種: '受けた'; 誰たち: readonly { 誰: string; 方: string; 時: number }[] }
  | { 種: 'まだ'; 分: number; 知らせる: boolean };

/**
 * **自分が呼んだ行の、いまの様子。**呼んでいない行・印の無い行は `null`（何も出さない）。
 *
 * 1 人でも受けていれば「受けた」と、受けた人を並べる（呼んだ全員か 1 人かは、名前で読める）。
 */
export function 受けを見る(行: 会話行, 呼んだ名: readonly string[], 今: number): 受けの様子 | null {
  if (!行.mine || 行.印 === undefined || 呼んだ名.length === 0) return null;
  if (行.受け && 行.受け.length > 0) return { 種: '受けた', 誰たち: 行.受け };
  if (行.時 === undefined) return null;
  const 分 = Math.max(0, Math.floor((今 - 行.時) / 60_000));
  return { 種: 'まだ', 分, 知らせる: 分 >= 知らせるまで };
}
