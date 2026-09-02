import { describe, expect, it } from 'vitest';
import { onLocalMediaReady, onRemote, start, type Action } from './negotiation';

const kinds = (actions: readonly Action[]) => actions.map((a) => a.kind);

describe('誰が offer を出すか（D38 を画面側でも守る）', () => {
  it('offer を出す側は、媒体が用意できたら offer を作る', () => {
    const [next, actions] = onLocalMediaReady(start(true));
    expect(kinds(actions)).toEqual(['create-offer']);
    expect(next.phase).toBe('awaiting-answer');
  });

  it('受ける側は、媒体が用意できても何もしない', () => {
    // ここで offer を作ると glare になる（D38）
    const [next, actions] = onLocalMediaReady(start(false));
    expect(actions).toEqual([]);
    expect(next.phase).toBe('awaiting-offer');
  });

  it('offer を二度作らない', () => {
    const [afterFirst] = onLocalMediaReady(start(true));
    const [, actions] = onLocalMediaReady(afterFirst);
    expect(actions).toEqual([]);
  });
});

describe('下ごしらえを受け取る順序', () => {
  it('受ける側は offer を受けて answer を返す', () => {
    const [next, actions] = onRemote(start(false), 'offer', 'SDP-OFFER');
    expect(kinds(actions)).toEqual(['apply-remote', 'create-answer']);
    expect(next.phase).toBe('ready');
  });

  it('出した側は answer を受けて出来上がる', () => {
    const [offered] = onLocalMediaReady(start(true));
    const [next, actions] = onRemote(offered, 'answer', 'SDP-ANSWER');
    expect(kinds(actions)).toEqual(['apply-remote']);
    expect(next.phase).toBe('ready');
  });

  it('**早すぎる ICE を捨てない。**溜めておいて、後で順番どおり入れる', () => {
    // 相手の ICE は answer より先に届くことがある。捨てると繋がらなくなる
    let s = start(true);
    [s] = onLocalMediaReady(s);
    let a1: readonly Action[];
    let a2: readonly Action[];
    [s, a1] = onRemote(s, 'ice', 'ICE-1');
    [s, a2] = onRemote(s, 'ice', 'ICE-2');
    expect(a1).toEqual([]);
    expect(a2).toEqual([]);
    expect(s.pendingIce).toEqual(['ICE-1', 'ICE-2']);

    let flushed: readonly Action[];
    [s, flushed] = onRemote(s, 'answer', 'SDP-ANSWER');
    expect(flushed).toEqual([
      { kind: 'apply-remote', step: 'answer', blob: 'SDP-ANSWER' },
      { kind: 'add-ice', blob: 'ICE-1' },
      { kind: 'add-ice', blob: 'ICE-2' },
    ]);
    expect(s.pendingIce).toEqual([]);
  });

  it('出来上がった後の ICE はそのまま入れる', () => {
    let s = start(false);
    [s] = onRemote(s, 'offer', 'SDP-OFFER');
    const [next, actions] = onRemote(s, 'ice', 'ICE-9');
    expect(actions).toEqual([{ kind: 'add-ice', blob: 'ICE-9' }]);
    expect(next.pendingIce).toEqual([]);
  });

  it('二度目の offer は取り込まない（既に張れている経路を壊さない）', () => {
    let s = start(false);
    [s] = onRemote(s, 'offer', 'SDP-OFFER');
    const [next, actions] = onRemote(s, 'offer', 'SDP-OFFER-2');
    expect(actions).toEqual([]);
    expect(next.phase).toBe('ready');
  });

  it('offer を待っている側に answer が来ても取り込まない', () => {
    const [next, actions] = onRemote(start(false), 'answer', 'SDP-ANSWER');
    expect(actions).toEqual([]);
    expect(next.phase).toBe('awaiting-offer');
  });
});
