import { describe, expect, it } from 'vitest';

import { 入退室の知らせ, 話の記録, type 会話行 } from './announce';

describe('入退室の知らせ', () => {
  it('入った人を、チャット欄に出す行にする', () => {
    // **名簿が動くだけでは、見ていない間に誰が来たか分からない**
    const 行 = 入退室の知らせ('入室', 'ABCDEFGH…', (k, v) => `${v.who} が${k === 'joined' ? '入室' : '退室'}しました`);
    expect(行.system).toBe(true);
    expect(行.body).toContain('ABCDEFGH…');
    expect(行.body).toContain('入室');
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
