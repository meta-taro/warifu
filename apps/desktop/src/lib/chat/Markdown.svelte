<script lang="ts">
  // **発言のマークダウンを絵にする**（0.1.44）。読むのは `./markdown`（HTML にはしない）。
  // `{@html}` を使わない —— 届いた文は、どの切れも字として出る。
  import Self from './Markdown.svelte';
  import { 塊に分ける, type 塊, type 字 } from './markdown';
  import { 本文を分ける } from './mention';
  import { openLink } from '$lib/bridge';

  type Props = {
    /** 本文。`塊` を渡したときは使わない（引用の中を描くとき） */
    本文?: string;
    塊たち?: 塊[];
    /** `@名前` として目立たせる名前たち */
    見分ける名: readonly string[];
    /** 自分が呼ばれる名前（地を付ける） */
    自分の呼ばれる名: readonly string[];
  };
  let { 本文 = '', 塊たち, 見分ける名, 自分の呼ばれる名 }: Props = $props();

  const 並び = $derived(塊たち ?? 塊に分ける(本文));

  /** http(s) の繋ぎだけ押せる（11-11）。ほかは字のまま */
  const 開ける先 = (先: string) => /^https?:\/\/\S+$/i.test(先);

  /**
   * **外のブラウザで開く。**画面の中では移らない（移ると割符の窓ごと置き換わる）。
   * 開けなかったら記録に残す（押しても何も起きない、を黙らせない）
   */
  function 外で開く(e: MouseEvent, 先: string) {
    e.preventDefault();
    void openLink(先).catch((err: unknown) => console.warn(`繋ぎを開けませんでした: ${String(err)}`));
  }
</script>

{#snippet 字たち(並べる: 字[])}{#each 並べる as 切, k (k)}{#if 切.種 === '符'}<code>{切.文}</code
    >{:else if 切.種 === '太'}<strong>{@render 呼びを分ける(切.文)}</strong
    >{:else if 切.種 === '繋' && 開ける先(切.先)}<a
        class="link"
        href={切.先}
        title={切.先}
        onclick={(e) => 外で開く(e, 切.先)}>{@render 呼びを分ける(切.文)}</a
      >{:else if 切.種 === '繋'}<span class="link" title={切.先}>{@render 呼びを分ける(切.文)}</span
    >{:else}{@render 呼びを分ける(切.文)}{/if}{/each}{/snippet}

{#snippet 呼びを分ける(文: string)}{#each 本文を分ける(文, 見分ける名) as 片, j (j)}{#if 片.呼び}<span
        class="mention"
        class:cc={片.写し === true}
        class:me={自分の呼ばれる名.includes(片.呼び)}>{片.文}</span
      >{:else}{片.文}{/if}{/each}{/snippet}

{#each 並び as b, i (i)}
  {#if b.種 === '段'}
    <p>{@render 字たち(b.字)}</p>
  {:else if b.種 === '見出し'}
    <p class="h" class:h1={b.深さ === 1}>{@render 字たち(b.字)}</p>
  {:else if b.種 === '符'}
    <pre>{b.文}</pre>
  {:else if b.種 === '罫'}
    <hr />
  {:else if b.種 === '引用'}
    <blockquote><Self 塊たち={b.中} {見分ける名} {自分の呼ばれる名} /></blockquote>
  {:else if b.種 === '箇条'}
    <div class="list">
      {#each b.項 as 項, k (k)}
        <div class="item" style:padding-left="{項.深さ * 1.2}em">
          <span class="bullet">{b.番号つき ? 項.印 : '・'}</span><span>{@render 字たち(項.字)}</span>
        </div>
      {/each}
    </div>
  {:else if b.種 === '表'}
    <div class="table">
      <table>
        <thead><tr>{#each b.頭 as 升, k (k)}<th>{@render 字たち(升)}</th>{/each}</tr></thead>
        <tbody>
          {#each b.体 as 行, r (r)}
            <tr>{#each 行 as 升, k (k)}<td>{@render 字たち(升)}</td>{/each}</tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
{/each}

<style>
  p,
  pre,
  blockquote,
  hr,
  .list,
  .table {
    margin: 0;
  }
  /* 塊どうしの間。吹き出しの中で詰まりすぎないように */
  * + p,
  * + pre,
  * + blockquote,
  * + .list,
  * + .table,
  * + hr {
    margin-top: 0.5em;
  }
  /* **打った改行を、そのまま見せる**（段落・箇条・表の升） */
  p,
  .item,
  th,
  td {
    white-space: pre-wrap;
  }
  .h {
    font-weight: 700;
  }
  .h1 {
    font-size: 1.1em;
  }
  code,
  pre {
    font-family: var(--font-mono);
    font-size: 0.92em;
    background: var(--bg-subtle);
    border-radius: var(--radius-sm);
  }
  code {
    padding: 0 3px;
  }
  pre {
    padding: 6px 8px;
    /* 符は折り返さず、はみ出たら横に送る（桁がずれると読めない） */
    white-space: pre;
    overflow-x: auto;
  }
  blockquote {
    padding-left: 8px;
    color: var(--text-secondary);
    border-left: 3px solid var(--border);
  }
  hr {
    border: none;
    border-top: 1px solid var(--border);
  }
  .item {
    display: flex;
    gap: 0.4em;
  }
  .bullet {
    flex: none;
    color: var(--text-secondary);
  }
  /* 表は吹き出しの幅を超えたら、表だけ横に送る */
  .table {
    overflow-x: auto;
  }
  table {
    border-collapse: collapse;
    white-space: normal;
  }
  th,
  td {
    padding: 2px 6px;
    text-align: left;
    vertical-align: top;
    border: 1px solid var(--border);
  }
  th {
    background: var(--bg-subtle);
  }
  /* 繋ぎ。http(s) は押すと外のブラウザで開く（画面の中では移らない）。先は title で見せる */
  .link {
    text-decoration: underline dotted;
  }
  a.link {
    color: var(--info-fg);
    text-decoration: underline;
    cursor: pointer;
    overflow-wrap: anywhere;
  }
  /* **`@名前` で呼んだ所**（0.1.35）。誰に話しかけたかが一目で分かるように */
  .mention {
    color: var(--info-fg);
    font-weight: 600;
  }
  /* 自分が呼ばれた所は、地も付けて目に入れる */
  .mention.me {
    padding: 0 2px;
    background: var(--info-bg);
    border-radius: var(--radius-sm);
  }
  /* **写し（cc@名前）**（0.1.47）。宛て先より控えめ（太字にしない） */
  .mention.cc {
    color: var(--text-secondary);
    font-weight: 400;
  }
</style>
