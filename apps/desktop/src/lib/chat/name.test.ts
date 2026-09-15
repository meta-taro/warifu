import { describe, expect, it } from 'vitest';
import { 行の差出人名 } from './name';
import type { 会話行 } from '$lib/meeting/announce';

const 行 = (o: Partial<会話行>): 会話行 => ({ who: '', body: '', mine: false, ...o });

describe('行の差出人名', () => {
  it('**同じ鍵の相手は、いつも同じ顔で出る。**名簿から引き直す', () => {
    // 2026-09-15、実機で**同じ人が 1 通目は名前・2 通目は鍵**で出た（#19）。
    // 行に焼き付けた名前を出していたので、**引けなかった回だけ鍵になっていた**
    const 名簿 = { KEY1: 'めたたろ（mac mini）' };
    expect(行の差出人名(行({ who: 'I3ILQUUJQHVS…', 顔の種: 'KEY1' }), 名簿)).toBe(
      'めたたろ（mac mini）',
    );
  });

  it('名簿に無ければ、行が持っている名前を出す（消さない）', () => {
    expect(行の差出人名(行({ who: 'I3ILQUUJQHVS…', 顔の種: 'KEY9' }), {})).toBe('I3ILQUUJQHVS…');
  });

  it('顔の種が無い行（知らせなど）は、そのまま', () => {
    expect(行の差出人名(行({ who: '会議', system: true }), { KEY1: 'あ' })).toBe('会議');
  });

  it('名簿の値が空なら、行のほうを使う（空文字で人を消さない）', () => {
    expect(行の差出人名(行({ who: 'zumen のエージェント', 顔の種: 'desk:zumen' }), { 'desk:zumen': '' })).toBe(
      'zumen のエージェント',
    );
  });
});
