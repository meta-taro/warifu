<script lang="ts">
  // 連絡帳の面。**割符の入口。**
  //
  // 「連絡帳があって、その人と、会議するのか、チャットするのか、
  // そしてメールの場合どうするのか、というダッシュボードが先に来るかと」
  // （オーナー・2026-09-07）
  //
  // **判断はここに置かない。**並びは `list.ts`、押せるかは `actions.ts` が決める。

  import type { Locale } from '$lib/i18n/locales';
  import { MESSAGES, format, type MessageKey } from '$lib/i18n/messages';
  import { 鍵の頭 } from '$lib/meeting/names';
  import Icon, { type IconName } from '$lib/ui/Icon.svelte';
  import Avatar from './Avatar.svelte';
  import { 名乗りを添えるか, 呼ぶ名 } from './claimed';
  import type { ProfileRow } from '$lib/bridge';
  import { できること, type 口の種類, type 状態 } from './actions';
  import { いまの様子, できることの案内, はじめの一歩を出すか } from './home';
  import { 在席の印 } from './presence';
  import { macの実体, 口を足す, 起こす } from './connect';
  import {
    机の印,
    部屋か,
    部屋のid,
    連絡帳を組む,
    type 行,
    type 素材 as 連絡帳の素材,
  } from './list';

  interface Props {
    locale: Locale;
    素材: 連絡帳の素材;
    /** いま選んでいる相手（公開鍵か、机の印）。 */
    選んでいる: string | null;
    /**
     * 相手を選ぶ。**同じ行をもう一度押したら外す**（`null`）。
     *
     * オーナー指摘（2026-09-10）——
     * 「**だれかを選んでしまうと、いまは二度とその画面に戻れません**」。
     * 外す道が無いのは、**入ったら出られない部屋**と同じである。
     */
    選ぶ: (key: string | null) => void;
    /** 口を押した。 */
    押す: (種類: 口の種類, 相手: 行) => void;
    /** 呼んでいる最中の相手。**二度押しを止める。** */
    呼んでいる: string | null;
    /** 呼び名を付ける。**空にすると忘れる**（名簿と同じ約束）。 */
    名前を付ける: (key: string, label: string) => void;
    /** 相手を戸口から降ろす。 */
    降ろす: (key: string) => void;
    /** 同じ PC の AI に「止まれ」と言う。 */
    止める: (呼び方: string) => void;
    /**
     * **いま会議キーなしで入れる相手**の公開鍵。
     *
     * 覚えている相手とは**別の集まり**である（呼び名を付けただけでは入れない）。
     * ここに居る相手にだけ「やめる」を出す —— **出しても効かない口を出さない**（D49）。
     */
    鍵なしで入れる: readonly string[];
    /**
     * 預かり所を置いているか（**D71**）。
     *
     * 置いていれば、**相手が起動していなくても**文字は届く。
     */
    預かり所がある?: boolean;
    /**
     * この端末のプロフィール（**人と、マイ PC エージェント**）。
     *
     * **書き換えられるのはこの端末の持ち主だけ。**AI 自身の口には無い ——
     * 自分の名前を書き換えられると、**同じ机の別のエージェントに化けられる。**
     */
    プロフィール?: readonly ProfileRow[];
    /** プロフィールを書く。**名前も紹介も空にすると消える。** */
    名乗りを書く?: (who: string, name: string, bio: string) => void;
    /**
     * **相手が名乗ったもの**（**D75**）。公開鍵 → 名前と紹介。
     *
     * **本人確認ではない。**こちらが付けた呼び名があれば、そちらが勝つ（**D46**）。
     */
    名乗られたもの?: Readonly<Record<string, { 名前: string; 紹介: string }>>;
    /** 差し替えた顔（`who` → 画面に出せる URL）。 */
    顔の画?: Readonly<Record<string, string>>;
    /** 顔を差し替える（`null` で既定へ戻す）。 */
    顔を差し替える?: (who: string, 場所: string | null) => void;
    /**
     * **こちらが書いた覚え書き**を残す（**相手の名乗りとは別**）。
     *
     * 「どの機械の、何をするエージェントか」を人が自分の言葉で残す所。
     */
    覚え書きを書く?: (key: string, note: string) => void;
    /** 部屋に名前を付ける（**画面の中だけ**）。 */
    部屋に名前を付ける?: (id: string, 名前: string) => void;
    /**
     * そのルームを抜ける。
     *
     * **抜ける口が画面に無かった**（窓を閉じるときだけ内部で抜けていた）——
     * オーナー・2026-09-10「**たとえば、ルームぬけれるの？ みたいなところとかね**」。
     */
    ルームを抜ける?: (id: string) => void;
    /**
     * その人ぶんの鍵を 1 本出して渡す（**D84**）。
     *
     * **住所を知らない相手を呼ぶ道が、画面に無かった** ——
     * 鍵をもらうのを待つしかなかった。**こちらから渡せる。**
     */
    鍵を渡す?: (相手: 行) => void;
    /**
     * そのルームに人を呼ぶ（鍵を 1 本出して渡す）。
     *
     * **ルームに誰も居ないとき、何もできなかった**（オーナー・2026-09-10
     * 「ルームに誰もいないとき、これなにしたらいいかわからないですが？」）——
     * 鍵を出す口はルームの面にしか無く、**このルームに呼ぶ**道が無かった。
     */
    ルームに呼ぶ?: (id: string) => void;
    /** もうその人に鍵を渡してあるか。 */
    鍵を渡してあるか?: (key: string) => boolean;
  }
  const {
    locale,
    素材,
    選んでいる,
    選ぶ,
    押す,
    呼んでいる,
    名前を付ける,
    降ろす,
    止める,
    鍵なしで入れる,
    預かり所がある = false,
    プロフィール = [],
    名乗りを書く = () => {},
    名乗られたもの = {},
    顔の画 = {},
    顔を差し替える = () => {},
    覚え書きを書く = () => {},
    部屋に名前を付ける = () => {},
    ルームを抜ける = () => {},
    鍵を渡す = () => {},
    ルームに呼ぶ = () => {},
    鍵を渡してあるか = () => false,
  }: Props = $props();

  /**
   * いま呼び名を付けている相手。**1 人ずつ。**
   *
   * **会議で会っただけの相手には、名前を付ける口が要る。**
   * 付けないと連絡帳に残らず、住所も覚えられない（住所だけの行は作らないため）。
   */
  let 書き換え中 = $state<string | null>(null);
  let 下書き = $state('');

  function 名付けを始める(行: 行) {
    if (行.種類 !== '人') return;
    書き換え中 = 行.key;
    下書き = 素材.覚えた.find((c) => c.key === 行.key)?.label ?? '';
  }
  function 名付けを決める(key: string) {
    名前を付ける(key, 下書き);
    書き換え中 = null;
  }

  const t = (key: MessageKey) => MESSAGES[locale][key];

  /** 写せたことを見せる時間（ms）。**押した手応えが無いと、人は二度押す。** */
  const 写した印の時間 = 1600;
  let 写した = $state(false);

  /**
   * 自分の公開鍵を写す。**渡すのは全桁。**
   *
   * 画面に出しているのは頭だけ（人が読むため）だが、
   * **渡すときに頭だけでは相手が使えない。**
   */
  async function 鍵を写す(key: string) {
    try {
      await navigator.clipboard.writeText(key);
    } catch {
      // **黙って失敗させない。**写せなければ、印を出さない
      return;
    }
    写した = true;
    setTimeout(() => (写した = false), 写した印の時間);
  }

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
          // AI の行は、その 1 つが着いているかどうかで決まる
          机に着いている: 相手.いま会議に居る,
          預かり所がある,
        })
      : [],
  );

  /**
   * 札の見た目。**4 枚とも、アプリと同じ線のアイコンで描く**
   * （絵文字を混ぜない —— オーナー指摘 2026-09-10「少々ださいですね。
   * アイコンがそうかんじさせます」）。
   */
  /**
   * 「この PC」の説明を開いているか。
   *
   * オーナー指摘（2026-09-10）——「**ここは、説明なしに人が理解できる仕様では
   * ないです**」。**印だけでは足りない** —— 何が ● で何が ○ かを書く。
   */
  let 説明を開いている = $state(false);

  /**
   * つなぎ方を出しているか（切れているエージェント用）。
   *
   * **グレーを緑にする道が、画面のどこにも無かった** ——
   * `warifu mcp` を知っている人だけが繋げる形だった（オーナー・2026-09-10
   * 「人はどう操作すると想定するか、そのためにはどんな機能がいるか」）。
   */
  let つなぎ方を開いている = $state(false);

  /** ルームを抜ける前の確かめ。**抜けると全員に伝わる。** */
  let 抜けるか確かめている = $state(false);

  /** 写した行（押した所に「写しました」と出す）。 */
  let 写した行 = $state<string | null>(null);

  /** その行を写す。**中身は端末で叩くコマンドだけ**（秘密は入らない）。 */
  async function 行を写す(中身: string) {
    await navigator.clipboard.writeText(中身);
    写した行 = 中身;
    setTimeout(() => (写した行 = null), 1500);
  }

  /** 顔を大きく出しているか（オーナー・2026-09-10「クリックしたら、大きく表示」）。 */
  let 顔を大きく = $state(false);

  /**
   * 初期アバターに戻す前の確かめ（オーナー・2026-09-10
   * 「おしたときにもーだるで確認してください。ほんとうか」）。
   *
   * **差し替えた画像は消える。**押した指が滑っただけで消えないようにする。
   */
  let 戻すか確かめている = $state(false);

  /** 誰も選んでいないときに出す様子。**数えるだけ**（`home.ts`）。 */
  const 様子 = $derived(いまの様子(素材, 預かり所がある));

  const 口の見た目: Record<口の種類, { icon: IconName; label: MessageKey }> = {
    chat: { icon: 'chat', label: 'act.chat' },
    group: { icon: 'people', label: 'act.group' },
    call: { icon: 'camera', label: 'act.call' },
    calendar: { icon: 'calendar', label: 'act.calendar' },
  };

  /** 状態の札。**色だけで言わない** —— 文字を必ず添える（DESIGN §4）。 */
  const 状態の札: Record<状態, { label: MessageKey; 色: 'ok' | 'wait' | 'no' }> = {
    できる: { label: 'act.state.ok', 色: 'ok' },
    条件つき: { label: 'act.state.wait', 色: 'wait' },
    まだできない: { label: 'act.state.no', 色: 'no' },
  };

  /** 押せる札のボタンの文言。**押せない札には出さない。** */
  const 押す文言: Partial<Record<口の種類, MessageKey>> = {
    group: 'act.group.action',
    call: 'act.call.action',
  };

  /**
   * 行に出す名前。
   *
   * **文言の鍵を持つのは 2 つだけ** —— 自分と、誰も着いていないときの
   * 「マイ PC エージェント」。着いている AI は**呼び方をそのまま持っている**
   * （`zumen の AI` など）ので、訳そうとすると空になる（2026-09-08 に実物で出た）。
   */
  /** 数えて呼ぶルームの印（`list.ts` が付ける）。 */
  const 数えたルーム = /^room\.nth:(\d+):(\d+)$/;

  function 名(行: 行): string {
    // **生の id は出さない。**名前が無いルームは「ルーム 1（3 人）」と数えて呼ぶ
    const 数えた = 数えたルーム.exec(行.name);
    if (数えた) return format(t('room.nth'), { n: 数えた[1], m: 数えた[2] });
    // **名乗っているなら、その名前で呼ぶ**（この端末の人とエージェントだけ・2026-09-08）
    const 名乗り = 名乗りを引く(行)?.name;
    if (名乗り) return 名乗り;
    if (行.種類 === '自分' || 行.key === 机の印) return t(行.name as MessageKey);
    // **相手の名乗りは、呼び名を付けていないときだけ名前になる**（D46 / `claimed.ts`）
    if (行.種類 === '人') {
      return 呼ぶ名(呼び名を付けているか(行) ? 行.name : undefined, 名乗られたもの[行.key], 行.name);
    }
    return 行.name;
  }

  /**
   * その行に、こちらが付けた呼び名があるか。
   *
   * 連絡帳の行は、覚えていなければ**鍵の頭**が名前になっている
   * （`呼び名` と同じ扱い）。**鍵の頭を「呼び名」と数えない。**
   */
  function 呼び名を付けているか(行: 行): boolean {
    return !行.name.endsWith('…');
  }

  /** いま机に着いているエージェントが 1 人でも居るか。 */
  const エージェントが居る = $derived(
    区画
      .find((s) => s.title === 'contacts.this')
      ?.行たち.some((r) => r.種類 === 'AI') ?? false,
  );

  /** いま名乗りを書き換えている行の key。 */
  let 名乗り書き中 = $state<string | null>(null);
  let 名前の下書き = $state('');
  let 紹介の下書き = $state('');
  /** 覚え書きの書きかけ。**相手ごとに入れ直す。** */
  let 覚え書きの下書き = $state('');

  /** 名乗りを書き始める。**いま書いてあるものを入れておく**（消してから書き直させない）。 */
  function 名乗りを始める(行: 行) {
    const いま = 名乗りを引く(行);
    名前の下書き = いま?.name ?? '';
    紹介の下書き = いま?.bio ?? '';
    名乗り書き中 = 行.key;
  }

  /** 部屋の名前を決める。**画面の中だけ。** */
  function 部屋の名前を決める(行: 行) {
    const id = 部屋のid(行.key);
    if (id) 部屋に名前を付ける(id, 名前の下書き);
  }

  /** 書いたものを決める。**空にすると、その 1 人ぶんが消える。** */
  function 名乗りを決める(行: 行) {
    const 誰 = 行.種類 === '自分' ? 'me' : 行.key;
    名乗りを書く(誰, 名前の下書き.trim(), 紹介の下書き.trim());
    名乗り書き中 = null;
  }

  /**
   * 選んだ相手が変わったら、書きかけを入れ直す。
   *
   * **前の相手に書きかけたものを、次の相手へ持ち越さない。**
   */
  $effect(() => {
    const 誰 = 選んでいる;
    if (!誰) return;
    if (部屋か(誰)) {
      名前の下書き = 相手?.name ?? '';
      return;
    }
    覚え書きの下書き = 素材.覚えた.find((c) => c.key === 誰)?.note ?? '';
  });

  /** その行のプロフィール。**この端末の人とエージェントにしかない。** */
  function 名乗りを引く(行: 行): ProfileRow | undefined {
    if (行.種類 === '自分') return プロフィール.find((p) => p.who === 'me');
    if (行.種類 !== 'AI') return undefined;
    // 机の行の key は `desk:<呼び方>`。**着いていないときの行（`desk:`）は素通し**
    return プロフィール.find((p) => p.who === 行.key);
  }

  /**
   * その行に「どの席か」を添えるか。
   *
   * **名乗った名前と、どこで動いているかは別のものである。**
   * 名乗りだけを出すと、`git-qa` の席のエージェントが「zumen」と名乗ったときに
   * **人が取り違える**（**D75**）。
   */
  function 席を添える(行: 行): boolean {
    return 行.種類 === 'AI' && !!名乗りを引く(行)?.name;
  }

  /**
   * 席の札に出す文字。
   *
   * **「どこで動いているか」だけでよい。**行そのものがエージェントの行なので、
   * 「◯◯ のエージェント」の後半は繰り返しになる。
   */
  function 席の札(行: 行): string {
    return 行.name.replace(/ のエージェント$/, '');
  }

  /**
   * **読み上げ・名前で押すときの名**（`issues/3`）。
   *
   * 見えている文字を全部つながない。**1 つに決める。**
   * 部屋は**切り詰めた文字を名前にしない** —— 名前が付いていればそれ、
   * 無ければ id の全桁（`…` は目で見るためのものである）。
   */
  function 読み上げる名(行: 行): string {
    if (部屋のid(行.key)) return 名(行);
    const 席 = 席を添える(行) ? `（${席の札(行)}）` : '';
    // **色だけで言わない。**読み上げにも つながっている／切れている を入れる
    const 印 = 在席の印(行);
    const 在席 = 印 ? `（${印 === 'つながっている' ? t('presence.on') : t('presence.off')}）` : '';
    return `${名(行)}${席}${在席}`;
  }

  /** 差し替えた顔があれば、画面に出せる URL。 */
  function 顔の画像(行: 行): string | null {
    const 誰 = 行.種類 === '自分' ? 'me' : 行.key;
    return 顔の画[誰] ?? null;
  }

  /** その行の顔を差し替えられるか（**この端末の人と、この端末のエージェントだけ**）。 */
  function 顔を差し替えられるか(行: 行): boolean {
    return 行.種類 === '自分' || (行.種類 === 'AI' && 行.key !== 机の印);
  }

  /** いま差し替えた顔を持っているか。 */
  function 顔を差し替えているか(行: 行): boolean {
    return !!顔の画像(行);
  }
