// **答える側は、相手の offer でできた枠を引き取って送る**（2026-09-29）。
//
// 9/18、「相手が入った瞬間にカメラを掴まない」ために、掴む前から送る枠を
// `addTransceiver` で足しておく形にした。**offer を出す側ではそれで良い**が、
// **答える側では、相手の offer でできた枠と別の枠になり、答えに載らない。**
// 交渉されないまま `currentDirection: null` で残り、映像のトラックを入れても
// outbound-rtp が 1 本も立たない（Windows の WebView2 で CDP から確かめた）。
//
// 前の形（`addTrack`）は、相手の offer でできた枠を使い回していたので、この穴は無かった。

/** 枠のうち、ここで見る所だけ（本物の `RTCRtpTransceiver` と同じ形）。 */
export interface 枠の形 {
  receiver: { track: { kind: string } | null };
  stopped?: boolean;
}

/**
 * **答える側が、送る用に引き取る枠**を選ぶ。
 *
 * 相手の offer でできた、その種の枠のうち、止めていない・まだ使っていないもの。
 * **offer がまだ来ていなければ `null`**（来てから引き取る）。
 */
export function 引き取る枠<T extends 枠の形>(
  枠たち: readonly T[],
  種: 'audio' | 'video',
  使っている: ReadonlySet<T>,
): T | null {
  return (
    枠たち.find((t) => !t.stopped && t.receiver.track?.kind === 種 && !使っている.has(t)) ?? null
  );
}
