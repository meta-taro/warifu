<script lang="ts">
  // 左右の仕切り。**右の列の幅を、人がドラッグで変える。**
  // ダブルクリックで初めの幅に戻す（覚えた幅も捨てる）。
  //
  // 幅は親が持つ（`bind:幅`）。`null` は「初めの幅」—— CSS の既定に任せる。
  // 仕切りのすぐ右の要素を、右の列とみなす。
  import { MESSAGES, type MessageKey } from '../i18n/messages';
  import type { Locale } from '../i18n/locales';
  import { 一歩, 左に残す, 引いた幅, 幅を収める, 覚えた幅を読む } from './divider';

  interface Props {
    locale: Locale;
    幅: number | null;
    /** 窓を閉じても幅を覚えておく鍵（面ごとに分ける） */
    覚える鍵: string;
    /** 左に必ず残す幅（CSS の max-width と同じ数にする） */
    残す?: number;
  }
  let { locale, 幅 = $bindable(), 覚える鍵, 残す = 左に残す }: Props = $props();
  const t = (key: MessageKey) => MESSAGES[locale][key];

  let 引いている = $state(false);
  let 始めの幅 = 0;
  let 始めの位置 = 0;

  // **覚えた幅が読めなくても、初めの幅で出す**（覚えるのは便利のためだけ）
  $effect(() => {
    try {
      幅 = 覚えた幅を読む(localStorage.getItem(覚える鍵));
    } catch {
      幅 = null;
    }
  });

  function 覚える(次: number | null) {
    try {
      if (次 === null) localStorage.removeItem(覚える鍵);
      else localStorage.setItem(覚える鍵, String(次));
    } catch {
      // 覚えられない窓（保存が禁じられている等）でも、いまの幅は効いている
    }
  }

  function 枠の幅(el: HTMLElement): number {
    return el.parentElement?.clientWidth ?? 0;
  }

  function 右の幅(el: HTMLElement): number {
    return (el.nextElementSibling as HTMLElement | null)?.offsetWidth ?? 0;
  }

  function 掴む(e: PointerEvent) {
    if (e.button !== 0) return;
    const el = e.currentTarget as HTMLElement;
    el.setPointerCapture(e.pointerId);
    始めの幅 = 右の幅(el);
    始めの位置 = e.clientX;
    引いている = true;
    e.preventDefault();
  }

  function 動かす(e: PointerEvent) {
    if (!引いている) return;
    const el = e.currentTarget as HTMLElement;
    幅 = 引いた幅(始めの幅, 始めの位置, e.clientX, 枠の幅(el), 残す);
  }

  function 放す() {
    if (!引いている) return;
    引いている = false;
    覚える(幅);
  }

  function 戻す() {
    幅 = null;
    覚える(null);
  }

  function 鍵で動かす(e: KeyboardEvent) {
    const el = e.currentTarget as HTMLElement;
    if (e.key === 'Home' || e.key === 'Enter') {
      戻す();
    } else if (e.key === 'ArrowLeft' || e.key === 'ArrowRight') {
      const 向き = e.key === 'ArrowLeft' ? 1 : -1;
      幅 = 幅を収める(右の幅(el) + 向き * 一歩, 枠の幅(el), 残す);
      覚える(幅);
    } else {
      return;
    }
    e.preventDefault();
  }
</script>

<!-- 掴める仕切り（focus できる separator）は操作できる部品である。 -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
<div
  class="仕切り"
  class:引いている
  role="separator"
  aria-orientation="vertical"
  aria-label={t('pane.divider')}
  title={t('pane.divider')}
  tabindex="0"
  onpointerdown={掴む}
  onpointermove={動かす}
  onpointerup={放す}
  onpointercancel={放す}
  ondblclick={戻す}
  onkeydown={鍵で動かす}
></div>

<style>
  /* **列の間の隙間そのものを、掴める所にする。**細い線だけだと掴めない */
  .仕切り {
    flex: none;
    align-self: stretch;
    width: var(--space-4);
    cursor: col-resize;
    touch-action: none;
    position: relative;
  }
  .仕切り::after {
    content: '';
    position: absolute;
    top: 0;
    bottom: 0;
    left: 50%;
    width: 2px;
    transform: translateX(-50%);
    border-radius: 1px;
    background: transparent;
    transition: background 120ms;
  }
  .仕切り:hover::after,
  .仕切り:focus-visible::after,
  .仕切り.引いている::after {
    background: var(--border);
  }
  .仕切り:focus-visible {
    outline: none;
  }
</style>
