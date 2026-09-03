<script lang="ts">
  import '$lib/styles/tokens.css';
  import ContextMenu from '$lib/ui/ContextMenu.svelte';
  import { resolveLocale, type Locale } from '$lib/i18n/locales';

  let { children } = $props();

  // 右クリックは窓のどこでも出る。**画面ごとに置かない**（置き忘れが英語のまま残る）
  const locale: Locale = resolveLocale(
    typeof navigator === 'undefined' ? [] : [...navigator.languages],
  );
</script>

<ContextMenu {locale} />

<div class="app">
  {@render children()}
</div>

<style>
  /* **窓そのものは動かさない。**帯（タイトルバー）は上に据えたまま、
     中身だけが動くようにする。ここを開けると帯まで一緒にスクロールする */
  :global(html) {
    height: 100%;
    overflow: hidden;
  }
  :global(body) {
    height: 100%;
    overflow: hidden;
    margin: 0;
    background: var(--bg-app);
    color: var(--text-primary);
    font-family: var(--font-ui);
    font-size: var(--text-base-size);
    line-height: var(--text-base-line);
    -webkit-font-smoothing: antialiased;
  }
  .app {
    display: flex;
    flex-direction: column;
    height: 100%;
    /* 子（帯と中身）の側で動かす。ここでは動かさない */
    overflow: hidden;
  }
</style>
