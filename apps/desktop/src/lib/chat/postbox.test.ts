import { describe, expect, it } from 'vitest';

import { どう送るか, 留守中の行 } from './postbox';

const 素材 = (足す: Partial<Parameters<typeof どう送るか>[0]> = {}) => ({
  宛先: null,
  選んでいる: null,
  その人はルームに居る: false,
  預かり所がある: false,
  ...足す,
});

describe('どう送るか', () => {
  it('この機械のエージェントを選んでいれば、そのエージェントへ', () => {
    expect(どう送るか(素材({ 宛先: 'zumen の AI', 選んでいる: 'desk:zumen の AI' }))).toEqual({
      種類: 'この機械',
      宛先: 'zumen の AI',
    });
  });

  it('誰も選んでいなければ、いまのルームへ', () => {
    expect(どう送るか(素材())).toEqual({ 種類: 'ルーム' });
  });

  it('選んだ人が同じルームに居るなら、その場で渡す', () => {
    expect(どう送るか(素材({ 選んでいる: 'ABC', その人はルームに居る: true }))).toEqual({
      種類: 'ルーム',
    });
  });

  it('選んだ人が居なくても、預かり所があれば預ける', () => {
    // **これが LINE との差だった。**相手が起動していないと消えていた
    expect(どう送るか(素材({ 選んでいる: 'ABC', 預かり所がある: true }))).toEqual({
      種類: '預ける',
      key: 'ABC',
    });
  });

  it('預かり所が無ければ、送れないと理由を出す', () => {
    // **黙って捨てない。**打った文字が消えたように見える（2026-09-07）
    expect(どう送るか(素材({ 選んでいる: 'ABC' }))).toEqual({
      種類: '送れない',
      訳: 'send.absent',
    });
  });

  it('ルームを選んでいるときは、そのルームへ', () => {
    expect(どう送るか(素材({ 選んでいる: 'room:abc' }))).toEqual({ 種類: 'ルーム' });
  });
});

describe('留守中の行', () => {
  it('留守中に届いた分だと分かる形にする', () => {
    // **時刻は出した側の時計である。**いま届いたように見せない
    const 秒 = Math.floor(new Date(2026, 8, 5, 9, 7).getTime() / 1000);
    const 行 = 留守中の行('田中', 'おはよう', 秒);
    expect(行.who).toBe('田中');
    expect(行.body).toBe('おはよう');
    expect(行.at).toBe('09:07');
    expect(行.留守中).toBe(true);
    expect(行.mine).toBe(false);
  });
});
