<script lang="ts">
  // **顔。**公開鍵（やエージェントの名乗り）から描く。
  //
  // **上げてもらう画像にしない**（`avatar.ts` の注記）——
  // 鍵が同じなら同じ絵になるので、**別人が同じ顔を出せない。**
  // 差し替えたい人のために、`画像` を渡せば置き換わる。

  import { マス, 顔の主を決める, 顔を描く } from './avatar';

  interface Props {
    /** 顔の種（公開鍵、または `desk:<名乗り>`）。 */
    種: string;
    /** 一辺の px。 */
    大きさ?: number;
    /** 差し替えた画像（あれば、こちらを出す）。 */
    画像?: string | null;
    /** 読み上げ用の名前。**絵だけで人を指さない。** */
    名?: string;
    /**
     * **人か、エージェントかの印を右上に出す**（2026-10-21）。
     * 渡した文字は、印に重ねたときの説明になる（`人` / `エージェント` を訳したもの）。
     * 渡さなければ、印は出さない。
     */
    主の名?: (主: '人' | 'エージェント') => string;
  }

  const { 種, 大きさ = 22, 画像 = null, 名 = '', 主の名 }: Props = $props();

  const 顔 = $derived(顔を描く(種));
  const 主 = $derived(顔の主を決める(種));
  /** 印の一辺。**小さすぎると形が読めない**ので 10px より小さくしない */
  const 印の大きさ = $derived(Math.max(10, Math.round(大きさ * 0.55)));
</script>

{#if 主の名}
  <span class="owned" title={主の名(主)}>
    {@render 絵()}
    <!-- **右上。**右下は在エージェントの丸（`ContactsPane`）が使っている -->
    <svg
      class="owner {主 === 'エージェント' ? 'agent' : 'person'}"
      width={印の大きさ}
      height={印の大きさ}
      viewBox="0 0 24 24"
      aria-hidden="true"
    >
      <circle cx="12" cy="12" r="12" class="disc" />
      {#if 主 === 'エージェント'}
        <!-- エージェント：アンテナのある四角い頭 -->
        <path d="M12 4v3" />
        <circle cx="12" cy="3.6" r="1.4" class="dot" />
        <rect x="5.5" y="7.5" width="13" height="10" rx="2.5" />
        <circle cx="9.5" cy="12.5" r="1.4" class="dot" />
        <circle cx="14.5" cy="12.5" r="1.4" class="dot" />
      {:else}
        <!-- 人：頭と肩 -->
        <circle cx="12" cy="9" r="3.6" />
        <path d="M5.5 19.5c.8-3.6 3.4-5.5 6.5-5.5s5.7 1.9 6.5 5.5" />
      {/if}
    </svg>
  </span>
{:else}
  {@render 絵()}
{/if}

{#snippet 絵()}

{#if 画像}
  <!-- **差し替えた顔。**人が選んだ画像なので、鍵とは結び付いていない -->
  <img class="face" src={画像} alt={名} width={大きさ} height={大きさ} />
{:else}
  <svg
    class="face"
    width={大きさ}
    height={大きさ}
    viewBox="0 0 {マス} {マス}"
    role="img"
    aria-label={名}
  >
    <rect width={マス} height={マス} fill="var(--bg-sunken)" />
    {#each 顔.ます as 行, y (y)}
      {#each 行 as 塗る, x (x)}
        {#if 塗る}
          <rect {x} {y} width="1" height="1" fill={顔.色} />
        {/if}
      {/each}
    {/each}
  </svg>
{/if}
{/snippet}

<style>
  .face {
    flex: none;
    border-radius: var(--radius-sm);
    /* 画像を差し替えたときに、縦横比が崩れないようにする */
    object-fit: cover;
    background: var(--bg-sunken);
  }

  /* ── 人か、エージェントかの印 ─────────────────────────────
     **形で言う。**色だけにしない（重ねると説明が出る） */
  .owned {
    position: relative;
    display: inline-flex;
    flex: none;
    line-height: 0;
  }
  .owner {
    position: absolute;
    top: -3px;
    right: -4px;
    fill: none;
    stroke: var(--text-secondary);
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .owner .disc {
    fill: var(--bg-app);
    stroke: var(--border);
    stroke-width: 1.5;
  }
  .owner .dot {
    fill: var(--text-secondary);
    stroke: none;
  }
  /* **エージェントは目立たせる。**人の発言と取り違えないための印である */
  .owner.agent {
    stroke: var(--accent);
  }
  .owner.agent .dot {
    fill: var(--accent);
  }
</style>
