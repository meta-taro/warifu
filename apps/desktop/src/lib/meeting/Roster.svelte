<script lang="ts">
  // 名簿（DESIGN.md §7）。**現在数と定員の両方**を出す（§4.3 / D27）。
  import LinkBadge from '../link/LinkBadge.svelte';
  import Icon from '../ui/Icon.svelte';
  import { MESSAGES, format } from '../i18n/messages';
  import type { Locale } from '../i18n/locales';
  import type { LinkPath } from '../link/path';
  import { clampCapacity } from './roster';
  import { 呼び名 } from './names';

  export interface Member {
    /** 公開鍵（base32・全桁）。**名前ではなくこれで数える**（呼び名は変わる） */
    key: string;
    /** 自分か */
    me?: boolean;
    /** 主催者はひとりだけ */
    host?: boolean;
    path: LinkPath;
  }

  interface Props {
    locale: Locale;
    members: readonly Member[];
    capacity: number;
    /**
     * 公開鍵 → 画面に出す名前。
     *
     * **呼ぶ名の決め方は 1 か所に置く**（`routes/+page.svelte` の `画面での名`）——
     * 2 か所で決めると、**同じ人が会話と名簿で別の名前で出る**
     * （2026-09-09 に実物で見た）。
     */
    names?: Readonly<Record<string, string>>;
    /** 呼び名を付ける。**空にすると忘れる** */
    onRename?: (key: string, label: string) => void;
    /**
     * いまこのPCにつながっている、同じ PC の AI。
     *
     * **ルームに誰が居るかを見たときに、AI が居ないのは不自然である**
     * （`issues/012`「この PC で会議するとき、私とあなたはセットでしょっていう」）。
     *
     * **`n / 定員` には数えない。**定員はルームに居る人の数である（§4.3 / **D27**）。
     * 数に入れると、**割符 1 本 = 1 人**（D12）とずれる。
     */
    このPCのAIたち?: readonly string[];
  }
  let { locale, members, capacity, names = {}, onRename, このPCのAIたち = [] }: Props = $props();

  const t = (key: keyof (typeof MESSAGES)[Locale]) => MESSAGES[locale][key];

  /** いま名前を付けている相手（公開鍵）。**1 人ずつ** */
  let 書き換え中 = $state<string | null>(null);
  let 下書き = $state('');

  function 始める(key: string) {
    書き換え中 = key;
    下書き = names[key] ?? '';
  }
  function 決める(key: string) {
    onRename?.(key, 下書き);
    書き換え中 = null;
  }

  // 招待に書かれた定員をそのまま信じない（D27）
  const shown = $derived(clampCapacity(capacity));
</script>

<section class="roster" aria-label={MESSAGES[locale]['app.name']}>
  <header>
    <Icon name="people" />
    <span class="count"
      >{format(MESSAGES[locale]['roster.capacity'], {
        current: members.length,
        capacity: shown,
      })}</span
    >
  </header>
  <ul>
    {#each members as m (m.key)}
      <li>
        {#if 書き換え中 === m.key}
          <!-- **その場で付ける。**別の画面へ行かせない -->
          <input
            type="text"
            bind:value={下書き}
            placeholder={t('roster.name.placeholder')}
            onkeydown={(e) => {
              if (e.key === 'Enter') 決める(m.key);
              if (e.key === 'Escape') 書き換え中 = null;
            }}
          />
          <button type="button" class="quiet" onclick={() => 決める(m.key)}>
            {t('roster.name.save')}
          </button>
        {:else}
          <span class="name">{呼び名(names, m.key)}{m.me ? `（${t('tile.me')}）` : ''}</span>
          {#if m.host}<span class="host">主催</span>{/if}
          <LinkBadge {locale} path={m.path} />
          {#if onRename && !m.me}
            <!-- **鍵の頭では、人もエージェントも見分けが付かない。**呼び名を付けられるようにする -->
            <button
              type="button"
              class="quiet"
              title={t('roster.name.action')}
              aria-label={t('roster.name.action')}
              onclick={() => 始める(m.key)}>{t('roster.name.action')}</button
            >
          {/if}
        {/if}
      </li>
    {/each}
  </ul>

  {#if このPCのAIたち.length > 0}
    <!-- **このエージェントに居るもの。**ルームの人数（定員）には数えない -->
    <h3>{MESSAGES[locale]['contacts.this']}</h3>
    <ul class="desk">
      {#each このPCのAIたち as 呼び方 (呼び方)}
        <li><span class="name">{呼び方}</span></li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  h3 {
    margin: var(--space-2) var(--space-3) 2px;
    font-size: var(--text-2xs-size);
    font-weight: 600;
    color: var(--text-tertiary);
  }
  /* **ルームの人と見分けが付く形にする。**同じ並びに混ぜない */
  ul.desk li {
    color: var(--text-secondary);
  }
  .roster {
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--bg-subtle);
    overflow: hidden;
    min-width: 280px;
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    color: var(--text-tertiary);
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
  /* **形は `forms.css`。**行の中に収めるぶんだけ詰める */
  .roster li input[type='text'] {
    flex: 1;
    min-width: 0;
    min-height: 28px;
    font-size: var(--text-xs-size);
    padding: 3px var(--space-2);
  }
  .roster li button.quiet {
    flex: none;
    background: transparent;
    color: var(--accent);
    border: 1px solid var(--accent-border);
    border-radius: var(--radius-sm);
    padding: 2px var(--space-2);
    font-size: var(--text-2xs-size);
    cursor: pointer;
  }
  .roster li button.quiet:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
</style>
