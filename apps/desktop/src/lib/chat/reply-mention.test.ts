import { describe, expect, it } from 'vitest';
import type { 会話行 } from '$lib/meeting/announce';
import { 返信で呼ぶ名, 返信の呼びを足す, 返信の呼びを外す } from './reply-mention';

const 行 = (上書き: Partial<会話行>): 会話行 => ({ who: 'X', body: 'b', mine: false, ...上書き });
const 名簿 = { KEYHUMAN: '梶原さん', KEYNONAME: '名無しさん' };

describe('返信で呼ぶ名', () => {
  it('相手の機械のエージェントなら名札', () => {
    const 名 = 返信で呼ぶ名(行({ 顔の種: 'ABCDEFGHIJKL（はじエス担当（kaji PC）（haji-s.com））' }), 名簿, '名無しさん');
    expect(名).toBe('はじエス担当（kaji PC）（haji-s.com）');
  });
  it('この機械のエージェントなら名', () => {
    expect(返信で呼ぶ名(行({ who: 'dbboard（mac mini）', 顔の種: 'desk:dbboard のエージェント', agent: true }), 名簿, '名無しさん')).toBe('dbboard（mac mini）');
  });
  it('人なら連絡帳の名前', () => {
    expect(返信で呼ぶ名(行({ 顔の種: 'KEYHUMAN' }), 名簿, '名無しさん')).toBe('梶原さん');
  });
  it('名前の無い人・自分・知らせは呼ばない', () => {
    expect(返信で呼ぶ名(行({ 顔の種: 'KEYNONAME' }), 名簿, '名無しさん')).toBeNull();
    expect(返信で呼ぶ名(行({ 顔の種: 'UNKNOWNKEY' }), 名簿, '名無しさん')).toBeNull();
    expect(返信で呼ぶ名(行({ 顔の種: 'KEYHUMAN', mine: true }), 名簿, '名無しさん')).toBeNull();
    expect(返信で呼ぶ名(行({ system: true }), 名簿, '名無しさん')).toBeNull();
  });
});

describe('返信の呼びを足す', () => {
  it('下書きの頭に @名 を入れる', () => {
    expect(返信の呼びを足す('', '梶原さん')).toEqual({ 本文: '@梶原さん ', 足した: '@梶原さん ' });
    expect(返信の呼びを足す('了解です', '梶原さん')).toEqual({ 本文: '@梶原さん 了解です', 足した: '@梶原さん ' });
  });
  it('もう呼んでいれば足さない（名前だけで呼んでいても）', () => {
    expect(返信の呼びを足す('@梶原さん 了解', '梶原さん')).toEqual({ 本文: '@梶原さん 了解', 足した: null });
    expect(返信の呼びを足す('了解 @tadori', 'tadori（ZenBook）')).toEqual({ 本文: '了解 @tadori', 足した: null });
  });
});

describe('返信の呼びを外す', () => {
  it('足したまま触っていない頭だけを外す', () => {
    expect(返信の呼びを外す('@梶原さん 了解', '@梶原さん ')).toBe('了解');
    expect(返信の呼びを外す('@梶原さん ', '@梶原さん ')).toBe('');
  });
  it('人が直した頭は外さない', () => {
    expect(返信の呼びを外す('@梶原さん、了解', '@梶原さん ')).toBe('@梶原さん、了解');
    expect(返信の呼びを外す('了解', null)).toBe('了解');
  });
});
