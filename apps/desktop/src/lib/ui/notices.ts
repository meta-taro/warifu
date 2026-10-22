// **上の帯の知らせを、閉じられるようにする**（0.1.36）。
//
// 帯は消す口が無く、2 段取ったまま画面を狭めていた。
// **×で閉じる。閉じた分は左の「知らせ」からあとで見られる**（見落としても取り戻せる）。
// 同じ知らせがもう一度起きたら、また帯に出す（新しく起きたことは見せる）。

export interface 知らせの行 {
  文: string;
  /** `HH:MM`。 */
  時刻: string;
}

/** 覚えておく数。古いものから捨てる。 */
const 覚える数 = 50;

export function 記録に足す(記録: readonly 知らせの行[], 文: string, 時刻: string): 知らせの行[] {
  if (文 === '') return 記録 as 知らせの行[];
  if (記録.at(-1)?.文 === 文) return 記録 as 知らせの行[];
  return [...記録, { 文, 時刻 }].slice(-覚える数);
}

export function 閉じる(閉じた: ReadonlySet<string>, 文: string): Set<string> {
  return new Set([...閉じた, 文]);
}

/** その知らせがもう一度起きたとき（帯にまた出す）。 */
export function 閉じたから外す(閉じた: ReadonlySet<string>, 文: string): Set<string> {
  const 次 = new Set(閉じた);
  次.delete(文);
  return 次;
}

export function 出すか(文: string, 閉じた: ReadonlySet<string>): boolean {
  return 文 !== '' && !閉じた.has(文);
}
