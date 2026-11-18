// **発言の中のグラフ**（```chart の囲み）。
//
// エージェントは毎朝の数を表で貼る。表のままだと、増えたか減ったか・どこが大きいかが一目で分からない。
// 囲みの中の表を読み、画面が棒・線・積み上げで描く（描くのは `Chart.svelte`）。
//
// 決めたこと ——
//
// 1. **中身は表だけ。**Markdown の表か、CSV / TSV。1 列目が横軸、2 列目から先が値の列
// 2. 形は囲みの頭（```chart bar / line / stack）。書かなければ、1 列目が日付なら線、ほかは棒
// 3. 題は表の前の 1 行（`# 題` か `title: 題`）
// 4. **描けない書き方は、理由を返す。**黙って字に戻さない（画面がその位置に理由と元の字を出す）
// 5. 外のものは何も読まない（画像・スクリプト・URL）。数と名前だけ

export type 図の形 = '棒' | '縦棒' | '線' | '積み上げ' | '円';

export interface 図の行 {
  /** 横軸（1 列目）の字。 */
  名: string;
  /** 値の列ごとの数。空・読めない升は `null`（描かない）。 */
  値: (number | null)[];
}

export interface 図 {
  形: 図の形;
  題: string | null;
  /** 1 列目の見出し。 */
  横: string;
  /** 値の列の見出し。 */
  系列: string[];
  行たち: 図の行[];
}

export type 読んだ図 = { 図: 図 } | { 訳: string };

/** 描く行の上限。越えたら断る（吹き出しの中で読めない）。 */
export const 図の行の上限 = 60;
/** 値の列の上限（色で見分けられる数）。 */
export const 図の系列の上限 = 6;

const 形の名: Record<string, 図の形> = {
  bar: '棒',
  bars: '棒',
  line: '線',
  stack: '積み上げ',
  stacked: '積み上げ',
  column: '縦棒',
  columns: '縦棒',
  pie: '円',
  donut: '円',
};

/** 円の切れの上限。多いと細くて読めない（多ければ棒で）。 */
export const 円の切れの上限 = 6;

/** 囲みの頭（```の後ろ）がグラフか。`chart` で始まるものだけ。 */
export function グラフの囲みか(情報: string): boolean {
  return /^chart(\s|$)/i.test(情報.trim());
}

/** 日付らしい字か（`2026-11-17`・`11-17`・`11/17`・`2026/11/17`）。 */
function 日付らしいか(字: string): boolean {
  return /^(\d{4}[-/])?\d{1,2}[-/]\d{1,2}$/.test(字.trim());
}

/** 升の字を数にする。`1,234`・`12%`・`**6,614**` も読む。空は `null`、読めなければ `undefined`。 */
export function 数にする(升: string): number | null | undefined {
  const 字 = 升.replace(/\*\*/g, '').replace(/[,，\s]/g, '').replace(/[%％]$/, '');
  if (字 === '' || 字 === '-' || 字 === '—') return null;
  const 数 = Number(字);
  return Number.isFinite(数) ? 数 : undefined;
}

function 表の升(行: string): string[] {
  let 中 = 行.trim();
  if (中.startsWith('|')) 中 = 中.slice(1);
  if (中.endsWith('|')) 中 = 中.slice(0, -1);
  return 中.split('|').map((升) => 升.trim());
}

const 表の区切り = /^\s*\|?\s*:?-{3,}:?\s*(\|\s*:?-{3,}:?\s*)*\|?\s*$/;

/** 表の行たちを升に分ける（Markdown の表・TSV・CSV）。 */
function 升に分ける(行たち: string[]): string[][] {
  if (行たち[0].includes('|')) return 行たち.filter((行) => !表の区切り.test(行)).map(表の升);
  const 区切り = 行たち[0].includes('\t') ? '\t' : ',';
  return 行たち.map((行) => 行.split(区切り).map((升) => 升.trim()));
}

