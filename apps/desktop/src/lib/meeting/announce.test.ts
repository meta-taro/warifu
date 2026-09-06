import { describe, expect, it } from 'vitest';

import { 入退室の知らせ, 話の記録, 送ってよい, いま時刻, type 会話行 } from './announce';

describe('入退室の知らせ', () => {
  it('入った人を、チャット欄に出す行にする', () => {
    // **名簿が動くだけでは、見ていない間に誰が来たか分からない**
    const 行 = 入退室の知らせ('入室', 'ABCDEFGH…', (k, v) => `${v.who} が${k === 'joined' ? '入室' : '退室'}しました`);
    expect(行.system).toBe(true);
    expect(行.body).toContain('ABCDEFGH…');
    expect(行.body).toContain('入室');
  });

  it('落ちた相手を「退室しました」と書かない', () => {
    // **実物で食い違っていた**（2026-09-04・画面を見て出た）。
    // 知らせの帯は「経路が切れました」と出しているのに、
    // チャット欄の同じ相手が「退室しました」になっていた。
    // 退室は本人の意思、切断は事故 —— **人は前者を待たないが、後者は待つ。**
    const 鍵 = { joined: 'j', left: 'l', lost: 'x' } as const;
    const 退室 = 入退室の知らせ('退室', 'ABCDEFGH…', (k, v) => `${v.who}:${鍵[k]}`);
    const 切断 = 入退室の知らせ('切断', 'ABCDEFGH…', (k, v) => `${v.who}:${鍵[k]}`);
    expect(切断.body).not.toBe(退室.body);
    expect(切断.body).toContain('x');
    expect(切断.system).toBe(true);
  });

  it('知らせは自分の発言ではない', () => {
    const 行 = 入退室の知らせ('入室', 'X', () => 'X が入室しました');
    expect(行.mine).toBe(false);
  });
});

describe('話の記録', () => {
  it('中身を書かない。長さと相手だけ', () => {
    // **チャットの中身をログへ出さない。**会議の中身が warifu.log に残る
    expect(話の記録('送信', 'ABCDEFGHIJKL…', 'こんにちは')).toBe(
      '送信: 文字（5 文字）を ABCDEFGHIJKL… へ',
    );
    expect(話の記録('受信', 'ABCDEFGHIJKL…', 'ながいながいながい')).toBe(
      '受信: 文字（9 文字）を ABCDEFGHIJKL… から',
    );
  });

  it('中身そのものは、どこにも出さない', () => {
    const 秘密 = 'これは会議の中身です';
    expect(話の記録('受信', 'X', 秘密)).not.toContain(秘密);
  });
});

describe('会話行', () => {
  it('人の発言と、知らせを見分けられる', () => {
    const 発言: 会話行 = { who: '自分', body: 'やあ', mine: true };
    expect(発言.system).toBeUndefined();
  });
});

describe('Enter で送ってよいか', () => {
  it('変換確定の Enter では送らない', () => {
    // **2026-09-06 にオーナーが実際に踏んだ。**
    // 「まって、」「あと」「エンターで」が変換のたびに別々の発言として飛んだ
    expect(送ってよい({ key: 'Enter', isComposing: true })).toBe(false);
  });

  it('isComposing を出さない WebView でも、229 で分かる', () => {
    // 古い WebView は isComposing を出さない。**保険を外さない**
    expect(送ってよい({ key: 'Enter', keyCode: 229 })).toBe(false);
  });

  it('変換していない Enter では送る', () => {
    expect(送ってよい({ key: 'Enter', isComposing: false, keyCode: 13 })).toBe(true);
  });

  it('Enter 以外では送らない', () => {
    expect(送ってよい({ key: 'a' })).toBe(false);
  });
});

describe('いま時刻', () => {
  it('HH:MM で出す。秒は出さない', () => {
    // **無いと、あとから読み返せない**（2026-09-06 の「何時に投稿したかわからないです」）
    expect(いま時刻(new Date(2026, 8, 6, 9, 5))).toBe('09:05');
    expect(いま時刻(new Date(2026, 8, 6, 23, 59))).toBe('23:59');
  });
});
