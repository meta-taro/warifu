<script lang="ts">
  // **受信箱**（0.1.45・DESIGN §1・読むだけ）。決めてもらうための 1 枚目の案。
  //
  // - 札（仕分け）ごとにまとめ、人が見るべきものを上に（`order.ts`）
  // - 札には「なぜそうなったか」を添える。件名の語で決めた札は差出人の申告である
  // - 開いても既読にしない。本文は文字だけ。外の画像は読み込まず、数だけ言う
  // - 送る押し所は置かない（D49）

  import { onMount } from 'svelte';

  import { mailAccounts, mailList, mailOpen, mailSecretSet, type 控えた口, type 開いたメール } from '$lib/bridge';
  import { メールの映り } from './seen.svelte';
  import type { Locale } from '$lib/i18n/locales';
  import { MESSAGES, format, type MessageKey } from '$lib/i18n/messages';
  import Icon from '$lib/ui/Icon.svelte';
  import { 本文を見やすく } from './body';
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

  // **控えた口**（0.1.45）。秘密は入っているかだけを見る。入れるのは人がここで打つ（エージェントは入れられない）
  let 口たち = $state<控えた口[]>([]);
  let 入れている = $state<string | null>(null);
  let 打った = $state('');
  let 置いている = $state(false);
  let 置けない = $state<string | null>(null);
  let 置いた = $state<string | null>(null);

  async function 口を読み直す() {
    try {
      口たち = (await mailAccounts()) ?? [];
    } catch (e) {
      置けない = String((e as { message?: string })?.message ?? e);
    }
  }

  function 入れ始める(user: string) {
    入れている = user;
    打った = '';
    置けない = null;
    置いた = null;
  }

  async function 秘密を置く(user: string) {
    置いている = true;
    置けない = null;
    try {
      await mailSecretSet(user, 打った);
      置いた = user;
      入れている = null;
      await 口を読み直す();
      // **入れたら受信箱も読み直す**（11-10）。入れる前の「読めなかった」が残り、1 通も出ないままだった
      void 読み直す();
    } catch (e) {
      置けない = String((e as { message?: string })?.message ?? e);
    } finally {
      // **打った字は残さない**（置けても置けなくても消す）
      打った = '';
      置いている = false;
    }
  }

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
    void 口を読み直す();
    return () => {
      if (メールの映り.いま) メールの映り.いま = { ...メールの映り.いま, 開いている: false };
    };
  });

  // **欄の様子を写しへ渡す**（11-10・`app_view`）。本文・差出人・件名は渡さない
  $effect(() => {
    const 文言: string[] = [];
    if (置けない) 文言.push(置けない);
    if (読めない) 文言.push(読めない);
    if (落ちた.length > 0) 文言.push(format(t('mail.failed'), { list: 落ちた.join('／') }));
    if (控え無し) 文言.push(t('mail.none'));
    メールの映り.いま = {
      開いている: true,
      口たち: 口たち.map((口) => ({ user: 口.user, kind: 口.kind, 秘密がある: 口.秘密がある })),
      件数: 読んでいる || 読めない ? null : 行たち.length,
      文言,
    };
  });
</script>

<div class="mail">
  <section class="list card" aria-label={t('mail.title')}>
    <header>
      <h2><Icon name="mail" size={18} />{t('mail.title')}</h2>
      <!-- **［読み直す］は控えた口も読み直す**（11-10）。開いたときに 1 回しか読まず、
           あとからエージェントが足した口が「控えた口 0」のまま［秘密を入れる］が出なかった -->
      <button
        type="button"
        class="quiet"
        onclick={() => {
          void 読み直す();
          void 口を読み直す();
        }}
        disabled={読んでいる}>{t('mail.reload')}</button
      >
    </header>
    <p class="hint">{t('mail.readonly')}</p>

    <details class="accounts" open={口たち.some((口) => !口.秘密がある)}>
      <summary>{t('mail.accounts.title')}<span class="count">{口たち.length}</span></summary>
      <!-- **一覧が読めなかったことを、欄の外にも出す**（11-10）。入力欄の中にしか出さず、読めないまま「0」に見えた -->
      {#if 置けない && 入れている === null}<p class="warn" role="alert">{置けない}</p>{/if}
      {#if 口たち.length === 0}
        <p class="hint">{t('mail.accounts.none')}</p>
      {/if}
      <ul>
        {#each 口たち as 口 (口.user)}
          <li class="account">
            <span class="who" title={口.user}>{口.user}</span>
            <span class="kind">{t(`mail.kind.${口.kind}`)}</span>
            <span class="secret" class:missing={!口.秘密がある}
              >{t(口.秘密がある ? 'mail.secret.ok' : 'mail.secret.missing')}</span
            >
            {#if 置いた === 口.user}<span class="saved">{t('mail.secret.saved')}</span>{/if}
            {#if 口.送る && !口.読む}<span class="hint">{t('mail.send.later')}</span>{/if}
            {#if 入れている === 口.user}
              <form class="secret-form" onsubmit={(e) => { e.preventDefault(); void 秘密を置く(口.user); }}>
                <p class="hint">{t(口.kind === 'resend' ? 'mail.secret.hint.apikey' : 'mail.secret.hint.password')}</p>
                <input
                  type="password"
                  autocomplete="off"
                  spellcheck="false"
                  aria-label={t('mail.secret.enter')}
                  bind:value={打った}
                />
                <div class="buttons">
                  <button type="submit" disabled={置いている || 打った.trim() === ''}>{t('mail.secret.save')}</button>
                  <button type="button" class="quiet" onclick={() => { 入れている = null; 打った = ''; }}>{t('mail.secret.cancel')}</button>
                </div>
                {#if 置けない}<p class="warn" role="alert">{置けない}</p>{/if}
              </form>
            {:else}
              <button type="button" class="quiet small" onclick={() => 入れ始める(口.user)}
                >{t(口.秘密がある ? 'mail.secret.redo' : 'mail.secret.enter')}</button
              >
            {/if}
          </li>
        {/each}
      </ul>
    </details>

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
        <pre class="body">{本文を見やすく(中身.本文)}</pre>
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
    overflow-x: hidden;
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
  .accounts {
    padding: 6px 8px;
    background: var(--bg-subtle);
    border-radius: var(--radius-md);
  }
  .account {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 4px 8px;
    padding: 6px 0;
    border-top: 1px solid var(--border);
    font-size: var(--text-xs-size);
  }
  .account:first-child {
    border-top: 0;
  }
  .account .who {
    flex: 1 1 100%;
    font-size: var(--text-sm-size);
    color: var(--text-primary);
  }
  .kind {
    color: var(--text-secondary);
  }
  .secret {
    padding: 0 6px;
    color: var(--success-fg);
    background: var(--success-bg);
    border-radius: var(--radius-full);
  }
  .secret.missing {
    color: var(--warning-fg);
    background: var(--warning-bg);
  }
  .saved {
    color: var(--success-fg);
  }
  .small {
    font-size: var(--text-xs-size);
  }
  .secret-form {
    display: flex;
    flex: 1 1 100%;
    flex-direction: column;
    gap: 6px;
  }
  .buttons {
    display: flex;
    gap: 6px;
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
