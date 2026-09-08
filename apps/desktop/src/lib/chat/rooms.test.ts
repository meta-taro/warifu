import { describe, expect, it } from 'vitest';

import { その部屋の会話, 足す, 畳む, 見る部屋, 机の部屋, type 部屋の会話 } from './rooms';
import type { 会話行 } from '$lib/meeting/announce';

const 行 = (body: string): 会話行 => ({ who: '自分', body, mine: true });

describe('部屋ごとの会話', () => {
  it('部屋を分けて持てる', () => {
    // **混ぜると、相手ごとの会話に見えない**（2026-09-08 オーナー指摘）
    let 会話: 部屋の会話 = {};
    会話 = 足す(会話, 'A', 行('Aへ'));
    会話 = 足す(会話, 'B', 行('Bへ'));
    expect(その部屋の会話(会話, 'A').map((r) => r.body)).toEqual(['Aへ']);
    expect(その部屋の会話(会話, 'B').map((r) => r.body)).toEqual(['Bへ']);
  });

  it('足しても、他の部屋を触らない', () => {
    // **見ていない部屋の会話が壊れないこと**
    let 会話: 部屋の会話 = 足す({}, 'A', 行('1'));
    const 前 = その部屋の会話(会話, 'A');
    会話 = 足す(会話, 'B', 行('2'));
    expect(その部屋の会話(会話, 'A')).toEqual(前);
  });

  it('知らない部屋は空', () => {
    // **空を「まだ無い」と区別しない**
    expect(その部屋の会話({}, 'どこか')).toEqual([]);
  });

  it('部屋を選んでいなければ空', () => {
    expect(その部屋の会話(足す({}, 'A', 行('x')), null)).toEqual([]);
  });

  it('畳むと会話ごと消える', () => {
    // **閉じれば消える**（履歴は持たない・issues/010）
    const 会話 = 畳む(足す({}, 'A', 行('x')), 'A');
    expect(その部屋の会話(会話, 'A')).toEqual([]);
  });

  it('畳んでも、他の部屋は残る', () => {
    let 会話: 部屋の会話 = 足す({}, 'A', 行('a'));
    会話 = 足す(会話, 'B', 行('b'));
    会話 = 畳む(会話, 'A');
    expect(その部屋の会話(会話, 'B').map((r) => r.body)).toEqual(['b']);
  });
});

describe('いま見る部屋', () => {
  it('同じ PC の AI を選んだら、机の部屋', () => {
    // **机は部屋ではないが、会話は分けて持つ**
    expect(見る部屋('desk:zumen の AI', 'ROOM1')).toBe(机の部屋);
  });

  it('AI を選んでいなければ、いま居る部屋', () => {
    expect(見る部屋(null, 'ROOM1')).toBe('ROOM1');
    expect(見る部屋('BBBBBBBB', 'ROOM1')).toBe('ROOM1');
  });

  it('部屋にも居らず AI も選んでいなければ、どこも見ていない', () => {
    expect(見る部屋(null, null)).toBeNull();
  });
});

describe('部屋を選ぶ', () => {
  it('部屋を選んだら、その部屋を見る', () => {
    // **見ていない部屋も生きている。**選び直すだけで経路は切れない
    expect(見る部屋('room:ROOM2', 'ROOM1')).toBe('ROOM2');
  });

  it('AI を選ぶほうが優先される（机は部屋ではない）', () => {
    expect(見る部屋('desk:zumen の AI', 'ROOM1')).toBe(机の部屋);
  });
});

describe('部屋に居ない人を選んだとき', () => {
  it('その人との会話を見る（預かり所ごしの 1 対 1）', () => {
    expect(見る部屋('ABC', 'room-1', false)).toBe('contact:ABC');
  });

  it('同じ部屋に居るなら、部屋の会話を見る', () => {
    expect(見る部屋('ABC', 'room-1', true)).toBe('room-1');
  });

  it('留守中に届いた分を、部屋の会話に混ぜない', () => {
    // 混ぜると「いまの部屋で言われたこと」として並ぶ
    expect(見る部屋('ABC', null, false)).not.toBe(null);
    expect(見る部屋(null, 'room-1')).toBe('room-1');
  });
});
