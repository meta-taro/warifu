// **繋ぎを QR コードにする**（0.1.53）。PC で割符を見ていて、その URL をそのままスマホで開きたいときに使う。
// スマホのカメラで読めば開ける。符号にするのは `uqr`（依存なし・MIT）。絵は画面が四角を並べて描く（HTML を流し込まない）。

import { encode } from 'uqr';

/** 周りの白い余白（升の数）。読み取りの決まりで 4 升。 */
export const 余白 = 4;

/** **QR の升目。**`大きさ` は余白を含む 1 辺の升の数、`黒` は塗る升（左上が 0,0）。 */
export type QRの升目 = { 大きさ: number; 黒: readonly (readonly [number, number])[] };

/** URL を QR の升目にする。誤りの直しは M（画面に出すので、汚れに強くするより升を細かくしない）。 */
export function QRにする(先: string): QRの升目 {
  const 符 = encode(先, { ecc: 'M', border: 0 });
  const 黒: [number, number][] = [];
  符.data.forEach((行, y) => {
    行.forEach((塗る, x) => {
      if (塗る) 黒.push([x + 余白, y + 余白]);
    });
  });
  return { 大きさ: 符.size + 余白 * 2, 黒 };
}

/** 升目を 1 本の SVG の道（`d`）にする。升ごとに要素を作ると、長い URL で数千になる。 */
export function 道にする(升目: QRの升目): string {
  return 升目.黒.map(([x, y]) => `M${x} ${y}h1v1h-1z`).join('');
}
