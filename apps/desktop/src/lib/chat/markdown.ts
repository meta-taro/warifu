// **発言のマークダウンを読む**（0.1.44）。
//
// エージェントは表・見出し・箇条書きで書く。生の `|` や `**` のまま並ぶと、長い発言ほど読めない。
//
// **HTML にしない。**塊と字の並びに分けるだけで、絵にするのは Svelte（`Markdown.svelte`）。
// 届いた文に `<script>` や `<img onerror>` が入っていても、ただの字として出る。
// 繋ぎ（`[字](先)`）も押せる形にしない —— 画面の中で外の頁へ移ると、割符の窓ごと置き換わる。

/** 行の中のひと切れ。 */
export type 字 =
  | { 種: '文'; 文: string }
  | { 種: '太'; 文: string }
  | { 種: '符'; 文: string }
  | { 種: '繋'; 文: string; 先: string };

/** 箇条書きの 1 項。`印` は書かれた印のまま（`-`・`1.` など）。 */
export type 項 = { 深さ: number; 印: string; 字: 字[] };

/** 塊。 */
export type 塊 =
  | { 種: '段'; 字: 字[] }
  | { 種: '見出し'; 深さ: number; 字: 字[] }
  | { 種: '符'; 文: string }
  | { 種: '箇条'; 番号つき: boolean; 項: 項[] }
  | { 種: '引用'; 中: 塊[] }
  | { 種: '表'; 頭: 字[][]; 体: 字[][][] }
  | { 種: '罫' };

const 囲い = /^\s*(```|~~~)/;
const 見出し = /^(#{1,6})\s+(.*?)\s*#*\s*$/;
const 罫 = /^\s*([-*_])(\s*\1){2,}\s*$/;
const 引用 = /^\s*>\s?(.*)$/;
const 箇条 = /^(\s*)([-*+]|\d{1,9}[.)])\s+(.*)$/;
const 表の区切り = /^\s*\|?\s*:?-{3,}:?\s*(\|\s*:?-{3,}:?\s*)*\|?\s*$/;

/** 本文を塊に分ける。 */
export function 塊に分ける(本文: string): 塊[] {
  const 行たち = 本文.replace(/\r\n?/g, '\n').split('\n');
  const 塊たち: 塊[] = [];
  let i = 0;
  while (i < 行たち.length) {
    const 行 = 行たち[i];
    if (行.trim() === '') {
      i++;
      continue;
    }
    const 読んだ = 一塊を読む(行たち, i);
    塊たち.push(読んだ.塊);
    i = 読んだ.次;
  }
  return 塊たち;
}

function 一塊を読む(行たち: string[], i: number): { 塊: 塊; 次: number } {
  const 行 = 行たち[i];
  const 囲いの頭 = 囲い.exec(行);
  if (囲いの頭) return 符を読む(行たち, i, 囲いの頭[1]);
  const 見 = 見出し.exec(行);
  if (見) return { 塊: { 種: '見出し', 深さ: 見[1].length, 字: 字に分ける(見[2]) }, 次: i + 1 };
  if (表の頭か(行たち, i)) return 表を読む(行たち, i);
  if (罫.test(行)) return { 塊: { 種: '罫' }, 次: i + 1 };
  if (引用.test(行)) return 引用を読む(行たち, i);
  if (箇条.test(行)) return 箇条を読む(行たち, i);
  return 段を読む(行たち, i);
}

function 符を読む(行たち: string[], i: number, 印: string): { 塊: 塊; 次: number } {
  const 中: string[] = [];
  let j = i + 1;
  while (j < 行たち.length && !行たち[j].trimStart().startsWith(印)) 中.push(行たち[j++]);
  // 閉じていなければ最後まで（閉じの行は飛ばす）
  return { 塊: { 種: '符', 文: 中.join('\n') }, 次: j + 1 };
}

function 表の頭か(行たち: string[], i: number): boolean {
  return 行たち[i].includes('|') && i + 1 < 行たち.length && 表の区切り.test(行たち[i + 1]);
}

function 升に分ける(行: string): 字[][] {
  let 中 = 行.trim();
  if (中.startsWith('|')) 中 = 中.slice(1);
  if (中.endsWith('|') && !中.endsWith('\\|')) 中 = 中.slice(0, -1);
  return 中.split(/(?<!\\)\|/).map((升) => 字に分ける(升.trim().replaceAll('\\|', '|')));
}

function 表を読む(行たち: string[], i: number): { 塊: 塊; 次: number } {
  const 頭 = 升に分ける(行たち[i]);
  const 体: 字[][][] = [];
  let j = i + 2;
  while (j < 行たち.length && 行たち[j].includes('|') && 行たち[j].trim() !== '') 体.push(升に分ける(行たち[j++]));
  return { 塊: { 種: '表', 頭, 体 }, 次: j };
}

function 引用を読む(行たち: string[], i: number): { 塊: 塊; 次: number } {
  const 中: string[] = [];
  let j = i;
  for (let m = 引用.exec(行たち[j] ?? ''); m; m = 引用.exec(行たち[j] ?? '')) {
    中.push(m[1]);
    j++;
  }
  return { 塊: { 種: '引用', 中: 塊に分ける(中.join('\n')) }, 次: j };
}

function 箇条を読む(行たち: string[], i: number): { 塊: 塊; 次: number } {
  const 項たち: 項[] = [];
  const 番号つき = /\d/.test(箇条.exec(行たち[i])?.[2] ?? '');
  let j = i;
  for (let m = 箇条.exec(行たち[j] ?? ''); m; m = 箇条.exec(行たち[j] ?? '')) {
    項たち.push({ 深さ: Math.floor(m[1].replaceAll('\t', '  ').length / 2), 印: m[2], 字: 字に分ける(m[3]) });
    j++;
  }
  return { 塊: { 種: '箇条', 番号つき, 項: 項たち }, 次: j };
}

/** 段落。**打った改行はそのまま残す**（チャットでは 1 行ずつ改行して書く人が多い）。 */
function 段を読む(行たち: string[], i: number): { 塊: 塊; 次: number } {
  const 中 = [行たち[i]];
  let j = i + 1;
  while (j < 行たち.length && 行たち[j].trim() !== '' && !別の塊の頭か(行たち, j)) 中.push(行たち[j++]);
  return { 塊: { 種: '段', 字: 字に分ける(中.join('\n')) }, 次: j };
}

function 別の塊の頭か(行たち: string[], j: number): boolean {
  const 行 = 行たち[j];
  return (
    囲い.test(行) || 見出し.test(行) || 引用.test(行) || 箇条.test(行) || 表の頭か(行たち, j)
  );
}

const 字の印 = /`([^`\n]+)`|\*\*([^*\n]+?)\*\*|\[([^\]\n]+)\]\(([^)\s]+)\)/g;

/** 行の中を、ふつうの字・太字・符・繋ぎに分ける。 */
export function 字に分ける(文: string): 字[] {
  const 字たち: 字[] = [];
  let 頭 = 0;
  for (const m of 文.matchAll(字の印)) {
    if (m.index > 頭) 字たち.push({ 種: '文', 文: 文.slice(頭, m.index) });
    if (m[1] !== undefined) 字たち.push({ 種: '符', 文: m[1] });
    else if (m[2] !== undefined) 字たち.push({ 種: '太', 文: m[2] });
    else 字たち.push({ 種: '繋', 文: m[3], 先: m[4] });
    頭 = m.index + m[0].length;
  }
  if (頭 < 文.length) 字たち.push({ 種: '文', 文: 文.slice(頭) });
  return 字たち;
}
