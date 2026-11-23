// **人が押すものに、カーソルで押させない印が付いているか**（11-12）。
//
// エージェントの専用カーソル（`cursor.ts`）は、`data-agent-hands-off` の中を押さない・打たない。
// 印が外れると、エージェントが自分で自分に許し（札）を出せてしまう。画面を書き換えて印を落としたら、ここで止める。
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const 読む = (道: string) => readFileSync(fileURLToPath(new URL(道, import.meta.url)), 'utf-8');

describe('人が押すものには、押させない印がある', () => {
  const 画面 = 読む('../../routes/+page.svelte');
  const 会話 = 読む('../chat/ChatPanel.svelte');

  it('札の頼みの帯・常設の札の一覧・入りますかの帯', () => {
    expect(画面).toMatch(/class="invited pass-ask"[^>]*data-agent-hands-off/);
    expect(画面).toMatch(/class="standing-list"[^>]*data-agent-hands-off/);
    // メールを送ってよいか（D132）。送るのは人
    expect(画面).toMatch(/class="invited mail-ask"[^>]*data-agent-hands-off/);
    const 入りますか = 画面.match(/<div class="invited" [^>]*>/g) ?? [];
    expect(入りますか.length).toBeGreaterThan(0);
    for (const 帯 of 入りますか) expect(帯).toContain('data-agent-hands-off');
  });

  it('メールの口の「端末の許可で送る」（0.1.53）', () => {
    const メール = 読む('../mail/MailPane.svelte');
    expect(メール).toMatch(/class="terminal-send"[^>]*data-agent-hands-off/);
  });

  it('メールを読ませる（11-20）と、画面の前に居ないときの承認（0.1.55）', () => {
    const メール = 読む('../mail/MailPane.svelte');
    const 切り替え = メール.match(/<label class="read-standing"[^>]*>/g) ?? [];
    expect(切り替え.length).toBe(2);
    for (const 一つ of 切り替え) expect(一つ).toContain('data-agent-hands-off');
    expect(メール).toMatch(/data-agent-hands-off[\s\S]{0,200}留守の承認を決める/);
  });

  it('会話の［送る］と、秘密の確かめ', () => {
    expect(会話).toMatch(/onclick=\{\(\) => 出す\(\)\}[^>]*data-agent-hands-off/);
    expect(会話).toMatch(/class="secret-check"[^>]*data-agent-hands-off/);
  });
});
