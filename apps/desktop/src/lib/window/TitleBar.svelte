<script lang="ts">
  // 自作タイトルバー（DESIGN.md §8 / D34）。OS の枠は使わない。
  import { getVersion } from '@tauri-apps/api/app';
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

  /**
   * **いま動いている版**（オーナー指示・2026-09-10
   * 「現状のばーじよんが、ヘッダーかふったーにうっすら記載してください」）。
   *
   * **うっすら出す。**目立たせる所ではないが、
   * **どの版を触っているか分からないまま報告が来る**のがいちばん困る
   * （「直ったはず」と「直っていない」が、版の違いだけで起きる）。
   *
   * 読めなければ**出さない**（画面の外で動かしているときなど）。
   * 「不明」と書くより、無いほうがよい。
   */
  let 版 = $state('');
  $effect(() => {
    void getVersion()
      .then((v) => (版 = v))
      .catch(() => {
        版 = '';
      });
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
  <span class="lead">
    <!--
      **割符の印。**アプリの顔（`icons/`）と同じ物を、帯の大きさで描く。
      **`--accent` を使わない**（DESIGN.md §4-A）—— アクセントは
      「操作できる一点」の色で、アプリの顔はそこから外してある。
      2026-09-08 まで、ここだけアクセント色の四角が置かれていた。
    -->
    <!--
      寸法は `scripts/make-icon.py` の比をそのまま使う（16 を 1 とする）——
      角丸 0.22 ／ 板 0.50 × 0.66 ／ 割れ目 0.095 ／ 振れ幅 0.105 ／ 歯 2 つ。
      **目分量で描き直さない。**アイコンと帯で形が違うと、同じ物に見えない。
    -->
    <svg class="brand-mark" viewBox="0 0 16 16" aria-hidden="true" focusable="false">
      <rect width="16" height="16" rx="3.52" fill="#cdb295" />
      <rect x="4" y="2.72" width="8" height="10.56" rx="0.72" fill="#7b5a3e" />
      <!-- 割れ目。**両側が同じ線で抜かれる = 噛み合う**（片方をずらして描かない） -->
      <path
        d="M9.68 2.72 L6.32 5.36 L9.68 8 L6.32 10.64 L9.68 13.28"
        fill="none"
        stroke="#cdb295"
        stroke-width="1.52"
      />
    </svg>
    {t('app.name')}
    {#if 版}
      <span class="version">{版}</span>
    {/if}
  </span>
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
  /*
    **版はうっすら。**名前と同じ強さで出すと、名前が読みにくくなる。
    数字は等幅にする（0.1.10 と 0.1.9 が並んだときに桁がずれない）。
  */
  .version {
    font-weight: 400;
    font-size: 0.72rem;
    color: var(--text-secondary);
    opacity: 0.6;
    font-variant-numeric: tabular-nums;
  }
  .brand-mark {
    width: 14px;
    height: 14px;
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
