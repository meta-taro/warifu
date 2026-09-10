<script lang="ts">
  // 会話。**連絡帳の面でも会議の面でも、同じ 1 本を出す。**
  //
  // **相手ごとの会話があるように見せない**（DESIGN §2 原則 7）。
  // 送った文字は会議に居る全員と、マイ PC エージェント に届く ——
  // 個別に届くと誤解した人は、見られたくないものを書く。
  //
  // **判断はここに置かない。**押せるかどうかは `$lib/meeting/stage` の
  // `届く先がある` が決める。ここは出すだけ。

  import { 送ってよい, type 会話行 } from '$lib/meeting/announce';
  import type { Locale } from '$lib/i18n/locales';
  import { MESSAGES, format, type MessageKey } from '$lib/i18n/messages';
  import Icon from '$lib/ui/Icon.svelte';

  interface Props {
    locale: Locale;
    会話: readonly 会話行[];
    /** 打ったものに届く先があるか。 */
    届く先がある: boolean;
    /**
     * いま打てない訳（文言の鍵）。打てるなら `null`。
     *
     * **ルームの話を、エージェントの話にすり替えない**（`chat/delivery.ts`）——
     * 繋がっていないエージェントを選んでいるのに「鍵を渡して、入ってもらうと
     * 送れます」と出ていた（2026-09-10 に実物で出た）。
     */
    打てない訳?: string | null;
    /** 会議に人が居るか（案内の文言が変わる）。 */
    会議中: boolean;
    /** このPCにつながっている人数。 */
    このPCの人数: number;
    /** 送る。**中身は呼ぶ側が持つ**（下書きもここでは持たない）。 */
    送る: (body: string) => void;
    /**
     * 選んだ相手のすぐ隣に並ぶか。
     *
     * **並ぶなら「その人との会話ではない」と先に言う。**
     * 言わないと、個別に届くと誤解した人が見られたくないものを書く。
     */
    相手ごとではない?: boolean;
    /**
     * **いま届く先。**呼び名をそのまま並べる。
     *
     * 「1 対 1 なのか 1 対 N なのか」は最初に人が気にする所である
     * （オーナー・2026-09-07）。**言葉で説明するより、並べたほうが早い** ——
     * 1 人しか並ばなければ 1 対 1、3 人並べば 1 対 3 である。
     */
    届く先: readonly string[];
  }

  const {
    locale,
    会話,
    届く先がある,
    打てない訳 = null,
    会議中,
    このPCの人数,
    送る,
    相手ごとではない = false,
    届く先,
  }: Props = $props();

  const t = (key: MessageKey) => MESSAGES[locale][key];
  let 下書き = $state('');

  function 出す() {
    const body = 下書き.trim();
    if (!body) return;
    送る(body);
    下書き = '';
  }
</script>

