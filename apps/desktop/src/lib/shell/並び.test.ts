import { describe, expect, it } from 'vitest';
import { 初めの入退の面 } from './並び';

describe('ルームの面の並び（2026-09-24）', () => {
  it('初めに出すのは「入る」', () => {
    // **つくるのは主催だけ。**招かれた側のほうが人数が多い
    expect(初めの入退の面).toBe('入る');
  });
});
