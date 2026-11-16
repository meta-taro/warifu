import { describe, expect, it } from 'vitest';
import { CLIの知らせ } from './cli';

describe('CLIの知らせ', () => {
  // 2026-09-12 —— 画面だけ上げると CLI は古いまま残る。症状は「繋がらない」だけなので、
  // CLI が古いせいだと気づけない。

  it('**古い CLI は、場所と版を名指しで言う**', () => {
    expect(CLIの知らせ({ 古い: { 場所: '/Users/x/.local/bin/warifu', 版: '0.1.2' } })).toEqual({
      鍵: 'cli.old',
      場所: '/Users/x/.local/bin/warifu',
      版: '0.1.2',
    });
  });

  it('**束に CLI が入っている画面は、古い CLI を入れ替えられると言う**（内部の Issue 021）', () => {
    expect(CLIの知らせ({ 古い: { 場所: '/Users/x/.local/bin/warifu', 版: '0.1.2' } }, true)).toEqual({
      鍵: 'cli.oldReplaceable',
      場所: '/Users/x/.local/bin/warifu',
      版: '0.1.2',
      置ける: true,
    });
  });

  it('入れ替えられても、新しい CLI は入れ替えない（画面を上げてもらう）', () => {
    expect(CLIの知らせ({ 新しい: { 場所: '/usr/local/bin/warifu', 版: '0.1.5' } }, true)).toEqual({
      鍵: 'cli.ahead',
      場所: '/usr/local/bin/warifu',
      版: '0.1.5',
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

  it('画面の中のコマンドが古いまま残ったら、入れ直しを頼む（入れ替えの押し所は出さない）', () => {
    const 場所 = 'C:/Users/x/AppData/Local/warifu/cli/warifu.exe';
    expect(CLIの知らせ({ 束が古い: { 場所, 版: '0.1.46' } }, true)).toEqual({
      鍵: 'cli.bundleStale',
      場所,
      版: '0.1.46',
    });
  });
});
