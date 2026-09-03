<script lang="ts">
  // 右クリックのメニュー（D35）。
  //
  // **WebView が出す既定のメニューは、こちらの辞書を通らない。**英語のまま出る。
  // さらに開発者向けの項目（要素を検証・再読み込み）まで見えてしまう。
  // だから既定を止めて、**自前で出す。**
  //
  // **貼り付けだけは、こちらから実行できないことがある。**
  // WebView は script からの読み取りを許さない場合があり、そのときは
  // **黙って何も起きないのではなく、⌘V を案内する。**
  import { MESSAGES, type MessageKey } from '../i18n/messages';
  import type { Locale } from '../i18n/locales';
  import Icon from './Icon.svelte';

  interface Props {
    locale: Locale;
  }
  let { locale }: Props = $props();
  const t = (key: MessageKey) => MESSAGES[locale][key];

  let open = $state(false);
  let x = $state(0);
  let y = $state(0);
  let editable = $state(false);
  let hasSelection = $state(false);
  let target = $state<HTMLInputElement | HTMLTextAreaElement | null>(null);
  let hint = $state('');

  function 開く(e: MouseEvent) {
    e.preventDefault();
    const el = e.target as HTMLElement | null;
    target =
      el instanceof HTMLTextAreaElement || el instanceof HTMLInputElement ? el : null;
    editable = target !== null && !target.readOnly;
    hasSelection = (window.getSelection()?.toString() ?? '') !== '' || 選択がある();
    // 何もできない場所では出さない。**空のメニューを出さない**
    if (!target && !hasSelection) return;
    x = e.clientX;
    y = e.clientY;
    hint = '';
    open = true;
  }

  function 選択がある(): boolean {
    if (!target) return false;
    return target.selectionStart !== target.selectionEnd;
  }

  function 閉じる() {
    open = false;
    hint = '';
  }

  function 切り取る() {
    document.execCommand('cut');
    閉じる();
  }

  function 写す() {
    document.execCommand('copy');
    閉じる();
  }

  async function 貼る() {
    try {
      const text = await navigator.clipboard.readText();
      if (!target) return;
      const s = target.selectionStart ?? target.value.length;
      const e = target.selectionEnd ?? s;
      target.value = target.value.slice(0, s) + text + target.value.slice(e);
      target.dispatchEvent(new Event('input', { bubbles: true }));
      閉じる();
    } catch {
      // **黙らない。**読み取りが許されない環境がある
      hint = t('edit.pasteHint');
    }
  }

  function 全部選ぶ() {
    target?.select();
    閉じる();
  }
</script>

<svelte:window
  oncontextmenu={開く}
  onclick={閉じる}
  onkeydown={(e) => e.key === 'Escape' && 閉じる()}
  onblur={閉じる}
/>

{#if open}
  <div class="menu" style="left: {x}px; top: {y}px" role="menu" tabindex="-1">
    {#if editable}
      <button type="button" role="menuitem" onclick={切り取る} disabled={!hasSelection}>
        {t('edit.cut')}
      </button>
    {/if}
    <button type="button" role="menuitem" onclick={写す} disabled={!hasSelection}>
      <Icon name="copy" />{t('edit.copy')}
    </button>
    {#if editable}
      <button type="button" role="menuitem" onclick={貼る}>{t('edit.paste')}</button>
    {/if}
    {#if target}
      <button type="button" role="menuitem" onclick={全部選ぶ}>{t('edit.selectAll')}</button>
    {/if}
    {#if hint}
      <p class="hint">{hint}</p>
    {/if}
  </div>
{/if}

<style>
  .menu {
    position: fixed;
    z-index: 100;
    min-width: 168px;
    display: flex;
    flex-direction: column;
    padding: var(--space-1);
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-md);
  }
  button {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 5px var(--space-2);
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-primary);
    font: inherit;
    font-size: var(--text-sm-size);
    text-align: left;
  }
  button:hover:not(:disabled) {
    background: var(--bg-hover);
  }
  button:disabled {
    color: var(--text-tertiary);
  }
  button:focus-visible {
    outline: 3px solid var(--accent-subtle);
    outline-offset: -3px;
  }
  .hint {
    margin: var(--space-1) var(--space-2) var(--space-1);
    font-size: var(--text-2xs-size);
    line-height: var(--text-2xs-line);
    color: var(--text-tertiary);
    max-width: 200px;
  }
</style>
