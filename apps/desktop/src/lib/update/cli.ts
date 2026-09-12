// **同じ機械の CLI が古いままか。**
//
// 2026-09-12、Mac Air のエージェントの指摘（`.claude/issues/017`）——
//
// > 案内は「アプリを終了して開き直すと更新の案内が出ます」でしたが、CLI は別バイナリで、
// > そのままだと 0.1.2 のまま残ります。**アプリだけ上げた人は直ったつもりで直っていません。**
// > しかも症状は「繋がらない」だけなので、CLI が古いせいだと気づけません。
//
// **言うのは、言わなければ気づけないときだけ。**
// 入れていない人・版が読めない人に警告を出すと、**消せない警告**になる。

import type { CLIの様子 } from '$lib/bridge';
import type { MessageKey } from '$lib/i18n/messages';

/** 画面に出す知らせ。**何も言わないなら `null`。** */
export type 知らせ = { 鍵: MessageKey; 場所: string; 版: string } | null;

/** CLI の様子から、出す知らせを決める。 */
export function CLIの知らせ(様子: CLIの様子): 知らせ {
	if (様子 === '無い') return null;
	if ('古い' in 様子) return { 鍵: 'cli.old', ...様子.古い };
	if ('新しい' in 様子) return { 鍵: 'cli.ahead', ...様子.新しい };
	// 同じ／読めない は黙る（**決めつけない**）
	return null;
}
