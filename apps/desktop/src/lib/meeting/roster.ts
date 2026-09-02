// 会議の定員（DESIGN.md §4.3 / D27 / D15）。
//
// **正本は Rust 側**（`crates/warifu-meeting/src/roster.rs`）である。
// ここに同じ数値を持つのは、画面が入力を弾くために要るからで、
// **ずれたら落ちるようにテストで縛ってある**（roster.test.ts）。
//
// D15 の約束は「送る側でも受け取る側でも数える」。画面で弾いても、
// 名簿を受け取る側は別に数える。ここを通ったから安全、ということにしない。

/** 1 人の会議は作れない。会議として成立しない。 */
export const MIN_CAPACITY = 2;

/** 定員を指定しないときの既定。**上限ではない。** */
export const DEFAULT_CAPACITY = 12;

/** フルメッシュとして成立しうる外枠。**これを超えるには中継が要り、D7 が未決。** */
export const HARD_LIMIT = 16;

/**
 * 入力された定員を、受け取れる範囲へ収める。
 *
 * **読めない値は既定へ落とす。**黙って外枠にしない —
 * 壊れた入力が「一番大きい定員」になるのは、外枠を置いた意味を失う。
 */
export function clampCapacity(value: number): number {
  if (!Number.isInteger(value)) return DEFAULT_CAPACITY;
  if (value < MIN_CAPACITY) return MIN_CAPACITY;
  if (value > HARD_LIMIT) return HARD_LIMIT;
  return value;
}

/**
 * まだ入れるか。
 *
 * **招待に書かれた定員をそのまま信じない**（D27）。
 * 巨大な数を名乗る招待で席を確保させられないよう、外枠で数え直す。
 */
export function canAdmit(current: number, capacity: number): boolean {
  return current < clampCapacity(capacity);
}
