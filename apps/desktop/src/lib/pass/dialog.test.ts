import { describe, expect, it } from 'vitest';
import { 既定の許しを選ぶ, 窓に出す頼み } from './dialog';

describe('札の頼みの窓', () => {
  const 取り消すまで = ['inbox.open.structured', 'inbox.open.raw'];

  it('取り消すまで出せる動作は、Enter で取り消すまで通す', () => {
    expect(既定の許しを選ぶ('inbox.open.raw', 取り消すまで)).toBe('取り消すまで');
  });

  it('ほかの動作は、Enter で 24 時間通す（本体はそれより長く出さない）', () => {
    expect(既定の許しを選ぶ('mail.send', 取り消すまで)).toBe('24時間');
    expect(既定の許しを選ぶ('room.invite', 取り消すまで)).toBe('24時間');
  });

  it('窓にはいちばん古い頼みを 1 つ出し、残りの数を言う', () => {
    expect(窓に出す頼み([])).toEqual({ いま: null, ほか: 0 });
    expect(窓に出す頼み(['a'])).toEqual({ いま: 'a', ほか: 0 });
    expect(窓に出す頼み(['a', 'b', 'c'])).toEqual({ いま: 'a', ほか: 2 });
  });
});
