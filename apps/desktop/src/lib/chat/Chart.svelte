<script lang="ts">
  // **発言の中のグラフを描く**（11-17・読むのは `./chart`）。
  //
  // 棒と積み上げは**横向き**にする。横軸の名前（店・SNS など）は日本語で長く、縦の棒の下には収まらない。
  // 線と縦の棒は日付の並び（毎日の推移）。円は 1 列の内訳。**色だけで言わない** —— 値の列が 2 つ以上なら凡例を出し、棒の端に数を書く。
  import { 目盛り, 数の字, 行の合計, 円の切れ, 名を書く番 as 書く番, 線は動きに合わせるか, type 図 as 図の型 } from './chart';

  let { 図 }: { 図: 図の型 } = $props();

  /** 値の列の色。意味の色を借りる（明るい画面・暗い画面の両方で決めてある）。 */
  const 色たち = [
    'var(--info-fg)',
    'var(--warning-fg)',
    'var(--success-fg)',
    'var(--danger-fg)',
    'var(--neutral-fg)',
    'var(--accent)',
  ];
  const 色 = (k: number) => 色たち[k % 色たち.length];

  const 幅 = 360;
  const 名の幅 = 96;
  const 数の幅 = 44;
  const 棒の高さ = 12;
  const 間 = 6;

  const 全部の値 = $derived(図.行たち.flatMap((r) => r.値.filter((v): v is number => v !== null)));
  const 刻み = $derived(
    図.形 === '積み上げ'
      ? 目盛り(0, Math.max(...図.行たち.map(行の合計)))
      : 目盛り(Math.min(...全部の値), Math.max(...全部の値), 4, !(図.形 === '線' && 線は動きに合わせるか(全部の値))),
  );
  const 下 = $derived(刻み[0]);
  const 上 = $derived(刻み[刻み.length - 1]);

  // ── 横の棒（棒・積み上げ） ──
  const 描く幅 = 幅 - 名の幅 - 数の幅;
  const 横へ = (v: number) => 名の幅 + ((v - 下) / (上 - 下)) * 描く幅;
  const 一行の高さ = $derived(図.形 === '積み上げ' ? 棒の高さ : 棒の高さ * 図.系列.length);
  const 棒の図の高さ = $derived(図.行たち.length * (一行の高さ + 間) + 18);

  // ── 線 ──
  const 線の高さ = 180;
  const 線の左 = 44;
  const 線の右 = 12;
  const 線の上 = 8;
  const 線の下 = 22;
  const 線の横 = (i: number) =>
    図.行たち.length === 1
      ? (線の左 + 幅 - 線の右) / 2
      : 線の左 + (i / (図.行たち.length - 1)) * (幅 - 線の左 - 線の右);
  const 線の縦 = (v: number) => 線の上 + (1 - (v - 下) / (上 - 下)) * (線の高さ - 線の上 - 線の下);
  /** 横軸の名前。6 つまでは全部、多いと重なるので間引く。 */
  const 名を書く番 = $derived(書く番(図.行たち.length));

  // ── 縦の棒（線と同じ枠。日ごとに 1 本、列が 2 つ以上なら並べる） ──
  const 帯 = $derived((幅 - 線の左 - 線の右) / Math.max(1, 図.行たち.length));
  const 柱の幅 = $derived((帯 * 0.7) / 図.系列.length);
  const 柱の左 = (i: number, k: number) => 線の左 + 帯 * i + 帯 * 0.15 + 柱の幅 * k;
  const 帯の中 = (i: number) => 線の左 + 帯 * (i + 0.5);

  // ── 円（ドーナツ） ──
  const 円の大きさ = 140;
  const 半径 = 60;
  const 内の半径 = 34;
  const 切れたち = $derived(円の切れ(図.行たち));
  function 切れの道(始め: number, 終わり: number): string {
    const 中 = 円の大きさ / 2;
    // 1 切れだけ（100%）は、弧が閉じて消えるので、わずかに欠く
    const 終 = 終わり - 始め >= 1 ? 始め + 0.9999 : 終わり;
    const 点 = (t: number, r: number) =>
      `${(中 + r * Math.sin(2 * Math.PI * t)).toFixed(2)},${(中 - r * Math.cos(2 * Math.PI * t)).toFixed(2)}`;
    const 大 = 終 - 始め > 0.5 ? 1 : 0;
    return `M${点(始め, 半径)}A${半径},${半径} 0 ${大} 1 ${点(終, 半径)}L${点(終, 内の半径)}A${内の半径},${内の半径} 0 ${大} 0 ${点(始め, 内の半径)}Z`;
  }

  function 線の道(k: number): string {
    let 道 = '';
    let 続き = false;
    for (const [i, r] of 図.行たち.entries()) {
      const v = r.値[k];
      if (v === null) {
        続き = false;
        continue;
      }
      道 += `${続き ? 'L' : 'M'}${線の横(i).toFixed(1)},${線の縦(v).toFixed(1)}`;
      続き = true;
    }
    return 道;
  }

  /** 読み上げ用の説明（絵を見られない人にも数が伝わるように）。 */
  const 説明 = $derived(
    `${図.題 ?? 'グラフ'}。` +
      図.行たち
        .map((r) => `${r.名}: ${r.値.map((v, k) => `${図.系列[k]} ${v === null ? 'なし' : 数の字(v)}`).join('、')}`)
        .join('。'),
  );
