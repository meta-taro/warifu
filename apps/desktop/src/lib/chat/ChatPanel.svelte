<script lang="ts">
  // 会話。**連絡帳の面でも会議の面でも、同じ 1 本を出す。**
  //
  // **相手ごとの会話があるように見せない**（DESIGN §2 原則 7）。
  // 送った文字は会議に居る全員と、マイ PC エージェント に届く ——
  // 個別に届くと誤解した人は、見られたくないものを書く。
  //
  // **判断はここに置かない。**押せるかどうかは `$lib/meeting/stage` の
  // `届く先がある` が決める。ここは出すだけ。

  import { 送ってよい, 差出人の顔, type 会話行 } from '$lib/meeting/announce';
  import { エージェントの名を言い換える, 行の差出人名 } from './name';
  import type { 見出し } from './destination';
  import Avatar from '$lib/contacts/Avatar.svelte';
  import { 新しい行が来たとき, 底に着いた, type 位置 } from './scroll';
  import type { Locale } from '$lib/i18n/locales';
  import { MESSAGES, format, type MessageKey } from '$lib/i18n/messages';
  import Icon from '$lib/ui/Icon.svelte';
  import { tick, untrack } from 'svelte';
  import { 画像を拾う } from './image';
  import { 部屋を替える, type 下書きの控え as 下書きの控えの型 } from './draft';
  import { 吹き出しの形 } from './bubble';
  import Reach from './Reach.svelte';
  import { 呼べる名に広げる, 候補を入れる, 候補を出す, 宛ての名, 宛てられたか, 本文を分ける, type 打ちかけ } from './mention';

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
    /** この機械につながっている人数。 */
    この機械の人数: number;
    /** 送る。**中身は呼ぶ側が持つ**（下書きもここでは持たない）。 */
    送る: (body: string) => void;
    /**
     * **いま見ている部屋**（ルームの鍵、またはこの機械のルーム）。
     *
     * 部屋を替えたら打ちかけの文を部屋ごとに控える（`./draft`・2026-10-23）。
     * 控えないと、ASUS の部屋に書きかけた文が開発の部屋の欄にそのまま残る。
     */
    部屋?: string | null;
    /** 画像を送る。**渡されなければ、画像の口を出さない**（預かり所ごしなど、送れない所） */
    画像を送る?: (画像: File) => void;
    /**
     * **画像が届く相手がいまいるか**（ルームにほかの機械の人がいるか）。
     *
     * 居なくても**画像の口は消さない**（2026-10-20）。消すと、人には「送れない」としか見えなかった。
     * 薄く出して、なぜ押せないかを添える。
     */
    画像の届く先がある?: boolean;
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
     * （2026-09-07）。**言葉で説明するより、並べたほうが早い** ——
     * 1 人しか並ばなければ 1 対 1、3 人並べば 1 対 3 である。
     */
    届く先: readonly string[];
    /** 届く先の顔の種（`届く先` と同じ順）。**渡されれば、名前を顔つきで並べる** */
    届く先の顔?: readonly string[];
    /** 差し替えた顔（種 → 画像）。 */
    顔の画?: Readonly<Record<string, string>>;
    /**
     * **鍵 → いまの呼び名。**行に焼き付けた名前ではなく、**描くときに引き直す**。
     *
     * 2026-09-15、**同じ人が 1 通目は名前・2 通目は鍵**で出た（#19）——
     * **人からは 2 人居るように見える。**渡されなければ、行の名前をそのまま出す
     */
    画面での名簿?: Readonly<Record<string, string>>;
    /**
     * **打ち込み欄に文字があるかを、呼ぶ側へ伝える**（`lib/chat/keep.ts`）。
     *
     * 打ちかけているあいだ宛先を据え置くために要る ——
     * **封で打ちかけている最中に相手がルームへ入ると、送信で全員へ出てしまう。**
     */
    下書きが変わった?: (ある: boolean) => void;
    /** **宛先を据え置いている**なら、その旨を出す。 */
    据え置いている?: boolean;
    /** 据え置きを解いて、新しい宛先へ移る。 */
    新しい宛先へ移る?: () => void;
    /**
     * **いま打つと誰に届くのか**（`chat/destination.ts` の `宛先の見出し`）。
     *
     * いま打つとビデオ会議なのかグループチャットなのかが分かりにくい（2026-09-14）。
     *
     * **注意書きではなく、見出しにする**（**D107**）——
     * 「ここは選んだ相手だけの会話ではありません」は**打ち消し**であって、
     * **誰に届くかを言っていない。**
     *
     * 渡されなければ、これまでどおり「文字で話す」を出す。
     */
    宛先?: 見出し | null;
    /**
     * **この端末の人が `@` で呼ばれる名前**（0.1.35・メンション）。
     * 呼ばれた行に印を付ける。渡されなければ印は付かない
     */
    自分の呼ばれる名?: readonly string[];
    /** 会話の上に届く先を出すか。連絡帳はまんなかの欄に出すので出さない（0.1.36）。 */
    届く先を出す?: boolean;
  }

  const {
    locale,
    会話,
    届く先がある,
    打てない訳 = null,
    会議中,
    この機械の人数,
    送る,
    画像を送る,
    画像の届く先がある = true,
    相手ごとではない = false,
    届く先,
    届く先の顔 = [],
    顔の画 = {},
    画面での名簿 = {},
    下書きが変わった,
    部屋 = null,
    据え置いている = false,
    新しい宛先へ移る,
    宛先 = null,
    自分の呼ばれる名 = [],
    届く先を出す = true,
  }: Props = $props();

  const t = (key: MessageKey) => MESSAGES[locale][key];
  /** 顔の右上の印に重ねたときの説明。**人か、エージェントか**（2026-10-21）。 */
  const 主の名 = (主: '人' | 'エージェント') =>
    t(主 === 'エージェント' ? 'avatar.owner.agent' : 'avatar.owner.person');
  /** エージェントの名札を、この画面の言葉で出す（元の名札は変えない） */
  const 見せる名 = (名: string) =>
    エージェントの名を言い換える(名, (n) => format(t('chat.agentOf'), { name: n }));
  let 下書き = $state('');
  /** 部屋ごとの打ちかけの文（画面の中だけ。どこにも書かない）。 */
  let 下書きの控え: 下書きの控えの型 = {};
  let 見ていた部屋: string | null = untrack(() => 部屋);
  // **部屋を替えたら、欄の文を前の部屋に控え、新しい部屋の分を出す**（2026-10-23）
  $effect(() => {
    const 次 = 部屋;
    untrack(() => {
      if (次 === 見ていた部屋) return;
      const 替えた = 部屋を替える(下書きの控え, 見ていた部屋, 次, 下書き);
      下書きの控え = 替えた.控え;
      下書き = 替えた.下書き;
      見ていた部屋 = 次;
    });
  });
  /** 大きく見ている画像（`blob:` の URL）。 */
  let 大きく見る = $state<string | null>(null);
  let 画像の口: HTMLInputElement | undefined = $state();

  function 選んだ画像を送る(e: Event) {
    const 口 = e.currentTarget as HTMLInputElement;
    const f = 口.files?.[0];
    // **同じ画像をもう一度選べるように空ける**（空けないと change が来ない）
    口.value = '';
    if (f && 画像を送る) 画像を送る(f);
  }

  function 貼った(e: ClipboardEvent) {
    if (!画像を送る) return;
    const f = 画像を拾う(e.clipboardData?.items);
    // **文字の貼り付けは邪魔しない。**画像が入っていたときだけ取る
    if (!f) return;
    e.preventDefault();
    画像を送る(f);
  }

  /**
   * **見出し。**宛先を渡されていれば、それを名乗る（**D107**）。
   *
   * いま打つとビデオ会議なのかグループチャットなのかが分かりにくい（2026-09-14）。
   *
   * **「文字で話す」は、何をする所かを言っているが、誰に届くかを言っていない。**
   * 人が迷っていたのはそこである。
   */
  const 見出しの文 = $derived.by(() => {
    if (!宛先) return t('chat.title');
    const 文 = t(宛先.鍵 as MessageKey);
    return 宛先.値 ? format(文, 宛先.値) : 文;
  });

  // **下書きの有無を呼ぶ側へ伝える**（宛先を据え置くのに要る・`lib/chat/keep.ts`）
  $effect(() => {
    下書きが変わった?.(下書き.trim() !== '');
  });

  // **会話欄の送り方**（判断は `./scroll` に置いてある。ここは DOM を触るだけ）。
  let 欄: HTMLDivElement | undefined = $state();
  let 新着 = $state(0);
  let 前の行数 = 0;
  /**
   * **人が動かしたときの位置を覚える。**
   *
   * 行が増えた**あと**に測ってはいけない —— 増えた分だけ `scrollHeight` が伸びるので、
   * 底で見ていた人まで「底に居ない」と判定され、**追従が止まる。**
   * 初めは 0 のまま（`底にいるか` は「溢れていない＝底」と読む）。
   */
  let いまの位置: 位置 = { 上: 0, 見える高さ: 0, 全体: 0 };

  /** **動きを控える設定なら、跳ばす。**（`prefers-reduced-motion`・DESIGN §6） */
  function なめらかにできるか(): boolean {
    if (typeof window === 'undefined' || !window.matchMedia) return false;
    return !window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  }

  function 下へ送る(なめらか = true) {
    if (!欄) return;
    欄.scrollTo({
      top: 欄.scrollHeight,
      behavior: なめらか && なめらかにできるか() ? 'smooth' : 'auto'
    });
    新着 = 0;
  }

  function 動かした() {
    if (!欄) return;
    いまの位置 = { 上: 欄.scrollTop, 見える高さ: 欄.clientHeight, 全体: 欄.scrollHeight };
    const 次 = 底に着いた(いまの位置);
    if (次 !== null) 新着 = 次;
  }

  // **行が増えたら、送るか数えるかを決める。**
  // `untrack` が要る —— 新着 を読みながら書くと、書いた値でまた走って深さが尽きる
  // （2026-09-11 に未読で踏んだ。画面が丸ごと空になった）
  $effect(() => {
    const 数 = 会話.length;
    const 増えた = 数 - 前の行数;
    前の行数 = 数;
    const 自分の発言か = 会話[数 - 1]?.mine === true;
    const 見え = untrack(() => 新しい行が来たとき(いまの位置, 増えた, 新着, 自分の発言か));
    新着 = 見え.新着;
    if (見え.追う) 下へ送る(増えた > 0);
  });

  /**
   * **絵は、行が来たあとで高さを持つ。**底を追っていた人のために、開いたらもう一度送る。
   * 読み返している人（新着がある）は動かさない
   */
  function 絵が開いた() {
    if (新着 === 0) 下へ送る(false);
  }

  function 出す() {
    const body = 下書き.trim();
    if (!body) return;
    送る(body);
    下書き = '';
    呼びかけ = null;
  }

  // **`@名前` で呼ぶ**（0.1.35・メンション）。人数が多いと、誰に話しかけたか分からない。
  // **呼んでも届く先は変わらない**（ルームの全員に見える）。そのことを打つ欄の下で言う
  /** 呼べる名前。いま届く相手を、画面と同じ名前で */
  const 呼べる名 = $derived(届く先.map(見せる名));
  /** 印を付けて読む名前。届く相手のほか、名簿の名前と自分の名前も */
  const 見分ける名 = $derived(
    呼べる名に広げる([...呼べる名, ...Object.values(画面での名簿), ...自分の呼ばれる名])
  );
  const 呼んでいる名 = $derived(宛ての名(下書き, 呼べる名に広げる(呼べる名)));
  let 打つ欄: HTMLTextAreaElement | undefined = $state();
  let 呼びかけ = $state<打ちかけ | null>(null);
  let 選んでいる = $state(0);

  function 候補を見直す() {
    if (!打つ欄) return;
    // **欄の値を直に読む。**`bind:value` が下書きへ写すより先にここが走ることがある
    const 値 = 打つ欄.value;
    // **変換中は打つ位置の端が `selectionEnd` にある**（`selectionStart` は変換の頭のまま）
    呼びかけ = 候補を出す(値, 打つ欄.selectionEnd ?? 値.length, 呼べる名);
    if (選んでいる >= (呼びかけ?.候補.length ?? 0)) 選んでいる = 0;
  }

  function 候補を選ぶ(名: string) {
    if (!呼びかけ || !打つ欄) return;
    const 次 = 候補を入れる(打つ欄.value, 打つ欄.selectionStart ?? 打つ欄.value.length, 呼びかけ.頭, 名);
    下書き = 次.下書き;
    呼びかけ = null;
    const 欄 = 打つ欄;
    // **描き直してから焦点と位置を戻す**（先に置くと、描き直しで焦点が外れていた）
    void tick().then(() => {
      欄.focus();
      欄.setSelectionRange(次.位置, 次.位置);
    });
  }

  /** 候補が出ているときのキー。**扱ったら true**（送らない） */
  function 候補のキー(e: KeyboardEvent): boolean {
    // **日本語入力の確定の Enter で選ばない**（`送ってよい` と同じ見分け方・229 は古い WebView の保険）
    if (!呼びかけ || e.isComposing || e.keyCode === 229) return false;
    const 数 = 呼びかけ.候補.length;
    if (e.key === 'ArrowDown') 選んでいる = (選んでいる + 1) % 数;
    else if (e.key === 'ArrowUp') 選んでいる = (選んでいる + 数 - 1) % 数;
    else if ((e.key === 'Enter' && !e.shiftKey && !e.altKey) || e.key === 'Tab')
      候補を選ぶ(呼びかけ.候補[選んでいる]);
    else if (e.key === 'Escape') 呼びかけ = null;
    else return false;
    e.preventDefault();
    return true;
  }
