// 相手をどう呼ぶか。
//
// **鍵の頭 12 文字だけでは、人にもエージェントにも見分けが付かない。**
// 2026-09-06 に画面のチャットで実際に起きた —— 別の機械のエージェントが送った
// 「エージェントから画面へ。届いていますか。」の差出人が `67R54JO7ND6P…` で、
// **誰が言ったのか画面からは分からなかった。**
//
// 覚えている相手（`warifu-vault` の `contacts.tsv`・CLI の `warifu contacts` と同じ置き場所）
// は呼び名で出す。覚えていなければ鍵の頭で出す —— **知らない相手を、知っているように見せない。**

/** 鍵は長い。**先頭だけ出す**（全桁は会議キーの欄で選べる）。 */
export function 鍵の頭(key: string): string {
  return key.length > 12 ? `${key.slice(0, 12)}…` : key;
}

/**
 * 表に出す名前。
 *
 * @param 名簿 公開鍵（全桁）→ 呼び名
 */
export function 呼び名(名簿: Readonly<Record<string, string>>, key: string): string {
  const 覚えている = 名簿[key];
  return 覚えている && 覚えている.trim() !== '' ? 覚えている : 鍵の頭(key);
}
