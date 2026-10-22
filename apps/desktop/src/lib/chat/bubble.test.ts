import { describe, expect, it } from 'vitest';
import { 吹き出しの形 } from './bubble';
import type { 会話行 } from '$lib/meeting/announce';

const 行 = (o: Partial<会話行>): 会話行 => ({ who: '', body: 'やあ', mine: false, ...o });
const 鍵 = '4VKCVUBF4RNLZ7DG2XFNZVIWHQNXVYNGWTYTAOALSPKNGWTZA3WA';

describe('吹き出しの形', () => {
  it('自分が打った行は右・人', () => {
    expect(吹き出しの形(行({ mine: true }))).toEqual({ 側: '右', 主: '人' });
  });

  it('この機械のエージェントの行も右（同じ機械から出た）・エージェント', () => {
    expect(吹き出しの形(行({ agent: true, 顔の種: 'desk:zumen' }))).toEqual({ 側: '右', 主: 'エージェント' });
  });

  it('ほかの機械の人の行は左・人', () => {
    expect(吹き出しの形(行({ 顔の種: 鍵 }))).toEqual({ 側: '左', 主: '人' });
  });

  it('ほかの機械のエージェント（札付きの名）は左・エージェント', () => {
    // 機械が `鍵の頭（札）` の形で渡してくる（lib.rs の受信）
    expect(吹き出しの形(行({ 顔の種: '4VKCVUBF…（連動 のエージェント（docs））' }))).toEqual({
      側: '左',
      主: 'エージェント',
    });
  });

  it('会議からの知らせは真ん中', () => {
    expect(吹き出しの形(行({ system: true }))).toEqual({ 側: '中', 主: '人' });
  });

  it('留守中に預かり所から届いた行は、顔の種が無くても左・人', () => {
    expect(吹き出しの形(行({ 留守中: true }))).toEqual({ 側: '左', 主: '人' });
  });
});
