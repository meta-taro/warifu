<script lang="ts">
  // 連絡帳の面。**割符の入口。**
  //
  // 「連絡帳があって、その人と、会議するのか、チャットするのか、
  // そしてメールの場合どうするのか、というダッシュボードが先に来るかと」
  // （オーナー・2026-09-07）
  //
  // **判断はここに置かない。**並びは `list.ts`、押せるかは `actions.ts` が決める。

  import type { Locale } from '$lib/i18n/locales';
  import { MESSAGES, type MessageKey } from '$lib/i18n/messages';
  import { 鍵の頭 } from '$lib/meeting/names';
  import Icon, { type IconName } from '$lib/ui/Icon.svelte';
  import { できること, type 口の種類 } from './actions';
  import { 机の印, 連絡帳を組む, type 行, type 素材 as 連絡帳の素材 } from './list';

  interface Props {
    locale: Locale;
    素材: 連絡帳の素材;
    /** いま選んでいる相手（公開鍵か、机の印）。 */
    選んでいる: string | null;
    選ぶ: (key: string) => void;
    /** 口を押した。 */
    押す: (種類: 口の種類, 相手: 行) => void;
    /** 呼んでいる最中の相手。**二度押しを止める。** */
    呼んでいる: string | null;
  }
  const { locale, 素材, 選んでいる, 選ぶ, 押す, 呼んでいる }: Props = $props();

  const t = (key: MessageKey) => MESSAGES[locale][key];

  const 区画 = $derived(連絡帳を組む(素材));
  const 相手 = $derived(
    区画.flatMap((s) => s.行たち).find((r) => r.key === 選んでいる) ?? null,
  );
  const 口たち = $derived(
    相手
      ? できること({
          種類: 相手.種類,
          住所を覚えている: 相手.住所を覚えている,
          いま会議に居る: 相手.いま会議に居る,
          机の人数: 素材.机の人数,
        })
      : [],
  );

  const 口の見た目: Record<口の種類, { icon: IconName; label: MessageKey }> = {
    chat: { icon: 'chat', label: 'act.chat' },
    call: { icon: 'people', label: 'act.call' },
    mail: { icon: 'mail', label: 'act.mail' },
  };

  /** 行に出す名前。**この PC の 2 行だけ文言の鍵を持つ。** */
  function 名(行: 行): string {
    if (行.種類 === '自分' || 行.種類 === 'AI') return t(行.name as MessageKey);
    return 行.name;
  }
</script>

<div class="pane">
  <div class="list">
    {#each 区画 as 一区画 (一区画.title)}
      <h2>{t(一区画.title as MessageKey)}</h2>
      {#if 一区画.行たち.length === 0}
        <p class="hint">{t('contacts.empty')}</p>
      {/if}
      {#each 一区画.行たち as 行 (行.key)}
        <button
          type="button"
          class="row"
          class:on={行.key === 選んでいる}
          onclick={() => 選ぶ(行.key)}
        >
          <Icon name={行.key === 机の印 ? 'desk' : 'people'} size={16} />
          <span class="name">{名(行)}</span>
          <!-- **在席は出さない。**相手が起動しているかは分からない -->
          {#if 行.key === 机の印}
            <span class="sub">{行.いま会議に居る ? `${素材.机の人数}` : '0'}</span>
          {/if}
        </button>
      {/each}
    {/each}
  </div>

  <div class="person">
    {#if !相手}
      <p class="hint">{t('contacts.pick')}</p>
    {:else}
      <h2>{名(相手)}</h2>
      {#if 相手.種類 === '人'}
        <p class="key">
          <span class="label">{t('contacts.key.label')}</span>{鍵の頭(相手.key)}
        </p>
      {/if}
      {#if 相手.種類 === 'AI' && 素材.机の人数 === 0}
        <p class="hint">{t('contacts.desk.none')}</p>
      {/if}

      <div class="acts">
        {#each 口たち as 口 (口.種類)}
          <div class="act">
            <button
              type="button"
              class:primary={口.種類 !== 'mail'}
              disabled={!口.押せる || 呼んでいる === 相手.key}
              onclick={() => 押す(口.種類, 相手)}
            >
              <Icon name={口の見た目[口.種類].icon} size={16} />
              {口.種類 === 'call' && 呼んでいる === 相手.key
                ? t('act.call.working')
                : t(口の見た目[口.種類].label)}
            </button>
            <!-- **押せないなら、理由を必ず出す。**
                 理由の無い「押せない」は、使う人には壊れているとしか見えない -->
            {#if 口.訳}<p class="why">{t(口.訳 as MessageKey)}</p>{/if}
          </div>
        {/each}
      </div>

      {#if 相手.種類 === '人'}
        <!-- **相手が起動しているかは分からない。**分からないと出す（§2 原則 7） -->
        <p class="hint">{t('contacts.presence.none')}</p>
      {/if}
    {/if}
  </div>
</div>

<style>
  .pane {
    display: grid;
    grid-template-columns: 300px 1fr;
    gap: var(--space-3);
    flex: 1;
    min-height: 0;
  }
  .list {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-height: 0;
    overflow-y: auto;
    padding: var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--bg-subtle);
  }
  .person {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    min-height: 0;
    overflow-y: auto;
    padding: var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--bg-subtle);
  }
  h2 {
    margin: var(--space-2) 0 4px;
    font-size: var(--text-sm-size);
    font-weight: 600;
  }
  .list h2:first-child {
    margin-top: 0;
  }
  .hint {
    margin: 0;
    font-size: var(--text-xs-size);
    line-height: var(--text-xs-line);
    color: var(--text-tertiary);
  }
  .row {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 7px 8px;
    font: inherit;
    font-size: var(--text-sm-size);
    text-align: left;
    color: var(--text-primary);
    background: transparent;
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    cursor: pointer;
  }
  .row.on {
    background: var(--bg-app);
    border-color: var(--border);
  }
  .row .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .row .sub {
    font-family: var(--font-mono);
    font-size: var(--text-2xs-size);
    font-variant-numeric: tabular-nums;
    color: var(--text-tertiary);
  }
  .key {
    margin: 0;
    font-family: var(--font-mono);
    font-size: var(--text-xs-size);
    color: var(--text-secondary);
  }
  .key .label {
    margin-right: 6px;
    font-family: var(--font-sans);
    color: var(--text-tertiary);
  }
  .acts {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    margin-top: var(--space-2);
  }
  .act {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  button:not(.row) {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    padding: 8px 14px;
    font: inherit;
    font-size: var(--text-sm-size);
    color: var(--text-primary);
    background: var(--bg-app);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    cursor: pointer;
  }
  button.primary {
    color: var(--on-accent);
    background: var(--accent);
    border-color: transparent;
  }
  /* **押せないものは、押せないように見せる**（D49） */
  button:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
  .why {
    margin: 0;
    font-size: var(--text-xs-size);
    line-height: var(--text-xs-line);
    color: var(--text-tertiary);
  }
  @media (max-width: 860px) {
    .pane {
      grid-template-columns: 1fr;
    }
  }
</style>
