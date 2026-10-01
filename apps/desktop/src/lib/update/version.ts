// 更新の判定（DESIGN.md §10 / D36）。
//
// **署名が確かめられていない更新は「ある」ことにしない。**
// 更新経路は E2EE の外側にあり、ここが開いていると経路の暗号も戸口も判定も素通りする。
// だから「署名を省ける形」をこの層に作らない — `signatureVerified` は必須の引数である。

export type UpdateState = { kind: 'none' } | { kind: 'available'; version: string };

export interface UpdateLookup {
  /** 今動いている版。 */
  current: string;
  /** 配布元が名乗った版。見つからなければ null。 */
  found: string | null;
  /** **minisign の検証が通ったか。**省略できない。 */
  signatureVerified: boolean;
}

/** `1.10.0` > `1.9.0`。文字列順では比べない。 */
export function compareVersions(a: string, b: string): -1 | 0 | 1 {
  const pa = parseVersion(a);
  const pb = parseVersion(b);
  if (pa === null || pb === null) return 0;

  const length = Math.max(pa.length, pb.length);
  for (let i = 0; i < length; i++) {
    // 桁数が違っても比べられるように、無い桁は 0 として扱う（`1.2` と `1.2.0` は同じ）
    const left = pa[i] ?? 0;
    const right = pb[i] ?? 0;
    if (left > right) return 1;
    if (left < right) return -1;
  }
  return 0;
}

/** **読めない版は新しいと判定しない。**分からないものを新しい側へ倒さない。 */
export function isNewer(candidate: string, current: string): boolean {
  if (parseVersion(candidate) === null || parseVersion(current) === null) return false;
  return compareVersions(candidate, current) === 1;
}

/**
 * 配布元から得たものを、画面に出す状態へ落とす。
 *
 * 知らせるのは **署名が通っていて、かつ新しいとき**だけ。
 * どちらかが欠けたら `none` — 「たぶん更新がある」を表す状態を持たない（DESIGN.md §2 原則 7）。
 */
export function updateStateFrom(lookup: UpdateLookup): UpdateState {
  if (!lookup.signatureVerified) return { kind: 'none' };
  if (lookup.found === null) return { kind: 'none' };
  if (!isNewer(lookup.found, lookup.current)) return { kind: 'none' };
  return { kind: 'available', version: lookup.found };
}

function parseVersion(value: string): number[] | null {
  if (!/^\d+(\.\d+)*$/.test(value)) return null;
  return value.split('.').map((part) => Number.parseInt(part, 10));
}
