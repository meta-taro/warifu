// **同じ機械の CLI が古いままか。**
//
// 2026-09-12（内部の Issue 017）——
// CLI は別バイナリなので、アプリを開き直して更新しても CLI は古い版のまま残る。
// **アプリだけ上げた人は直ったつもりで直っていない。**
// しかも症状は「繋がらない」だけなので、CLI が古いせいだと気づけない。
//
// **言うのは、言わなければ気づけないときだけ。**
// 入れていない人・版が読めない人に警告を出すと、**消せない警告**になる。

import type { CLIの様子 } from '$lib/bridge';
import type { MessageKey } from '$lib/i18n/messages';

/**
 * 画面に出す知らせ。**何も言わないなら `null`。**
 * `置ける` は、束の中の CLI で入れ替えられるとき（内部の Issue 021）—— 帯に押し所を出す。
 */
export type 知らせ = { 鍵: MessageKey; 場所: string; 版: string; 置ける?: true } | null;

/**
 * CLI の様子から、出す知らせを決める。
 * `置ける` は、この画面の束に CLI が入っているか（macOS の配布物だけ）。
 * **入れ替えるのは古いときだけ**（新しい CLI は、画面を上げてもらう）。
 */
export function CLIの知らせ(様子: CLIの様子, 置ける = false): 知らせ {
	if (様子 === '無い') return null;
	if ('古い' in 様子) {
		return 置ける ? { 鍵: 'cli.oldReplaceable', ...様子.古い, 置ける: true } : { 鍵: 'cli.old', ...様子.古い };
	}
	if ('新しい' in 様子) return { 鍵: 'cli.ahead', ...様子.新しい };
	// 同じ／読めない は黙る（**決めつけない**）
	return null;
}
