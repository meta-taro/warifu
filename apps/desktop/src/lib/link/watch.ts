// 経路の表示を落ち着かせる（DESIGN.md §4.1 / D29 と同じ姿勢）。
//
// `pathFromStats` はその瞬間の答えを返す。それをそのまま出すと、
// **ICE が固まるまでの間に表示が跳ねる**（一瞬 host 同士になり、次の瞬間 relay になる）。
// D29 が Governor について書いたのと同じ問題で、**見ている側には
// 「変わり続ける表示」のほうが辛い。**
//
// ただし D29 と非対称にする所がある。
//
// - **「直接」を名乗るのは待つ。**確かめてからでないと、経路の性質を偽ることになる
// - **「中継」「不明」へ落ちるのは即座に。**通信の性質が変わったことを隠さない（§2 原則 7）
//
// 落とすのは速く、上げるのはゆっくり — 向きは D29 と同じで、
// **何を「上げる」とみなすかが違う**（画質ではなく、こちらの主張の強さである）。

import type { LinkPath } from './path';

/** 「直接」を名乗るのに要る、連続した観測の回数。 */
export const PROMOTE_AFTER = 3;

export interface WatchState {
  /** 画面へ出す状態。 */
  shown: LinkPath;
  /** 直接が続いている回数。他を見たら 0 に戻る。 */
  streak: number;
}

/** **測る前は「不明」。**「たぶん繋がっている」を初期値にしない。 */
export function initialWatch(): WatchState {
  return { shown: 'unknown', streak: 0 };
}

/** 観測を 1 つ受け取って、次の表示を決める。 */
export function observe(state: WatchState, seen: LinkPath): WatchState {
  if (seen !== 'direct') {
    // 弱いほうへは待たずに動く
    return { shown: seen, streak: 0 };
  }
  if (state.shown === 'direct') {
    return state;
  }
  const streak = state.streak + 1;
  if (streak < PROMOTE_AFTER) {
    return { shown: state.shown, streak };
  }
  return { shown: 'direct', streak: 0 };
}
