import { describe, expect, it } from 'vitest';
import { 引き取る枠 } from './frames';

// **答える側は、相手の offer でできた枠を引き取って送る**（2026-09-29）。
//
// 答える側で、送るための枠 2 本が交渉されず `currentDirection: null` のままだった
// （Windows の WebView2 で CDP から確かめた）。**9/18 に `addTransceiver` で新しい枠を足す形にしたため**、
// 答える側では相手の offer でできた枠と別の、**答えに載らない枠**になっていた。
// → 映像のトラックは付いているのに outbound-rtp が 0 本。
const 枠 = (種: 'audio' | 'video', 止めた = false) => ({
  receiver: { track: { kind: 種 } },
  stopped: 止めた,
});

describe('答える側が引き取る枠', () => {
  it('相手の offer でできた、その種の枠を返す', () => {
    const 枠たち = [枠('audio'), 枠('video')];
    expect(引き取る枠(枠たち, 'video', new Set())).toBe(枠たち[1]);
    expect(引き取る枠(枠たち, 'audio', new Set())).toBe(枠たち[0]);
  });

  it('**まだ offer が来ていなければ、無い**（来てから引き取る）', () => {
    expect(引き取る枠([], 'video', new Set())).toBeNull();
  });

  it('止めた枠・もう使っている枠は引き取らない', () => {
    const 使用中 = 枠('video');
    const 枠たち = [枠('video', true), 使用中, 枠('video')];
    expect(引き取る枠(枠たち, 'video', new Set([使用中]))).toBe(枠たち[2]);
  });
});
