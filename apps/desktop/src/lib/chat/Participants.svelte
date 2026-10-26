<script lang="ts">
  // **この部屋に誰が居るか**を、まんなかの欄の空いている所に並べる（0.1.38・`participants.ts`）。
  import Avatar from '$lib/contacts/Avatar.svelte';
  import type { Locale } from '$lib/i18n/locales';
  import { MESSAGES, format, type MessageKey } from '$lib/i18n/messages';
  import { エージェントの名を言い換える } from './name';
  import type { 参加者 } from './participants';

  interface Props {
    locale: Locale;
    並び: readonly 参加者[];
    /** 機械の鍵から、画面で呼ぶ名前。 */
    名: (key: string) => string;
    顔の画?: Readonly<Record<string, string>>;
    /** この機械のエージェントの名札（届く先と同じ形・`連動（docs）`）。 */
    席の名札?: (席: string) => string;
  }

  const { locale, 並び, 名, 顔の画 = {}, 席の名札 = (席: string) => 席 }: Props = $props();

  const t = (key: MessageKey) => MESSAGES[locale][key];
  const 主の名 = (主: '人' | 'エージェント') =>
    t(主 === 'エージェント' ? 'avatar.owner.agent' : 'avatar.owner.person');
  const エージェントの名 = (札: string) =>
    エージェントの名を言い換える(札, (n) => format(t('chat.agentOf'), { name: n }));
  const 相手にエージェントが居ないか = $derived(
    並び.some((p) => !p.自分 && p.エージェントたち.length === 0),
  );
</script>

{#if 並び.length > 0}
  <section class="people" aria-label={format(t('room.people'), { n: String(並び.length) })}>
    <h3>{format(t('room.people'), { n: String(並び.length) })}</h3>
    <ul>
      {#each 並び as p (p.key)}
        <li>
          <div class="who">
            <Avatar 種={p.key} 大きさ={24} 画像={顔の画[p.key] ?? null} 名={名(p.key)} {主の名} />
            <span class="name" title={名(p.key)}>{名(p.key)}</span>
            {#if p.自分}<span class="chip">{t('contacts.me')}</span>{/if}
            {#if p.主催}<span class="chip host">{t('room.people.host')}</span>{/if}
          </div>
          {#if p.エージェントたち.length > 0}
            <ul class="agents">
              {#each p.エージェントたち as 札 (札)}
                <li>
                  <span class="kind">{t('avatar.owner.agent')}</span>
                  <span class="name" title={エージェントの名(p.自分 ? 席の名札(札) : 札)}>{エージェントの名(p.自分 ? 席の名札(札) : 札)}</span>
                </li>
              {/each}
            </ul>
          {/if}
        </li>
      {/each}
    </ul>
    {#if 相手にエージェントが居ないか}
      <p class="hint">{t('room.people.hint')}</p>
    {/if}
  </section>
{/if}

<style>
  .people {
    margin-top: var(--space-4);
    padding-top: var(--space-3);
    border-top: 1px solid var(--border);
  }
  h3 {
    margin: 0 0 var(--space-2);
    font-size: var(--text-xs-size);
    font-weight: 600;
    color: var(--text-secondary);
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }
  .who {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-width: 0;
  }
  .who :global(.face) {
    border-radius: 50%;
    flex: none;
  }
  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: var(--text-sm-size);
  }
  .chip {
    flex: none;
    padding: 0 8px;
    border-radius: 999px;
    font-size: var(--text-xs-size);
    background: var(--neutral-bg);
    color: var(--neutral-fg);
  }
  .chip.host {
    background: var(--accent-subtle);
    color: var(--text-primary);
  }
  /* **機械の下に、その機械のエージェント**（顔の幅だけ下げる） */
  .agents {
    margin: var(--space-1) 0 0 32px;
    gap: var(--space-1);
  }
  .agents li {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-width: 0;
  }
  .kind {
    flex: none;
    padding: 0 6px;
    border: 1px dashed var(--border-strong);
    border-radius: 999px;
    font-size: var(--text-xs-size);
    color: var(--text-secondary);
  }
  .hint {
    margin: var(--space-2) 0 0;
    font-size: var(--text-xs-size);
    line-height: var(--text-xs-line);
    color: var(--text-tertiary);
  }
</style>
