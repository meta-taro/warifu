import { describe, expect, it } from 'vitest';

import { できること, 札の並び, type 口の種類, type 相手 } from './actions';

const 人: 相手 = {
  種類: '人',
  住所を覚えている: true,
  いま会議に居る: false,
  この機械につながっている: false,
};

const ai: 相手 = {
  種類: 'AI',
  住所を覚えている: false,
  いま会議に居る: false,
  この機械につながっている: true,
};

function 引く(相手: 相手, 種類: 口の種類) {
  const 見つけた = できること(相手).find((k) => k.種類 === 種類);
  if (!見つけた) throw new Error(`${種類} の札が無い`);
  return 見つけた;
}

describe('選んだ相手に出す札（ダッシュボード・D49）', () => {
  it('札は 4 枚、並びも固定', () => {
    // **押すたびに位置が変わると探させる。**
    // 「チャットをするのか、グループチャットをするのか、かれんだーで予定を
    // みるのか、ビデオ会議を開始するのか」（オーナー・2026-09-10）
    expect(札の並び).toEqual(['chat', 'group', 'call', 'calendar']);
    for (const 相手 of [人, { ...人, 住所を覚えている: false }, ai]) {
      expect(できること(相手).map((k) => k.種類)).toEqual([...札の並び]);
    }
  });

  it('どの札にも、状態と 1 行の訳が付く', () => {
    // **訳の無い札は、使う人には壊れているとしか見えない**
    for (const 相手 of [人, { ...人, いま会議に居る: true }, ai, { ...人, 預かり所がある: true }]) {
      for (const 口 of できること(相手)) {
        expect(口.訳, `${相手.種類}/${口.種類}`).toBeTruthy();
        expect(['できる', '条件つき', 'まだできない']).toContain(口.状態);
      }
    }
  });

  it('予定は、どの相手でも「まだできない」', () => {
    // 画面から予定表を読む口が、**まだ 1 本も無い**
    for (const 相手 of [人, ai]) {
      expect(引く(相手, 'calendar').状態).toBe('まだできない');
      expect(引く(相手, 'calendar').押せる).toBe(false);
      expect(引く(相手, 'calendar').訳).toBe('act.calendar.none');
    }
  });

  it('チャットの札は押せない（行を選んだ時点で会話は開いている）', () => {
    // 2026-09-08 オーナー指摘「これを押したら何が起こるかわかりません」
    for (const 相手 of [人, { ...人, 預かり所がある: true }, ai]) {
      expect(引く(相手, 'chat').押せる).toBe(false);
    }
  });

  it('預かり所が無ければ、チャットは「条件つき」', () => {
    // **繋がっている間だけ届く。**そこを黙ると、打ったのに消える
    expect(引く(人, 'chat').状態).toBe('条件つき');
    expect(引く(人, 'chat').訳).toBe('act.chat.live');
  });

  it('預かり所があれば、チャットは「できる」', () => {
    // 相手が起動していなくても、封のまま預かる（**D71**）
    const 置いてある = { ...人, 預かり所がある: true };
    expect(引く(置いてある, 'chat').状態).toBe('できる');
    expect(引く(置いてある, 'chat').訳).toBe('act.chat.postbox');
  });

  it('住所を覚えていない相手は、こちらから呼べない', () => {
    const 知らない = { ...人, 住所を覚えている: false };
    expect(引く(知らない, 'call').押せる).toBe(false);
    expect(引く(知らない, 'call').状態).toBe('まだできない');
    expect(引く(知らない, 'call').訳).toBe('act.address.none');
  });

  it('呼べる相手でも、ビデオ会議は「条件つき」', () => {
    // **別の網を越えて繋がったことを、一度も見ていない**（実測 0 件）。
    // 分からないものを「できる」と書かない（DESIGN §2 原則 7）
    expect(引く(人, 'call').押せる).toBe(true);
    expect(引く(人, 'call').状態).toBe('条件つき');
    expect(引く(人, 'call').訳).toBe('act.call.net');
  });

  it('いま会議に居る相手を、もう一度会議に呼ばない', () => {
    const 居る = { ...人, いま会議に居る: true };
    expect(引く(居る, 'call').押せる).toBe(false);
    expect(引く(居る, 'call').訳).toBe('act.already');
  });

  it('マイ PC エージェント を「会議に呼ぶ」札は押せない（同じこの機械につながっている）', () => {
    expect(引く(ai, 'call').押せる).toBe(false);
    expect(引く(ai, 'call').状態).toBe('できる');
    expect(引く(ai, 'call').訳).toBe('act.desk.local');
  });

  it('自分自身には、どの札も出さない', () => {
    const 自分: 相手 = { 種類: '自分', 住所を覚えている: true, いま会議に居る: true, この機械につながっている: true };
    expect(できること(自分)).toEqual([]);
  });

  it('押せる札は、グループチャットとビデオ会議だけ', () => {
    // **押しても何も起きない口を作らない。**
    // チャットは既に開いていて、予定はまだ無い
    expect(できること(人).filter((k) => k.押せる).map((k) => k.種類)).toEqual(['group', 'call']);
  });

  it('訳に、相手が起動しているかを混ぜない', () => {
    // **在エージェントは分からない**（`issues/010` 段 4 は後回し）。
    // ここは訳の全集合を固定して縛る見張りである
    const 出うる訳 = new Set([
      'act.chat.live',
      'act.chat.postbox',
      'act.chat.desk',
      'act.chat.desk.none',
      'act.group.desk.none',
      'act.call.desk.none',
      'act.group.what',
      'act.call.net',
      'act.address.none',
      'act.already',
      'act.desk.local',
      'act.calendar.none',
    ]);
    const 全部: 相手[] = [
      人,
      { ...人, 住所を覚えている: false },
      { ...人, いま会議に居る: true },
      { ...人, 預かり所がある: true },
      ai,
      { ...ai, この機械につながっている: false },
    ];
    for (const 一人 of 全部) {
      for (const 口 of できること(一人)) {
        expect(出うる訳.has(口.訳), `知らない訳: ${口.訳}`).toBe(true);
      }
    }
  });
});

