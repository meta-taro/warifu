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
  import { できること, type 口の種類 } from './actions';
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
    選ぶ: (key: string) => void;
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

  const 口の見た目: Record<口の種類, { icon: IconName; label: MessageKey }> = {
    chat: { icon: 'chat', label: 'act.chat' },
    call: { icon: 'people', label: 'act.call' },
    mail: { icon: 'mail', label: 'act.mail' },
  };

  /**
   * 行に出す名前。
   *
   * **文言の鍵を持つのは 2 つだけ** —— 自分と、誰も着いていないときの
   * 「マイ PC エージェント」。着いている AI は**呼び方をそのまま持っている**
   * （`zumen の AI` など）ので、訳そうとすると空になる（2026-09-08 に実物で出た）。
   */
  function 名(行: 行): string {
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
    const id = 部屋のid(行.key);
    if (id) {
      // 付けた名前があるか（`鍵の頭` は末尾を `…` にする）
      return 行.name.endsWith('…') ? id : 行.name;
    }
    const 席 = 席を添える(行) ? `（${席の札(行)}）` : '';
    return `${名(行)}${席}`;
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

<div class="pane">
  <div class="list">
    {#each 区画 as 一区画 (一区画.title)}
      <h2>{t(一区画.title as MessageKey)}</h2>
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
          onclick={() => 選ぶ(行.key)}
        >
          <!-- **部屋は顔を持たない。**人と AI にだけ顔を出す。
               顔は名前の言い換えなので、読み上げからは外す -->
          {#if 部屋か(行.key)}
            <Icon name="chat" size={16} />
          {:else}
            <span aria-hidden="true">
              <Avatar 種={行.key} 大きさ={20} 画像={顔の画像(行)} 名="" />
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
      <p class="hint">{t('contacts.pick')}</p>
    {:else}
      <h2>
        {#if 相手.種類 !== '部屋'}
          <Avatar 種={相手.key} 大きさ={40} 画像={顔の画像(相手)} 名={名(相手)} />
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
        **顔は落として差し替える**（オーナー・2026-09-08「ユーザによって差し替え可能にも」）。
        **PNG だけ・512px・64 KB まで** —— 受け取ったファイルをそのまま信じない
      -->
      {#if 顔を差し替えられるか(相手)}
        <p class="hint">{t('profile.face.drop')}</p>
        {#if 顔を差し替えているか(相手)}
          <div class="tail">
            <button type="button" class="quiet" onclick={() => 顔を差し替える(相手.種類 === '自分' ? 'me' : 相手.key, null)}>
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
        <p class="hint">{t('contacts.me.share')}</p>
        <p class="hint">{t('contacts.me.name')}</p>
      {/if}
      {#if 相手.種類 === '部屋'}
        <!-- **部屋は押して見るもの。**口は出さない（居るだけ） -->
        <p class="hint">
          {相手.いま会議に居る ? t('room.members.some') : t('room.alone')}
        </p>
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
        <!-- **「居ません」だけでは、どうすればよいか分からない。**手順まで出す -->
        <p class="hint">{t('contacts.desk.none')}</p>
        <p class="hint">{t('contacts.desk.how')}</p>
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
  .rename {
    display: flex;
    gap: 6px;
    padding: 2px 0;
  }
  .rename input {
    flex: 1;
    min-width: 0;
    box-sizing: border-box;
    padding: 5px 8px;
    font: inherit;
    font-size: var(--text-sm-size);
    color: var(--text-primary);
    background: var(--bg-app);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
  }
  /* **プロフィールは縦に積む。**名前と紹介は別の物である */
  .rename.profile {
    flex-direction: column;
  }
  .rename.profile textarea {
    box-sizing: border-box;
    padding: 5px 8px;
    font: inherit;
    font-size: var(--text-sm-size);
    line-height: var(--text-sm-line);
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
