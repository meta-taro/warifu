// **`@名前` で呼ぶ**（0.1.35・メンション）。
//
// 人数が多いと、誰に話しかけたのかが分からない。**呼ぶのは本文の中の `@名前` だけ** ——
// 届く先は変わらない（ルームの全員に見える）。呼ばれた人の行に印を付け、通知を鳴らす。
//
// 見分けの規則は `warifu-desk` の `宛てられたか` と同じにする（エージェントもそれで見分ける）。

/**
 * **呼ぶ印。**日本語入力では `@` が全角の `＠` で入るので、どちらも呼びとして読む
 * （`warifu-desk` の規則も同じ）。
 */
const 呼ぶ印 = ['@', '＠'];
const 呼ぶ印か = (c: string | undefined) => c !== undefined && 呼ぶ印.includes(c);

/** 名前の後ろに続いていたら「別の名前」とみなす字。 */
const 続きの字 = /[A-Za-z0-9_]/;

/** 見る名前に揃える。**1 文字の名前は見ない**（当たりすぎる）。長い順に並べる。 */
function 揃える(名前たち: readonly string[]): string[] {
  const 並び = [...new Set(名前たち.map((n) => n.trim()).filter((n) => [...n].length >= 2))];
  return 並び.sort((a, b) => b.length - a.length);
}

/** 本文の `i` から `@名` が始まっていて、後ろが名前の続きでないか。 */
function ここで呼んでいるか(本文: string, i: number, 名: string): boolean {
  if (!呼ぶ印か(本文[i]) || !本文.startsWith(名, i + 1)) return false;
  const 次 = 本文[i + 1 + 名.length];
  return 次 === undefined || !続きの字.test(次);
}

/**
 * **呼べる名前に広げる。**エージェントの名札は `名前（場所）` の形なので、`名前` だけでも呼べる。
 * 人が打つのは短いほうである（`@連動` と打ち、`@連動（docs）` とは打たない）。
 */
export function 呼べる名に広げる(名前たち: readonly string[]): string[] {
  const 広げた: string[] = [];
  for (const 名 of 名前たち) {
    広げた.push(名);
    const 頭 = /^(.+?)（[^（）]+）$/.exec(名.trim())?.[1];
    if (頭) 広げた.push(頭);
  }
  return [...new Set(広げた)];
}

/** 本文に `@名前` があるか。 */
export function 宛てられたか(本文: string, 名前たち: readonly string[]): boolean {
  return 宛ての名(本文, 名前たち).length > 0;
}

/** 本文で呼ばれている名前（出てきた順・重ねない）。 */
export function 宛ての名(本文: string, 名前たち: readonly string[]): string[] {
  const 名 = 揃える(名前たち);
  const 出た: string[] = [];
  for (const 片 of 本文を分ける(本文, 名)) {
    if (片.呼び && !出た.includes(片.呼び)) 出た.push(片.呼び);
  }
  return 出た;
}

/** `から` 以降で、次に呼ぶ印がある位置。無ければ -1。 */
function 次の印(本文: string, から: number): number {
  for (let i = から; i < 本文.length; i++) if (呼ぶ印か(本文[i])) return i;
  return -1;
}

/** 本文のひと切れ。`呼び` があれば、その名前を呼んでいる `@名前` の切れ。 */
export type 切れ = { 文: string; 呼び?: string };

/** 本文を、`@名前` とそれ以外に分ける（目立たせるため）。 */
export function 本文を分ける(本文: string, 名前たち: readonly string[]): 切れ[] {
  const 名 = 揃える(名前たち);
  const 切れたち: 切れ[] = [];
  let 頭 = 0;
  let i = 次の印(本文, 0);
  while (i !== -1 && 名.length > 0) {
    const 当たり = 名.find((n) => ここで呼んでいるか(本文, i, n));
    if (当たり) {
      if (i > 頭) 切れたち.push({ 文: 本文.slice(頭, i) });
      切れたち.push({ 文: 本文.slice(i, i + 1 + 当たり.length), 呼び: 当たり });
      頭 = i + 1 + 当たり.length;
      i = 次の印(本文, 頭);
    } else {
      i = 次の印(本文, i + 1);
    }
  }
  if (頭 < 本文.length) 切れたち.push({ 文: 本文.slice(頭) });
  return 切れたち;
}

/** 打ちかけの `@…`。`頭` は `@` の位置。 */
export type 打ちかけ = { 頭: number; 候補: string[] };

/** 候補を出す数。**多いと選べない** */
const 候補の数 = 6;

/**
 * **打っている所の `@…` に合う名前**を出す。無ければ `null`。
 *
 * `@` から打っている所までに空白が無いときだけ（`@` のあとに空白を打てば閉じる）。
 */
export function 候補を出す(
  下書き: string,
  位置: number,
  名前たち: readonly string[]
): 打ちかけ | null {
  const 前 = 下書き.slice(0, 位置);
  const 頭 = Math.max(前.lastIndexOf('@'), 前.lastIndexOf('＠'));
  if (頭 === -1) return null;
  // **語の途中の @ は呼びではない**（メールの番地など）
  if (頭 > 0 && !/\s/.test(前[頭 - 1])) return null;
  const 打った = 前.slice(頭 + 1);
  if (/\s/.test(打った)) return null;
  const 小 = 打った.toLowerCase();
  const 候補 = 揃える(名前たち)
    .filter((n) => n.toLowerCase().startsWith(小) && n !== 打った)
    .sort((a, b) => a.localeCompare(b))
    .slice(0, 候補の数);
  return 候補.length > 0 ? { 頭, 候補 } : null;
}

/** 候補を選んで、`@名前 ` を入れる。新しい下書きと、打つ位置を返す。 */
export function 候補を入れる(
  下書き: string,
  位置: number,
  頭: number,
  名: string
): { 下書き: string; 位置: number } {
  const 後 = 下書き.slice(位置).replace(/^\S*/, '');
  // **空白を重ねない。**後ろが空白で始まるなら、その空白の後ろへ打つ
  const 入れる = /^\s/.test(後) ? `@${名}` : `@${名} `;
  const 新 = 下書き.slice(0, 頭) + 入れる + 後;
  return { 下書き: 新, 位置: /^\s/.test(後) ? 頭 + 入れる.length + 1 : 頭 + 入れる.length };
}
