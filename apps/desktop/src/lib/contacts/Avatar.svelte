<script lang="ts">
  // **顔。**公開鍵（や席の名乗り）から描く。
  //
  // **上げてもらう画像にしない**（`avatar.ts` の注記）——
  // 鍵が同じなら同じ絵になるので、**別人が同じ顔を出せない。**
  // 差し替えたい人のために、`画像` を渡せば置き換わる。

  import { マス, 顔を描く } from './avatar';

  interface Props {
    /** 顔の種（公開鍵、または `desk:<名乗り>`）。 */
    種: string;
    /** 一辺の px。 */
    大きさ?: number;
    /** 差し替えた画像（あれば、こちらを出す）。 */
    画像?: string | null;
    /** 読み上げ用の名前。**絵だけで人を指さない。** */
    名?: string;
  }

  const { 種, 大きさ = 22, 画像 = null, 名 = '' }: Props = $props();

  const 顔 = $derived(顔を描く(種));
</script>

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

<style>
  .face {
    flex: none;
    border-radius: var(--radius-sm);
    /* 画像を差し替えたときに、縦横比が崩れないようにする */
    object-fit: cover;
    background: var(--bg-sunken);
  }
</style>
