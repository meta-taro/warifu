import { describe, expect, it } from 'vitest';

import { 既定の札, 口を足す, 起こす } from './connect';

describe('つなぎ方（グレーを緑にする）', () => {
  it('既定で許すのは、会話と自分の席の名乗りだけ', () => {
    // **受信箱も予定表も許さない**（要るなら人が足す・D56）
    expect([...既定の札]).toEqual(['chat.send', 'chat.read', 'profile.write']);
  });

  it('口を足すコマンドは、札を並べて出す', () => {
    expect(口を足す()).toBe(
      'claude mcp add warifu --scope user -- warifu mcp --allow chat.send --allow chat.read --allow profile.write',
    );
  });

  it('実体の場所を渡せる（PATH に無いことがある）', () => {
    expect(口を足す('/Applications/warifu.app/Contents/MacOS/warifu-cli')).toContain(
      '-- /Applications/warifu.app/Contents/MacOS/warifu-cli mcp',
    );
  });

  it('起こすコマンドは、席の名前を入れる', () => {
    expect(起こす('zumen')).toBe("warifu agent --as zumen --on 'claude -p'");
  });

  it('席の名前に使えない字が入っていても、そのまま埋めない', () => {
    // **命令は `'` で囲んで渡す。**引用を壊す字を入れない
    expect(起こす("zu'men; rm -rf /")).toBe("warifu agent --as zumenrm-rf --on 'claude -p'");
    expect(起こす('図面')).toBe("warifu agent --as agent --on 'claude -p'");
  });
});
