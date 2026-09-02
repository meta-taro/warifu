// 映像を張るまでの順序（M5-c / D38）。
//
// **`RTCPeerConnection` を触らない。**受け取った状態から「次に何をすべきか」を返すだけで、
// 実際に叩く所は呼ぶ側にある。こうしてあるので、**カメラも実機も無しで順序を固定できる。**
//
// ここが持っている知識は 3 つ。
//
// 1. **offer を出すのは一方だけ**（D38。鍵の並びで決まる。この層は結果だけ受け取る）
// 2. **早すぎる ICE を捨てない**（相手の ICE は answer より先に届くことがある）
// 3. **既に張れている経路を、後から来た offer で壊さない**

/** 下ごしらえの段。`warifu-meeting` の `Step` と対応する。 */
export type Step = 'offer' | 'answer' | 'ice';

/** 呼ぶ側にやってもらうこと。**この層は実行しない。** */
export type Action =
  | { kind: 'create-offer' }
  | { kind: 'create-answer' }
  | { kind: 'apply-remote'; step: 'offer' | 'answer'; blob: string }
  | { kind: 'add-ice'; blob: string };

export type Phase =
  /** 相手の offer を待っている（受ける側）。 */
  | 'awaiting-offer'
  /** 自分の offer に対する answer を待っている（出す側）。 */
  | 'awaiting-answer'
  /** 相手の素性が入り、ICE をそのまま入れられる。 */
  | 'ready';

export interface NegotiationState {
  phase: Phase;
  /** offer を出す側か。D38 の判定結果をそのまま持つ。 */
  offering: boolean;
  /** 自分の媒体が用意できたか。 */
  mediaReady: boolean;
  /** 相手の素性が入る前に届いた ICE。**順番を保つ。** */
  pendingIce: string[];
}

/** `offering` は D38 の `should_offer_to` の結果を渡す。 */
export function start(offering: boolean): NegotiationState {
  return {
    phase: offering ? 'awaiting-answer' : 'awaiting-offer',
    offering,
    mediaReady: false,
    pendingIce: [],
  };
}

/**
 * 自分のカメラ・マイクが用意できた。
 *
 * **出す側だけが offer を作る。**受ける側がここで作ると衝突する（D38）。
 */
export function onLocalMediaReady(
  state: NegotiationState,
): [NegotiationState, readonly Action[]] {
  if (state.mediaReady) return [state, []];
  const next = { ...state, mediaReady: true };
  if (!state.offering) return [next, []];
  return [next, [{ kind: 'create-offer' }]];
}

/** 相手から下ごしらえが 1 つ届いた。 */
export function onRemote(
  state: NegotiationState,
  step: Step,
  blob: string,
): [NegotiationState, readonly Action[]] {
  if (step === 'ice') return onIce(state, blob);
  if (step === 'offer') return onOffer(state, blob);
  return onAnswer(state, blob);
}

function onIce(state: NegotiationState, blob: string): [NegotiationState, readonly Action[]] {
  if (state.phase === 'ready') {
    return [state, [{ kind: 'add-ice', blob }]];
  }
  // **捨てない。**相手の素性が入る前に `addIceCandidate` を呼ぶと失敗し、
  // その候補は二度と来ない。落とすと「たまに繋がらない」という形の不具合になる
  return [{ ...state, pendingIce: [...state.pendingIce, blob] }, []];
}

function onOffer(state: NegotiationState, blob: string): [NegotiationState, readonly Action[]] {
  if (state.phase !== 'awaiting-offer') {
    // 既に張れている経路を、後から来た offer で壊さない。
    // 出す側にこれが来るのは、両方が offer した（glare）ときで、D38 の下では起きない
    return [state, []];
  }
  return [
    { ...state, phase: 'ready', pendingIce: [] },
    [{ kind: 'apply-remote', step: 'offer', blob }, { kind: 'create-answer' }, ...flush(state)],
  ];
}

function onAnswer(state: NegotiationState, blob: string): [NegotiationState, readonly Action[]] {
  if (state.phase !== 'awaiting-answer') {
    // 頼んでいない answer は取り込まない
    return [state, []];
  }
  return [
    { ...state, phase: 'ready', pendingIce: [] },
    [{ kind: 'apply-remote', step: 'answer', blob }, ...flush(state)],
  ];
}

/** 溜めておいた ICE を、**届いた順のまま**吐き出す。 */
function flush(state: NegotiationState): Action[] {
  return state.pendingIce.map((blob) => ({ kind: 'add-ice', blob }));
}
