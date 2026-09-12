import { describe, expect, it } from 'vitest';

import {
  入退室の知らせ,
  話の記録,
  送ってよい,
  いま時刻,
  差出人の顔,
  type 会話行,
} from './announce';

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

describe('改行と送信を分ける', () => {
  it('Shift+Enter では送らない（改行にする）', () => {
    // **2026-09-06 のオーナー要望**「改行できたらもっといいかな。シフトエンターとか？」
    expect(送ってよい({ key: 'Enter', shiftKey: true })).toBe(false);
  });

  it('Option（Alt）+Enter でも送らない', () => {
    // **人によって指が覚えている組み合わせが違う。**どちらも受ける
    expect(送ってよい({ key: 'Enter', altKey: true })).toBe(false);
  });

  it('修飾なしの Enter は送る', () => {
    expect(送ってよい({ key: 'Enter', shiftKey: false, altKey: false })).toBe(true);
  });

  it('変換中は、Shift を押していても送らない', () => {
    expect(送ってよい({ key: 'Enter', isComposing: true, shiftKey: true })).toBe(false);
  });
});

describe('差出人の顔', () => {
  // オーナー・2026-09-12「`6X7BDCXBJ3DW…` `KBN2GCQO35W…` これ、どなたなのか
  // わかりにくいのどうにかできないですかね。」
  //
  // **顔は連絡帳と同じものを出す**（`contacts/Avatar.svelte`）。
  // ここで別の絵を作ると、**同じ人が場所によって違う見た目になる。**
  const 鍵 = '6X7BDCXBJ3DWQ2HFVLTNZKPMS4YAEUGIR5OJC7XW3BNQHLFT2IDA';

  it('相手の発言には、その鍵の顔を出す', () => {
    expect(差出人の顔({ who: '6X7BDCXBJ3DW…', body: 'やあ', mine: false, 顔の種: 鍵 })).toBe(鍵);
  });

  it('この機械のエージェントには、連絡帳と同じ種を使う（`desk:` 付き）', () => {
    expect(
      差出人の顔({ who: 'zumen のエージェント', body: '直しました', mine: false, agent: true, 顔の種: 'desk:zumen のエージェント' })
    ).toBe('desk:zumen のエージェント');
  });

  it('**会議からの知らせには顔を出さない**（人ではない）', () => {
    expect(差出人の顔({ who: '', body: 'B役 が入りました', mine: false, system: true, 顔の種: 鍵 })).toBe(
      null
    );
  });

  it('**自分の発言には出さない**（自分が誰かは分かっている）', () => {
    expect(差出人の顔({ who: 'あなた', body: 'はい', mine: true, 顔の種: 鍵 })).toBe(null);
  });

  it('**種が無ければ顔を作らない**（知らない相手を、知っているように見せない）', () => {
    expect(差出人の顔({ who: '誰か', body: 'やあ', mine: false })).toBe(null);
  });
});