<div class="card chat">
  <h2><Icon name="chat" size={18} />{t('chat.title')}</h2>
  <!-- **1 本の会話であることを、先に言う。**個別に届くと思わせない。
       連絡帳では選んだ相手のすぐ隣に並ぶので、**放っておくと
       「その人との会話」に見える**（2026-09-07 にオーナーが実物で踏んだ）。 -->
  <!--
    **送れないときは出さない**（2026-09-10 に実物で出た）——
    打っても届かない状態で「みんなの会話です」と言っても、読む人には関係がない。
    出すのは**いま打てて、しかもルームへ流れるとき**だけ
  -->
  {#if 相手ごとではない && !打てない訳}
    <p class="hint strong">{t('chat.shared')}</p>
  {/if}
  <!-- **誰に届くかを、そのまま並べる。**「全員に届きます」より読みやすい -->
  {#if 届く先.length === 0}
    <p class="hint">{t('chat.reach.none')}</p>
  {:else}
    <p class="hint reach">{format(t('chat.reach'), { who: 届く先.join(' ／ ') })}</p>
  {/if}
  <div class="talk">
    {#if 会話.length === 0}
      <p class="hint">{t('chat.empty')}</p>
    {/if}
    {#each 会話 as line, i (i)}
      <!-- **いつの発言かを出す。**無いと、あとから読み返せない -->
      <p class="line" class:mine={line.mine} class:system={line.system} class:agent={line.agent}>
        {#if line.at}<span class="at">{line.at}</span>{/if}{#if line.留守中}<span class="late"
            >{t('chat.late')}</span
          >{/if}{#if !line.system}<b>{line.who}</b
          >{/if}{line.body}{#if line.届き}<span
            class="mark"
            class:none={line.届き.札 === 'mark.none'}
            title={t(line.届き.訳 as MessageKey)}>{t(line.届き.札 as MessageKey)}</span
          >{/if}
      </p>
    {/each}
  </div>
  <!--
    **相手が居ないときは押させない。**押せる形にしておいて「まだ誰も居ません」と
    返すのは、**押した人には「効かない」としか見えない**
    （2026-09-06 にオーナーから「チャット送るボタンきかないよ」と報告された）。
    **打ち込みは残す** —— 先に書いておいて、入ってきたら送りたいことがある。
  -->
  {#if 打てない訳}
    <!-- **打てない訳は、その場のものを出す**（ルームの話にすり替えない） -->
    <p class="hint">{t(打てない訳 as MessageKey)}</p>
  {:else if !会議中 && このPCの人数 > 0}
    <!-- **会議に人は居ないが、同じエージェントのエージェント は居る。**話しかけられる。
         **このPCに誰もつながっていないのに出さない** —— 預かり所ごしに 1 人へ
         預けるだけのときにも出ていた（2026-09-08 に実物で踏んだ） -->
    <p class="hint">{t('chat.desk')}</p>
  {/if}
  <div class="say">
    <!--
      **改行できる**（2026-09-06 のオーナー要望）。Shift+Enter / Option+Enter で改行、
      Enter で送る。`preventDefault` を忘れると、**送ったうえに改行が残る。**
      `input` ではなく `textarea` にしたのは、**改行を持てる欄が要る**ため。
    -->
    <textarea
      rows="1"
      bind:value={下書き}
      placeholder={会議中
        ? t('chat.placeholder')
        : このPCの人数 > 0
          ? t('chat.placeholder.desk')
          : t('chat.placeholder.nobody')}
      onkeydown={(e) => {
        if (送ってよい(e)) {
          e.preventDefault();
          出す();
        }
      }}
    ></textarea>
    <button type="button" onclick={出す} disabled={!届く先がある || !下書き.trim()}>
      {t('chat.send')}
    </button>
  </div>
</div>

<style>
  .card {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--bg-subtle);
  }
  .card.chat {
    /* **残りを取る。**打ち込み欄は底に固定され、行が増えても動かない。
       見出し・案内・打ち込み欄で 160px はほぼ埋まるので、
       会話欄の min-height と足し合わせた高さにする
       （2026-09-07・オーナーの画面で「まだ何もありません」が切れていた） */
    flex: 1;
    min-height: 320px;
  }
  h2 {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 0;
    font-size: var(--text-sm-size);
    font-weight: 600;
  }
  .hint.reach {
    color: var(--text-secondary);
  }
  .hint.strong {
    color: var(--text-secondary);
    font-weight: 600;
  }
  .hint {
    margin: 0;
    font-size: var(--text-xs-size);
    line-height: var(--text-xs-line);
    color: var(--text-tertiary);
  }
  .talk {
    display: flex;
    flex-direction: column;
    gap: 4px;
    /* **溢れたら中で動く。**外側（画面全体）を伸ばさない。
       空でも読める高さを持つ —— 0 だと潰れて 1 行すら切れる */
    flex: 1;
    min-height: 140px;
    overflow-y: auto;
    padding: var(--space-2);
    background: var(--bg-app);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
  }
  .line {
    margin: 0;
    /* **打った改行を、そのまま見せる。**折り返しも効かせる */
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    font-size: var(--text-sm-size);
    line-height: var(--text-sm-line);
    word-break: break-word;
  }
  /* **会議からの知らせ。**人の発言と見分けが付く形にする */
  /* **どこまで行ったかの札。**既読ではない（読んだかは分からない） */
  .mark {
    margin-left: 6px;
    padding: 0 6px;
    font-size: var(--text-2xs-size);
    color: var(--text-tertiary);
    background: var(--bg-sunken);
    border-radius: var(--radius-full);
    white-space: nowrap;
    cursor: help;
  }
  .mark.none {
    color: var(--warning-fg);
    background: var(--warning-bg);
  }

  .line.system {
    color: var(--text-tertiary);
    font-style: italic;
    text-align: center;
  }
  /* **留守中に届いた分**（D71）。時刻は「出した側の時計」なので、
     いま届いたように見せない。**印を付けて、そう分かるようにする** */
  .late {
    margin-right: 6px;
    padding: 0 4px;
    font-size: var(--text-2xs-size);
    color: var(--text-tertiary);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
  }
  /* 時刻は等幅で、桁を揃える（DESIGN.md §5）。**本文より前に出て、本文より弱い** */
  .at {
    margin-right: 6px;
    font-family: var(--font-mono);
    font-size: var(--text-2xs-size);
    font-variant-numeric: tabular-nums;
    color: var(--text-tertiary);
  }
  .line b {
    margin-right: 6px;
    font-weight: 500;
    color: var(--text-tertiary);
  }
  .line.mine b {
    color: var(--accent);
  }
  /* **同じエージェントのエージェント。**人の発言と一目で見分けが付く必要がある */
  .line.agent b {
    color: var(--text-secondary);
    font-weight: 600;
  }
  .say {
    display: flex;
    gap: var(--space-2);
  }
  /* **形は `forms.css`。**ここは幅と伸び方だけ（打ち込みは伸ばさない） */
  .say textarea {
    flex: 1;
    min-width: 0;
    min-height: 34px;
    max-height: 120px;
    resize: none;
  }
  button {
    padding: 7px 14px;
    font: inherit;
    font-size: var(--text-sm-size);
    color: var(--on-accent);
    background: var(--accent);
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    cursor: pointer;
  }
  /* **押せないものは、押せないように見せる** */
  button:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
</style>
