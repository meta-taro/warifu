import { describe, expect, it } from 'vitest';
import { 吹き出しの形, 機械に色を振る, 機械の頭, 機械の色合いたち, 畳むか, 畳む字数, 畳む行数 } from './bubble';
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

describe('畳むか', () => {
  it('短い発言は畳まない', () => {
    expect(畳むか('やあ')).toBe(false);
    expect(畳むか('あ'.repeat(畳む字数))).toBe(false);
    expect(畳むか(Array(畳む行数).fill('行').join('\n'))).toBe(false);
  });

  it('字数が線を越えたら畳む', () => {
    expect(畳むか('あ'.repeat(畳む字数 + 1))).toBe(true);
  });

  it('短い行でも、行数が線を越えたら畳む', () => {
    expect(畳むか(Array(畳む行数 + 1).fill('行').join('\n'))).toBe(true);
  });

  it('改行は字数に数えない', () => {
    expect(畳むか('あ\n'.repeat(4) + 'あ'.repeat(畳む字数 - 4))).toBe(false);
  });
});

describe('機械に色を振る（0.1.46）', () => {
  const 鍵A = 'EKBN2GCQO35WAMMOF7EHBN5SKV6Q3JL7BTEGCTSQ3IA6AAWQHAHA';
  const 鍵B = 'O4FXQZLARCFBJRLEKLSYIH4NTNPRLEFQEZL7C2PU3Z464KG47DEQ';
  const 行 = (上書き: Partial<会話行>): 会話行 => ({ who: 'x', body: 'y', mine: false, ...上書き });

  it('同じ機械なら、人でもエージェントでも同じ色合い', () => {
    const 色 = 機械に色を振る([行({ 顔の種: 鍵A }), 行({ 顔の種: 'EKBN2GCQO35W（yobitarou-ios）' })]);
    expect(色.size).toBe(1);
    expect(機械の色合いたち).toContain(色.get('EKBN2GCQO35W'));
  });
  it('別の機械どうしは、同じ会話の中で違う色合いになる（8 台まで）', () => {
    const 鍵たち = Array.from({ length: 8 }, (_, i) => `MACHINE${i}XXXXX` + 'A'.repeat(40));
    const 色 = 機械に色を振る(鍵たち.map((k) => 行({ 顔の種: k })));
    expect(new Set(色.values()).size).toBe(8);
    expect(機械に色を振る([行({ 顔の種: 鍵A }), 行({ 顔の種: 鍵B })]).size).toBe(2);
  });
  it('出てくる順や他の行に関わらず、1 台だけなら鍵から決まった色', () => {
    const 一人 = 機械に色を振る([行({ 顔の種: 鍵B })]).get('O4FXQZLARCFB');
    const 知らせ混じり = 機械に色を振る([行({ system: true }), 行({ 顔の種: 鍵B, body: 'z' })]).get('O4FXQZLARCFB');
    expect(一人).toBe(知らせ混じり);
  });
  it('この機械から出た発言・知らせ・種の無い行には付けない', () => {
    expect(機械の頭(行({ 顔の種: 鍵A, mine: true }))).toBeNull();
    expect(機械の頭(行({ 顔の種: 'desk:warifu のエージェント', agent: true }))).toBeNull();
    expect(機械の頭(行({ 顔の種: 'desk:warifu のエージェント' }))).toBeNull();
    expect(機械の頭(行({ system: true }))).toBeNull();
    expect(機械の頭(行({}))).toBeNull();
  });
});
