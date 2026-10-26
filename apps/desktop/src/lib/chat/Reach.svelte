<script lang="ts">
  // **誰に届くかを、顔つきで並べる**（2026-10-19）。
  //
  // 0.1.36 で会話の枠から切り出した。会話欄の上に 2 段取って会話を狭めていたので、
  // 連絡帳では**まんなかの欄の空いている所に縦に並べる**（`縦`）。
  import Avatar from '$lib/contacts/Avatar.svelte';
  import type { Locale } from '$lib/i18n/locales';
  import { MESSAGES, format, type MessageKey } from '$lib/i18n/messages';
  import { エージェントの名を言い換える } from './name';

  interface Props {
    locale: Locale;
    /** 届く先の名前（相手の機械の名札のまま）。 */
    届く先: readonly string[];
    /** 届く先ごとの顔の種。数が合わなければ字だけで出す。 */
    届く先の顔: readonly string[];
    顔の画?: Readonly<Record<string, string>>;
    /** 1 人 1 行で縦に並べるか（まんなかの欄）。 */
    縦?: boolean;
  }

  const { locale, 届く先, 届く先の顔, 顔の画 = {}, 縦 = false }: Props = $props();

  const t = (key: MessageKey) => MESSAGES[locale][key];
  const 主の名 = (主: '人' | 'エージェント') =>
    t(主 === 'エージェント' ? 'avatar.owner.agent' : 'avatar.owner.person');
  const 見せる名 = (名: string) =>
    エージェントの名を言い換える(名, (n) => format(t('chat.agentOf'), { name: n }));
</script>

{#if 届く先.length === 0}
  <p class="hint">{t('chat.reach.none')}</p>
{:else if 届く先の顔.length === 届く先.length}
  <div
    class="faces"
    class:縦
    aria-label={format(t('chat.reach'), { who: 届く先.map(見せる名).join(' ／ ') })}
  >
    <span class="faces-label">{format(t('chat.reach'), { who: '' }).trim()}</span>
    {#each 届く先 as 名, i (i)}
      <span class="who" title={見せる名(名)}>
        <Avatar 種={届く先の顔[i]} 大きさ={22} 画像={顔の画[届く先の顔[i]] ?? null} 名={見せる名(名)} {主の名} />
        <span class="who-name" title={見せる名(名)}>{見せる名(名)}</span>
      </span>
    {/each}
  </div>
{:else}
  <p class="hint reach">{format(t('chat.reach'), { who: 届く先.map(見せる名).join(' ／ ') })}</p>
{/if}

<style>
  .faces {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-1) var(--space-2);
  }
  /* **まんなかの欄では 1 人 1 行。**名前を切り詰めずに読める */
  .faces.縦 {
    flex-direction: column;
    align-items: stretch;
  }
  .faces-label {
    font-size: var(--text-xs-size);
    color: var(--text-secondary);
  }
  .who {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    max-width: 100%;
    padding: 2px 10px 2px 2px;
    border: 1px solid var(--border);
    border-radius: 999px;
    background: var(--bg-sunken);
    font-size: var(--text-xs-size);
    font-weight: 600;
  }
  .who :global(.face) {
    border-radius: 50%;
    flex: none;
  }
  .who-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .hint {
    margin: 0;
    font-size: var(--text-xs-size);
    line-height: var(--text-xs-line);
    color: var(--text-tertiary);
  }
  .hint.reach {
    color: var(--text-secondary);
  }
</style>
