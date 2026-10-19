// 左右の仕切り（右の列の幅を、人が動かす）。
//
// **会話の欄が細くて読みにくい**ので、右の列を人が広げられるようにする。
// 戻し方は 1 つ ——仕切りをダブルクリックすると、初めの幅に戻る。
//
// 幅は**右の列の幅**で持つ。窓を広げたとき、増えたぶんは左（主役の側）へ行く。

/** 右の列の幅の下限。**これより細いと、会話の行が 1 語ずつ折れる** */
export const 右の下限 = 260;
/** 左に必ず残す幅の既定。**左が潰れると、何を選んでいるかが見えなくなる**（面ごとに変えられる） */
export const 左に残す = 420;
/** 矢印キー 1 回で動く幅 */
export const 一歩 = 16;

/** 幅を、下限と（いまの枠で取れる）上限の間に収める。 */
export function 幅を収める(幅: number, 枠の幅: number, 残す = 左に残す): number {
  const 上限 = Math.max(右の下限, 枠の幅 - 残す);
  return Math.round(Math.min(上限, Math.max(右の下限, 幅)));
}

/**
 * ドラッグ中の幅。**仕切りを左へ動かすと、右の列は広がる。**
 */
export function 引いた幅(
  始めの幅: number,
  始めの位置: number,
  いまの位置: number,
  枠の幅: number,
  残す = 左に残す,
): number {
  return 幅を収める(始めの幅 + (始めの位置 - いまの位置), 枠の幅, 残す);
}

/** 覚えておいた幅を読む。**読めなければ無かったことにする**（初めの幅で出す）。 */
export function 覚えた幅を読む(書いたもの: string | null): number | null {
  if (書いたもの === null) return null;
  const 数 = Number(書いたもの);
  return Number.isFinite(数) && 数 >= 右の下限 ? Math.round(数) : null;
}
