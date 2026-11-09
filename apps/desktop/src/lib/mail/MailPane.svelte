<script lang="ts">
  // **受信箱**（0.1.45・DESIGN §1・読むだけ）。決めてもらうための 1 枚目の案。
  //
  // - 札（仕分け）ごとにまとめ、人が見るべきものを上に（`order.ts`）
  // - 札には「なぜそうなったか」を添える。件名の語で決めた札は差出人の申告である
  // - 開いても既読にしない。本文は文字だけ。外の画像は読み込まず、数だけ言う
  // - 送る押し所は置かない（D49）

  import { onMount } from 'svelte';

  import { mailList, mailOpen, type 開いたメール } from '$lib/bridge';
  import type { Locale } from '$lib/i18n/locales';
  import { MESSAGES, format, type MessageKey } from '$lib/i18n/messages';
  import Icon from '$lib/ui/Icon.svelte';
  import { 並べる, 畳む札か, type メールの行 } from './order';

  interface Props {
    locale: Locale;
  }
  const { locale }: Props = $props();
  const t = (key: MessageKey) => MESSAGES[locale][key];

  let 行たち = $state<メールの行[]>([]);
  let 落ちた = $state<string[]>([]);
  let 控え無し = $state(false);
  let 読んでいる = $state(true);
  let 読めない = $state<string | null>(null);
  let 選んだ = $state<メールの行 | null>(null);
  let 中身 = $state<開いたメール | null>(null);
  let 開いている = $state(false);
  let 開けない = $state<string | null>(null);

  const 組たち = $derived(並べる(行たち));

  const 札の名: Record<string, MessageKey> = {
    なりすまし疑い: 'mail.tag.spoof',
    迷惑: 'mail.tag.spam',
    請求: 'mail.tag.billing',
    受注: 'mail.tag.order',
    問い合わせ: 'mail.tag.inquiry',
    知らせ: 'mail.tag.notice',
    広告: 'mail.tag.ad',
    未分類: 'mail.tag.unsorted'
  };
  const 札を言う = (札: string) => t(札の名[札] ?? 'mail.tag.unsorted');

  async function 読み直す() {
    読んでいる = true;
    読めない = null;
    try {
      const 答え = await mailList();
      行たち = 答え?.行たち ?? [];
      落ちた = 答え?.落ちた ?? [];
      控え無し = 答え?.控え無し ?? false;
    } catch (e) {
      読めない = String((e as { message?: string })?.message ?? e);
    } finally {
      読んでいる = false;
    }
  }

  async function 選ぶ(行: メールの行) {
    選んだ = 行;
    中身 = null;
    開けない = null;
    開いている = true;
    try {
      中身 = await mailOpen(行.user, 行.uid);
    } catch (e) {
      開けない = String((e as { message?: string })?.message ?? e);
    } finally {
      開いている = false;
    }
  }

  function 時刻(秒: number | null): string {
    if (秒 === null) return '';
    const d = new Date(秒 * 1000);
    const 今日 = new Date().toDateString() === d.toDateString();
    return 今日
      ? d.toLocaleTimeString(locale, { hour: '2-digit', minute: '2-digit' })
      : d.toLocaleDateString(locale, { month: 'numeric', day: 'numeric' });
  }

  onMount(() => {
    void 読み直す();
  });
</script>

