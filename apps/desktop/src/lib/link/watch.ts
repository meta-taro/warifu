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

/**
 * **読めないのを何回まで見逃すか。**
 *
 * 2026-09-13、3 台で実測して分かった —— **`direct` ⇄ `unknown` が
 * 6 秒ごとに行き来していた。**
 *
 * `unknown` は「**通信の性質が変わった**」ではなく「**今回は読めなかった**」である
 * （`pathFromStats` は、成立している組が拾えないだけで `unknown` を返す）。
 * **1 回で取り下げると、戻すのに `PROMOTE_AFTER` 回かかるので揺れ続ける。**
 *
 * **続けて読めなければ、そのときは「不明」と言う**（分からないものを
 * 分かるように見せない・DESIGN §2 原則 7）。
 */
export const 読めないを許す回数 = 3;

export interface WatchState {
  /** 画面へ出す状態。 */
  shown: LinkPath;
  /** 直接が続いている回数。他を見たら 0 に戻る。 */
  streak: number;
  /** **続けて読めなかった回数。**読めたら 0 に戻る。 */
  読めない: number;
}

/** **測る前は「不明」。**「たぶん繋がっている」を初期値にしない。 */
export function initialWatch(): WatchState {
  return { shown: 'unknown', streak: 0, 読めない: 0 };
}

/** 観測を 1 つ受け取って、次の表示を決める。 */
export function observe(state: WatchState, seen: LinkPath): WatchState {
  // **中継は待たない。**読めなかったのではなく、**性質が変わった**から
  if (seen === 'relayed') {
    return { shown: 'relayed', streak: 0, 読めない: 0 };
  }
  if (seen === 'unknown') {
    const 読めない = state.読めない + 1;
    // **名乗る前なら、数え直すだけ**（取り下げる物がない）
    if (state.shown !== 'direct') {
      return { shown: state.shown, streak: 0, 読めない };
    }
    // **続けて読めなければ、そのときは「不明」と言う**
    if (読めない >= 読めないを許す回数) {
      return { shown: 'unknown', streak: 0, 読めない };
    }
    return { shown: state.shown, streak: state.streak, 読めない };
  }
  // ここから `direct`
  if (state.shown === 'direct') {
    return { ...state, 読めない: 0 };
  }
  const streak = state.streak + 1;
  if (streak < PROMOTE_AFTER) {
    return { shown: state.shown, streak, 読めない: 0 };
  }
  return { shown: 'direct', streak: 0, 読めない: 0 };
}
