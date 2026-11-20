/**
 * **書く欄の高さを、中身に合わせて伸ばす**（2026-11-20）。
 *
 * 長い文を書くとき、1〜2 行の高さのままでは前後が見えない。中身の高さ（`scrollHeight`）まで
 * 伸ばし、窓の高さの 4 割を超えたら、そこで止めて欄の中で巻く（会話の行を押し出しきらない）。
 */
export const 欄の最小 = 34;

/** 窓の高さに対する上限の割合。 */
export const 欄の上限の割合 = 0.4;

/** 中身の高さと窓の高さから、欄の高さ（px）を決める。上限が最小を下回る小さな窓でも最小は守る。 */
export function 欄の高さ(中身: number, 窓: number): number {
  const 上限 = Math.max(欄の最小, Math.floor(窓 * 欄の上限の割合));
  return Math.min(Math.max(中身, 欄の最小), 上限);
}

/** 上限で止めたか（止めたときだけ、欄の中で巻けるようにする）。 */
export function 上限で止めたか(中身: number, 窓: number): boolean {
  return 中身 > 欄の高さ(中身, 窓);
}
