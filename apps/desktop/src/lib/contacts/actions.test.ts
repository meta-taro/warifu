import { describe, expect, it } from 'vitest';

import { できること, type 相手 } from './actions';

const 人: 相手 = {
  種類: '人',
  住所を覚えている: true,
  いま会議に居る: false,
  机に着いている: false,
};

function 引く(相手: 相手, 種類: 'call' | 'mail') {
  const 見つけた = できること(相手).find((k) => k.種類 === 種類);
  if (!見つけた) throw new Error(`${種類} の口が無い`);
  return 見つけた;
}

describe('選んだ相手に出す口（D49）', () => {
  it('メールは、どの相手でも押せない', () => {
    // **送る経路が 1 本も無い**（SMTP はどこにも実装されていない）。
    // 押せる形にして断るのは、押した人には「壊れている」としか見えない
    expect(引く(人, 'mail').押せる).toBe(false);
    expect(引く({ ...人, 種類: 'AI', 机に着いている: true }, 'mail').押せる).toBe(false);
  });

  it('メールが押せない理由を必ず持つ', () => {
    expect(引く(人, 'mail').訳).toBe('act.mail.none');
  });

  it('住所を覚えていない相手は、こちらから呼べない', () => {
    const 知らない = { ...人, 住所を覚えている: false };
    expect(引く(知らない, 'call').押せる).toBe(false);
    expect(引く(知らない, 'call').訳).toBe('act.address.none');
  });

  it('文字を打つ口は、そもそも出さない', () => {
    // **行を選んだ時点で、その相手との会話は開いている**
    // （2026-09-08 オーナー指摘「これを押したら何が起こるかわかりません」）。
    // 押しても何も起きない口は、**誤解しか生まない**
    for (const 相手 of [人, { ...人, 住所を覚えている: false }]) {
      expect(できること(相手).map((k) => k.種類)).toEqual(['call', 'mail']);
    }
  });

  it('いま会議に居る相手を、もう一度会議に呼ばない', () => {
    const 居る = { ...人, いま会議に居る: true };
    expect(引く(居る, 'call').押せる).toBe(false);
    expect(引く(居る, 'call').訳).toBe('act.already');
  });

  it('エージェントの行にも、文字を打つ口は出さない', () => {
    const ai: 相手 = {
      種類: 'AI',
      住所を覚えている: false,
      いま会議に居る: false,
      机に着いている: true,
    };
    expect(できること(ai).map((k) => k.種類)).toEqual(['call', 'mail']);
  });

  it('マイ PC エージェント を「会議に呼ぶ」口は押せない（同じ机に着いている）', () => {
    const ai: 相手 = { 種類: 'AI', 住所を覚えている: false, いま会議に居る: false, 机に着いている: true };
    expect(引く(ai, 'call').押せる).toBe(false);
    expect(引く(ai, 'call').訳).toBe('act.desk.local');
  });

  it('自分自身には、どの口も出さない', () => {
    const 自分: 相手 = { 種類: '自分', 住所を覚えている: true, いま会議に居る: true, 机に着いている: true };
    expect(できること(自分)).toEqual([]);
  });

  it('押せない口には、必ず理由が付く', () => {
    // **理由の無い「押せない」は、壊れているとしか見えない**
    const 全部: 相手[] = [
      人,
      { ...人, 住所を覚えている: false },
      { ...人, いま会議に居る: true },
      { 種類: 'AI', 住所を覚えている: false, いま会議に居る: false, 机に着いている: false },
      { 種類: 'AI', 住所を覚えている: false, いま会議に居る: false, 机に着いている: true },
    ];
    for (const 一人 of 全部) {
      for (const 口 of できること(一人)) {
        if (!口.押せる) expect(口.訳, `${一人.種類}/${口.種類}`).not.toBeNull();
        else expect(口.訳, `${一人.種類}/${口.種類}`).toBeNull();
      }
    }
  });

  it('押せない理由に、相手が起動しているかを混ぜない', () => {
    // **在席は分からない**（`issues/010` 段 4 は後回し）。
    // 分からないものを分かるように見せない（DESIGN §2 原則 7）。
    // ここは理由の全集合を固定して縛る見張りである
    const 出うる理由 = new Set(['act.mail.none', 'act.address.none', 'act.already', 'act.desk.local', 'act.desk.empty']);
    const 全部: 相手[] = [
      人,
      { ...人, 住所を覚えている: false },
      { ...人, いま会議に居る: true },
      { 種類: 'AI', 住所を覚えている: false, いま会議に居る: false, 机に着いている: false },
    ];
    for (const 一人 of 全部) {
      for (const 口 of できること(一人)) {
        if (口.訳) expect(出うる理由.has(口.訳), `知らない理由: ${口.訳}`).toBe(true);
      }
    }
  });
});

describe('部屋', () => {
  it('部屋には、どの口も出さない', () => {
    // **部屋は押して見るもの。**部屋そのものに何かする口は無い
    const 部屋: 相手 = {
      種類: '部屋',
      住所を覚えている: false,
      いま会議に居る: true,
      机に着いている: false,
    };
    expect(できること(部屋)).toEqual([]);
  });
});

describe('預かり所を置いているとき', () => {
  it('会議には呼べない（いま繋がっていないと始まらない）', () => {
    const 口たち = できること({
      種類: '人',
      住所を覚えている: false,
      いま会議に居る: false,
      机に着いている: false,
      預かり所がある: true,
    });
    const call = 口たち.find((k) => k.種類 === 'call');
    expect(call?.押せる).toBe(false);
    expect(call?.訳).toBe('act.address.none');
  });
});
