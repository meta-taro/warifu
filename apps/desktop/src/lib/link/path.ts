// 経路が直接か中継かを判定する（DESIGN.md §4.1 / `issues/005` 満たすこと 2）。
//
// **分からないものを「直接」へ倒さない。**
// 繋がっていないことを繋がっているように見せない、というのが原則 7 である。
// 「たぶん直接」を表す状態は持たない — 直接・中継・不明の 3 つだけ。
//
// ここは純ロジックで、`RTCStatsReport` に依存しない。呼ぶ側が
// `[...report.values()]` を渡す（テストは素のオブジェクトで書ける）。

/** DESIGN.md §4.1 の 3 状態。 */
export type LinkPath = 'direct' | 'relayed' | 'unknown';

/** ICE の候補の種別。`relay` だけが TURN を経由している。 */
const CANDIDATE_TYPES = ['host', 'srflx', 'prflx', 'relay'] as const;
type CandidateType = (typeof CANDIDATE_TYPES)[number];

/** `RTCStats` のうち、この層が見る所だけ。 */
export interface RtcStatLike {
  id: string;
  type: string;
  state?: string;
  nominated?: boolean;
  localCandidateId?: string;
  remoteCandidateId?: string;
  candidateType?: string;
}

function isCandidateType(value: string | undefined): value is CandidateType {
  return typeof value === 'string' && (CANDIDATE_TYPES as readonly string[]).includes(value);
}

/**
 * 局所と相手の候補の種別から経路を決める。
 *
 * **どちらか一方でも `relay` なら中継。**片側だけ TURN を通っていても、
 * その通信は誰かのサーバを経由している。
 */
export function classifyPath(local: string | undefined, remote: string | undefined): LinkPath {
  if (!isCandidateType(local) || !isCandidateType(remote)) return 'unknown';
  if (local === 'relay' || remote === 'relay') return 'relayed';
  return 'direct';
}

/**
 * WebRTC の統計から経路を取り出す。
 *
 * 見るのは**成立している組**だけ。`nominated` が立っているものを優先する
 * （複数の組が succeeded で残ることがあり、実際に使われているのは nominated の側）。
 * 組が見つかっても候補が引けなければ **`unknown`** — 途中まで分かったことを、
 * 分かりきったことのように扱わない。
 */
export function pathFromStats(stats: readonly RtcStatLike[]): LinkPath {
  const pairs = stats.filter((s) => s.type === 'candidate-pair' && s.state === 'succeeded');
  if (pairs.length === 0) return 'unknown';

  const selected = pairs.find((p) => p.nominated === true) ?? pairs[0];
  const byId = new Map(stats.map((s) => [s.id, s]));
  const local = byId.get(selected.localCandidateId ?? '');
  const remote = byId.get(selected.remoteCandidateId ?? '');

  return classifyPath(local?.candidateType, remote?.candidateType);
}
