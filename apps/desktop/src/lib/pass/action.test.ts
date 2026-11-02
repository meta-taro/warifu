import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { LOCALES } from '$lib/i18n/locales';
import { MESSAGES, type MessageKey } from '$lib/i18n/messages';
import { 動作の言葉 } from './action';

/**
 * 札の帯に、動作の内部名（`chat.send`・`room.handoff`）を出さない（spec 08 S08-1）。
 *
 * **動作を足したら、ここが落ちる。**口（`warifu mcp`）が聞く動作と、画面が前もって出す札を
 * 元の Rust から拾い、4 つの言葉ぜんぶに言い方があるかを見る。
 */
const 口 = readFileSync(new URL('../../../../../crates/warifu-mcp/src/server.rs', import.meta.url), 'utf-8');
const 手渡し = readFileSync(
  new URL('../../../src-tauri/src/key_give.rs', import.meta.url),
  'utf-8',
);

function 動作たち(): string[] {
  const 拾った = [...口.matchAll(/通るか\("([a-z.]+)"\)/g)].map((m) => m[1]);
  // `inbox.open.{段}` は段ごとに別の札（`inbox_open` の match の腕）
  const 段 = [...口.matchAll(/"([a-z]+)" => Level::/g)].map((m) => `inbox.open.${m[1]}`);
  const 札 = /聞かずに渡す札: &str = "([a-z.]+)"/.exec(手渡し)?.[1];
  return [...new Set([...拾った, ...段, ...(札 ? [札] : [])])].sort();
}

describe('動作の言葉', () => {
  it('元の Rust から動作を拾えている（拾えなければ、この試験が空回りする）', () => {
    const 全部 = 動作たち();
    expect(全部).toContain('chat.send');
    expect(全部).toContain('room.handoff');
    expect(全部).toContain('inbox.open.raw');
    expect(全部.length).toBeGreaterThanOrEqual(20);
  });

  it.each(LOCALES)('%s: 口が聞く動作には、どれも言い方があり、内部名を出さない', (言葉) => {
    const t = (k: MessageKey) => MESSAGES[言葉][k];
    for (const 動作 of 動作たち()) {
      const 出す = 動作の言葉(動作, t);
      expect(出す, 動作).not.toBe(動作);
      expect(出す, 動作).not.toMatch(/^[a-z]+\.[a-z_.]+$/);
    }
  });

  it('知らない動作（新しい口）は、隠さずに内部名のまま出す', () => {
    const t = (k: MessageKey) => MESSAGES.ja[k];
    expect(動作の言葉('foo.bar', t)).toBe('foo.bar');
  });
});
