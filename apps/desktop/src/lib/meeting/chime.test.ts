import { describe, expect, it, vi } from 'vitest';

import { 入室の音, 退室の音, 鳴らす } from './chime';

describe('知らせの音', () => {
  it('入室は上がり、退室は下がる（聞いただけで向きが分かる）', () => {
    const 向き = (c: typeof 入室の音) => Math.sign(c.tones[c.tones.length - 1].hz - c.tones[0].hz);
    expect(向き(入室の音)).toBe(1);
    expect(向き(退室の音)).toBe(-1);
  });

  it('短い。会話をさえぎらない', () => {
    for (const 音 of [入室の音, 退室の音]) {
      const 全体 = 音.tones.reduce((s, t) => s + t.ms, 0);
      expect(全体).toBeLessThanOrEqual(400);
    }
  });

  it('音そのものはファイルにしない（参照だけ足してアップロードを忘れる形を作らない）', () => {
    // baseline §23 — アセットの参照とアップロードを分離させない。
    // その場で作る音なら、参照するファイルが無い
    for (const 音 of [入室の音, 退室の音]) {
      for (const t of 音.tones) {
        expect(t.hz).toBeGreaterThan(0);
        expect(t.ms).toBeGreaterThan(0);
      }
    }
  });
});

describe('鳴らす', () => {
  function 偽のAudioContext() {
    const 予定: Array<{ hz: number; at: number }> = [];
    const osc = {
      frequency: { value: 0 },
      connect: vi.fn(),
      start: vi.fn((at: number) => 予定.push({ hz: osc.frequency.value, at })),
      stop: vi.fn(),
    };
    const gain = { gain: { setValueAtTime: vi.fn(), linearRampToValueAtTime: vi.fn() }, connect: vi.fn() };
    return {
      予定,
      ctx: {
        currentTime: 0,
        destination: {},
        createOscillator: () => osc,
        createGain: () => gain,
        close: vi.fn(),
      },
    };
  }

  it('音の数だけ鳴らす', () => {
    const { ctx, 予定 } = 偽のAudioContext();
    鳴らす(ctx as unknown as AudioContext, 入室の音);
    expect(予定.length).toBe(入室の音.tones.length);
  });

  it('音を出せない環境でも落ちない', () => {
    // **音が鳴らないことは、会議に入れないことではない**
    expect(() => 鳴らす(null, 入室の音)).not.toThrow();
  });
});
