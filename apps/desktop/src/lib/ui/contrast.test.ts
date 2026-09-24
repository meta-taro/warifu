import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';

/**
 * **字と地の対比を、機械が数える**（2026-09-24・オーナー指示）。
 *
 * > **タブ切り替えはいいけど、見てくれチェックが抜けていると思うので、
 * > ワークフロー見直してください。**
 *
 * **同じ日に、字の色が当たっていない所が見つかった** ——
 * `ChatPanel` の【送る】が `var(--on-accent)` を書いていたが、**その名前は無かった。**
 * 当たらなかったので、**濃い木の上に濃い字**が乗っていた
 * （暗い所では、明るいカフェオレの上に白い字）。**どちらも読みにくい。**
 *
 * **人が気づくまで、誰も気づかなかった。**——**数えられるものは数える。**
 *
 * 目安は WCAG 2.1 ——
 *
 *   本文      **4.5 : 1**
 *   大きい字・部品の縁 **3 : 1**
 *
 * **落ちたら、色を変えるか、組み合わせをやめる。**数字を下げて通さない。
 */
const 道 = new URL('../styles/tokens.css', import.meta.url);

function 色たち(文: string, 頭: string): Record<string, string> {
  const 始め = 文.indexOf(頭);
  const 終わり = 文.indexOf('\n}', 始め);
  const 塊 = 文.slice(始め, 終わり);
  const 出: Record<string, string> = {};
  for (const m of 塊.matchAll(/(--[a-zA-Z0-9-]+):\s*(#[0-9a-fA-F]{3,8})\s*;/g)) 出[m[1]] = m[2];
  return 出;
}

/** `#rrggbb` を 0–1 の 3 つにする。 */
function 解く(色: string): [number, number, number] {
  const h = 色.slice(1);
  const 幅 = h.length <= 4 ? 1 : 2;
  const 取る = (i: number) => {
    const s = h.slice(i * 幅, i * 幅 + 幅);
    return parseInt(幅 === 1 ? s + s : s, 16) / 255;
  };
  return [取る(0), 取る(1), 取る(2)];
}

/** 相対輝度（WCAG 2.1 の式）。 */
function 明るさ(色: string): number {
  const [r, g, b] = 解く(色).map((v) => (v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4));
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

function 対比(前: string, 後ろ: string): number {
  const [a, b] = [明るさ(前), 明るさ(後ろ)].sort((x, y) => y - x);
  return (a + 0.05) / (b + 0.05);
}

/** **実際に組にして使っているもの**だけを見る（総当たりにしない）。 */
const 組たち: Array<{ 何: string; 字: string; 地: string; 目安: number }> = [
  // **押せる一点**（【送る】【ルームに入る】…）。ここが今日壊れていた
  { 何: '操作の色の上の字', 字: '--on-accent', 地: '--accent', 目安: 4.5 },
  // 本文
  { 何: '本文（アプリの地）', 字: '--text-primary', 地: '--bg-app', 目安: 4.5 },
  { 何: '本文（札の地）', 字: '--text-primary', 地: '--bg-subtle', 目安: 4.5 },
  { 何: '本文（沈んだ地）', 字: '--text-primary', 地: '--bg-sunken', 目安: 4.5 },
  { 何: '本文（浮いた地）', 字: '--text-primary', 地: '--bg-elevated', 目安: 4.5 },
  // 添えの字
  { 何: '添えの字（札の地）', 字: '--text-secondary', 地: '--bg-subtle', 目安: 4.5 },
  // **薄い字は 3:1 まで**（見出しの脇・数など、読み落としても筋が通る所）
  { 何: '薄い字（札の地）', 字: '--text-tertiary', 地: '--bg-subtle', 目安: 3 },
  { 何: '薄い字（沈んだ地）', 字: '--text-tertiary', 地: '--bg-sunken', 目安: 3 },
];

describe('字と地の対比', () => {
  const 文 = readFileSync(道, 'utf-8');
  const 明 = 色たち(文, ':root {');
  const 暗 = 色たち(文, ":root[data-theme='dark'] {");

  it('明るいほうも暗いほうも、色が読めている', () => {
    // **数え漏れで「0 件だから ok」にしない**（落とし穴 7 と同じ形）
    expect(Object.keys(明).length).toBeGreaterThan(20);
    expect(Object.keys(暗).length).toBeGreaterThan(20);
  });

  for (const 組 of 組たち) {
    it(`${組.何} が ${組.目安} : 1 以上（明・暗とも）`, () => {
      for (const [名, 棚] of [
        ['明るいほう', 明],
        ['暗いほう', 暗],
      ] as const) {
        const 字 = 棚[組.字];
        const 地 = 棚[組.地];
        expect(字, `${名}: ${組.字} が無い`).toBeTruthy();
        expect(地, `${名}: ${組.地} が無い`).toBeTruthy();
        const 数 = 対比(字, 地);
        expect(
          Math.round(数 * 100) / 100,
          `${名}: ${組.字}(${字}) が ${組.地}(${地}) の上で ${数.toFixed(2)} : 1`,
        ).toBeGreaterThanOrEqual(組.目安);
      }
    });
  }
});
