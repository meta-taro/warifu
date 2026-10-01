// **更新があるかを決める所**（**D81**）。
//
// ここには Tauri を持ち込まない。**版を比べるだけ**にしてあるので、
// 画面を立てずに試験できる。
//
// **不明なら「更新なし」にする。**読めない版を「新しい」と読むと、
// **在りもしない更新を出し続ける**ことになる（人は毎回押して、毎回何も起きない）。

/** 出せる更新。**画面に出すのはこれだけ。** */
export interface 新しい版 {
  /** 向こうの版（`0.1.16` など）。 */
  版: string;
  /** いま動いている版。 */
  いまの版: string;
  /** **何が変わったか**（release notes）。空なら出さない。 */
  中身: string;
  /** いつ出たか。分からなければ `null`。 */
  いつ: string | null;
}

/** 頭の `v` を落とす。`v0.1.0` と `0.1.0` を同じに扱う。 */
export function 版をそろえる(生: string): string {
  return 生.trim().replace(/^[vV]/, '');
}

interface 三つ組 {
  大: number;
  中: number;
  小: number;
}

/**
 * `v0.1.0` / `0.1` / `1` を数に開く。足りない所は 0 で埋め、
 * `-alpha.15` や `+build` は落とす。
 *
 * **数字として読めなければ `null`。**既定に落とさない ——
 * 落とすと、壊れた札が「新しい版」に化ける。
 */
export function 版を読む(生: string): 三つ組 | null {
  const 芯 = 版をそろえる(生).split(/[-+]/)[0];
  const 部 = 芯.split('.');
  const [大, 中, 小] = [部[0], 部[1] ?? '0', 部[2] ?? '0'];
  const 数字 = /^\d+$/;
  if (![大, 中, 小].every((x) => 数字.test(x))) return null;
  return { 大: Number(大), 中: Number(中), 小: Number(小) };
}

/**
 * 向こうがこちらより**新しいか**。
 *
 * **どちらかが読めなければ `false`。**「同じ」でも `false`。
 * 更新の口は**押せば何かが起きる**所なので、
 * 起きないものを出してはいけない（D49 と同じ構え）。
 */
export function 新しいか(いま: string, 向こう: string): boolean {
  const a = 版を読む(いま);
  const b = 版を読む(向こう);
  if (!a || !b) return false;
  if (b.大 !== a.大) return b.大 > a.大;
  if (b.中 !== a.中) return b.中 > a.中;
  return b.小 > a.小;
}

/** 落とした量を、人が読める割合にする。**総量が分からなければ `null`。** */
export function 進み具合(落とした: number, 全部: number | null): number | null {
  if (!全部 || 全部 <= 0) return null;
  return Math.min(100, Math.max(0, Math.round((落とした / 全部) * 100)));
}
