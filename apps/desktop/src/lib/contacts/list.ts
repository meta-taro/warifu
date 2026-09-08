// 連絡帳の並び。**3 つの区画に分ける。**
//
//   この PC        —— 自分と、この PC の AI（`issues/012`「人と AI はセット」）
//   いま会議に居る人 —— 覚えていなくても、いま繋がっている相手
//   覚えている相手   —— `contacts.tsv` に居る人
//
// **同じ人を二度出さない。**いま会議に居る覚えた相手は、上の区画にだけ出す。

import { 呼び名, 鍵の頭 } from '$lib/meeting/names';

/** 一覧の 1 行。 */
export interface 行 {
  /** 公開鍵（全桁）。**押したときに使う。** */
  key: string;
  /** 画面に出す名前。 */
  name: string;
  種類: '自分' | 'AI' | '人' | '部屋';
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
  /**
   * いま机に着いている顔ぶれ（呼び方の並び）。
   *
   * **数だけでは足りない。**1 台の PC で複数のエージェントが同じ机に着くので、
   * まとめて 1 行にすると**どれが着いているのか分からない**
   * （2026-09-08 オーナー指摘「このPCのどこで起動しているエージェントなのか」）。
   */
  机のAIたち: readonly string[];
  /** いま会議に居る相手の公開鍵（自分を含まない）。 */
  会議の相手: readonly string[];
  /** 覚えている相手。 */
  覚えた: readonly { key: string; label: string; has_address: boolean }[];
  /**
   * いま居る部屋。
   *
   * **持てても見えなければ切り替えようがない**（2026-09-08）。
   * 部屋を複数持てるようにした以上、一覧が要る。
   */
  部屋たち?: readonly { id: string; members: number; host: boolean }[];
  /**
   * **留守中に預かり所へ言葉を置いていった相手**の公開鍵（**D71** / **D72**）。
   *
   * 覚えていない相手からも届く。**行が無いと、届いた言葉を開く所が無い** ——
   * 受け取っておいて出さないのは、黙って捨てるのと同じに見える。
   */
  留守中に届いた?: readonly string[];
}

/** 机に着いている相手を指す、画面の中だけの印。**公開鍵ではない。** */
export const 机の印 = 'desk:';

/** 部屋を指す、画面の中だけの印。**公開鍵ではない。** */
export const 部屋の印 = 'room:';

/** その行が部屋か。 */
export function 部屋か(key: string): boolean {
  return key.startsWith(部屋の印);
}

/** 部屋の印から id を取り出す。部屋でなければ `null`。 */
export function 部屋のid(key: string): string | null {
  return 部屋か(key) ? key.slice(部屋の印.length) : null;
}

/** その行が机の相手か。 */
export function 机の相手か(key: string): boolean {
  return key.startsWith(机の印);
}

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
  ];
  if (素材.机のAIたち.length === 0) {
    // **誰も着いていなくても行は消さない。**
    // 消すと「この PC に AI が居ない」ではなく「そういう仕組みが無い」と読まれる
    このPC.push({
      key: 机の印,
      name: 'contacts.desk',
      種類: 'AI',
      住所を覚えている: false,
      いま会議に居る: false,
    });
  } else {
    // **1 つずつ行にする。**まとめると、どれが着いているのか分からない
    for (const 呼び方 of 素材.机のAIたち) {
      このPC.push({
        key: `${机の印}${呼び方}`,
        name: 呼び方,
        種類: 'AI',
        住所を覚えている: false,
        いま会議に居る: true,
      });
    }
  }

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

  // **部屋は、居るときだけ出す。**空の見出しを並べない
  const 部屋: 行[] = (素材.部屋たち ?? []).map((r) => ({
    key: `${部屋の印}${r.id}`,
    // **部屋の名前はまだ無い。**id の頭で見分ける（段 3 で名前を付ける）
    name: 鍵の頭(r.id),
    種類: '部屋' as const,
    住所を覚えている: false,
    いま会議に居る: r.members > 1,
  }));
  if (部屋.length > 0) 区画たち.push({ title: 'contacts.rooms', 行たち: 部屋 });
  // **誰も居ない区画そのものを出さない**（見出しだけが並ぶ画面にしない）
  if (会議.length > 0) 区画たち.push({ title: 'contacts.inmeeting', 行たち: 会議 });

  // **留守中に置いていった相手。**覚えていなくても、開く所を出す
  // （**同じ人を二度出さない** —— 覚えている相手なら、下の区画にもう居る）
  const 出た = new Set([...会議に居る, ...素材.覚えた.map((c) => c.key), 素材.自分]);
  const 留守中: 行[] = (素材.留守中に届いた ?? [])
    .filter((key) => !出た.has(key))
    .map((key) => ({
      key,
      name: 呼び名(名簿, key),
      種類: '人' as const,
      // **住所は知らない。**預かり所ごしに届いただけで、居場所は分からない
      住所を覚えている: false,
      いま会議に居る: false,
    }));
  if (留守中.length > 0) 区画たち.push({ title: 'contacts.late', 行たち: 留守中 });

  区画たち.push({ title: 'contacts.saved', 行たち: 覚えた });
  return 区画たち;
}
