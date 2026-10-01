import { describe, expect, it } from 'vitest';
import { ふさがりの言い方 } from './firewall';

describe('ファイアウォールの言い方を、規則の有無で分ける', () => {
  // PR #16（Windows の人・2026-09-14）——
  //
  // > 実機では規則を 3 つ足したあとも同じ文言が出続け、
  // > **足した人が「足したのに直らない」で止まりました**（#11 で 3 時間）。
  //
  // **規則があるのに「ファイアウォールが止めています」と言うのは、嘘に近い。**
  // `warifu-guard` が実行ファイルごとに調べてくれるので、そこを見て言い分ける。

  it('**規則が無いなら、名指しで言って直し方も出す**', () => {
    expect(ふさがりの言い方({ state: 'blocked', rules: null })).toEqual({
      鍵: 'link.blocked',
      直し方を出す: true,
    });
  });

  it('**規則があるなら、ふさがりの話をしない**（別の原因なので疑わせない）', () => {
    expect(ふさがりの言い方({ state: 'open', rules: 2 })).toEqual({
      鍵: 'link.blocked.notit',
      直し方を出す: false,
    });
  });

  it('**調べられないなら、そう言うだけ**（足してくださいとは言わない）', () => {
    // **足りている人を止めない**（DESIGN §2 原則 7）
    expect(
      ふさがりの言い方({ state: 'unknown', rules: null, detail: 'powershell がありません' })
    ).toEqual({
      鍵: 'link.blocked.unknown',
      直し方を出す: false,
      detail: 'powershell がありません',
    });
  });

  it('調べる口が無い OS（macOS など）でも、黙らない', () => {
    // `warifu-guard` は Windows 以外では「分からない」を返す。**「無い」ではない**
    expect(ふさがりの言い方({ state: 'unknown', rules: null }).鍵).toBe('link.blocked.unknown');
  });
});
