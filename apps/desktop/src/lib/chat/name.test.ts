import { describe, expect, it } from 'vitest';
import { エージェントの名を言い換える, 行の差出人名 } from './name';
import type { 会話行 } from '$lib/meeting/announce';

const 行 = (o: Partial<会話行>): 会話行 => ({ who: '', body: '', mine: false, ...o });

describe('行の差出人名', () => {
  it('**同じ鍵の相手は、いつも同じ顔で出る。**名簿から引き直す', () => {
    // 2026-09-15、実機で**同じ人が 1 通目は名前・2 通目は鍵**で出た（#19）。
    // 行に焼き付けた名前を出していたので、**引けなかった回だけ鍵になっていた**
    const 名簿 = { KEY1: '佐藤（mac mini）' };
    expect(行の差出人名(行({ who: 'PEERAAAAAAAA…', 顔の種: 'KEY1' }), 名簿)).toBe(
      '佐藤（mac mini）',
    );
  });

  it('名簿に無ければ、行が持っている名前を出す（消さない）', () => {
    expect(行の差出人名(行({ who: 'PEERAAAAAAAA…', 顔の種: 'KEY9' }), {})).toBe('PEERAAAAAAAA…');
  });

  it('顔の種が無い行（知らせなど）は、そのまま', () => {
    expect(行の差出人名(行({ who: '会議', system: true }), { KEY1: 'あ' })).toBe('会議');
  });

  it('名簿の値が空なら、行のほうを使う（空文字で人を消さない）', () => {
    expect(行の差出人名(行({ who: 'alpha のエージェント', 顔の種: 'desk:alpha' }), { 'desk:alpha': '' })).toBe(
      'alpha のエージェント',
    );
  });
});

describe('エージェントの名を言い換える', () => {
  const 英 = (名: string) => `${名} (agent)`;

  it('「X のエージェント」は、見ている人の言葉で言い直す', () => {
    // 名札は机が日本語で組む（相手へもその形で届く・識別に使う）。**出すときだけ**言い直す
    expect(エージェントの名を言い換える('note-taker のエージェント', 英)).toBe('note-taker (agent)');
  });

  it('エージェントでない名は、そのまま', () => {
    expect(エージェントの名を言い換える('Sam', 英)).toBe('Sam');
  });

  it('名乗りの付いた名札（括弧つき）も言い直す', () => {
    expect(エージェントの名を言い換える('docs のエージェント（議事メモ）', 英)).toBe(
      'docs (agent)（議事メモ）',
    );
  });
});
