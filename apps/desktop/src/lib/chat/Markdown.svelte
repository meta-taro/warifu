<script lang="ts">
  // **発言のマークダウンを絵にする**（0.1.44）。読むのは `./markdown`（HTML にはしない）。
  // `{@html}` を使わない —— 届いた文は、どの切れも字として出る。
  import Self from './Markdown.svelte';
  import Chart from './Chart.svelte';
  import { 塊に分ける, type 塊, type 字 } from './markdown';
  import { 本文を分ける } from './mention';
  import { openLink } from '$lib/bridge';
  import { QRにする, 道にする } from './qr';

  type QRの文言 = { 押し口: string; 説明: string; 添え: string; 閉じる: string };
  const QRの既定: QRの文言 = {
    押し口: 'QR',
    説明: 'QR コードで出す（スマホで開く）',
    添え: 'スマホのカメラで読むと、この URL が開きます',
    閉じる: '閉じる',
  };

  type Props = {
    /** 本文。`塊` を渡したときは使わない（引用の中を描くとき） */
    本文?: string;
    塊たち?: 塊[];
    /** `@名前` として目立たせる名前たち */
    見分ける名: readonly string[];
    /** 自分が呼ばれる名前（地を付ける） */
    自分の呼ばれる名: readonly string[];
    /** グラフの周りの文言（画面の言葉で）。 */
    文言?: { 表で見る: string; 描けない: string; QR?: QRの文言 };
  };
  let {
    本文 = '',
    塊たち,
    見分ける名,
    自分の呼ばれる名,
    文言 = { 表で見る: '表で見る', 描けない: 'グラフにできません：{why}' },
  }: Props = $props();

  const 並び = $derived(塊たち ?? 塊に分ける(本文));
  const QRの字 = $derived(文言.QR ?? QRの既定);

  /**
   * **QR コードで出している URL**（0.1.53）。PC で見ている繋ぎを、スマホのカメラで読んで開くため。
   * 1 つの発言で 1 つだけ出す。
   */
  let QRの先 = $state<string | null>(null);
  const QRの升目 = $derived(QRの先 === null ? null : QRにする(QRの先));

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
      ><button type="button" class="qr" title={QRの字.説明} aria-label={QRの字.説明} onclick={() => (QRの先 = 切.先)}
        >{QRの字.押し口}</button
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
    <p class="h" class:h1={b.深さ === 1} class:h2={b.深さ === 2} class:h3={b.深さ === 3}>{@render 字たち(b.字)}</p>
  {:else if b.種 === '符'}
    <pre>{b.文}</pre>
  {:else if b.種 === '図'}
    <div class="figure">
      {#if '図' in b.読んだ}
        <Chart 図={b.読んだ.図} />
        <details><summary>{文言.表で見る}</summary><pre>{b.文}</pre></details>
      {:else}
        <p class="why">{文言.描けない.replace('{why}', b.読んだ.訳)}</p>
        <pre>{b.文}</pre>
      {/if}
    </div>
  {:else if b.種 === '罫'}
    <hr />
  {:else if b.種 === '引用'}
    <blockquote><Self 塊たち={b.中} {見分ける名} {自分の呼ばれる名} {文言} /></blockquote>
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

{#if QRの先 !== null && QRの升目 !== null}
  <!-- 窓の外を押すか Esc で閉じる。絵は四角を並べた SVG（HTML を流し込まない） -->
  <div class="qr-back" role="presentation" onclick={() => (QRの先 = null)}>
    <div
      class="qr-card"
      role="dialog"
      aria-modal="true"
      aria-label={QRの字.説明}
      tabindex="-1"
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => {
        if (e.key === 'Escape') QRの先 = null;
      }}
    >
      <svg
        class="qr-code"
        viewBox="0 0 {QRの升目.大きさ} {QRの升目.大きさ}"
        shape-rendering="crispEdges"
        role="img"
        aria-label={QRの先}
      >
        <rect width={QRの升目.大きさ} height={QRの升目.大きさ} fill="#fff" />
        <path d={道にする(QRの升目)} fill="#000" />
      </svg>
      <p class="qr-hint">{QRの字.添え}</p>
      <p class="qr-url" title={QRの先}>{QRの先}</p>
      <!-- svelte-ignore a11y_autofocus -->
      <button type="button" class="qr-close" autofocus onclick={() => (QRの先 = null)}>{QRの字.閉じる}</button>
    </div>
  </div>
{/if}

<style>
  p,
  pre,
  blockquote,
  hr,
  .list,
  .table,
  .figure {
    margin: 0;
  }
  /* 塊どうしの間。吹き出しの中で詰まりすぎないように */
  * + p,
  * + pre,
  * + blockquote,
  * + .list,
  * + .table,
  * + .figure,
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
  /* 見出しは深さで大きさを変える（# が いちばん大きい・#### より深いものは本文と同じ大きさの太字） */
  .h {
    font-weight: 700;
    line-height: 1.35;
  }
  .h1 {
    font-size: 1.35em;
  }
  .h2 {
    font-size: 1.2em;
  }
  .h3 {
    font-size: 1.08em;
  }
  * + .h {
    margin-top: 0.8em;
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
  /* グラフにできなかった理由。元の字はその下に出す */
  .why {
    color: var(--warning-fg);
  }
  details {
    margin-top: 4px;
    font-size: 0.9em;
    color: var(--text-secondary);
  }
  summary {
    cursor: pointer;
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
  /* **QR の押し口**（0.1.53）。繋ぎのすぐ後ろに小さく置く。本文の字より目立たせない */
  .qr {
    margin-left: 3px;
    padding: 0 4px;
    font: inherit;
    font-size: 0.75em;
    line-height: 1.4;
    color: var(--text-secondary);
    background: transparent;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    cursor: pointer;
    vertical-align: 1px;
  }
  .qr:hover {
    color: var(--info-fg);
    border-color: var(--info-fg);
  }
  .qr-back {
    position: fixed;
    inset: 0;
    z-index: 1000;
    display: grid;
    place-items: center;
    background: rgb(0 0 0 / 0.45);
  }
  .qr-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    max-width: min(92vw, 340px);
    padding: 16px;
    background: var(--bg-elevated);
    color: var(--text-primary);
    border-radius: var(--radius-md);
    box-shadow: 0 8px 24px rgb(0 0 0 / 0.3);
  }
  /* 白地と余白は QR の決まり（暗い画面でも白のまま。黒地に反転すると読めない端末がある） */
  .qr-code {
    width: 240px;
    height: 240px;
    max-width: 80vw;
    max-height: 80vw;
  }
  .qr-hint {
    font-size: 0.9em;
    color: var(--text-secondary);
    text-align: center;
  }
  .qr-url {
    max-width: 100%;
    font-size: 0.8em;
    color: var(--text-secondary);
    overflow-wrap: anywhere;
    text-align: center;
  }
  .qr-close {
    font: inherit;
    padding: 4px 16px;
    cursor: pointer;
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
