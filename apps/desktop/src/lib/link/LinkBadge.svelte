<script lang="ts">
  // 経路インジケータ（DESIGN.md §4.1）。
  // **色だけで言わない。**色・ラベル・形の三重で示す（§2 原則 6）。
  // 色覚多様性でも、greyscale のスクリーンショットでも読めるようにするため。
  import { MESSAGES, type MessageKey } from '../i18n/messages';
  import type { Locale } from '../i18n/locales';
  import type { LinkPath } from './path';

  interface Props {
    locale: Locale;
    path: LinkPath;
  }
  let { locale, path }: Props = $props();

  const LABEL: Record<LinkPath, MessageKey> = {
    direct: 'link.direct',
    relayed: 'link.relayed',
    unknown: 'link.unknown',
  };
</script>

<span class="badge {path}">
  <!-- 形も状態を運ぶ: ● 塗り / ◐ 半分 / ○ 抜き -->
  <span class="dot {path}" aria-hidden="true"></span>
  {MESSAGES[locale][LABEL[path]]}
</span>

<style>
  .badge {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 3px var(--space-3) 3px 10px;
    border-radius: var(--radius-full);
    border: 1px solid transparent;
    font-size: var(--text-xs-size);
    line-height: var(--text-xs-line);
    font-weight: var(--text-2xs-weight);
    white-space: nowrap;
  }
  .badge.direct {
    background: var(--link-direct-bg);
    color: var(--link-direct-fg);
  }
  .badge.relayed {
    background: var(--link-relayed-bg);
    color: var(--link-relayed-fg);
  }
  .badge.unknown {
    background: var(--link-unknown-bg);
    color: var(--link-unknown-fg);
  }
  .dot {
    width: 9px;
    height: 9px;
    border-radius: var(--radius-full);
    flex: none;
  }
  .dot.direct {
    background: currentColor;
  }
  .dot.relayed {
    /* 半分だけ塗る。中継は異常ではないが、隠しもしない */
    background: linear-gradient(90deg, currentColor 50%, transparent 50%);
    box-shadow: inset 0 0 0 1.5px currentColor;
  }
  .dot.unknown {
    background: transparent;
    box-shadow: inset 0 0 0 1.5px currentColor;
  }
</style>
