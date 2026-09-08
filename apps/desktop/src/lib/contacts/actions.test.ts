import { describe, expect, it } from 'vitest';

import { できること, type 相手 } from './actions';

const 人: 相手 = {
  種類: '人',
  住所を覚えている: true,
  いま会議に居る: false,
  机に着いている: false,
};

function 引く(相手: 相手, 種類: 'chat' | 'call' | 'mail') {
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

  it('住所を覚えていない相手には、チャットも押せない', () => {
    // **繋いでからでないと文字は流れない**（会議の中の知らせとして飛ぶ）
    const 知らない = { ...人, 住所を覚えている: false };
    expect(引く(知らない, 'chat').押せる).toBe(false);
  });

  it('住所を覚えている相手には、チャットも会議も押せる', () => {
    expect(引く(人, 'chat').押せる).toBe(true);
    expect(引く(人, 'call').押せる).toBe(true);
  });

  it('いま会議に居る相手を、もう一度会議に呼ばない', () => {
    const 居る = { ...人, いま会議に居る: true };
    expect(引く(居る, 'call').押せる).toBe(false);
    expect(引く(居る, 'call').訳).toBe('act.already');
  });

  it('いま会議に居る相手には、住所を知らなくてもチャットできる', () => {
    const 居る = { ...人, 住所を覚えている: false, いま会議に居る: true };
    expect(引く(居る, 'chat').押せる).toBe(true);
  });

  it('この PC の AI は、机に着いていればチャットできる', () => {
    const ai: 相手 = { 種類: 'AI', 住所を覚えている: false, いま会議に居る: false, 机に着いている: true };
    expect(引く(ai, 'chat').押せる).toBe(true);
  });

  it('机に誰も着いていなければ、この PC の AI にも送れない', () => {
    const ai: 相手 = { 種類: 'AI', 住所を覚えている: false, いま会議に居る: false, 机に着いている: false };
    expect(引く(ai, 'chat').押せる).toBe(false);
    expect(引く(ai, 'chat').訳).toBe('act.desk.empty');
  });

  it('この PC の AI を「会議に呼ぶ」口は押せない（同じ机に着いている）', () => {
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
  it('住所を知らない相手にも、チャットは押せる', () => {
    // **預けるのに要るのは公開鍵だけ**（封は相手しか開けられない・D71）
    const 口たち = できること({
      種類: '人',
      住所を覚えている: false,
      いま会議に居る: false,
      机に着いている: false,
      預かり所がある: true,
    });
    expect(口たち.find((k) => k.種類 === 'chat')?.押せる).toBe(true);
  });

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
