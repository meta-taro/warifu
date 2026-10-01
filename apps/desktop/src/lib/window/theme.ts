// テーマ（**OS に合わせる / ライト / ダーク**）。
//
// **描く前に当てるのは `app.html` の頭のスクリプト**である。後から当てると、
// 暗いテーマでも一瞬だけ白く光る。ここが持つのは「選び直したあと」だけ。
//
// **覚えるのは画面側**（`localStorage`）。Rust からは読めないので、
// メニューの印を付け直すときは、こちらから渡す。

/** テーマの選び方。**Rust 側（`menu.rs` の `THEMES`）と綴りを揃える。** */
export type テーマ = 'auto' | 'light' | 'dark';

/** 覚えておく場所。**`app.html` の頭のスクリプトと同じ鍵を使う。** */
export const 覚える鍵 = 'warifu-theme';

/** 知らない値は「OS に合わせる」に落とす。**どれでもない状態を作らない。** */
export function 読み取る(値: string | null | undefined): テーマ {
  return 値 === 'light' || 値 === 'dark' ? 値 : 'auto';
}

/**
 * その選びで、いま何色になるか。
 *
 * `auto` は OS に従う。**`auto` を「ライト」と決め打ちしない** ——
 * 決め打つと、暗い OS で白い画面が出る。
 */
export function 当てる色(選び: テーマ, OSが暗いか: boolean): 'light' | 'dark' {
  if (選び === 'auto') return OSが暗いか ? 'dark' : 'light';
  return 選び;
}
