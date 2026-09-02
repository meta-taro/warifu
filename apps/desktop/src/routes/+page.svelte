<script lang="ts">
  // M5-a の器。**まだ映像は出さない**（M5-b 以降）。
  import TitleBar from '$lib/window/TitleBar.svelte';
  import { MESSAGES, format } from '$lib/i18n/messages';
  import { resolveLocale, type Locale } from '$lib/i18n/locales';

  const locale: Locale = resolveLocale(
    typeof navigator === 'undefined' ? [] : [...navigator.languages],
  );
  const t = MESSAGES[locale];
</script>

<TitleBar {locale} status={format(t['roster.capacity'], { current: 0, capacity: 12 })} />

<main>
  <p class="empty">{t['app.name']}</p>
  <p class="hint">
    <!-- 空状態。まだ誰とも繋がっていないことを、繋がっているように見せない（DESIGN.md §2 原則 7） -->
    {t['link.unknown']}
  </p>
</main>

<style>
  main {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-2);
  }
  .empty {
    margin: 0;
    font-size: var(--text-xl-size);
    line-height: var(--text-xl-line);
    font-weight: var(--text-xl-weight);
    letter-spacing: var(--tracking-tight);
  }
  .hint {
    margin: 0;
    color: var(--text-tertiary);
    font-size: var(--text-sm-size);
    line-height: var(--text-sm-line);
  }
</style>
