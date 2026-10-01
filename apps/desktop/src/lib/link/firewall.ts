// **ファイアウォールの言い方を、規則の有無で分ける。**
//
// PR #16（Windows の人・2026-09-14）——
//
// > 実機では規則を 3 つ足したあとも同じ文言が出続け、
// > **足した人が「足したのに直らない」で止まりました**（`gh issue 11` で 3 時間）。
//
// **規則があるのに「ファイアウォールが止めています」と言うのは、嘘に近い。**
// `warifu-guard`（PR #16）が**実行ファイルごとに**調べてくれるので、そこを見て言い分ける。
//
// **「分からない」を「無い」に倒さない**（DESIGN §2 原則 7）——
// 調べる手段が失敗しただけで「足してください」と言うと、**足りている人を止める。**

import type { MessageKey } from '$lib/i18n/messages';

/** Rust から返る様子（`firewall_state`・PR #16）。 */
export type 遮りの様子 = {
	state: 'open' | 'blocked' | 'unknown';
	/** 規則の件数（`open` のときだけ）。 */
	rules: number | null;
	/**
	 * 調べられなかった理由（`unknown` のときだけ）。
	 *
	 * **Rust からは `null` で来る**（`Option<String>` を serde が落とす）。
	 */
	detail?: string | null;
	/** 見た実行ファイル（直し方に要るので、人に見せてよい）。 */
	program?: string | null;
};

/** 画面に出す言い方。 */
export type 言い方 = {
	鍵: MessageKey;
	/** **管理者の 2 行を渡すか**（規則が無いときだけ）。 */
	直し方を出す: boolean;
	/** 調べられなかった理由（あれば、そのまま出す）。 */
	detail?: string;
};

/** 規則の有無で、言い方を決める。 */
export function ふさがりの言い方(様子: 遮りの様子): 言い方 {
	if (様子.state === 'blocked') {
		return { 鍵: 'link.blocked', 直し方を出す: true };
	}
	if (様子.state === 'open') {
		// **規則はある。**ふさがりの話をしない —— 疑わせると、直った人が止まる
		return { 鍵: 'link.blocked.notit', 直し方を出す: false };
	}
	return 様子.detail
		? { 鍵: 'link.blocked.unknown', 直し方を出す: false, detail: 様子.detail }
		: { 鍵: 'link.blocked.unknown', 直し方を出す: false };
	// **`null` は「理由が無い」と同じに扱う**（Rust の `Option` がそう来る）
}
