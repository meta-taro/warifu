<script lang="ts">
  // **札の頼みの窓**（0.1.57）。前は画面の上の細い帯で、人が気づかずに席が止まった。
  //
  // - 頼みが来たら画面の真ん中に窓を出す（本体は割符の窓も前に出す）
  // - Enter で既定の許し（取り消すまで出せる動作はそれ、ほかは 24 時間）。ほかに［今回だけ］［断る］
  // - 離れていても Claude Code の確認で通すかを、ここで切り替えられる（［メール］の奥から移した）
  // - どのボタンもエージェントのカーソルでは押せない（`data-agent-hands-off`）
  import type { 待っている札 } from '$lib/bridge';
  import type { Locale } from '$lib/i18n/locales';
  import { MESSAGES, format, type MessageKey } from '$lib/i18n/messages';
  import { 既定の許しを選ぶ, 窓に出す頼み } from './dialog';

  interface Props {
    locale: Locale;
    待ちたち: readonly 待っている札[];
    取り消すまで出せる札: readonly string[];
    名を言う: (頼んだ人: string) => string;
    動作を言う: (動作: string) => string;
    離れていても通す: boolean;
    今回だけ: (動作: string) => void;
    一日通す: (動作: string) => void;
    取り消すまで通す: (動作: string) => void;
    断る: (動作: string) => void;
    離れていても通すを決める: (on: boolean) => void;
  }
  const p: Props = $props();
  const t = (key: MessageKey) => MESSAGES[p.locale][key];

  const 窓 = $derived(窓に出す頼み(p.待ちたち));
  const 既定 = $derived(窓.いま ? 既定の許しを選ぶ(窓.いま.動作, p.取り消すまで出せる札) : '24時間');

  function 既定で許す() {
    if (!窓.いま) return;
    if (既定 === '取り消すまで') p.取り消すまで通す(窓.いま.動作);
    else p.一日通す(窓.いま.動作);
  }

  let 既定のボタン = $state<HTMLButtonElement | null>(null);
  // **窓が出たら既定のボタンに手を置く**（Enter で押せる）
  $effect(() => {
    if (窓.いま && 既定のボタン) 既定のボタン.focus();
  });
</script>

{#if 窓.いま}
  {@const 待ち = 窓.いま}
  <div class="幕" role="presentation">
    <div
      class="箱 pass-dialog"
      role="alertdialog"
      aria-modal="true"
      aria-labelledby="pass-dialog-title"
      data-agent-hands-off
    >
      <p class="what" id="pass-dialog-title">{t('pass.dialog.title')}</p>
      <p class="ask">
        <strong>{format(t('pass.who'), { who: p.名を言う(待ち.頼んだ人), what: p.動作を言う(待ち.動作) })}</strong>
      </p>
      <!-- **なぜ要るかを必ず出す。**出せない頼みは、ここへ来ない（本体が受け付けていない） -->
      <p class="why"><span class="label">{t('pass.dialog.why')}</span>{待ち.訳}</p>

      <label class="away">
        <input
          type="checkbox"
          checked={p.離れていても通す}
          onchange={(e) => p.離れていても通すを決める(e.currentTarget.checked)}
        />
        <span>{t('away.approve.label')}</span>
      </label>
      {#if p.離れていても通す}<p class="hint">{t('away.approve.hint')}</p>{/if}

      <div class="tail">
        <button type="button" class="quiet" onclick={() => p.断る(待ち.動作)}>{t('pass.dialog.refuse')}</button>
        <button type="button" class="quiet" onclick={() => p.今回だけ(待ち.動作)}>{t('pass.dialog.once')}</button>
        <button type="button" bind:this={既定のボタン} onclick={既定で許す}>
          {t(既定 === '取り消すまで' ? 'pass.dialog.allowUntilRevoked' : 'pass.dialog.allowDay')}
        </button>
      </div>
      {#if 窓.ほか > 0}
        <p class="hint more">{format(t('pass.dialog.more'), { n: String(窓.ほか) })}</p>
      {/if}
    </div>
  </div>
{/if}

<style>
  .pass-dialog {
    width: 440px;
  }
  .ask {
    margin: 0;
    color: var(--text-primary);
  }
  .why {
    margin: 0;
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-md);
    background: var(--bg-subtle);
    color: var(--text-primary);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .why .label {
    margin-right: var(--space-2);
    color: var(--text-secondary);
    font-size: var(--text-sm-size);
  }
  .away {
    display: flex;
    align-items: flex-start;
    gap: var(--space-2);
    margin-top: var(--space-2);
    font-size: var(--text-sm-size);
    color: var(--text-primary);
  }
  .away input {
    margin-top: 3px;
  }
  .hint {
    margin: 0;
    font-size: var(--text-sm-size);
    color: var(--text-secondary);
  }
  .more {
    text-align: right;
  }
</style>
