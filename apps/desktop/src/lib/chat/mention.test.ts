import { describe, expect, it } from 'vitest';

import {
  写しか,
  写しの名,
  呼べる名に広げる,
  候補を入れる,
  候補を出す,
  宛ての名,
  宛てられたか,
  本文を分ける,
  頭の呼びを外す
} from './mention';

const 名 = ['連動', 'docs', 'docs のエージェント', '人A'];

describe('呼ばれたか', () => {
  it('@名前 があれば呼んでいる', () => {
    expect(宛てられたか('@連動 これ見て', 名)).toBe(true);
    expect(宛てられたか('確認お願い @docs', 名)).toBe(true);
  });

  it('名前の続きが英数字なら別の名前（Rust の規則と同じ）', () => {
    expect(宛てられたか('@docs2 へ', 名)).toBe(false);
  });

  it('@ が無ければ呼んでいない', () => {
    expect(宛てられたか('連動 はどう？', 名)).toBe(false);
  });

  it('1 文字の名前は見ない', () => {
    expect(宛てられたか('@a', ['a'])).toBe(false);
  });

  it('長い名前を先に当てる', () => {
    expect(宛ての名('@docs のエージェント 見て', 名)).toEqual(['docs のエージェント']);
  });

  it('同じ人を 2 度呼んでも 1 人', () => {
    expect(宛ての名('@連動 と @人A と @連動', 名)).toEqual(['連動', '人A']);
  });
});

describe('本文を分ける', () => {
  it('@名前 だけを切り出す', () => {
    expect(本文を分ける('やあ @連動 見て', 名)).toEqual([
      { 文: 'やあ ' },
      { 文: '@連動', 呼び: '連動' },
      { 文: ' 見て' }
    ]);
  });

  it('知らない @ はそのまま', () => {
    expect(本文を分ける('a@example.com', 名)).toEqual([{ 文: 'a@example.com' }]);
  });

  it('名前が無ければ 1 切れ', () => {
    expect(本文を分ける('@連動', [])).toEqual([{ 文: '@連動' }]);
  });
});

describe('候補', () => {
  it('@ のあとに打った頭で絞る', () => {
    expect(候補を出す('ねえ @do', 6, 名)).toEqual({ 頭: 3, 候補: ['docs', 'docs のエージェント'] });
  });

  it('@ だけなら全員（名前 4 つと、全員を呼ぶ all・全員）', () => {
    expect(候補を出す('@', 1, 名)?.候補.length).toBe(6);
  });

  it('人数が多くても、@ だけなら全員を出す（6 人で切らない）', () => {
    // 11-12、ルームの全員が候補に出ず、名前を当てずっぽうに打つしかなかった
    const 大勢 = Array.from({ length: 14 }, (_, i) => `席${String(i).padStart(2, '0')}`);
    expect(候補を出す('@', 1, 大勢)?.候補.length).toBe(14 + 2);
  });

  it('語の途中の @ は候補を出さない（メールの番地）', () => {
    expect(候補を出す('a@d', 3, 名)).toBeNull();
  });

  it('空白を打ったら閉じる', () => {
    expect(候補を出す('@連動 ', 4, 名)).toBeNull();
  });

  it('選ぶと @名前 が入り、空白を重ねない', () => {
    expect(候補を入れる('ねえ @do 見て', 6, 3, 'docs')).toEqual({
      下書き: 'ねえ @docs 見て',
      位置: 9
    });
  });

  it('末尾で選ぶと、空白を足して続けて打てる', () => {
    expect(候補を入れる('@連', 2, 0, '連動')).toEqual({ 下書き: '@連動 ', 位置: 4 });
  });
});

describe('呼べる名に広げる', () => {
  it('名札「名前（場所）」は、名前だけでも呼べる', () => {
    // 括弧の中（場所・機械の名前）でも呼べる（11-12・機械ごとのメンション）
    expect(呼べる名に広げる(['連動（docs）', '人A'])).toEqual(['連動（docs）', '連動', 'docs', '人A']);
    // エージェントは名札（`probe-b`）で呼ばれる。画面の「probe-b のエージェント」からも引けるようにする
    expect(呼べる名に広げる(['probe-b のエージェント'])).toEqual(['probe-b のエージェント', 'probe-b']);
  });

  it('名札が英数字の語で始まるなら、その語（フォルダ名）でも呼べる', () => {
    // 席はフォルダ名（`docs-site`）でも起きる。画面だけが見分けず、→ の札に出なかった
    const 名 = 呼べる名に広げる(['docs-site 担当AI（B の PC）']);
    expect(名).toContain('docs-site');
    expect(宛ての名('@docs-site（地図） @人A 見て', [...名, '人A'])).toEqual(['docs-site', '人A']);
    // 日本語で始まる名札は広げない
    expect(呼べる名に広げる(['クロちゃん 相談役'])).toEqual(['クロちゃん 相談役']);
  });

  it('広げた名前で、短く呼んだ所も見分ける', () => {
    expect(本文を分ける('@連動 確認', 呼べる名に広げる(['連動（docs）']))).toEqual([
      { 文: '@連動', 呼び: '連動' },
      { 文: ' 確認' }
    ]);
  });
});

describe('全角の ＠', () => {
  it('日本語入力の ＠ でも呼べる', () => {
    expect(宛てられたか('＠連動 見て', 名)).toBe(true);
    expect(本文を分ける('＠連動', 名)).toEqual([{ 文: '＠連動', 呼び: '連動' }]);
  });

  it('＠ を打っても候補が出て、選ぶと半角の @ になる', () => {
    expect(候補を出す('＠連', 2, 名)).toEqual({ 頭: 0, 候補: ['連動'] });
    expect(候補を入れる('＠連', 2, 0, '連動')).toEqual({ 下書き: '@連動 ', 位置: 4 });
  });
});

