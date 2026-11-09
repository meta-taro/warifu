// **本文を見やすくする**（0.1.45）。**見せ方だけ**を変える —— メールそのもの（受信箱）は変えない。
//
// - フォームから来る知らせなどは、文字だけの本文に `&amp;` を書いてくる。よく使う参照だけを字に戻す
//   （戻した字は Svelte が字として出すので、`<script>` になってもタグにはならない）
// - 空行が何行も続く本文は、読む所が下へ逃げる。続く空行は 1 つにまとめる

const 名前の参照: Record<string, string> = { amp: '&', lt: '<', gt: '>', quot: '"', apos: "'", nbsp: ' ' };

function 参照を戻す(文: string): string {
  return 文.replace(/&(#x[0-9a-fA-F]{1,6}|#\d{1,7}|[a-zA-Z]{2,8});/g, (元, 中: string) => {
    if (中.startsWith('#x') || 中.startsWith('#X')) return 符号から(parseInt(中.slice(2), 16), 元);
    if (中.startsWith('#')) return 符号から(parseInt(中.slice(1), 10), 元);
    return 名前の参照[中] ?? 元;
  });
}

function 符号から(n: number, 元: string): string {
  // 制御文字や範囲の外は戻さない
  if (!Number.isFinite(n) || n < 0x20 || n > 0x10ffff || (n >= 0xd800 && n <= 0xdfff)) return 元;
  return String.fromCodePoint(n);
}

/** 本文を見やすくする。 */
export function 本文を見やすく(本文: string): string {
  return 参照を戻す(本文)
    .replace(/\r\n?/g, '\n')
    .replace(/\n[ \t]*(?=\n)/g, '\n')
    .replace(/\n{3,}/g, '\n\n')
    .replace(/^\n+|\n+$/g, '');
}
