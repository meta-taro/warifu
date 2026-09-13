import { describe, expect, it } from 'vitest';
import { 経路が付かないと言うか, 黙っている秒 } from './blocked';

const 場 = (o: Partial<Parameters<typeof 経路が付かないと言うか>[0]> = {}) => ({
  相手が居る: true,
  経路: 'unknown' as const,
  入ってからの秒: 黙っている秒 + 1,
  ...o,
});

describe('経路が付かないことを言うか', () => {
  // gh issue 9 / 11（ASUS・2026-09-12）——
  //
  // > `warifu-desktop.exe` に Inbound / Outbound の Allow を手で足したところ、
  // > 同じ手順・同じ鍵で入れるようになりました。
  // > **インストーラが規則を作らず、画面も許可を聞きません。**
  //
  // 利用者からは「**コマンドは動くのに画面だけ繋がらない**」という、
  // いちばん切り分けにくい形になる。**画面が黙っているのがいちばん悪い。**

  it('相手が居るのに経路が付かないままなら、言う', () => {
    expect(経路が付かないと言うか(場())).toBe(true);
  });

  it('**すぐには言わない**（握手の途中で脅かさない）', () => {
    expect(経路が付かないと言うか(場({ 入ってからの秒: 黙っている秒 - 1 }))).toBe(false);
  });

  it('経路が付いたら言わない', () => {
    expect(経路が付かないと言うか(場({ 経路: 'direct' }))).toBe(false);
    expect(経路が付かないと言うか(場({ 経路: 'relayed' }))).toBe(false);
  });

  it('**相手が居ないなら言わない**（誰も来ていないだけ）', () => {
    expect(経路が付かないと言うか(場({ 相手が居る: false }))).toBe(false);
  });
});
