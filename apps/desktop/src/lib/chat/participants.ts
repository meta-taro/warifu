// **この部屋に誰が居るか**（0.1.38）。
//
// 見出しは「（2 人）」と数だけを出していて、**その 2 人が誰か**は画面のどこにも無かった。
// 届く先（`Reach.svelte`）は文字が届く相手の機械だけで、自分も、相手の機械に乗っている
// エージェントも出ない。数と、見えている顔が合わなかった。
//
// 機械（公開鍵）ごとに 1 行。その下に、その機械のエージェントを添える ——
// 自分の機械は机の席、相手の機械は**この部屋で話した**名札から拾う
// （相手の機械のエージェントの一覧は線に載っていないので、話した分しか分からない）。

import type { 会話行 } from '$lib/meeting/announce';

export interface 参加者 {
  /** 機械の公開鍵（全桁）。 */
  key: string;
  主催: boolean;
  自分: boolean;
  /** その機械のエージェントの名札。 */
  エージェントたち: string[];
}

/** 相手の機械のエージェントの名札は `短く(鍵)（札）` で届く（公開鍵は `（` を含まない）。 */
function 名札を分ける(種: string): { 頭: string; 札: string } | null {
  const 位置 = 種.indexOf('（');
  if (位置 <= 0 || !種.endsWith('）')) return null;
  return { 頭: 種.slice(0, 位置).replace(/…$/, ''), 札: 種.slice(位置 + 1, -1) };
}

function 話したエージェント(会話: readonly 会話行[], 鍵: string): string[] {
  const 拾った: string[] = [];
  for (const 行 of 会話) {
    if (行.system || 行.mine || !行.顔の種) continue;
    const 分けた = 名札を分ける(行.顔の種);
    if (!分けた || 分けた.頭 === '' || !鍵.startsWith(分けた.頭)) continue;
    if (!拾った.includes(分けた.札)) 拾った.push(分けた.札);
  }
  return 拾った;
}

/**
 * 部屋の名簿（自分を含む）から、出す順に並べる。主催 → 自分 → 名簿の順。
 */
export function 参加者を並べる(
  名簿: readonly string[],
  主催の鍵: string | null,
  自分の鍵: string | null,
  この機械のエージェント: readonly string[],
  会話: readonly 会話行[],
): 参加者[] {
  const 並び = 名簿.map(
    (key): 参加者 => ({
      key,
      主催: key === 主催の鍵,
      自分: key === 自分の鍵,
      エージェントたち: key === 自分の鍵 ? [...この機械のエージェント] : 話したエージェント(会話, key),
    }),
  );
  const 順 = (p: 参加者) => (p.主催 ? 0 : p.自分 ? 1 : 2);
  // `sort` は安定なので、同じ順位は名簿の順のまま
  return 並び.sort((a, b) => 順(a) - 順(b));
}
