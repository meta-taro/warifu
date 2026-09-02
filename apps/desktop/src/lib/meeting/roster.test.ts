import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { canAdmit, clampCapacity, DEFAULT_CAPACITY, HARD_LIMIT, MIN_CAPACITY } from './roster';

describe('会議の定員（DESIGN.md §4.3 / D27 / D15）', () => {
  it('既定は 12、外枠は 16、下は 2', () => {
    expect(DEFAULT_CAPACITY).toBe(12);
    expect(HARD_LIMIT).toBe(16);
    expect(MIN_CAPACITY).toBe(2);
  });

  it('**Rust 側の正本と一致している**', () => {
    // 数値を 2 か所に持つと必ずずれる。ずれた瞬間に落ちるようにしておく。
    // 正本は crates/warifu-meeting/src/roster.rs（送る側でも受け取る側でも数える・D15）
    const rs = readFileSync(
      new URL('../../../../../crates/warifu-meeting/src/roster.rs', import.meta.url),
      'utf8',
    );
    const read = (name: string) => {
      const m = rs.match(new RegExp(`pub const ${name}: usize = (\\d+);`));
      if (!m) throw new Error(`${name} を roster.rs から読めなかった`);
      return Number(m[1]);
    };
    expect(read('DEFAULT_CAPACITY')).toBe(DEFAULT_CAPACITY);
    expect(read('HARD_LIMIT')).toBe(HARD_LIMIT);
  });

  it('外枠を超える定員を受け取らない（招待が大きな数を名乗っても）', () => {
    expect(clampCapacity(999)).toBe(HARD_LIMIT);
    expect(clampCapacity(17)).toBe(HARD_LIMIT);
    expect(clampCapacity(16)).toBe(16);
  });

  it('1 人の会議は作れない', () => {
    expect(clampCapacity(1)).toBe(MIN_CAPACITY);
    expect(clampCapacity(0)).toBe(MIN_CAPACITY);
    expect(clampCapacity(-3)).toBe(MIN_CAPACITY);
  });

  it('読めない値は既定へ落とす（黙って外枠にしない）', () => {
    expect(clampCapacity(Number.NaN)).toBe(DEFAULT_CAPACITY);
    expect(clampCapacity(7.5)).toBe(DEFAULT_CAPACITY);
    expect(clampCapacity(Number.POSITIVE_INFINITY)).toBe(DEFAULT_CAPACITY);
  });

  it('定員に達したら入れない', () => {
    expect(canAdmit(6, 12)).toBe(true);
    expect(canAdmit(11, 12)).toBe(true);
    expect(canAdmit(12, 12)).toBe(false);
    expect(canAdmit(13, 12)).toBe(false);
  });

  it('定員そのものが外枠を超えていたら、外枠で数える', () => {
    // 招待に書かれた定員をそのまま信じない（D27）
    expect(canAdmit(16, 999)).toBe(false);
  });
});
