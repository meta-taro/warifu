<script lang="ts">
  // 自作タイトルバー（DESIGN.md §8 / D34）。OS の枠は使わない。
  import { MESSAGES, type MessageKey } from '../i18n/messages';
  import type { Locale } from '../i18n/locales';
  import { controlsFor } from './titlebar';
  import { isMaximized, runControl } from './controls';

  interface Props {
    locale: Locale;
    /** 帯の中央に出す、今の状態。無ければ空のままにする。 */
    status?: string;
  }
  let { locale, status = '' }: Props = $props();

  let maximized = $state(false);
  $effect(() => {
    void isMaximized().then((v) => (maximized = v));
  });

  const t = (key: MessageKey) => MESSAGES[locale][key];
  const controls = $derived(controlsFor(maximized));

  async function press(id: 'minimize' | 'maximize' | 'close') {
    await runControl(id);
    maximized = await isMaximized();
  }
</script>

<!-- 地そのものが掴む所。中身は pointer-events:none で地へ貫通させる -->
<div class="bar" data-tauri-drag-region>
  <span class="lead"><span class="brand-dot"></span>{t('app.name')}</span>
  <span class="center">{status}</span>
  <span class="ctrls">
    {#each controls as c (c.id)}
      <button
        type="button"
        class:danger={c.danger}
        title={t(c.labelKey)}
        aria-label={t(c.labelKey)}
        onclick={() => press(c.id)}>{c.glyph}</button
      >
    {/each}
  </span>
</div>

<style>
  .bar {
    display: flex;
    align-items: stretch;
    height: var(--topbar-h);
    background: var(--bg-subtle);
    border-bottom: 1px solid var(--border);
    color: var(--text-primary);
    font-size: var(--text-sm-size);
    line-height: var(--text-sm-line);
    user-select: none;
  }
  /* 掴む所を広く取る。ボタンだけは貫通させない */
  .lead,
  .center {
    pointer-events: none;
  }
  .lead {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: 0 var(--space-4);
    font-weight: 600;
  }
  .brand-dot {
    width: 10px;
    height: 10px;
    border-radius: 3px;
    background: var(--accent);
    flex: none;
  }
  .center {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    min-width: 0;
    color: var(--text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .ctrls {
    display: flex;
    align-items: stretch;
  }
  /* 右上角にフル高で密着させる（Fitts の法則・DESIGN.md §8） */
  .ctrls button {
    width: 44px;
    border: 0;
    background: transparent;
    color: var(--text-secondary);
    font: inherit;
    font-size: var(--text-xs-size);
    cursor: default;
    transition: background var(--dur-fast) var(--ease), color var(--dur-fast) var(--ease);
  }
  .ctrls button:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }
  .ctrls button.danger:hover {
    background: var(--danger-bg);
    color: var(--danger-fg);
  }
  .ctrls button:focus-visible {
    outline: 3px solid var(--accent-subtle);
    outline-offset: -3px;
  }
</style>
