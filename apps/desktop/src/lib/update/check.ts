// **更新を確かめて、入れ替える**（**D81**）。
//
// プラグインを触るのはここだけ。決めごと（新しいか・進み具合）は
// `notice.ts` にあって、そちらは画面を立てずに試験できる。
//
// **落とす前に署名を検めるのはプラグインの側**（`tauri.conf.json` の `pubkey`）。
// 検めずに落として実行すると、**更新の口が「何でも実行させる口」になる。**

import { relaunch } from '@tauri-apps/plugin-process';
import { check, type Update } from '@tauri-apps/plugin-updater';

import { 新しいか, type 新しい版 } from './notice';

/** いま待っている更新。**`確かめる` が置いて、`入れ替える` が使う。** */
let 待っているもの: Update | null = null;

/**
 * 向こうに新しい版があるか尋ねる。**無ければ `null`。**
 *
 * **繋がらなくても投げっぱなしにしない** —— 呼ぶ側が黙って捨てられるように、
 * 例外はそのまま上げる（更新の確認で画面を止めない）。
 */
export async function 確かめる(): Promise<新しい版 | null> {
  const 見つけた = await check();
  待っているもの = 見つけた;
  if (!見つけた) return null;
  // **プラグインも版で絞るが、こちらでも見る。**
  // 置き場所を書き換えられたときに、同じ版や古い版を出さないため
  if (!新しいか(見つけた.currentVersion, 見つけた.version)) {
    待っているもの = null;
    return null;
  }
  return {
    版: 見つけた.version,
    いまの版: 見つけた.currentVersion,
    中身: 見つけた.body ?? '',
    いつ: 見つけた.date ?? null,
  };
}

/** 落とす途中の知らせ。 */
export interface 落とし具合 {
  /** 全部で何バイトか。**分からないことがある。** */
  全部: number | null;
  /** ここまでに落とした量。 */
  落とした: number;
  /** 落とし終わって、入れ替えが済んだか。 */
  済んだ: boolean;
}

/**
 * 待っている更新を落として入れ替える。**入れ替えても、まだ古いほうが動いている** ——
 * 立て直すのは [`立て直す`] を呼んだとき。
 */
export async function 入れ替える(知らせる: (状態: 落とし具合) => void): Promise<void> {
  const これ = 待っているもの;
  if (!これ) throw new Error('待っている更新がありません');
  let 全部: number | null = null;
  let 落とした = 0;
  await これ.downloadAndInstall((出来事) => {
    if (出来事.event === 'Started') {
      全部 = 出来事.data.contentLength ?? null;
      落とした = 0;
      知らせる({ 全部, 落とした, 済んだ: false });
      return;
    }
    if (出来事.event === 'Progress') {
      落とした += 出来事.data.chunkLength;
      知らせる({ 全部, 落とした, 済んだ: false });
      return;
    }
    知らせる({ 全部, 落とした, 済んだ: true });
  });
}

/** 入れ替えたものに乗り換える。**これを呼ぶまで、古いほうが動いている。** */
export const 立て直す = (): Promise<void> => relaunch();