</script>

<div class="card chat">
  <!-- **宛先を見出しにする**（**D107**）。
       「文字で話す」は**何をする所か**を言っているが、
       **誰に届くか**を言っていない —— 人が迷っていたのはそこである。 -->
  <h2><Icon name="chat" size={18} />{見出しの文}</h2>
  <!-- **注意書きは出さない**（見出しが宛先を名乗るので要らない）。
       **ただし宛先を渡されていない呼び出しでは、これまでどおり出す** ——
       見出しが「文字で話す」のままなら、打ち消しが無いと
       「その人との会話」に見える（2026-09-07 に実物で出た）。 -->
  {#if !宛先 && 相手ごとではない && !打てない訳}
    <p class="hint strong">{t('chat.shared')}</p>
  {/if}
  <!-- **誰に届くかを、そのまま並べる。**「全員に届きます」より読みやすい。
       連絡帳では、まんなかの欄に出すので、ここには出さない（0.1.36） -->
  {#if 届く先を出す}
    <Reach {locale} {届く先} {届く先の顔} {顔の画} />
  {/if}
  <div class="talkwrap">
    <div class="talk" bind:this={欄} onscroll={動かした}>
    {#if 会話.length === 0}
      <p class="hint">{t('chat.empty')}</p>
    {/if}
    {#each 会話 as line, i (i)}
      {@const 形 = 吹き出しの形(line)}
      {@const 宛て = !line.mine && !line.system && 宛てられたか(line.body, 自分の呼ばれる名)}
      {#if line.system}
        <!-- **会議からの知らせ**は吹き出しにしない。真ん中に小さく -->
        <p class="line system">
          {#if line.at}<span class="at">{line.at}</span>{/if}{line.body}
        </p>
      {:else}
        <!-- **この機械から出た発言は右、ほかの機械からは左**（0.1.36）。
             長い発言でも「どこからどこまでが 1 通か」「誰のか」が読めるように、
             名前と時刻を上に、本文を吹き出しに分ける -->
        <div
          class="line"
          class:right={形.側 === '右'}
          class:mine={line.mine}
          class:agent={形.主 === 'エージェント'}
          class:to-me={宛て}
        >
          <div class="head">
            {#if 差出人の顔(line)}<Avatar 種={差出人の顔(line) ?? ''} 大きさ={14} 名="" />{/if}
            <!-- 自分の行は右に寄っているので、名前を繰り返さない -->
            {#if !line.mine}<b>{見せる名(行の差出人名(line, 画面での名簿))}</b>{/if}
            <!-- **人か、エージェントか**を字で言う（色だけに頼らない） -->
            {#if 形.主 === 'エージェント'}<span class="kind">{t('avatar.owner.agent')}</span>{/if}
            {#if line.留守中}<span class="late">{t('chat.late')}</span>{/if}
            {#if line.at}<span class="at">{line.at}</span>{/if}
          </div>
          <div class="bubble">{#each 本文を分ける(line.body, 見分ける名) as 片, j (j)}{#if 片.呼び}<span
                class="mention"
                class:me={自分の呼ばれる名.includes(片.呼び)}>{片.文}</span
              >{:else}{片.文}{/if}{/each}{#if line.画像}<button
                type="button"
                class="pic"
                onclick={() => (大きく見る = line.画像 ?? null)}
                ><img
                  onload={絵が開いた}
                  src={line.画像}
                  alt={format(t('chat.image.alt'), { who: 見せる名(行の差出人名(line, 画面での名簿)) })}
                /></button
              >{/if}</div>
          {#if 宛て || line.届き}
            <div class="foot">
              {#if 宛て}<span class="mark to-me-mark">{t('chat.mention.me')}</span>{/if}
              {#if line.届き}<span
                  class="mark"
                  class:none={line.届き.札 === 'mark.none'}
                  title={t(line.届き.訳 as MessageKey)}>{t(line.届き.札 as MessageKey)}</span
                >{/if}
            </div>
          {/if}
        </div>
      {/if}
    {/each}
    </div>
    <!-- **新しい発言が来たことを、読み返している人にも知らせる。**
         下へ引っ張らない（読んでいた所を失う）。押したら底まで送る。
         **既読ではない**（D94）—— 数えているのは「まだ見ていない画面の下」である -->
    {#if 新着 > 0}
      <button
        type="button"
        class="newer"
        title={t('chat.new.title')}
        onclick={() => 下へ送る()}
      >
        {format(t('chat.new'), { n: String(新着) })}
        <Icon name="arrow-down" size={14} />
      </button>
    {/if}
  </div>
  <!--
    **相手が居ないときは押させない。**押せる形にしておいて「まだ誰も居ません」と
    返すのは、**押した人には「効かない」としか見えない**
    （2026-09-06 に実物で出た）。
    **打ち込みは残す** —— 先に書いておいて、入ってきたら送りたいことがある。
  -->
  <!-- **据え置いたことを必ず言う**（`lib/chat/keep.ts`）。
       黙って据え置くと、**打った物が思った所へ行かない**のと同じくらい悪い -->
  {#if 据え置いている}
    <p class="hint strong">
      {t('chat.keep')}
      {#if 新しい宛先へ移る}
        <button type="button" class="quiet" onclick={() => 新しい宛先へ移る?.()}>
          {t('chat.keep.switch')}
        </button>
      {/if}
    </p>
  {/if}
  {#if 打てない訳}
    <!-- **打てない訳は、その場のものを出す**（ルームの話にすり替えない） -->
    <p class="hint">{t(打てない訳 as MessageKey)}</p>
  {:else if !会議中 && この機械の人数 > 0}
    <!-- **会議に人は居ないが、同じエージェントのエージェント は居る。**話しかけられる。
         **この機械に誰もつながっていないのに出さない** —— 預かり所ごしに 1 人へ
         預けるだけのときにも出ていた（2026-09-08 に実物で踏んだ） -->
    <p class="hint">{t('chat.desk')}</p>
  {/if}
  {#if 呼んでいる名.length > 0}
    <!-- **呼んでも、届く先は変わらない。**「その人にだけ」と誤解させない -->
    <p class="hint mention-to">{format(t('chat.mention.to'), { who: 呼んでいる名.map((n) => `@${n}`).join(' ') })}</p>
  {/if}
  {#if 呼びかけ}
    <div class="mention-pick" role="listbox" aria-label={t('chat.mention.pick')} title={t('chat.mention.pick')}>
      {#each 呼びかけ.候補 as 名, i (名)}
        <button
          type="button"
          role="option"
          aria-selected={i === 選んでいる}
          class:picked={i === 選んでいる}
          onmousedown={(e) => {
            // **欄から焦点を奪わない**（奪うと打っている位置を失う）
            e.preventDefault();
            候補を選ぶ(名);
          }}>@{名}</button
        >
      {/each}
    </div>
  {/if}
  <div class="say">
    <!--
      **改行できる**（2026-09-06）。Shift+Enter / Option+Enter で改行、
      Enter で送る。`preventDefault` を忘れると、**送ったうえに改行が残る。**
      `input` ではなく `textarea` にしたのは、**改行を持てる欄が要る**ため。
    -->
    <textarea
      rows="1"
      bind:this={打つ欄}
      bind:value={下書き}
      oninput={候補を見直す}
      oncompositionupdate={() => queueMicrotask(候補を見直す)}
      oncompositionend={候補を見直す}
      onclick={候補を見直す}
      onblur={() => (呼びかけ = null)}
      placeholder={会議中
        ? t('chat.placeholder')
        : この機械の人数 > 0
          ? t('chat.placeholder.desk')
          : t('chat.placeholder.nobody')}
      onkeydown={(e) => {
        if (候補のキー(e)) return;
        if (送ってよい(e)) {
          e.preventDefault();
          出す();
        }
      }}
      onpaste={貼った}
    ></textarea>
    {#if 画像を送る}
      <!-- **画像の口。**貼り付け・ドラッグでも送れる。大きい画像は縮めてから送る -->
      <input
        bind:this={画像の口}
        type="file"
        accept="image/jpeg,image/png,image/webp,image/gif"
        hidden
        onchange={選んだ画像を送る}
      />
      <button
        type="button"
        class="quiet attach"
        title={画像の届く先がある ? t('chat.image.attach') : t('chat.image.nobody')}
        aria-label={画像の届く先がある ? t('chat.image.attach') : t('chat.image.nobody')}
        disabled={!画像の届く先がある}
        onclick={() => 画像の口?.click()}
      >
        <Icon name="image" size={18} />
      </button>
    {/if}
    <button type="button" onclick={出す} disabled={!届く先がある || !下書き.trim()}>
      {t('chat.send')}
    </button>
  </div>
</div>

{#if 大きく見る}
  <!-- **押した画像を大きく見る。**Esc か、どこかを押せば閉じる -->
  <div
    class="lightbox"
    role="dialog"
    aria-modal="true"
    tabindex="-1"
    onclick={() => (大きく見る = null)}
    onkeydown={(e) => e.key === 'Escape' && (大きく見る = null)}
  >
    <img src={大きく見る} alt="" />
    <button type="button" class="quiet close" onclick={() => (大きく見る = null)}>
      {t('chat.image.close')}
    </button>
  </div>
{/if}

<style>
  /* 会話の中の画像。**行の幅に収め、縦に長いものは切らずに縮める** */
  .pic {
    display: block;
    margin: var(--space-1) 0 0;
    padding: 0;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: none;
    cursor: zoom-in;
    max-width: 100%;
    overflow: hidden;
  }
  .pic img {
    display: block;
    max-width: 100%;
    max-height: 280px;
    object-fit: contain;
  }
  /* **送るより目立たせない。**主役は文字の欄と「送る」 */
  .attach {
    flex: none;
    align-self: stretch;
    padding: 0 var(--space-2);
    border: 1px solid var(--border);
    background: transparent;
    color: var(--text-secondary);
  }
  .attach:hover:not(:disabled) {
    color: var(--text-primary);
    border-color: var(--border-strong);
  }
  .lightbox {
    position: fixed;
    inset: 0;
    z-index: 50;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: var(--space-4);
    background: rgba(0, 0, 0, 0.8);
    cursor: zoom-out;
  }
  .lightbox img {
    max-width: 100%;
    max-height: 100%;
    object-fit: contain;
  }
  .lightbox .close {
    position: absolute;
    top: var(--space-3);
    right: var(--space-3);
    color: #fff;
  }
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
       （2026-09-07 に「まだ何もありません」が切れていた） */
    flex: 1;
    /*
      **底の打ち込み欄を、必ず画面の中に置く**（2026-09-24）。

      320px だと、上に札が 2 枚あるだけで**打ち込み欄が窓の下へ出る** ——
      空のときは薄い横線 1 本しか見えず、**それは打ち込み欄の上端**であって、欄が無かったのではない。

      **「まだ何もありません」が切れない高さ**（2026-09-07）は保ちつつ、
      **押し出さない**ところまで下げる。
    */
    min-height: 240px;
    /* **はみ出させない。**中身が高くても、次の札に重ねない（2026-09-24） */
    overflow: hidden;
  }
  h2 {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 0;
    font-size: var(--text-sm-size);
    font-weight: 600;
  }
  /* 届く先の顔。**目を引く**ように、字より大きい顔と枠で並べる */
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
  /* **札を会話欄の上に浮かせるための枠。**会話欄の高さを取らない */
  .talkwrap {
    position: relative;
    display: flex;
    flex: 1;
    min-height: 0;
  }
  .talk {
    display: flex;
    flex-direction: column;
    gap: 4px;
    /* **溢れたら中で動く。**外側（画面全体）を伸ばさない。
       空でも読める高さを持つ —— 0 だと潰れて 1 行すら切れる。
       **240px にした**（2026-09-12・会話欄が狭かった）——
       140px だと 5〜6 行で埋まり、読み返すたびに動かすことになる */
    flex: 1;
    /*
      **床は低くする。**広いときは `flex: 1` が伸ばすので、ここは「狭いときの下限」である。

      **240px にしていたので、札からはみ出していた**（2026-09-24 に実物で見た）——
      見出し・案内・打ち込み欄で 100px 要るのに、札の下限は 240px。
      **中身のほうが高くなり、次の札に重なって、打ち込み欄が読めなくなっていた。**

      2026-09-12 に狭かったのは**広いときの話**で、
      そこは `flex: 1` が受け持つ。**床を上げる話ではなかった。**
    */
    min-height: 120px;
    /* **なめらかに送る**（2026-09-12）。
       動きを控える設定の人には効かせない（DESIGN §6） */
    scroll-behavior: smooth;
    overflow-y: auto;
    padding: var(--space-2);
    background: var(--bg-app);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
  }
  /* **1 通＝1 つの吹き出し**（0.1.36）。上に名前と時刻、下に本文。
     ぶら下げ（字下げ）で揃えていた頃は、長い発言ほど区切りが読めなかった */
  .line {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 2px;
    margin: 0 0 var(--space-2);
    font-size: var(--text-sm-size);
    line-height: var(--text-sm-line);
  }
  /* **この機械から出た発言は右**（自分とこの機械のエージェント） */
  .line.right {
    align-items: flex-end;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 6px;
    max-width: 100%;
  }
  .bubble {
    max-width: min(80%, 42em);
    padding: 6px 10px;
    /* **打った改行を、そのまま見せる。**折り返しも効かせる */
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    word-break: break-word;
    color: var(--text-primary);
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
  }
  .line.mine .bubble {
    background: var(--accent-subtle);
    border-color: var(--accent-border);
  }
  /* **エージェントの吹き出しは点線で囲む。**人の発言と取り違えないように
     （字の札「エージェント」と二重に言う。色だけに頼らない） */
  .line.agent .bubble {
    background: var(--bg-subtle);
    border-style: dashed;
    border-color: var(--border-strong);
  }
  .kind {
    padding: 0 6px;
    font-size: var(--text-2xs-size);
    color: var(--neutral-fg);
    background: var(--neutral-bg);
    border-radius: var(--radius-full);
    white-space: nowrap;
  }
  .foot {
    display: flex;
    gap: 4px;
  }
  .foot .mark {
    margin-left: 0;
  }
  @media (prefers-reduced-motion: reduce) {
    .talk {
      scroll-behavior: auto;
    }
  }
  /* **新しい発言の札。**会話欄の底に浮かせる（行を押し下げない） */
  .newer {
    position: absolute;
    bottom: var(--space-2);
    left: 50%;
    transform: translateX(-50%);
    display: flex;
    align-items: center;
    gap: 4px;
    width: auto;
    padding: 4px 10px;
    font-size: var(--text-xs-size);
    border-radius: var(--radius-full);
    box-shadow: var(--shadow-sm, 0 1px 4px rgb(0 0 0 / 0.25));
  }
  /* **会議からの知らせ。**人の発言と見分けが付く形にする */
  /* **差出人の顔。**連絡帳と同じ絵を出す（同じ人が場所によって変わらない）。
     行の中に並ぶので、字の高さに合わせて少し下げる */
  .line :global(.face) {
    flex: none;
  }
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
  /* **`@名前` で呼んだ所**（0.1.35）。誰に話しかけたかが一目で分かるように */
  .mention {
    color: var(--info-fg);
    font-weight: 600;
  }
  /* 自分が呼ばれた所は、地も付けて目に入れる */
  .mention.me {
    padding: 0 2px;
    background: var(--info-bg);
    border-radius: var(--radius-sm);
  }
  /* **自分が呼ばれた発言。**吹き出しの左に帯を付ける */
  .line.to-me .bubble {
    box-shadow: inset 3px 0 0 var(--accent);
    background: var(--accent-subtle);
  }
  .mark.to-me-mark {
    color: var(--info-fg);
    background: var(--info-bg);
    cursor: default;
  }
  /* **呼ぶ相手の候補。**打つ欄のすぐ上に並べる */
  .mention-pick {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }
  /* **選んでいない候補は控えめに**（主要な操作に見せない・DESIGN.md §2） */
  .mention-pick button {
    width: auto;
    padding: 2px 8px;
    font-size: var(--text-xs-size);
    color: var(--text-secondary);
    background: transparent;
    border: 1px solid var(--border);
    border-radius: var(--radius-full);
  }
  .mention-pick button.picked {
    color: var(--info-fg);
    background: var(--info-bg);
    border-color: var(--info-fg);
  }

  /* 知らせの行は時刻と本文が横に並ぶので、間を空ける */
  .line.system .at {
    margin-right: 6px;
  }
  .line.system {
    display: block;
    color: var(--text-tertiary);
    font-style: italic;
    text-align: center;
  }
  /* **留守中に届いた分**（D71）。時刻は「出した側の時計」なので、
     いま届いたように見せない。**印を付けて、そう分かるようにする** */
  .late {
    padding: 0 4px;
    font-size: var(--text-2xs-size);
    color: var(--text-tertiary);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
  }
  /* 時刻は等幅で、桁を揃える（DESIGN.md §5）。**本文より前に出て、本文より弱い** */
  .at {
    font-family: var(--font-mono);
    font-size: var(--text-2xs-size);
    font-variant-numeric: tabular-nums;
    color: var(--text-tertiary);
  }
  .line b {
    font-weight: 500;
    color: var(--text-tertiary);
  }
  .line.mine b {
    color: var(--accent);
  }
  /* **エージェント。**人の発言と一目で見分けが付く必要がある */
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