describe('全員を呼ぶ', () => {
  it('@all と @全員 は誰にも当たる', () => {
    expect(宛てられたか('@all 版を上げてください', ['連動'])).toBe(true);
    expect(宛てられたか('＠全員 見て', ['連動'])).toBe(true);
    expect(宛てられたか('@ALL 見て', [])).toBe(true);
    expect(宛てられたか('@allow を付ける', ['連動'])).toBe(false);
  });
  it('目立たせる切れになる', () => {
    expect(本文を分ける('@全員 確認', ['連動'])).toEqual([{ 文: '@全員', 呼び: '全員' }, { 文: ' 確認' }]);
  });
  it('候補に出る', () => {
    expect(候補を出す('@全', 2, ['連動'])?.候補).toEqual(['全員']);
  });
});

describe('頭の呼びを外す（0.1.45・宛ての行に出すので本文で繰り返さない）', () => {
  const 名 = ['PdM（ドスパラ）', 'PdM', '連動'];
  it('頭に並んだ @名 だけを外す', () => {
    expect(頭の呼びを外す('@PdM（ドスパラ） dbboard です。', 名)).toBe('dbboard です。');
    expect(頭の呼びを外す('@PdM @連動 2 件です', 名)).toBe('2 件です');
    expect(頭の呼びを外す('＠全員 確認を', 名)).toBe('確認を');
  });
  it('文の途中の @名 は残す', () => {
    expect(頭の呼びを外す('確認お願い @連動', 名)).toBe('確認お願い @連動');
  });
  it('知らない @ で始まるなら外さない', () => {
    expect(頭の呼びを外す('@だれか こんにちは', 名)).toBe('@だれか こんにちは');
  });
  it('呼びしか無いなら、そのまま', () => {
    expect(頭の呼びを外す('@PdM', 名)).toBe('@PdM');
  });
});

describe('コードで囲んだ所の呼びは数えない（0.1.46）', () => {
  it('説明の中で触れただけなら呼んでいない', () => {
    expect(宛てられたか('PdM の `@all`（#675）では起きました', ['連動'])).toBe(false);
    expect(宛てられたか('`@連動` で呼ぶと当たります', ['連動'])).toBe(false);
    expect(宛てられたか('```\n@all\n```', ['連動'])).toBe(false);
  });
  it('囲みの外なら呼んでいる', () => {
    expect(宛ての名('@連動 `@all` の件です', ['連動'])).toEqual(['連動']);
    expect(宛てられたか('`コード` のあとで @all', ['連動'])).toBe(true);
  });
});

describe('cc@名前 は写し（0.1.47・メールの CC）', () => {
  const 名 = ['PdM', '連動', 'md-business'];
  it('cc@ で呼んだ相手は宛て先でなく写し', () => {
    expect(宛ての名('@連動 直して cc@PdM cc@md-business', 名)).toEqual(['連動']);
    expect(写しの名('@連動 直して cc@PdM cc@md-business', 名)).toEqual(['PdM', 'md-business']);
    expect(宛てられたか('@連動 直して cc@PdM', ['PdM'])).toBe(false);
    expect(写しか('@連動 直して cc@PdM', ['PdM'])).toBe(true);
  });
  it('大文字・全角の ＠ も写し', () => {
    expect(写しか('CC@PdM 念のため', ['PdM'])).toBe(true);
    expect(写しか('Cc＠PdM 念のため', ['PdM'])).toBe(true);
  });
  it('宛て先にも書かれたら宛て先が勝つ', () => {
    expect(宛てられたか('@PdM 見て cc@PdM', ['PdM'])).toBe(true);
    expect(写しか('@PdM 見て cc@PdM', ['PdM'])).toBe(false);
    expect(写しの名('@PdM 見て cc@PdM', ['PdM'])).toEqual([]);
  });
  it('cc の前が英数字なら印ではない', () => {
    expect(宛てられたか('acc@PdM', ['PdM'])).toBe(true);
    expect(写しか('acc@PdM', ['PdM'])).toBe(false);
  });
  it('cc@all は全員が写し', () => {
    expect(写しか('版を上げました cc@all', ['連動'])).toBe(true);
    expect(宛てられたか('版を上げました cc@all', ['連動'])).toBe(false);
  });
  it('本文を分けると cc まで含めた 1 切れになる', () => {
    expect(本文を分ける('見て cc@PdM', 名)).toEqual([{ 文: '見て ' }, { 文: 'cc@PdM', 呼び: 'PdM', 写し: true }]);
  });
  it('頭に並んだ宛てと写しは外す', () => {
    expect(頭の呼びを外す('@連動 cc@PdM 直して', 名)).toBe('直して');
  });
  it('cc@ を打ちかけても候補が出て、cc は残る', () => {
    const 出た = 候補を出す('見て cc@P', 7, 名);
    expect(出た).toEqual({ 頭: 5, 候補: ['PdM'] });
    expect(候補を入れる('見て cc@P', 7, 5, 'PdM').下書き).toBe('見て cc@PdM ');
  });
});

describe('機械ごとのメンション', () => {
  it('席の名前の括弧の中（機械の名前）でも呼べて、英字の大文字・小文字は問わない', () => {
    // 11-12、機械ごとのメンションを足した
    const 名 = 呼べる名に広げる(['dbboard（mac mini）', '割符（mac mini）（warifu）', 'sshboard（mac air）']);
    expect(名).toContain('mac mini');
    expect(名).toContain('warifu');
    expect(宛ての名('@Mac mini 網を見て', 名)).toEqual(['mac mini']);
    expect(宛ての名('@mac air どう？', 名)).toEqual(['mac air']);
  });
});
