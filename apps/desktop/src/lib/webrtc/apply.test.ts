import { describe, expect, it, vi } from 'vitest';
import { applyAction, type PeerLike } from './apply';

/** 呼ばれたことだけを覚える偽物。**実機もカメラも要らない。** */
function 偽の経路() {
  const calls: string[] = [];
  const pc: PeerLike = {
    createOffer: async () => {
      calls.push('createOffer');
      return { type: 'offer', sdp: 'SDP-OFFER' };
    },
    createAnswer: async () => {
      calls.push('createAnswer');
      return { type: 'answer', sdp: 'SDP-ANSWER' };
    },
    setLocalDescription: async (d) => {
      calls.push(`setLocal:${d.type}`);
    },
    setRemoteDescription: async (d) => {
      calls.push(`setRemote:${d.type}`);
    },
    addIceCandidate: async (c) => {
      calls.push(`addIce:${c}`);
    },
  };
  return { pc, calls };
}

describe('決めた手を実際に打つ（M5-c2）', () => {
  it('offer は作って、自分に入れて、相手へ送る', async () => {
    const { pc, calls } = 偽の経路();
    const send = vi.fn();

    await applyAction(pc, { kind: 'create-offer' }, send);

    expect(calls).toEqual(['createOffer', 'setLocal:offer']);
    expect(send).toHaveBeenCalledWith('offer', 'SDP-OFFER');
  });

  it('answer も同じ順序', async () => {
    const { pc, calls } = 偽の経路();
    const send = vi.fn();

    await applyAction(pc, { kind: 'create-answer' }, send);

    expect(calls).toEqual(['createAnswer', 'setLocal:answer']);
    expect(send).toHaveBeenCalledWith('answer', 'SDP-ANSWER');
  });

  it('相手の素性は入れるだけ。送り返さない', async () => {
    const { pc, calls } = 偽の経路();
    const send = vi.fn();

    await applyAction(pc, { kind: 'apply-remote', step: 'offer', blob: 'SDP-X' }, send);

    expect(calls).toEqual(['setRemote:offer']);
    expect(send).not.toHaveBeenCalled();
  });

  it('ICE は入れるだけ', async () => {
    const { pc, calls } = 偽の経路();
    const send = vi.fn();

    await applyAction(pc, { kind: 'add-ice', blob: 'ICE-1' }, send);

    expect(calls).toEqual(['addIce:ICE-1']);
    expect(send).not.toHaveBeenCalled();
  });

  it('**失敗を握り潰さない**', async () => {
    const { pc } = 偽の経路();
    pc.addIceCandidate = async () => {
      throw new Error('候補が壊れている');
    };
    await expect(applyAction(pc, { kind: 'add-ice', blob: 'x' }, vi.fn())).rejects.toThrow(
      '候補が壊れている',
    );
  });
});