</script>

<!--
  **Esc でどの幕も閉じる**（DESIGN §10-A）。
  開いている幕が無いときは何もしない
-->
<svelte:window
  onkeydown={(e) => {
    if (e.key !== 'Escape') return;
    顔を大きく = false;
    説明を開いている = false;
    戻すか確かめている = false;
    つなぎ方を開いている = false;
    抜けるか確かめている = false;
  }}
/>

<div class="pane">
  <div class="list">
    <!--
      **右上の角に置く**（オーナー・2026-09-10「みぎのはしっこなんですよ。うえの。
      かどっこ」「字にくっつけないで」）。**見出しの文字に付けない。**
    -->
    <button
      type="button"
      class="ask"
      title={t('help.open')}
      aria-label={t('help.open')}
      aria-expanded={説明を開いている}
      onclick={() => (説明を開いている = !説明を開いている)}>?</button>
    {#each 区画 as 一区画 (一区画.title)}
      <h2>
        <!--
          **「この PC」だけに ? を置く。**ここは説明なしには読めない ——
          緑とグレーが何を指すのか、なぜ相手の人には丸が無いのか（オーナー・2026-09-10）。

          **見出しの左の端に置き、押すと浮いて出る**（畳んで開く形にしない ——
          オーナー・2026-09-10「ポップアップです。あこーでぃおんにしないでください」）
        -->
        {t(一区画.title as MessageKey)}
      </h2>
      {#if 一区画.行たち.length === 0}
        <p class="hint">{t('contacts.empty')}</p>
      {/if}
      <!--
        **「マイ PC エージェント」という 1 人は居ない。**
        誰も着いていないときに行を出すと、それが 1 人に見える ——
        「あなたが接続したならあなたはあなたで一意で、git-qa, zumen が接続したら
        また別の人です」（オーナー・2026-09-08）。**出すのは案内である。**
      -->
      {#if 一区画.title === 'contacts.this' && !エージェントが居る}
        <p class="hint">{t('contacts.desk.none')}</p>
      {/if}
      {#each 一区画.行たち as 行 (行.key)}
        <!-- **呼び名を変える口は、名前の隣の鉛筆だけ。**
             一覧にも出すと入力欄が 2 つ並び、どちらに打てばよいか分からなくなる
             （2026-09-07 に実物で出た）。**同じことをする口を 2 つ置かない。** -->
        <!--
          **表に出す名前は 1 つだけ**（`issues/3`）。
          中の文字を全部つなぐと `"図面くん 図面くん zumen"` になり、
          **名前で押せず、読み上げも同じ語を繰り返す。**
          **切り詰めた文字を名前にしない** —— `…` は目で見るためのものである。
        -->
        <button
          type="button"
          class="row"
          class:on={行.key === 選んでいる}
          aria-label={読み上げる名(行)}
          aria-pressed={行.key === 選んでいる}
          onclick={() => 選ぶ(行.key === 選んでいる ? null : 行.key)}
        >
          <!-- **部屋は顔を持たない。**人と AI にだけ顔を出す。
               顔は名前の言い換えなので、読み上げからは外す -->
          {#if 部屋か(行.key)}
            <Icon name="chat" size={16} />
          {:else}
            <!--
              **印は顔のバッヂで出す**（オーナー・2026-09-10「アバターの画像に
              バッヂで。緑の丸、グレーの丸」）。
              出すのは**分かる所だけ**（`presence.ts`）—— 相手の人には出さない
            -->
            <span class="face" aria-hidden="true">
              <Avatar 種={行.key} 大きさ={20} 画像={顔の画像(行)} 名="" />
              {#if 在席の印(行)}
                <span
                  class="badge {在席の印(行) === 'つながっている' ? 'on' : 'off'}"
                  title={在席の印(行) === 'つながっている' ? t('presence.on') : t('presence.off')}
                ></span>
              {/if}
            </span>
          {/if}
          <span class="name">{名(行)}</span>
          <!--
            **名乗りと席の両方を出す**（**D75**）。
            席が `git-qa` のエージェントが「zumen」と名乗ることはできるので、
            **名前だけを見て取り違えない形**にする
          -->
          {#if 席を添える(行)}<span class="seat">{席の札(行)}</span>{/if}
          <!-- **在席は出さない。**相手が起動しているかは分からない -->
          <!-- **在席は出さない。**着いているかどうかは、行が在ること自体で分かる -->
        </button>
      {/each}
    {/each}
  </div>

  <div class="person">
    {#if !相手}
      <!--
        **初期画面のダッシュボード**（オーナー指摘 2026-09-10
        「**私の依頼は、『相手を選ぶと、できることが出ます。』という
        初期画面のダッシュボードです**」）。

        それまで**1 行だけ**だった。開いた人が最初に見る所なのに、
        **この道具が何をするものか**が書いていなかった。
      -->
      <h2 class="home"><span class="who">{t('home.title')}</span></h2>
      <p class="hint">{t('home.lead')}</p>

      <!-- **いまの様子。**数えているだけで、判断はしていない -->
      <section class="now">
        <h4>{t('home.now')}</h4>
        <dl>
          <div>
            <dt>{t('home.seats')}</dt>
            <dd>{format(t('home.people'), { n: 様子.席 })}</dd>
          </div>
          <div>
            <dt>{t('home.rooms')}</dt>
            <!-- **室と人を分けて出す。**「1 人」だけでは、何が 1 人か分からない -->
            <dd>{format(t('home.rooms.n'), { n: 様子.ルーム, m: 様子.ルームの人 })}</dd>
          </div>
          <div>
            <dt>{t('home.contacts')}</dt>
            <dd>{format(t('home.people'), { n: 様子.覚えた })}</dd>
          </div>
          <div>
            <dt>{t('home.postbox')}</dt>
            <dd>{様子.預かり所 ? t('home.postbox.on') : t('home.postbox.off')}</dd>
          </div>
        </dl>
      </section>

      <!-- **札は選んだあとと同じ形。**あちらは「その相手に」、ここは「この道具で」 -->
      <div class="acts">
        {#each できることの案内() as 案内 (案内.種類)}
          <div class="act" class:dim={案内.状態 === 'まだできない'}>
            <div class="top">
              <span class="glyph"><Icon name={口の見た目[案内.種類].icon} size={17} /></span>
              <span class="title">{t(口の見た目[案内.種類].label)}</span>
              <span class="tag {状態の札[案内.状態].色}">{t(状態の札[案内.状態].label)}</span>
            </div>
            <p class="why">{t(案内.訳 as MessageKey)}</p>
          </div>
        {/each}
      </div>

      {#if はじめの一歩を出すか(様子)}
        <!-- **ここから先へ進めない人にだけ、どこを押すかを言う** -->
        <section class="howto">
          <h4>{t('home.next')}</h4>
          <p class="hint">{t('home.next.hint')}</p>
        </section>
      {/if}
    {:else}
      <h2>
        {#if 相手.種類 !== '部屋'}
          <!-- **押したら大きく出す。**人は押せば大きくなると思う（オーナー・2026-09-10） -->
          <button
            type="button"
            class="face big"
            title={t('face.big.open')}
            aria-label={t('face.big.open')}
            onclick={() => (顔を大きく = true)}
          >
            <Avatar 種={相手.key} 大きさ={40} 画像={顔の画像(相手)} 名={名(相手)} />
          </button>
        {/if}
        <span class="who">{名(相手)}</span>
        {#if 席を添える(相手)}<span class="seat">{席の札(相手)}</span>{/if}
        <!-- **まだ誰も着いていない行には出さない。**
             その行は「そういう仕組みがある」という案内であって、1 人ではない -->
        {#if 相手.種類 === '自分' || (相手.種類 === 'AI' && 相手.key !== 机の印)}
          <!-- **この端末の人と AI は、この端末の持ち主が書く。**
               名前の隣の鉛筆 1 つだけ（呼び名と同じ置き方） -->
          <button
            type="button"
            class="pencil"
            title={t('profile.edit')}
            aria-label={t('profile.edit')}
            onclick={() => 名乗りを始める(相手)}
          >
            <Icon name="pencil" size={15} />
          </button>
        {/if}
        {#if 相手.種類 === '人'}
          <!-- **名前の隣に置く。**名前を書き換える口は、名前のそばにあるのが普通である
               （2026-09-07 オーナー指摘「名前をつけるの配置が悪いです」） -->
          <button
            type="button"
            class="pencil"
            title={t('roster.name.action')}
            aria-label={t('roster.name.action')}
            onclick={() => 名付けを始める(相手)}
          >
            <Icon name="pencil" size={15} />
          </button>
        {/if}
      </h2>
      {#if 書き換え中 === 相手.key}
        <div class="rename">
          <input
            type="text"
            bind:value={下書き}
            placeholder={t('roster.name.placeholder')}
            onkeydown={(e) => {
              if (e.key === 'Enter') 名付けを決める(相手.key);
              if (e.key === 'Escape') 書き換え中 = null;
            }}
          />
          <button type="button" class="quiet" onclick={() => 名付けを決める(相手.key)}>
            {t('roster.name.save')}
          </button>
        </div>
      {/if}
      <!--
        **自分の行は「プロフィール」である。**
        人は自分の名前を押したら、名前やアバターを直せると思う（オーナー・2026-09-08）。
        **warifu には自分で名乗る名前が無い** —— 呼び名は相手が付ける（**D46**）ので、
        そのことを書く。**書かないと「まだ作っていないだけ」に見える。**

        **ここに口を出すなら、効く口だけにする。**
        公開鍵を渡すのは実際にやることなので、コピーだけを置く
        （オーナー・2026-09-08「なにもつかえないなら、あることは誤解しか生みません」）。
      -->
      <!-- **書いた紹介は、名前のすぐ下に出す。**プロフィールはそういう形をしている -->
      {#if 名乗りを引く(相手)?.bio}
        <p class="bio">{名乗りを引く(相手)?.bio}</p>
      {/if}

      {#if 名乗り書き中 === 相手.key}
        <div class="rename profile">
          <input
            type="text"
            bind:value={名前の下書き}
            placeholder={t('profile.name')}
            onkeydown={(e) => {
              if (e.key === 'Enter') 名乗りを決める(相手);
              if (e.key === 'Escape') 名乗り書き中 = null;
            }}
          />
          <textarea
            rows="2"
            bind:value={紹介の下書き}
            placeholder={t('profile.bio')}
            onkeydown={(e) => {
              if (e.key === 'Escape') 名乗り書き中 = null;
            }}
          ></textarea>
          <div class="tail">
            <button type="button" class="quiet" onclick={() => 名乗りを決める(相手)}>
              {t('profile.save')}
            </button>
            <button type="button" class="quiet" onclick={() => (名乗り書き中 = null)}>
              {t('profile.cancel')}
            </button>
          </div>
        </div>
      {/if}

      <!--
        **できることのダッシュボード**（オーナー承認 2026-09-10）。
        「チャットをするのか、グループチャットをするのか、かれんだーで予定を
        みるのか、ビデオ会議を開始するのか」—— **4 枚で固定**し、
        どの札にも**できる／条件つき／まだできない**と、その 1 行を必ず付ける。
      -->
      {#if 口たち.length > 0}
        <div class="acts">
          {#each 口たち as 口 (口.種類)}
            <div class="act" class:dim={口.状態 === 'まだできない'}>
              <div class="top">
                <span class="glyph"><Icon name={口の見た目[口.種類].icon} size={17} /></span>
                <span class="title">{t(口の見た目[口.種類].label)}</span>
                <span class="tag {状態の札[口.状態].色}">{t(状態の札[口.状態].label)}</span>
              </div>
              <!-- **訳はどの札にも出す。**
                   訳の無い札は、使う人には壊れているとしか見えない -->
              <p class="why">{t(口.訳 as MessageKey)}</p>
              {#if 口.押せる && 押す文言[口.種類]}
                <button
                  type="button"
                  class="go primary"
                  disabled={呼んでいる === 相手.key}
                  onclick={() => 押す(口.種類, 相手)}
                >
                  {口.種類 === 'call' && 呼んでいる === 相手.key
                    ? t('act.call.working')
                    : t(押す文言[口.種類] as MessageKey)}
                </button>
              {/if}
            </div>
          {/each}
        </div>
      {/if}

      <!--
        **顔は落として差し替える**（オーナー・2026-09-08「ユーザによって差し替え可能にも」）。
        **PNG / JPG / WebP を受け取り、切り取って 512px・64 KB の WebP で置く**（D87）——
        受け取ったファイルをそのまま信じない
      -->
      {#if 顔を差し替えられるか(相手)}
        <p class="hint">{t('profile.face.drop')}</p>
        {#if 顔を差し替えているか(相手)}
          <div class="tail">
            <!-- **押しても、すぐには消さない。**確かめてから消す -->
            <button type="button" class="quiet" onclick={() => (戻すか確かめている = true)}>
              {t('profile.face.clear')}
            </button>
          </div>
        {/if}
      {/if}

      {#if 相手.種類 === 'AI' && 相手.key !== 机の印 && 名乗り書き中 !== 相手.key}
        <!-- **書けるのは持ち主だけ**であることを、書く所のそばで言う -->
        <p class="hint">{t('profile.ai.hint')}</p>
      {/if}

      {#if 相手.種類 === '自分'}
        <p class="hint">{t('contacts.me.what')}</p>
        <p class="key">
          <span class="label">{t('contacts.key.label')}</span>{鍵の頭(相手.key)}
        </p>
        <div class="tail">
          <button type="button" class="quiet" onclick={() => 鍵を写す(相手.key)}>
            <Icon name={写した ? 'check' : 'copy'} size={16} />
            {写した ? t('contacts.me.copied') : t('contacts.me.copy')}
          </button>
        </div>
        <p class="hint">{t('contacts.me.key.what')}</p>
        <p class="hint">{t('contacts.me.share')}</p>
        <p class="hint">{t('contacts.me.name')}</p>

        <!--
          **手順を書く**（オーナー・2026-09-09
          「公開鍵をコピーできるのは何の目的か、どんな手順で相手（知り合いの人間）と
          つながれるかわかりません」）。

          **鍵を出す口はあったが、順番が無かった。**押せる口を並べただけでは、
          初めての人は**どれから押すのか**が分からない。
        -->
        <section class="howto">
          <h4>{t('howto.title')}</h4>
          <ol>
            <li>{t('howto.1')}</li>
            <li>{t('howto.2')}</li>
            <li>{t('howto.3')}</li>
            <li>{t('howto.4')}</li>
          </ol>
          <!-- **公開鍵と部屋の鍵を取り違えさせない。**別のものである -->
          <p class="hint">{t('howto.key')}</p>
        </section>
      {/if}
      {#if 相手.種類 === '部屋'}
        <!-- **部屋は押して見るもの。**口は出さない（居るだけ） -->
        <p class="hint">
          {相手.いま会議に居る ? t('room.members.some') : t('room.alone')}
        </p>
        <!-- **その場で呼べる。**鍵を出す口を、ルームの面まで探しに行かせない -->
        <div class="tail">
          <button
            type="button"
            class="primary"
            onclick={() => {
              const id = 部屋のid(相手.key);
              if (id) ルームに呼ぶ(id);
            }}
          >
            <Icon name="key" size={16} />{t('room.invite')}
          </button>
        </div>
        <!--
          **部屋に名前を付けられる**（オーナー・2026-09-08
          「部屋名も決められないと、どの部屋？って人間はなります」）。
          **名前はこの画面の中だけ** —— 部屋 id はその場限りのものなので、
          置き場所へ書くと使い終わった名前が溜まっていく
        -->
        <div class="rename">
          <input
            type="text"
            value={名前の下書き}
            placeholder={t('room.name')}
            oninput={(e) => (名前の下書き = e.currentTarget.value)}
            onkeydown={(e) => {
              if (e.key === 'Enter') 部屋の名前を決める(相手);
            }}
          />
          <button type="button" class="quiet" onclick={() => 部屋の名前を決める(相手)}>
            {t('profile.save')}
          </button>
        </div>
        <p class="hint">{t('room.name.hint')}</p>
        <!-- **抜ける口を置く。**入ったら出られない部屋にしない -->
        <div class="tail">
          <button type="button" class="quiet" onclick={() => (抜けるか確かめている = true)}>
            {t('room.leave')}
          </button>
        </div>
      {/if}
      {#if 相手.種類 === '人'}
        <!--
          **本人の名乗りは、呼び名と違うときだけ添える**（**D75** / `claimed.ts`）。
          名乗った名前は誰でも真似できるので、**呼び名と食い違っていることが
          その場で分かる必要がある。**
        -->
        {#if 名乗りを添えるか(呼び名を付けているか(相手) ? 相手.name : undefined, 名乗られたもの[相手.key])}
          <p class="hint">
            {format(t('contacts.claimed'), { name: 名乗られたもの[相手.key].名前 })}
          </p>
        {/if}
        {#if 名乗られたもの[相手.key]?.紹介}
          <p class="bio">{名乗られたもの[相手.key].紹介}</p>
        {/if}
        <p class="key">
          <span class="label">{t('contacts.key.label')}</span>{鍵の頭(相手.key)}
        </p>
        <!--
          **こちらが書く覚え書き**（オーナー・2026-09-08
          「こちらでこのひとはこういうひとって決められるようにしたい」）。
          **相手が名乗ったものとは別** —— 名乗りは相手の都合で変わるが、これは変わらない
        -->
        <div class="rename profile">
          <textarea
            rows="2"
            bind:value={覚え書きの下書き}
            placeholder={t('contacts.note')}
          ></textarea>
          <div class="tail">
            <button
              type="button"
              class="quiet"
              onclick={() => 覚え書きを書く(相手.key, 覚え書きの下書き)}
            >
              {t('profile.save')}
            </button>
          </div>
        </div>
        <p class="hint">{t('contacts.note.hint')}</p>
      {/if}
      {#if 相手.種類 === 'AI' && 相手.いま会議に居る}
        <!-- **落とすしか止め方が無い状態にしない**（`issues/014`）。
             相手を殺すのではなく、受けた側が自分で降りる -->
        <div class="tail">
          <button type="button" class="quiet" onclick={() => 止める(相手.name)}>
            {t('act.desk.stop')}
          </button>
        </div>
        <p class="why">{t('act.desk.stop.hint')}</p>
      {/if}
      {#if 相手.種類 === 'AI' && !相手.いま会議に居る}
        <!--
          **「切れています」だけでは、どうすればよいか分からない。**
          叩くものを画面が出す（`connect.ts`）—— **割符が代わりに叩かない**（D56）
        -->
        <p class="hint">{t('contacts.desk.none')}</p>
        <div class="tail">
          <button type="button" class="primary" onclick={() => (つなぎ方を開いている = true)}>
            <Icon name="link" size={16} />{t('connect.open')}
          </button>
        </div>
      {/if}

      <!--
        **文字を打つ口は置かない**（2026-09-08 オーナー指摘
        「これを押したら何が起こるかわかりません」）。
        **行を選んだ時点で、その相手との会話は開いている** ——
        押しても何も起きない口は、誤解しか生まない。**どこに打つかだけを言う。**
      -->
      <!--
        **自分には出さない**（オーナー・2026-09-10「マイプロフの『話しかけるときは、
        右の欄に打ちます。』をけしてください」）——
        **自分を選んでいる間、右の欄は出ない。**自分あての口が無いためである。
        出ない欄を案内するのは、嘘である。
      -->
      {#if 相手.種類 !== '部屋' && 相手.種類 !== '自分'}
        <p class="hint">{t('contacts.where')}</p>
      {/if}

      {#if 相手.種類 === '人'}
        <!--
          **こちらから鍵を渡せる**（**D84**）。
          住所を知らない相手を呼ぶ道が、画面に無かった ——
          **鍵をもらうのを待つしかなかった。**
        -->
        <div class="tail">
          <button type="button" class="primary" onclick={() => 鍵を渡す(相手)}>
            <Icon name="key" size={16} />{t('key.hand')}
          </button>
        </div>
        {#if 鍵を渡してあるか(相手.key)}
          <p class="why">{t('key.hand.again')}</p>
        {/if}
        <!-- **相手が起動しているかは分からない。**分からないと出す（§2 原則 7） -->
        <p class="hint">{t('contacts.presence.none')}</p>


        {#if 鍵なしで入れる.includes(相手.key)}
          <!-- **一度通した相手は、閉じても忘れない**（D58）。だから取り消す口が要る。
               **覚えているだけの相手には出さない** —— 出しても効かない -->
          <div class="tail">
            <button type="button" class="quiet" onclick={() => 降ろす(相手.key)}>
              {t('contacts.forget')}
            </button>
          </div>
          <p class="why">{t('contacts.forget.hint')}</p>
        {/if}
      {/if}
    {/if}
  </div>

  <!--
    **説明はまんなかに出す**（オーナー・2026-09-10「ぽっぷする、まんなかに、
    バックは透過の黒とか。普通そう実装する」）。**畳んで開く形にしない。**
  -->
  {#if 説明を開いている}
    <div class="幕" role="dialog" aria-modal="true" aria-label={t('help.this.title')}>
      <div class="箱 help">
        <p class="what">{t('help.this.title')}</p>
        <dl>
          <div>
            <dt><span class="badge on" aria-hidden="true"></span></dt>
            <dd>{t('help.this.me')}</dd>
          </div>
          <div>
            <dt><span class="badge on" aria-hidden="true"></span></dt>
            <dd>{t('help.this.on')}</dd>
          </div>
          <div>
            <dt><span class="badge off" aria-hidden="true"></span></dt>
            <dd>{t('help.this.off')}</dd>
          </div>
        </dl>
        <p class="foot">{t('help.this.others')}</p>
        <div class="tail">
          <button type="button" class="quiet" onclick={() => (説明を開いている = false)}>
            {t('help.close')}
          </button>
        </div>
      </div>
    </div>
  {/if}

  <!-- **抜ける前に確かめる。**抜けたことは全員に伝わる -->
  {#if 抜けるか確かめている && 相手 && 部屋のid(相手.key)}
    <div class="幕" role="dialog" aria-modal="true" aria-label={t('room.leave.confirm')}>
      <div class="箱">
        <p class="what">{t('room.leave.confirm')}</p>
        <p class="hint">{t('room.leave.hint')}</p>
        <div class="tail">
          <button
            type="button"
            onclick={() => {
              const id = 部屋のid(相手.key);
              if (id) ルームを抜ける(id);
              抜けるか確かめている = false;
            }}
          >
            {t('room.leave.do')}
          </button>
          <button type="button" class="quiet" onclick={() => (抜けるか確かめている = false)}>
            {t('profile.cancel')}
          </button>
        </div>
      </div>
    </div>
  {/if}

  <!-- **つなぎ方。**叩くものを出す（写せる） -->
  {#if つなぎ方を開いている && 相手}
    <div class="幕" role="dialog" aria-modal="true" aria-label={t('connect.title')}>
      <div class="箱 つなぎ方">
        <p class="what">{t('connect.title')}</p>
        <p class="hint">{t('connect.lead')}</p>

        <p class="step">{t('connect.step1')}</p>
        {#each [口を足す(), 口を足す(macの実体)] as 一行 (一行)}
          <div class="cmd">
            <code>{一行}</code>
            <button type="button" class="quiet" onclick={() => void 行を写す(一行)}>
              <Icon name={写した行 === 一行 ? 'check' : 'copy'} size={14} />
              {写した行 === 一行 ? t('connect.copied') : t('connect.copy')}
            </button>
          </div>
        {/each}

        <p class="step">{t('connect.step2')}</p>
        <p class="step">{t('connect.step3')}</p>

        <!--
          **もう動いているエージェントには、貼って頼める**（オーナー・2026-09-10
          「すでに立ち上げているエージェントにコピペして依頼するみたいな案内もほしいかも」）。
          **この PC のエージェント向け** —— 別の端末のものは、そちらで叩いてもらう
        -->
        <p class="step">{t('connect.paste')}</p>
        <p class="hint">{t('connect.paste.hint')}</p>
        {#each [format(t('connect.paste.body'), { cmd: 口を足す() })] as 文 (文)}
          <div class="cmd paste">
            <code>{文}</code>
            <button type="button" class="quiet" onclick={() => void 行を写す(文)}>
              <Icon name={写した行 === 文 ? 'check' : 'copy'} size={14} />
              {写した行 === 文 ? t('connect.copied') : t('connect.copy')}
            </button>
          </div>
        {/each}

        <p class="step">{t('connect.wake')}</p>
        <div class="cmd">
          <code>{起こす(相手.name)}</code>
          <button type="button" class="quiet" onclick={() => void 行を写す(起こす(相手.name))}>
            <Icon name={写した行 === 起こす(相手.name) ? 'check' : 'copy'} size={14} />
            {写した行 === 起こす(相手.name) ? t('connect.copied') : t('connect.copy')}
          </button>
        </div>

        <p class="foot">{t('connect.docs')}</p>
        <div class="tail">
          <button type="button" class="quiet" onclick={() => (つなぎ方を開いている = false)}>
            {t('help.close')}
          </button>
        </div>
      </div>
    </div>
  {/if}

  <!-- **顔を大きく。**押した所を大きくするだけで、他には何もしない -->
  {#if 顔を大きく && 相手 && 相手.種類 !== '部屋'}
    <div
      class="幕"
      role="button"
      tabindex="-1"
      aria-label={t('help.close')}
      onclick={() => (顔を大きく = false)}
      onkeydown={(e) => {
        if (e.key === 'Escape' || e.key === 'Enter') 顔を大きく = false;
      }}
    >
      <!-- **閉じる口は置かない。**どこを押しても閉じる（オーナー・2026-09-10） -->
      <div class="大きい顔">
        <Avatar 種={相手.key} 大きさ={280} 画像={顔の画像(相手)} 名={名(相手)} />
        <p class="who">{名(相手)}</p>
      </div>
    </div>
  {/if}

  <!-- **戻す前に確かめる。**差し替えた画像は消える -->
  {#if 戻すか確かめている && 相手}
    <div class="幕" role="dialog" aria-modal="true" aria-label={t('face.clear.confirm')}>
      <div class="箱">
        <p class="what">{t('face.clear.confirm')}</p>
        <p class="hint">{t('face.clear.confirm.hint')}</p>
        <div class="tail">
          <button
            type="button"
            onclick={() => {
              顔を差し替える(相手.種類 === '自分' ? 'me' : 相手.key, null);
              戻すか確かめている = false;
            }}
          >
            {t('face.clear.do')}
          </button>
          <button type="button" class="quiet" onclick={() => (戻すか確かめている = false)}>
            {t('profile.cancel')}
          </button>
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .pane {
    display: grid;
    grid-template-columns: 260px minmax(0, 1fr);
    gap: var(--space-3);
    flex: 1;
    min-height: 0;
  }
  .list {
    position: relative;
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
    /*
      **窓の幅ではなく、この欄の幅で切り替える**（オーナー・2026-09-10
      「これまんなかつぶれちゃうので、れすぽんしぶぐあいをチェックしてください」）。
      窓が広くても、この欄は 340px しかないことがある ——
      そのとき札を 2 列にすると、**題が 1 文字ずつ縦に折れる**
    */
    container-type: inline-size;
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
    display: flex;
    align-items: center;
    gap: 6px;
    margin: var(--space-2) 0 4px;
    font-size: var(--text-sm-size);
    font-weight: 600;
  }
  /* **名前の隣の鉛筆。**押せることは分かるが、名前より前に出ない */
  button.pencil {
    display: inline-flex;
    padding: 3px;
    color: var(--text-tertiary);
    background: transparent;
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    cursor: pointer;
  }
  button.pencil:hover {
    color: var(--text-primary);
    background: var(--bg-app);
    border-color: var(--border);
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
  .howto {
    margin-top: 1rem;
    padding: 0.75rem 0.9rem;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--bg-sunken);
  }

  .howto h4 {
    margin: 0 0 0.5rem;
    font-size: 0.85rem;
    font-weight: 600;
  }

  .howto ol {
    margin: 0;
    padding-left: 1.2rem;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    font-size: 0.82rem;
    line-height: 1.5;
  }

  .howto .hint {
    margin-top: 0.5rem;
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
  /*
    **狭いときは、一覧と相手を縦に積む**（2026-09-10・縦長で実測）。
    260px ＋ 相手を横に並べると、札が窮屈で題が折れる
  */
  @media (max-width: 720px) {
    .pane {
      grid-template-columns: 1fr;
      grid-template-rows: auto auto;
    }
    .list {
      max-height: 260px;
    }
    /*
      **入れ子の巻物をやめる。**縦長では、面ぜんぶが 1 本の巻物になる ——
      相手の欄だけが 270px の窓になっていて、**できることの札が見えなかった**
      （2026-09-10 に縦長で実測）
    */
    .person {
      overflow: visible;
      min-height: 0;
    }
  }
  /* もっと狭ければ、札は 1 列 */
  @media (max-width: 520px) {
    .acts {
      grid-template-columns: 1fr;
    }
  }

  /* ── 在席の丸 ─────────────────────────────────────────────
     **色だけで言わない。**押したときの説明（title）と ? の説明で言う */
  .face {
    position: relative;
    display: inline-flex;
    flex: none;
    line-height: 0;
  }
  .badge {
    position: absolute;
    right: -1px;
    bottom: -1px;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    /* 地と同じ色で縁を取る。**顔の上に乗っても輪郭が読める** */
    box-shadow: 0 0 0 2px var(--bg-subtle);
  }
  .badge.on {
    background: #3fa45b;
  }
  .badge.off {
    background: var(--text-tertiary);
  }
  .row.on .badge {
    box-shadow: 0 0 0 2px var(--bg-app);
  }

  /* ── 「?」 ─────────────────────────────────────────────
     **縁を持たない淡い丸。**見出しの左の端に置く */
  .ask {
    position: absolute;
    top: var(--space-3);
    right: var(--space-3);
    z-index: 2;
    width: 18px;
    height: 18px;
    padding: 0;
    display: inline-grid;
    place-items: center;
    font: inherit;
    font-size: 10px;
    font-weight: 700;
    line-height: 1;
    color: var(--text-tertiary);
    background: var(--bg-sunken);
    border: none;
    border-radius: var(--radius-full);
    cursor: help;
    transition:
      color var(--dur-fast) var(--ease),
      background var(--dur-fast) var(--ease);
  }
  .ask:hover,
  .ask[aria-expanded='true'] {
    color: var(--accent);
    background: var(--accent-subtle);
  }

  /* 説明の中身。**幕はほかの窓と同じ**（`.幕` / `.箱`） */
  .help {
    width: 380px;
    font-size: var(--text-sm-size);
    line-height: var(--text-base-line);
    color: var(--text-secondary);
  }
  .help dl {
    display: grid;
    gap: var(--space-3);
    margin: var(--space-2) 0 0;
  }
  .help dl > div {
    display: grid;
    grid-template-columns: 10px 1fr;
    gap: var(--space-3);
    align-items: start;
  }
  .help dt,
  .help dd {
    margin: 0;
  }
  .help dd {
    color: var(--text-primary);
  }
  /* **相手に丸が無い理由。**線で仕切って、上の 3 行と分ける */
  .help .foot {
    margin: var(--space-4) calc(-1 * var(--space-5)) 0;
    padding: var(--space-3) var(--space-5) 0;
    border-top: 1px solid var(--border);
  }
  .help .badge {
    position: static;
    display: block;
    width: 10px;
    height: 10px;
    margin-top: 6px;
    box-shadow: none;
  }

  /* ── 顔を大きく／戻す確かめ ──────────────────────────── */
  /* 押せる顔。**枠も地も持たせない**（顔そのものが押す所である） */
  .face.big {
    padding: 0;
    border: none;
    background: none;
    border-radius: var(--radius-full);
    cursor: zoom-in;
    transition: transform var(--dur-fast) var(--ease);
  }
  .face.big:hover {
    transform: scale(1.06);
  }

  /*
    **幕・箱・大きい顔の見た目は `lib/styles/overlay.css` に置いた。**
    Svelte の `<style>` はその部品の中だけに当たるので、
    ここに書くと `+page.svelte` の幕が素のままになる（2026-09-10 に実物で出た）。
  */
  /* つなぎ方。**叩く行は等幅で、そのまま写せる形に置く** */
  .つなぎ方 {
    width: 560px;
  }
  .つなぎ方 .step {
    margin: var(--space-3) 0 0;
    font-size: var(--text-sm-size);
    font-weight: 600;
    color: var(--text-primary);
  }
  .つなぎ方 .foot {
    margin: var(--space-4) calc(-1 * var(--space-5)) 0;
    padding: var(--space-3) var(--space-5) 0;
    border-top: 1px solid var(--border);
    font-size: var(--text-xs-size);
    color: var(--text-secondary);
  }
  .cmd {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin-top: var(--space-2);
    padding: var(--space-2) var(--space-2) var(--space-2) var(--space-3);
    border-radius: var(--radius-sm);
    background: var(--bg-sunken);
  }
  /* 貼る文は長い。**折り返して全部見せる**（コマンドは 1 行のまま） */
  .cmd.paste code {
    white-space: pre-wrap;
    overflow-x: visible;
  }
  .cmd code {
    flex: 1;
    min-width: 0;
    overflow-x: auto;
    white-space: nowrap;
    font-family: var(--font-mono);
    font-size: var(--text-xs-size);
    color: var(--text-primary);
  }
  .cmd button {
    flex: none;
  }

  .箱 .tail {
    margin-top: var(--space-3);
    justify-content: flex-end;
  }

  /* いまの様子。**数字は等幅で並べる**（桁が動くと読み違える） */
  .now dl {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
    gap: var(--space-2);
    margin: var(--space-2) 0 0;
  }
  .now dt {
    font-size: var(--text-xs-size);
    color: var(--text-secondary);
  }
  .now dd {
    margin: 2px 0 0;
    font-size: var(--text-md-size);
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }

  /* できることの札。**枠線 1 本を共有して並べる**（1 枚ずつ影を付けない） */
  .acts {
    /* **縮ませない。**縮むと最後の札が切れる（2026-09-10 に実物で出た） */
    flex: none;
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 1px;
    margin-top: var(--space-3);
    background: var(--border);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    overflow: hidden;
  }
  /* **欄が狭ければ 1 列。**題を折らせない */
  @container (max-width: 460px) {
    .acts {
      grid-template-columns: 1fr;
    }
    .now dl {
      grid-template-columns: repeat(auto-fit, minmax(120px, 1fr));
    }
  }
  .act {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    padding: var(--space-3) var(--space-3) var(--space-4);
    background: var(--bg-elevated);
  }
  .act .top {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }
  .act .title {
    font-weight: 600;
    /* **1 文字ずつ縦に折れるのを止める。**溢れたら省略する */
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .act .why {
    margin: 0;
  }
  .glyph {
    width: 30px;
    height: 30px;
    flex: none;
    display: grid;
    place-items: center;
    border-radius: var(--radius-sm);
    border: 1px solid var(--accent-border);
    background: var(--accent-subtle);
    color: var(--accent);
  }
  /* 状態の札。**色だけで言わない** —— 文字が主で、地は補助である */
  .tag {
    margin-left: auto;
    font-size: var(--text-2xs-size);
    line-height: var(--text-2xs-line);
    letter-spacing: 0.04em;
    padding: 0 8px;
    border-radius: var(--radius-full);
    white-space: nowrap;
  }
  .tag.ok {
    background: var(--success-bg);
    color: var(--success-fg);
  }
  .tag.wait {
    background: var(--warning-bg);
    color: var(--warning-fg);
  }
  .tag.no {
    background: var(--neutral-bg);
    color: var(--neutral-fg);
  }
  /* **まだできない札は、沈ませる。**消さない —— 無いことを読ませる */
  .act.dim .glyph,
  .act.dim .title {
    opacity: 0.55;
  }
  .act .go {
    margin-top: var(--space-2);
    align-self: flex-start;
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
    color: var(--text-on-accent);
    background: var(--accent);
    border-color: transparent;
  }
  /* **押せないものは、押せないように見せる**（D49） */
  button:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
  .rename {
    display: flex;
    gap: 6px;
    padding: 2px 0;
  }
  /* **形は `forms.css`。**ここは幅だけ */
  .rename input {
    flex: 1;
    min-width: 0;
  }
  /* **プロフィールは縦に積む。**名前と紹介は別の物である */
  .rename.profile {
    flex-direction: column;
  }
  .rename.profile textarea {
    color: var(--text-primary);
    background: var(--bg-app);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    resize: none;
  }
  /* **名前は切らない。**狭ければ席の札のほうが下へ回る ——
     名前が「図…」になるくらいなら、2 行になったほうがよい */
  h2 {
    flex-wrap: wrap;
  }
  h2 .who {
    white-space: nowrap;
  }
  /* **どの席か。**名乗りより弱く、しかし読める（取り違えを防ぐためのもの） */
  .seat {
    flex: none;
    font-size: var(--text-2xs-size);
    color: var(--text-tertiary);
    white-space: nowrap;
  }
  /* 書いた紹介。**名前のすぐ下** */
  .bio {
    margin: 0 0 var(--space-2);
    font-size: var(--text-sm-size);
    line-height: var(--text-sm-line);
    color: var(--text-secondary);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .tail {
    display: flex;
    gap: var(--space-2);
    margin-top: var(--space-2);
  }
  button.quiet {
    padding: 5px 10px;
    font-size: var(--text-xs-size);
    color: var(--text-secondary);
    background: transparent;
    border-color: var(--border);
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