</script>

<figure class="chart">
  {#if 図.題}<figcaption class="title">{図.題}</figcaption>{/if}
  {#if 図.系列.length > 1 && 図.形 !== '円'}
    <div class="legend">
      {#each 図.系列 as 名, k (k)}
        <span class="key"><span class="swatch" style:background={色(k)}></span>{名}</span>
      {/each}
    </div>
  {/if}

  {#if 図.形 === '線'}
    <svg viewBox="0 0 {幅} {線の高さ}" role="img" aria-label={説明}>
      {#each 刻み as v (v)}
        <line class="grid" x1={線の左} x2={幅 - 線の右} y1={線の縦(v)} y2={線の縦(v)} />
        <text class="tick" x={線の左 - 4} y={線の縦(v) + 3} text-anchor="end">{数の字(v)}</text>
      {/each}
      {#each 図.系列 as _, k (k)}
        <path class="line" d={線の道(k)} style:stroke={色(k)} />
        {#each 図.行たち as r, i (i)}
          {#if r.値[k] !== null}
            <circle cx={線の横(i)} cy={線の縦(r.値[k] ?? 0)} r="2.5" style:fill={色(k)}>
              <title>{r.名}・{図.系列[k]}: {数の字(r.値[k] ?? 0)}</title>
            </circle>
          {/if}
        {/each}
      {/each}
      {#each 図.行たち as r, i (i)}
        {#if 名を書く番.has(i)}
          <text
            class="tick"
            x={線の横(i)}
            y={線の高さ - 6}
            text-anchor={図.行たち.length === 1 ? 'middle' : i === 0 ? 'start' : i === 図.行たち.length - 1 ? 'end' : 'middle'}
            >{r.名}</text
          >
        {/if}
      {/each}
    </svg>
  {:else if 図.形 === '縦棒'}
    <svg viewBox="0 0 {幅} {線の高さ}" role="img" aria-label={説明}>
      {#each 刻み as v (v)}
        <line class="grid" x1={線の左} x2={幅 - 線の右} y1={線の縦(v)} y2={線の縦(v)} />
        <text class="tick" x={線の左 - 4} y={線の縦(v) + 3} text-anchor="end">{数の字(v)}</text>
      {/each}
      {#each 図.行たち as r, i (i)}
        {#each r.値 as v, k (k)}
          {#if v !== null}
            <rect
              x={柱の左(i, k)}
              y={Math.min(線の縦(0), 線の縦(v))}
              width={Math.max(1, 柱の幅 - 1)}
              height={Math.max(1, Math.abs(線の縦(v) - 線の縦(0)))}
              style:fill={色(k)}
            >
              <title>{r.名}・{図.系列[k]}: {数の字(v)}</title>
            </rect>
          {/if}
        {/each}
        {#if 名を書く番.has(i)}
          <text class="tick" x={帯の中(i)} y={線の高さ - 6} text-anchor="middle">{r.名}</text>
        {/if}
      {/each}
    </svg>
  {:else if 図.形 === '円'}
    <div class="pie">
      <svg viewBox="0 0 {円の大きさ} {円の大きさ}" role="img" aria-label={説明} class="donut">
        {#each 図.行たち as r, i (i)}
          {#if 切れたち[i].割合 > 0}
            <path d={切れの道(切れたち[i].始め, 切れたち[i].終わり)} style:fill={色(i)}>
              <title>{r.名}: {数の字(r.値[0] ?? 0)}（{Math.round(切れたち[i].割合 * 100)}%）</title>
            </path>
          {/if}
        {/each}
        <text class="total" x={円の大きさ / 2} y={円の大きさ / 2 + 4} text-anchor="middle"
          >{数の字(図.行たち.reduce((s, r) => s + (r.値[0] ?? 0), 0))}</text
        >
      </svg>
      <div class="pie-keys">
        {#each 図.行たち as r, i (i)}
          <span class="key"
            ><span class="swatch" style:background={色(i)}></span>{r.名}<span class="num"
              >{数の字(r.値[0] ?? 0)}（{Math.round(切れたち[i].割合 * 100)}%）</span
            ></span
          >
        {/each}
      </div>
    </div>
  {:else}
    <svg viewBox="0 0 {幅} {棒の図の高さ}" role="img" aria-label={説明}>
      {#each 刻み as v (v)}
        <line class="grid" x1={横へ(v)} x2={横へ(v)} y1="0" y2={棒の図の高さ - 14} />
        <text class="tick" x={横へ(v)} y={棒の図の高さ - 3} text-anchor="middle">{数の字(v)}</text>
      {/each}
      {#each 図.行たち as r, i (i)}
        {@const 頭 = i * (一行の高さ + 間)}
        <text class="name" x={名の幅 - 6} y={頭 + 一行の高さ / 2 + 4} text-anchor="end">
          {r.名.length > 9 ? `${r.名.slice(0, 8)}…` : r.名}<title>{r.名}</title>
        </text>
        {#if 図.形 === '積み上げ'}
          {@const 端 = r.値.map((_, k) => r.値.slice(0, k).reduce<number>((s, v) => s + (v ?? 0), 0))}
          {#each r.値 as v, k (k)}
            {#if v !== null && v > 0}
              <rect x={横へ(端[k])} y={頭} width={横へ(端[k] + v) - 横へ(端[k])} height={棒の高さ} style:fill={色(k)}>
                <title>{r.名}・{図.系列[k]}: {数の字(v)}</title>
              </rect>
            {/if}
          {/each}
          <text class="value" x={横へ(行の合計(r)) + 4} y={頭 + 棒の高さ / 2 + 4}>{数の字(行の合計(r))}</text>
        {:else}
          {#each r.値 as v, k (k)}
            {#if v !== null}
              {@const 左 = Math.min(横へ(0), 横へ(v))}
              <rect
                x={左}
                y={頭 + k * 棒の高さ}
                width={Math.max(1, Math.abs(横へ(v) - 横へ(0)))}
                height={棒の高さ - 2}
                style:fill={色(k)}
              >
                <title>{r.名}・{図.系列[k]}: {数の字(v)}</title>
              </rect>
              <text class="value" x={Math.max(横へ(0), 横へ(v)) + 4} y={頭 + k * 棒の高さ + 棒の高さ / 2 + 3}>{数の字(v)}</text>
            {/if}
          {/each}
        {/if}
      {/each}
    </svg>
  {/if}
</figure>

<style>
  .chart {
    margin: 0;
    max-width: 480px;
  }
  .title {
    font-weight: 700;
    margin-bottom: 4px;
  }
  .legend {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 12px;
    margin-bottom: 4px;
    font-size: 0.9em;
    color: var(--text-secondary);
  }
  .key {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }
  .swatch {
    display: inline-block;
    width: 10px;
    height: 10px;
    border-radius: 2px;
  }
  svg {
    display: block;
    width: 100%;
    height: auto;
    overflow: visible;
  }
  .grid {
    stroke: var(--border);
    stroke-width: 1;
  }
  .tick,
  .value,
  .name {
    font-size: 10px;
    fill: var(--text-secondary);
  }
  .name {
    fill: var(--text-primary);
  }
  .line {
    fill: none;
    stroke-width: 2;
  }
  /* 円は左に輪、右に名前・数・割合（色だけで言わない） */
  .pie {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
  }
  .donut {
    width: 140px;
    flex: none;
  }
  .total {
    font-size: 13px;
    font-weight: 700;
    fill: var(--text-primary);
  }
  .pie-keys {
    display: flex;
    flex-direction: column;
    gap: 2px;
    font-size: 0.9em;
  }
  .num {
    margin-left: 6px;
    color: var(--text-secondary);
  }
</style>
