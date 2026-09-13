import { describe, expect, it } from 'vitest';
import { ふさがりの直し方 } from './fix';

describe('ふさがりの直し方', () => {
  // gh issue 9（ASUS・2026-09-12）——
  //
  // > 画面用の規則を作るには管理者権限が要るので、こちらでは足せません。人に頼みます。
  //
  // **割符は管理者権限を要求しない**（D104）。代わりに**直し方をその場で渡す。**

  it('Windows なら、管理者で打つ 2 行を返す', () => {
    const 文 = ふさがりの直し方('Windows NT 10.0; Win64; x64');
    expect(文).toContain('New-NetFirewallRule');
    // **受信と送信の両方**（片方だけでは繋がらない）
    expect(文).toContain('Inbound');
    expect(文).toContain('Outbound');
    // **場所を決め打ちで書く**（人に探させない）
    expect(文).toContain('warifu-desktop.exe');
  });

  it('**macOS には返さない**（打つものが違う）', () => {
    expect(ふさがりの直し方('Macintosh; Intel Mac OS X 10_15_7')).toBe(null);
  });

  it('分からない環境には返さない（当てずっぽうを渡さない）', () => {
    expect(ふさがりの直し方('')).toBe(null);
    expect(ふさがりの直し方('X11; Linux x86_64')).toBe(null);
  });
});
