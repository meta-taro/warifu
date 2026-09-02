<script lang="ts">
  // 名簿（DESIGN.md §7）。**現在数と定員の両方**を出す（§4.3 / D27）。
  import LinkBadge from '../link/LinkBadge.svelte';
  import { MESSAGES, format } from '../i18n/messages';
  import type { Locale } from '../i18n/locales';
  import type { LinkPath } from '../link/path';
  import { clampCapacity } from './roster';

  export interface Member {
    /** 表示名。**公開鍵そのものは出さない**（出すなら等幅で全桁・§5） */
    name: string;
    /** 主催者はひとりだけ */
    host?: boolean;
    path: LinkPath;
  }

  interface Props {
    locale: Locale;
    members: readonly Member[];
    capacity: number;
  }
  let { locale, members, capacity }: Props = $props();

  // 招待に書かれた定員をそのまま信じない（D27）
  const shown = $derived(clampCapacity(capacity));
</script>

<section class="roster" aria-label={MESSAGES[locale]['app.name']}>
  <header>
    <span class="count"
      >{format(MESSAGES[locale]['roster.capacity'], {
        current: members.length,
        capacity: shown,
      })}</span
    >
  </header>
  <ul>
    {#each members as m (m.name)}
      <li>
        <span class="name">{m.name}</span>
        {#if m.host}<span class="host">主催</span>{/if}
        <LinkBadge {locale} path={m.path} />
      </li>
    {/each}
  </ul>
</section>

<style>
  .roster {
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--bg-subtle);
    overflow: hidden;
    min-width: 280px;
  }
  header {
    display: flex;
    justify-content: flex-end;
    padding: var(--space-2) var(--space-3);
    background: var(--bg-sunken);
    border-bottom: 1px solid var(--border);
  }
  .count {
    font-family: var(--font-mono);
    /* 桁が動くと読み違える（DESIGN.md §5） */
    font-variant-numeric: tabular-nums;
    font-size: var(--text-xs-size);
    color: var(--text-secondary);
  }
  ul {
    margin: 0;
    padding: 0;
    list-style: none;
  }
  li {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    border-top: 1px solid var(--border);
    font-size: var(--text-sm-size);
    line-height: var(--text-sm-line);
  }
  li:first-child {
    border-top: 0;
  }
  .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .host {
    font-size: var(--text-2xs-size);
    font-weight: var(--text-2xs-weight);
    color: var(--accent);
    background: var(--accent-subtle);
    border: 1px solid var(--accent-border);
    border-radius: var(--radius-sm);
    padding: 1px 6px;
  }
</style>