/** 囲みの頭と中身から、図を読む。 */
export function 図を読む(情報: string, 中身: string): 読んだ図 {
  const 指定 = 情報.trim().split(/\s+/)[1]?.toLowerCase();
  if (指定 !== undefined && !(指定 in 形の名)) {
    return { 訳: `形「${指定}」は描けません（bar・column・line・stack・pie のどれか）` };
  }
  const 行たち = 中身.replace(/\r\n?/g, '\n').split('\n').filter((行) => 行.trim() !== '');
  let 題: string | null = null;
  const 題の行 = /^\s*(?:#{1,6}\s+|title:\s*)(.+)$/i.exec(行たち[0] ?? '');
  if (題の行) {
    題 = 題の行[1].trim();
    行たち.shift();
  }
  if (行たち.length < 2) return { 訳: '表がありません（見出しの行と、値の行が 1 つ以上要ります）' };

  const [頭, ...体] = 升に分ける(行たち);
  if (頭.length < 2) return { 訳: '列が足りません（1 列目が横軸、2 列目から先が値）' };
  const 系列 = 頭.slice(1);
  if (系列.length > 図の系列の上限) return { 訳: `値の列は ${図の系列の上限} つまでです（${系列.length} つあります）` };
  if (体.length > 図の行の上限) return { 訳: `行は ${図の行の上限} 行までです（${体.length} 行あります）` };

  const 図の行たち: 図の行[] = [];
  for (const [番, 升たち] of 体.entries()) {
    const 値: (number | null)[] = [];
    for (let k = 1; k <= 系列.length; k++) {
      const 数 = 数にする(升たち[k] ?? '');
      if (数 === undefined) return { 訳: `${番 + 1} 行目の「${升たち[k]}」は数として読めません` };
      値.push(数);
    }
    図の行たち.push({ 名: 升たち[0] ?? '', 値 });
  }
  if (図の行たち.every((r) => r.値.every((v) => v === null))) return { 訳: '数が 1 つもありません' };

  const 形 = 指定 !== undefined ? 形の名[指定] : 図の行たち.every((r) => 日付らしいか(r.名)) ? '線' : '棒';
  if (形 === '積み上げ' && 図の行たち.some((r) => r.値.some((v) => v !== null && v < 0))) {
    return { 訳: '積み上げに負の数は描けません' };
  }
  if (形 === '円') {
    const 訳 = 円にできないか(系列, 図の行たち);
    if (訳) return { 訳 };
  }
  return { 図: { 形, 題, 横: 頭[0], 系列, 行たち: 図の行たち } };
}

/** 円にできない訳。できれば `null`。 */
function 円にできないか(系列: string[], 行たち: 図の行[]): string | null {
  if (系列.length !== 1) return `円は値の列 1 つだけ描けます（${系列.length} つあります）`;
  if (行たち.length > 円の切れの上限) return `円は ${円の切れの上限} 切れまでです（${行たち.length} 行あります・多ければ bar で）`;
  if (行たち.some((r) => (r.値[0] ?? 0) < 0)) return '円に負の数は描けません';
  return null;
}

/**
 * 軸の目盛り。きりのよい刻みで最小から最大を覆う。棒は 0 から（長さで比べるので）。
 * 線は `ゼロから = false` で、動きの幅に合わせる（4,300 前後の毎日の増減が平らに潰れないように）。
 */
export function 目盛り(最小: number, 最大: number, 目安 = 4, ゼロから = true): number[] {
  const 下 = ゼロから ? Math.min(0, 最小) : 最小;
  const 上 = ゼロから ? Math.max(0, 最大) : 最大;
  if (上 === 下) return [0, 1];
  const 粗い = (上 - 下) / 目安;
  const 桁 = 10 ** Math.floor(Math.log10(粗い));
  const 刻み = [1, 2, 2.5, 5, 10].map((m) => m * 桁).find((s) => s >= 粗い) ?? 10 * 桁;
  const 出: number[] = [];
  for (let v = Math.floor(下 / 刻み) * 刻み; v < 上 + 刻み * 0.999; v += 刻み) {
    出.push(Math.round(v * 1e9) / 1e9);
  }
  return 出;
}

/** 数を短く書く（`6614` → `6,614`）。 */
export function 数の字(数: number): string {
  return 数.toLocaleString('ja-JP', { maximumFractionDigits: 2 });
}

/** 行の合計（積み上げの長さ）。 */
export function 行の合計(行: 図の行): number {
  return 行.値.reduce<number>((s, v) => s + (v ?? 0), 0);
}

/** 円の切れ（はじめと終わりの角度・割合）。角度は 0〜1（1 周）。 */
export function 円の切れ(行たち: 図の行[]): { 始め: number; 終わり: number; 割合: number }[] {
  const 合計 = 行たち.reduce((s, r) => s + (r.値[0] ?? 0), 0);
  let 今 = 0;
  return 行たち.map((r) => {
    const 割合 = 合計 > 0 ? (r.値[0] ?? 0) / 合計 : 0;
    const 切れ = { 始め: 今, 終わり: 今 + 割合, 割合 };
    今 += 割合;
    return 切れ;
  });
}

/** 横軸に名前を書く番。8 つまでは全部、多ければ間引いて 6 つ前後（最後は必ず書く・手前と重ねない）。 */
export function 名を書く番(数: number): Set<number> {
  if (数 <= 8) return new Set(Array.from({ length: 数 }, (_, i) => i));
  const 飛ばし = Math.ceil(数 / 6);
  const 番 = new Set<number>();
  for (let i = 0; i < 数; i += 飛ばし) 番.add(i);
  // 最後と、その手前の書いた所が重ならないように
  const 最後 = 数 - 1;
  for (const i of 番) if (i !== 最後 && 最後 - i < 飛ばし) 番.delete(i);
  番.add(最後);
  return 番;
}

/** 線の目盛りを 0 から始めないか。全部が正で、動きの幅が最大の半分より小さいとき。 */
export function 線は動きに合わせるか(値たち: number[]): boolean {
  if (値たち.length === 0) return false;
  const 最小 = Math.min(...値たち);
  const 最大 = Math.max(...値たち);
  return 最小 > 0 && (最大 - 最小) / 最大 < 0.5;
}

/**
 * 字の幅の見積もり（図の字の大きさ 10 のとき）。全角は 10、半角は 6。
 * SVG の中で字を測らずに決めるため（測ると描くたびに揺れる）。
 */
export function 字の幅(字: string): number {
  let 幅 = 0;
  for (const c of 字) 幅 += /[\u0000-\u00ff\uff61-\uff9f]/.test(c) ? 6 : 10;
  return 幅;
}

/**
 * **横の棒の名前の欄**（#8）。いちばん長い名前が入る幅にし、上限を越える名前だけ「…」で切る。
 * 前は 9 字で一律に切っていたので、左に余白があっても「M LABO S…」となり、どの行か分からなかった。
 */
export function 名の欄(名たち: readonly string[], 上限 = 150, 下限 = 48): { 幅: number; 切る: (名: string) => string } {
  const 余白 = 8;
  const 長さ = Math.max(0, ...名たち.map(字の幅));
  const 幅 = Math.min(上限, Math.max(下限, 長さ + 余白));
  const 入る = 幅 - 余白;
  const 切る = (名: string): string => {
    if (字の幅(名) <= 入る) return 名;
    let 出 = '';
    for (const c of 名) {
      if (字の幅(出 + c) + 字の幅('…') > 入る) break;
      出 += c;
    }
    return `${出}…`;
  };
  return { 幅, 切る };
}