describe('ルーム', () => {
  it('ルームには、どの札も出さない', () => {
    // **ルームは押して見るもの。**ルームそのものに何かする口は無い
    const ルーム: 相手 = {
      種類: 'ルーム',
      住所を覚えている: false,
      いま会議に居る: true,
      この機械につながっている: false,
    };
    expect(できること(ルーム)).toEqual([]);
  });
});

describe('預かり所を置いているとき', () => {
  it('会議には呼べない（いま繋がっていないと始まらない）', () => {
    const call = 引く(
      { ...人, 住所を覚えている: false, 預かり所がある: true },
      'call',
    );
    expect(call.押せる).toBe(false);
    expect(call.訳).toBe('act.address.none');
  });

  it('繋がっていないエージェントにも 4 枚出す（ぜんぶ「まだできない」）', () => {
    // オーナー指摘（2026-09-10）—— 「**なにもかわってないですけど**」
    // 「**身内 PC ではでないです**」
    //
    // 隠していたのは、押せないボタンに**繋がっていないときの理由ではない訳**を
    // 添えていたからである。**いまは札ごとに訳を持つ**（D88）ので隠さない
    const 口たち = できること({ ...ai, この機械につながっている: false });
    expect(口たち.map((k) => k.種類)).toEqual([...札の並び]);
    expect(口たち.every((k) => k.状態 === 'まだできない')).toBe(true);
    expect(口たち.every((k) => !k.押せる)).toBe(true);
    expect(口たち.map((k) => k.訳)).toEqual([
      'act.chat.desk.none',
      'act.group.desk.none',
      'act.call.desk.none',
      'act.calendar.none',
    ]);
  });
});
