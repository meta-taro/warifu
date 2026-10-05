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
 * 巨大な数を名乗る招待でエージェントを確保させられないよう、外枠で数え直す。
 */
export function canAdmit(current: number, capacity: number): boolean {
  return current < clampCapacity(capacity);
}

/**
 * **名簿の「主催」の札を、部屋の主催の鍵で付け直す**（2026-10-05）。
 *
 * 画面は起動したときに自分を主催として名簿に置いていた。鍵で他人の部屋に入っても
 * そのままだったので、ゲストの名簿で自分に「主催」が付いていた。
 * 主催の鍵が分からないとき（まだ部屋の一覧を読んでいない）は、手を付けない。
 */
export function 主催の札を付ける<T extends { key: string; host?: boolean }>(
  members: readonly T[],
  主催の鍵: string | null,
): T[] {
  if (!主催の鍵) return [...members];
  return members.map((m) => ({ ...m, host: m.key === 主催の鍵 }));
}