<div class="mail">
  <section class="list card" aria-label={t('mail.title')}>
    <header>
      <h2><Icon name="mail" size={18} />{t('mail.title')}</h2>
      <button type="button" class="quiet" onclick={() => void 読み直す()} disabled={読んでいる}>{t('mail.reload')}</button>
    </header>
    <p class="hint">{t('mail.readonly')}</p>

    {#if 読んでいる}
      <p class="hint">{t('mail.loading')}</p>
    {:else if 読めない}
      <p class="warn" role="alert">{読めない}</p>
    {:else if 控え無し}
      <p class="hint">{t('mail.none')}</p>
    {:else}
      {#if 落ちた.length > 0}
        <p class="warn" role="alert">{format(t('mail.failed'), { list: 落ちた.join('／') })}</p>
      {/if}
      {#if 組たち.length === 0}
        <p class="hint">{t('mail.empty')}</p>
      {/if}
      {#each 組たち as 組 (組.札)}
        <details class="group" class:spoof={組.札 === 'なりすまし疑い'} open={!畳む札か(組.札)}>
          <summary><span class="tag" data-tag={組.札}>{札を言う(組.札)}</span><span class="count">{組.行たち.length}</span></summary>
          <ul>
            {#each 組.行たち as 行 (行.user + 行.uid)}
              <li>
                <button
                  type="button"
                  class="row"
                  class:on={選んだ?.uid === 行.uid && 選んだ?.user === 行.user}
                  class:unread={!行.既読}
                  onclick={() => void 選ぶ(行)}
                >
                  <span class="who" title={行.差出人}>{行.名 ?? 行.差出人}</span>
                  <span class="at">{時刻(行.届いた)}</span>
                  <span class="subject" title={行.件名}>{行.件名 || t('mail.noSubject')}</span>
                </button>
              </li>
            {/each}
          </ul>
        </details>
      {/each}
    {/if}
  </section>

  <section class="read card" aria-live="polite">
    {#if !選んだ}
      <p class="hint">{t('mail.pick')}</p>
    {:else}
      <h3 class="subject-big">{選んだ.件名 || t('mail.noSubject')}</h3>
      <p class="from">{選んだ.名 ? `${選んだ.名} <${選んだ.差出人}>` : 選んだ.差出人}<span class="at">{時刻(選んだ.届いた)}</span></p>
      <div class="why" class:spoof={選んだ.札 === 'なりすまし疑い'}>
        <span class="tag" data-tag={選んだ.札}>{札を言う(選んだ.札)}</span>
        {#if 選んだ.理由.length > 0}
          <ul>
            {#each 選んだ.理由 as r (r)}<li>{r}</li>{/each}
          </ul>
        {:else}
          <span class="hint">{t('mail.noRule')}</span>
        {/if}
      </div>
      {#if 開いている}
        <p class="hint">{t('mail.opening')}</p>
      {:else if 開けない}
        <p class="warn" role="alert">{開けない}</p>
      {:else if 中身}
        {#if 中身.外の画像 > 0}<p class="note">{format(t('mail.images'), { n: String(中身.外の画像) })}</p>{/if}
        {#if 中身.添付.length > 0}<p class="note">{format(t('mail.attachments'), { names: 中身.添付.join('・') })}</p>{/if}
        <pre class="body">{中身.本文}</pre>
      {/if}
    {/if}
  </section>
</div>

<style>
  .mail {
    display: flex;
    flex: 1;
    min-width: 0;
    min-height: 0;
    gap: var(--space-4);
  }
  .card {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    min-height: 0;
    overflow-y: auto;
    padding: var(--space-4);
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
  }
  .list {
    width: 380px;
    flex: none;
  }
  .read {
    flex: 1;
    min-width: 0;
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
  }
  h2 {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 0;
    font-size: var(--text-md-size);
  }
  .hint {
    margin: 0;
    font-size: var(--text-xs-size);
    color: var(--text-secondary);
  }
  .warn {
    margin: 0;
    padding: 6px 8px;
    font-size: var(--text-xs-size);
    color: var(--warning-fg);
    background: var(--warning-bg);
    border-radius: var(--radius-md);
  }
  .group {
    border-top: 1px solid var(--border);
    padding-top: var(--space-2);
  }
  summary {
    display: flex;
    align-items: center;
    gap: 6px;
    cursor: pointer;
    list-style-position: inside;
  }
  .count {
    font-size: var(--text-xs-size);
    color: var(--text-secondary);
  }
  .tag {
    padding: 0 8px;
    font-size: var(--text-xs-size);
    color: var(--neutral-fg);
    background: var(--neutral-bg);
    border-radius: var(--radius-full);
    white-space: nowrap;
  }
  .tag[data-tag='なりすまし疑い'] {
    color: var(--danger-fg);
    background: var(--danger-bg);
    font-weight: 600;
  }
  .tag[data-tag='受注'],
  .tag[data-tag='問い合わせ'] {
    color: var(--info-fg);
    background: var(--info-bg);
  }
  .tag[data-tag='請求'] {
    color: var(--warning-fg);
    background: var(--warning-bg);
  }
  ul {
    margin: 4px 0 0;
    padding: 0;
    list-style: none;
  }
  .row {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 0 var(--space-2);
    width: 100%;
    padding: 6px 8px;
    text-align: left;
    color: var(--text-primary);
    background: transparent;
    border: 0;
    border-radius: var(--radius-md);
    cursor: pointer;
  }
  .row:hover {
    background: var(--bg-hover);
  }
  .row.on {
    background: var(--accent-subtle);
  }
  .row.unread .who {
    font-weight: 600;
  }
  .who,
  .subject {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .subject {
    grid-column: 1 / -1;
    font-size: var(--text-xs-size);
    color: var(--text-secondary);
  }
  .at {
    margin-left: var(--space-2);
    font-size: var(--text-xs-size);
    color: var(--text-secondary);
  }
  .subject-big {
    margin: 0;
    font-size: var(--text-md-size);
  }
  .from {
    margin: 0;
    font-size: var(--text-xs-size);
    color: var(--text-secondary);
  }
  .why {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-start;
    gap: 6px;
    padding: 6px 8px;
    font-size: var(--text-xs-size);
    background: var(--bg-subtle);
    border-radius: var(--radius-md);
  }
  .why.spoof {
    background: var(--danger-bg);
  }
  .why ul {
    margin: 0;
    padding-left: 1em;
    list-style: disc;
  }
  .note {
    margin: 0;
    font-size: var(--text-xs-size);
    color: var(--text-secondary);
  }
  .body {
    margin: 0;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    font-family: inherit;
    font-size: var(--text-sm-size);
    line-height: 1.7;
    color: var(--text-primary);
  }
</style>
