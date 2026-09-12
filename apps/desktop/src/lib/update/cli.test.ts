import { describe, expect, it } from 'vitest';
import { CLIの知らせ } from './cli';

describe('CLIの知らせ', () => {
  // Mac Air のエージェント・2026-09-12 ——
  // 「画面だけ上げた人は直ったつもりで直っていません。しかも症状は『繋がらない』
  //  だけなので、CLI が古いせいだと気づけません。」

  it('**古い CLI は、場所と版を名指しで言う**', () => {
    expect(CLIの知らせ({ 古い: { 場所: '/Users/x/.local/bin/warifu', 版: '0.1.2' } })).toEqual({
      鍵: 'cli.old',
      場所: '/Users/x/.local/bin/warifu',
      版: '0.1.2',
    });
  });

  it('画面のほうが古いときも言う（画面を上げ忘れている）', () => {
    expect(CLIの知らせ({ 新しい: { 場所: '/usr/local/bin/warifu', 版: '0.1.5' } })).toEqual({
      鍵: 'cli.ahead',
      場所: '/usr/local/bin/warifu',
      版: '0.1.5',
    });
  });

  it('同じ版なら何も言わない（黙っているのが正しい）', () => {
    expect(CLIの知らせ({ 同じ: { 場所: '/usr/local/bin/warifu', 版: '0.1.4' } })).toBe(null);
  });

  it('**CLI が無いのは不具合ではない**（入れていない人が普通）', () => {
    expect(CLIの知らせ('無い')).toBe(null);
  });

  it('**版が読めないときは決めつけない**（入れ直しても消えない警告を作らない）', () => {
    expect(CLIの知らせ({ 読めない: { 場所: '/x/warifu', 出たもの: 'command not found' } })).toBe(
      null
    );
  });
});
