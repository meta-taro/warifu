// **続けて送られた画像を、1 つの塊にまとめる**（0.1.62）。
//
// 画像を何枚も続けて送ると、1 枚ずつ名前と時刻の行が付いて縦に並び、会話が縦に長くなっていた。
// **同じ差出人が続けて送った画像だけの行**を、直前の行（画像の行か、文字の行）の塊に入れ、
// 画面では横に並べる。網の形は変えない（行は 1 枚ずつのまま届く）—— まとめるのは見せ方だけ。
//
// まとめるのは次のときだけ：
// - 同じ差出人（名前・この機械からか・エージェントか・顔の種が同じ）
// - 間に別の行が無い（会議からの知らせも、別の人の行も挟まない）
// - 前の行から数分以内
// - 続く行は**画像だけ**（本文が空・画像がある・返信でない）。文字の行が来たら塊は切れる

import type { 会話行 } from '$lib/meeting/announce';

/** 前の行からこれより間が空いたら、まとめない（ミリ秒）。 */
export const まとめる間 = 3 * 60 * 1000;

/** まとめ方。`頭` は塊の頭の行の位置 → 続く行の位置たち。`続き` は頭の後ろに描く行の位置。 */
export interface 画像のまとめ {
  頭: Map<number, number[]>;
  続き: Set<number>;
}

/** 塊に続けられる「画像だけの行」か。 */
export function 画像だけの行か(行: 会話行): boolean {
  return !行.system && 行.画像 !== undefined && 行.body.trim() === '' && 行.返信先 === undefined;
}

function 同じ差出人か(a: 会話行, b: 会話行): boolean {
  return a.who === b.who && a.mine === b.mine && a.agent === b.agent && a.顔の種 === b.顔の種;
}

/** 時刻が近いか。ミリ秒が両方にあればそれで、無ければ `HH:MM` が同じときだけ近いとみる。 */
function 近いか(前: 会話行, 後: 会話行, 間: number): boolean {
  if (前.時 !== undefined && 後.時 !== undefined) return Math.abs(後.時 - 前.時) <= 間;
  return 前.at !== undefined && 前.at === 後.at;
}

/** 塊の頭になれる行か（会議からの知らせ・残していない画像の行は頭にしない）。 */
function 頭になれるか(行: 会話行): boolean {
  return !行.system && !行.画像は残していない;
}

/**
 * **並んだ行から、画像の塊を探す。**位置は渡した配列の中の位置。
 *
 * 1 枚だけの行（続きの無い頭）は `頭` に入れない —— 今の見た目のまま描く。
 */
export function 画像をまとめる(行たち: readonly 会話行[], 間 = まとめる間): 画像のまとめ {
  const 頭 = new Map<number, number[]>();
  const 続き = new Set<number>();
  let いまの頭: number | null = null;
  for (let i = 0; i < 行たち.length; i++) {
    const 行 = 行たち[i];
    const 前 = i > 0 ? 行たち[i - 1] : undefined;
    if (
      いまの頭 !== null &&
      前 !== undefined &&
      画像だけの行か(行) &&
      同じ差出人か(行たち[いまの頭], 行) &&
      近いか(前, 行, 間)
    ) {
      const 並び = 頭.get(いまの頭) ?? [];
      並び.push(i);
      頭.set(いまの頭, 並び);
      続き.add(i);
      continue;
    }
    いまの頭 = 頭になれるか(行) ? i : null;
  }
  return { 頭, 続き };
}
