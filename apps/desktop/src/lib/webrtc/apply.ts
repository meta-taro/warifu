// 決めた手を実際に打つ（M5-c2）。
//
// `negotiation.ts` が「次に何をすべきか」を返し、ここが `RTCPeerConnection` を叩く。
// 分けてあるので、**順序の規則は機材なしで、打ち方は偽物で**それぞれ確かめられる。
//
// **失敗を握り潰さない。**`addIceCandidate` が失敗したら、その候補は失われる。
// 黙って続けると「たまに繋がらない」になり、原因が二度と分からなくなる。

import type { Action } from './negotiation';

/** `RTCPeerConnection` のうち、ここが使う所だけ。**偽物を差せるようにするため。** */
export interface PeerLike {
  createOffer(): Promise<{ type: string; sdp?: string }>;
  createAnswer(): Promise<{ type: string; sdp?: string }>;
  setLocalDescription(description: { type: string; sdp?: string }): Promise<void>;
  setRemoteDescription(description: { type: string; sdp?: string }): Promise<void>;
  addIceCandidate(candidate: string): Promise<void>;
}

/** 相手へ 1 通送る。実体は Tauri の `send_signal`（`bridge.ts`）。 */
export type Send = (step: 'offer' | 'answer' | 'candidate', blob: string) => void;

/**
 * 手を 1 つ打つ。
 *
 * 作った素性は **自分に入れてから送る**。順序を逆にすると、
 * 相手の返事が自分の状態より先に着く場合がある。
 */
export async function applyAction(pc: PeerLike, action: Action, send: Send): Promise<void> {
  switch (action.kind) {
    case 'create-offer': {
      const offer = await pc.createOffer();
      await pc.setLocalDescription(offer);
      send('offer', offer.sdp ?? '');
      return;
    }
    case 'create-answer': {
      const answer = await pc.createAnswer();
      await pc.setLocalDescription(answer);
      send('answer', answer.sdp ?? '');
      return;
    }
    case 'apply-remote': {
      // 入れるだけ。**送り返さない**（返すのは create-answer の役）
      await pc.setRemoteDescription({ type: action.step, sdp: action.blob });
      return;
    }
    case 'add-ice': {
      await pc.addIceCandidate(action.blob);
      return;
    }
  }
}
