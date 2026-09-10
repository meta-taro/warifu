<script lang="ts">
  // 面の切り替え。**帯には載せない**（帯は掴む所・DESIGN §8）。
  //
  // 意味はタブなので、見た目が縦に並んでいても `role="tablist"` を使う。

  import type { Locale } from '$lib/i18n/locales';
  import { MESSAGES, type MessageKey } from '$lib/i18n/messages';
  import type { 画面の状態 } from '$lib/meeting/stage';
  import Icon, { type IconName } from '$lib/ui/Icon.svelte';
  import { 面の並び, 面の印, type 面 } from './panes';

  interface Props {
    locale: Locale;
    いまの面: 面;
    状態: 画面の状態;
    選ぶ: (面: 面) => void;
  }
  const { locale, いまの面, 状態, 選ぶ }: Props = $props();

  const t = (key: MessageKey) => MESSAGES[locale][key];

  const 見た目: Record<面, { icon: IconName; label: MessageKey }> = {
    連絡帳: { icon: 'book', label: 'pane.contacts' },
    会議: { icon: 'people', label: 'pane.meeting' },
    予定: { icon: 'calendar', label: 'pane.schedule' },
  };

  const 印の文言: Record<画面の状態, MessageKey | null> = {
    会議前: null,
    会議中: 'meeting.status.live',
    待っている: 'meeting.status.waiting',
  };
</script>

<div class="rail" role="tablist" aria-orientation="vertical">
  {#each 面の並び as 一つ (一つ)}
    {@const 印 = 面の印(一つ, 状態)}
    <!--
      **見出しと状態を、1 つの名前に混ぜない**（`issues/3`）。
      混ぜると `"部屋 相手を待っています"` が名前になり、**その名前では押せない。**
      状態は `aria-describedby` の側へ回す。
    -->
    <button
      type="button"
      role="tab"
      aria-selected={一つ === いまの面}
      aria-label={t(見た目[一つ].label)}
      aria-describedby={印 && 印の文言[印] ? `印-${一つ}` : undefined}
      class:on={一つ === いまの面}
      onclick={() => 選ぶ(一つ)}
    >
      <Icon name={見た目[一つ].icon} size={20} />
      <span class="name">{t(見た目[一つ].label)}</span>
      <!-- **色だけで言わない。**印には必ず文字を添える（DESIGN §2 原則 6） -->
      {#if 印 && 印の文言[印]}<span class="mark" id="印-{一つ}">{t(印の文言[印])}</span>{/if}
    </button>
  {/each}
</div>

<style>
  .rail {
    display: flex;
    flex-direction: column;
    gap: 4px;
    flex: none;
    width: 92px;
    padding: var(--space-2) 6px;
    border-right: 1px solid var(--border);
  }
  button {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
    padding: 8px 4px;
    font: inherit;
    font-size: var(--text-2xs-size);
    color: var(--text-secondary);
    background: transparent;
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    cursor: pointer;
  }
  button.on {
    color: var(--text-primary);
    background: var(--bg-subtle);
    border-color: var(--border);
  }
  .name {
    font-size: var(--text-2xs-size);
    /* **折らない。**「ルーム」が「ルー／ム」に割れていた（2026-09-10・縦長で実測） */
    white-space: nowrap;
  }
  .mark {
    font-size: var(--text-2xs-size);
    color: var(--accent);
  }
  /* 狭いときは横に寝かせる。**縦のままだと会話が痩せる** */
  @media (max-width: 860px) {
    .rail {
      flex-direction: row;
      /* **上に敷く帯にする。**幅を持たせないと、縦に寝た列の中で真ん中に浮く
         （2026-09-10 に縦長で実測。3 つのタブが宙に並んでいた） */
      width: 100%;
      justify-content: flex-start;
      padding: 6px var(--space-2);
      border-right: none;
      border-bottom: 1px solid var(--border);
    }
    button {
      flex: none;
      flex-direction: row;
      gap: 6px;
      padding: 6px 10px;
    }
  }
</style>
