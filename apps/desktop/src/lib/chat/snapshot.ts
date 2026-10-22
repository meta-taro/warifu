// **画面の写し**（0.1.34）。エージェントの `app_view` に返す文字を書く。
//
// 本体の知っていること（`room_status`）と、人の画面に出ていることは食い違うことがある ——
// 10-22、届いた文字が別のルームの欄に積まれていた。**人が見ているものを、そのまま字にする。**
// 席どうしで見比べるので、並びと書き方は毎回同じにする。

import type { 会話行 } from '$lib/meeting/announce';

/** 写しの 1 ルーム分。 */
export interface 写しのルーム {
  名前: string;
  /** 人数（自分を含む）。分からなければ `null`。 */
  人数: number | null;
  行たち: readonly 会話行[];
}

/** 写しに要るもの。 */
export interface 写しの素材 {
  版: string;
  /** 題字（窓の上に出ている文字）。 */
  題字: string;
  /** 上に出ている知らせ。無ければ `null`。 */
  知らせ: string | null;
  /** 左で選んでいるもの（エージェントなど）。ルームを見ていれば `null`。 */
  選んでいる: string | null;
  /** 開いているルーム。無ければ `null`。 */
  見ている: (写しのルーム & { 見出し: string; 届く先: readonly string[] }) | null;
  /** ほかのルーム。 */
  ほか: readonly 写しのルーム[];
}

/** 写しの上限（バイト）。本体の `写しの上限` と同じ。 */
export const 写しの上限 = 3500;

const バイト数 = (文: string) => new TextEncoder().encode(文).length;

function 行を書く(行: 会話行, 本文の字数: number): string {
  const 本文 = 行.画像 ? '［画像］' : 行.body.replace(/\s+/g, ' ').trim();
  const 詰めた = 本文.length > 本文の字数 ? `${本文.slice(0, 本文の字数)}…` : 本文;
  if (行.system) return `  ${行.at ?? ''} （${詰めた}）`;
  const 誰 = 行.mine ? '自分' : 行.who;
  return `  ${行.at ?? ''} ${誰}「${詰めた}」`;
}

function 書く(素材: 写しの素材, 見る行数: number, ほかの行数: number, 本文の字数: number): string {
  const 文: string[] = [`版 ${素材.版}`, `題字 ${素材.題字 || 'なし'}`];
  if (素材.知らせ) 文.push(`知らせ ${素材.知らせ}`);
  if (素材.選んでいる) 文.push(`選んでいる ${素材.選んでいる}`);
  const 見る = 素材.見ている;
  if (見る) {
    文.push(`開いているルーム ${見る.名前}（${見る.人数 ?? '?'} 人）`);
    文.push(`見出し ${見る.見出し}`);
    文.push(`届く先 ${見る.届く先.join('・') || 'なし'}`);
    const 行たち = 見る.行たち.slice(-見る行数);
    文.push(`チャット欄（最後の ${行たち.length} 行 / 全 ${見る.行たち.length} 行）`);
    for (const 行 of 行たち) 文.push(行を書く(行, 本文の字数));
  } else {
    文.push('開いているルーム なし');
  }
  for (const 部屋 of 素材.ほか) {
    const 行たち = 部屋.行たち.slice(-ほかの行数);
    文.push(`ほかのルーム ${部屋.名前}（${部屋.人数 ?? '?'} 人・全 ${部屋.行たち.length} 行）`);
    for (const 行 of 行たち) 文.push(行を書く(行, 本文の字数));
  }
  return 文.join('\n');
}

/**
 * **写しを書く。**上限に収まるまで、行数と本文の長さを詰めていく。
 * 詰めても収まらなければ、本体が最後に切る。
 */
export function 画面の写しを書く(素材: 写しの素材): string {
  const 詰め方: readonly [number, number, number][] = [
    [20, 3, 80],
    [12, 2, 60],
    [8, 1, 40],
    [5, 0, 30],
  ];
  let 文 = '';
  for (const [見る, ほか, 字数] of 詰め方) {
    文 = 書く(素材, 見る, ほか, 字数);
    if (バイト数(文) <= 写しの上限) return 文;
  }
  return 文;
}
