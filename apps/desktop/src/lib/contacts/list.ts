// 連絡帳の並び。**3 つの区画に分ける。**
//
//   この PC        —— 自分と、この PC の AI（`issues/012`「人と AI はセット」）
//   いま会議に居る人 —— 覚えていなくても、いま繋がっている相手
//   覚えている相手   —— `contacts.tsv` に居る人
//
// **同じ人を二度出さない。**いま会議に居る覚えた相手は、上の区画にだけ出す。

import { 呼び名 } from '$lib/meeting/names';

/** 一覧の 1 行。 */
export interface 行 {
  /** 公開鍵（全桁）。**押したときに使う。** */
  key: string;
  /** 画面に出す名前。 */
  name: string;
  種類: '自分' | 'AI' | '人';
  住所を覚えている: boolean;
  いま会議に居る: boolean;
}

/** 区画 1 つ。 */
export interface 区画 {
  /** 見出しの文言の鍵。 */
  title: string;
  行たち: 行[];
}

/** 連絡帳を作るのに要るもの。 */
export interface 素材 {
  /** 自分の公開鍵。 */
  自分: string;
  /** 机に着いている人数（この PC の AI）。 */
  机の人数: number;
  /** いま会議に居る相手の公開鍵（自分を含まない）。 */
  会議の相手: readonly string[];
  /** 覚えている相手。 */
  覚えた: readonly { key: string; label: string; has_address: boolean }[];
}

/** この PC の AI を指す、画面の中だけの印。**公開鍵ではない。** */
export const 机の印 = 'desk:local';

/**
 * 連絡帳を組み立てる。
 *
 * **知らない相手を、知っているように見せない。**
 * 覚えていない相手は鍵の頭で出す（`呼び名` と同じ扱い）。
 */
export function 連絡帳を組む(素材: 素材): 区画[] {
  const 名簿 = Object.fromEntries(素材.覚えた.map((c) => [c.key, c.label]));
  const 住所あり = new Set(素材.覚えた.filter((c) => c.has_address).map((c) => c.key));
  const 会議に居る = new Set(素材.会議の相手);

  const このPC: 行[] = [
    {
      key: 素材.自分,
      name: 'contacts.me',
      種類: '自分',
      住所を覚えている: false,
      いま会議に居る: true,
    },
    // **机に誰も着いていなくても行は消さない。**
    // 消すと「この PC に AI が居ない」ではなく「そういう仕組みが無い」と読まれる
    {
      key: 机の印,
      name: 'contacts.desk',
      種類: 'AI',
      住所を覚えている: false,
      いま会議に居る: 素材.机の人数 > 0,
    },
  ];

  const 会議: 行[] = 素材.会議の相手.map((key) => ({
    key,
    name: 呼び名(名簿, key),
    種類: '人' as const,
    住所を覚えている: 住所あり.has(key),
    いま会議に居る: true,
  }));

  // **いま会議に居る人を、覚えている相手にも出さない**（同じ人が二度出る）
  const 覚えた: 行[] = 素材.覚えた
    .filter((c) => !会議に居る.has(c.key))
    .map((c) => ({
      key: c.key,
      name: c.label,
      種類: '人' as const,
      住所を覚えている: c.has_address,
      いま会議に居る: false,
    }))
    .sort((a, b) => a.name.localeCompare(b.name));

  const 区画たち: 区画[] = [{ title: 'contacts.this', 行たち: このPC }];
  // **誰も居ない区画そのものを出さない**（見出しだけが並ぶ画面にしない）
  if (会議.length > 0) 区画たち.push({ title: 'contacts.inmeeting', 行たち: 会議 });
  区画たち.push({ title: 'contacts.saved', 行たち: 覚えた });
  return 区画たち;
}
