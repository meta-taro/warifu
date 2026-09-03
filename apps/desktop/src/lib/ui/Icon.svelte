<script lang="ts" module>
  // 画面のアイコン（DESIGN.md §2「静けさ優先」）。
  //
  // **外から読み込まない。**CSP は外部を読まない設計（`tauri.conf.json`）で、
  // アイコンフォントも CDN も使えない。**同梱の SVG を線で描く。**
  //
  // - 線は `currentColor`。**色を持たない**ので、置いた場所の文字色に従う
  // - 既定 16px。文字の隣に置いても行が跳ねない大きさ
  // - **文字の代わりにしない。**ラベルと一緒に出す（§2 原則 6・色や形だけで言わない）

  export type IconName =
    | 'mic'
    | 'mic-off'
    | 'camera'
    | 'camera-off'
    | 'blur'
    | 'headphones'
    | 'key'
    | 'enter'
    | 'people'
    | 'copy'
    | 'check';

  /** 24×24 の線画。塗りは持たない。 */
  const PATHS: Record<IconName, string[]> = {
    mic: ['M12 4a3 3 0 0 1 3 3v5a3 3 0 0 1-6 0V7a3 3 0 0 1 3-3z', 'M5 11a7 7 0 0 0 14 0', 'M12 18v3', 'M9 21h6'],
    'mic-off': [
      'M12 4a3 3 0 0 1 3 3v5a3 3 0 0 1-6 0V7a3 3 0 0 1 3-3z',
      'M5 11a7 7 0 0 0 14 0',
      'M12 18v3',
      'M9 21h6',
      'M4 3l16 18',
    ],
    camera: ['M3 7.5A1.5 1.5 0 0 1 4.5 6h9A1.5 1.5 0 0 1 15 7.5v9A1.5 1.5 0 0 1 13.5 18h-9A1.5 1.5 0 0 1 3 16.5z', 'M15 10.5l6-3.5v10l-6-3.5z'],
    'camera-off': [
      'M3 7.5A1.5 1.5 0 0 1 4.5 6h9A1.5 1.5 0 0 1 15 7.5v9A1.5 1.5 0 0 1 13.5 18h-9A1.5 1.5 0 0 1 3 16.5z',
      'M15 10.5l6-3.5v10l-6-3.5z',
      'M3 3l18 18',
    ],
    blur: ['M12 8.5a2.5 2.5 0 1 0 0-5 2.5 2.5 0 0 0 0 5z', 'M6 20a6 6 0 0 1 12 0', 'M3 6h2', 'M19 6h2', 'M2.5 10.5H4', 'M20 10.5h1.5'],
    headphones: ['M4 14v-2a8 8 0 0 1 16 0v2', 'M4 14h3v6H5.5A1.5 1.5 0 0 1 4 18.5z', 'M20 14h-3v6h1.5a1.5 1.5 0 0 0 1.5-1.5z'],
    key: ['M16 5a4 4 0 1 0 0 8 4 4 0 0 0 0-8z', 'M13.2 11.8L4 21', 'M7 18l2 2', 'M5.2 19.8l1.6 1.6'],
    enter: ['M12 4H5a1 1 0 0 0-1 1v14a1 1 0 0 0 1 1h7', 'M14 12h7', 'M18 8l4 4-4 4'],
    copy: ['M9 9h9a1 1 0 0 1 1 1v9a1 1 0 0 1-1 1H9a1 1 0 0 1-1-1v-9a1 1 0 0 1 1-1z', 'M5 15H4a1 1 0 0 1-1-1V5a1 1 0 0 1 1-1h9a1 1 0 0 1 1 1v1'],
    check: ['M4 12.5l5 5L20 6.5'],
    people: ['M9 11a3 3 0 1 0 0-6 3 3 0 0 0 0 6z', 'M3 20a6 6 0 0 1 12 0', 'M16 5.6a3 3 0 0 1 0 5.8', 'M17.5 14.2A6 6 0 0 1 21 20'],
  };
</script>

<script lang="ts">
  interface Props {
    name: IconName;
    /** 一辺の px。行の中に置くなら 16、見出しなら 18。 */
    size?: number;
  }
  let { name, size = 16 }: Props = $props();
</script>

<!-- 文字の隣に置く飾り。**読み上げには出さない**（ラベルが別にある） -->
<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width="1.75"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
  focusable="false"
>
  {#each PATHS[name] as d (d)}
    <path {d} />
  {/each}
</svg>

<style>
  svg {
    flex: none;
    /* 文字の並びに沿わせる。行の高さを押し広げない */
    vertical-align: -0.15em;
  }
</style>
