// **通話ができる前に着いた下ごしらえを、捨てずに溜める。**
//
// `gh issue 9` の後半（ASUS・2026-09-12）——
//
// > offer / candidate が「通話を作る」より先に着いて捨てられています。
// > 経路が落ちていなくても、ここで取りこぼす見込みがあります。
//
// **文字は割符の経路で流れるが、映像は WebRTC が握手できないと乗らない。**
// 握手の最初の玉（`offer`）を捨てると、経路は**永久に `unknown`** のままになる。
//
// **順番を変えてはいけない** —— `offer` の前に `candidate` を食わせると握手が壊れる。

import type { SignalPayload } from '$lib/bridge';

/** 相手ごとに溜めておく上限。**際限なく溜めない**（来続けても手元が膨らまない）。 */
export const 溜める上限 = 32;

/** 相手の鍵 → 着いた順の下ごしらえ。 */
export type 溜め = Map<string, SignalPayload[]>;

/** 溜める。上限を超えたら**古いほうから捨てる**（新しい候補のほうが通る見込みが高い）。 */
export function 溜める(箱: 溜め, from: string, 玉: SignalPayload): void {
	const 列 = 箱.get(from) ?? [];
	列.push(玉);
	while (列.length > 溜める上限) 列.shift();
	箱.set(from, 列);
}

/** 取り出す。**取り出したら空になる**（二度食わせない）。 */
export function 取り出す(箱: 溜め, from: string): SignalPayload[] {
	const 列 = 箱.get(from) ?? [];
	箱.delete(from);
	return 列;
}

/** 忘れる。**相手が抜けたら捨てる** —— 次に来た人へ渡さない。 */
export function 忘れる(箱: 溜め, from: string): void {
	箱.delete(from);
}
