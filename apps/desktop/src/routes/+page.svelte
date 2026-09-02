<script lang="ts">
  // M5-b の器。**まだ映像は出さない。**
  // ここに出ているのは、経路と名簿の見え方を実物で確かめるための静的な状態である。
  import TitleBar from '$lib/window/TitleBar.svelte';
  import Roster, { type Member } from '$lib/meeting/Roster.svelte';
  import { MESSAGES, format } from '$lib/i18n/messages';
  import { DEFAULT_CAPACITY } from '$lib/meeting/roster';
  import { resolveLocale, type Locale } from '$lib/i18n/locales';

  const locale: Locale = resolveLocale(
    typeof navigator === 'undefined' ? [] : [...navigator.languages],
  );
  const t = MESSAGES[locale];

  // **見本である。**実際の名簿は M5-c で warifu-meeting から流れてくる。
  const members: Member[] = [
    { name: '自分', host: true, path: 'direct' },
    { name: '相手 A', path: 'relayed' },
    { name: '相手 B', path: 'unknown' },
  ];
</script>

<TitleBar
  {locale}
  status={format(t['roster.capacity'], { current: members.length, capacity: DEFAULT_CAPACITY })}
/>

<main>
  <div class="stage">
    <p class="pending">映像はまだ出しません（M5-c）</p>
  </div>
  <aside>
    <Roster {locale} {members} capacity={DEFAULT_CAPACITY} />
  </aside>
</main>

<style>
  main {
    flex: 1;
    display: grid;
    grid-template-columns: 1fr auto;
    gap: var(--space-4);
    padding: var(--space-4);
    align-items: start;
  }
  .stage {
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 320px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--bg-sunken);
  }
  .pending {
    margin: 0;
    color: var(--text-tertiary);
    font-size: var(--text-sm-size);
  }
  /* 窓を狭めたら縦に積む。名簿を先に畳まない（人数と経路が一番見たいもの） */
  @media (max-width: 720px) {
    main {
      grid-template-columns: 1fr;
    }
  }
</style>
