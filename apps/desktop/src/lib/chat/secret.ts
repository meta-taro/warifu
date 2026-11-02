// 送る前に、秘密らしいものを見つける（0.1.42）。
//
// 割符の会話は、送った時点で各機械へ写しが届く。**編集も削除もできない**（P2P）。
// だから止められるのは送る前だけである。2026-11-02、中継のトークンが全機の会話に流れた。
//
// **決まりは本体（`crates/warifu-desk/src/secret.rs`）と同じ。**どちらも
// `crates/warifu-desk/tests/secret-cases.tsv` の例文で試すので、片方だけ変えれば落ちる。
// 見つけたものの**値は返さない**（返すと、止めた所でまた写る）。種類だけを返す。

import type { MessageKey } from '$lib/i18n/messages';

export type 秘密の種類 = '16進' | '名前＝値' | 'キーの形' | '秘密鍵' | 'ルームキー';

/** 種類の言い方（辞書の鍵）。本体が止めたとき（`chat.secret`）も、この名前で返ってくる */
export const 秘密の言い方: Record<秘密の種類, MessageKey> = {
  '16進': 'chat.secret.kind.hex',
  '名前＝値': 'chat.secret.kind.assign',
  キーの形: 'chat.secret.kind.apikey',
  秘密鍵: 'chat.secret.kind.pem',
  ルームキー: 'chat.secret.kind.roomkey',
};

/** 見つけた種類を、見ている人の言葉で並べる。知らない名前（新しい本体）はそのまま出す */
export function 秘密を言う(種類たち: readonly string[], t: (k: MessageKey) => string): string {
  return 種類たち.map((種) => (種 in 秘密の言い方 ? t(秘密の言い方[種 as 秘密の種類]) : 種)).join('・');
}

/** 並べる順（本体の enum の順）。 */
const 順: readonly 秘密の種類[] = ['16進', '名前＝値', 'キーの形', '秘密鍵', 'ルームキー'];

/** 16 進の並びを秘密と見る長さ。git の指紋（40）・md5（32）は人がよく貼るので通す */
const 十六進の長さ = 48;

/** 名の知れたキーの頭と、その後ろに続く字の最小の数。 */
const キーの頭: readonly (readonly [string, number])[] = [
  ['sk-ant-', 20],
  ['sk-', 20],
  ['ghp_', 30],
  ['gho_', 30],
  ['ghs_', 30],
  ['ghu_', 30],
  ['ghr_', 30],
  ['github_pat_', 30],
  ['AKIA', 16],
  ['ASIA', 16],
  ['xoxb-', 10],
  ['xoxp-', 10],
  ['xoxa-', 10],
  ['xoxr-', 10],
  ['AIza', 30],
  ['glpat-', 20],
];

/** 名前に入っていれば、その値を秘密と見る語（大文字にして比べる）。 */
const 秘密の名前 = ['TOKEN', 'SECRET', 'PASSWORD', 'PASSWD', 'API_KEY', 'APIKEY', 'ACCESS_KEY', 'PRIVATE_KEY'];

const 鍵の字 = /[A-Za-z0-9_-]/;

/** 本文に入っている秘密らしいものの種類（重なりなし・本体と同じ順）。無ければ空。 */
export function 秘密らしいもの(本文: string): 秘密の種類[] {
  const 見つけた = new Set<秘密の種類>();
  if (本文.includes('-----BEGIN') && 本文.includes('PRIVATE KEY')) 見つけた.add('秘密鍵');
  if (ルームキーがある(本文)) 見つけた.add('ルームキー');
  if (長い十六進がある(本文)) 見つけた.add('16進');
  if (キーの形がある(本文)) 見つけた.add('キーの形');
  if (名前と値がある(本文)) 見つけた.add('名前＝値');
  return 順.filter((種) => 見つけた.has(種));
}

/** `位置` の前が英数字でないか（語の頭か）。 */
function 語の頭か(本文: string, 位置: number): boolean {
  return 位置 === 0 || !/[A-Za-z0-9]/.test(本文[位置 - 1]);
}

/** `位置` から鍵の字が続く数。 */
function 鍵の字の数(本文: string, 位置: number): number {
  let n = 0;
  while (位置 + n < 本文.length && 鍵の字.test(本文[位置 + n])) n++;
  return n;
}

function* 出てくる所(本文: string, 語: string): Generator<number> {
  for (let i = 本文.indexOf(語); i !== -1; i = 本文.indexOf(語, i + 1)) yield i;
}

function ルームキーがある(本文: string): boolean {
  for (const i of 出てくる所(本文, 'WARIFU1-')) {
    const 体 = /^[A-Z2-7#]*/.exec(本文.slice(i + 'WARIFU1-'.length))?.[0] ?? '';
    if (体.length >= 40 && 体.includes('#')) return true;
  }
  return false;
}

function 長い十六進がある(本文: string): boolean {
  // 英数字の切れ目で区切った語ごとに見る。語ぜんぶが 16 進で、長いものだけ
  return 本文.split(/[^A-Za-z0-9]/).some((語) => 語.length >= 十六進の長さ && /^[0-9A-Fa-f]+$/.test(語));
}

function キーの形がある(本文: string): boolean {
  return キーの頭.some(([頭, 最小]) => {
    for (const i of 出てくる所(本文, 頭)) {
      if (語の頭か(本文, i) && 鍵の字の数(本文, i + 頭.length) >= 最小) return true;
    }
    return false;
  });
}

function 名前と値がある(本文: string): boolean {
  for (let i = 0; i < 本文.length; i++) {
    if (本文[i] !== '=' && 本文[i] !== ':') continue;
    const 名前 = /[A-Za-z0-9_.-]*$/.exec(本文.slice(0, i).trimEnd())?.[0] ?? '';
    const 大 = 名前.toUpperCase();
    if (!秘密の名前.some((語) => 大.includes(語))) continue;
    if (値らしい(本文.slice(i + 1).trimStart())) return true;
  }
  return false;
}

/** `=` `:` の後ろが、置き場所の印（`<…>` `${…}` `***`）でない、本物らしい値か。 */
function 値らしい(後ろ: string): boolean {
  const 剥いた = 後ろ.replace(/^["']+/, '');
  if (/^[<${*]/.test(剥いた)) return false;
  const 値 = /^[^\s"',;]*/.exec(剥いた)?.[0] ?? '';
  // 本体は ASCII だけを値と見る（日本語の続きは値ではない）
  if (!/^[\x21-\x7e]*$/.test(値)) return false;
  return 値.length >= 8 && (/\d/.test(値) || 値.length >= 16);
}
