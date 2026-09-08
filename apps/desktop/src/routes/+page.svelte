<script lang="ts">
  // 会議の画面（M5）。**判断はここに置かない** — 規則は lib 側の純ロジックが持つ。
  //
  // 画面の文言は**すべて辞書から取る**（D35）。ここに日本語を直接書かない。
  import TitleBar from '$lib/window/TitleBar.svelte';
  import Roster, { type Member } from '$lib/meeting/Roster.svelte';
  import LinkBadge from '$lib/link/LinkBadge.svelte';
  import Icon from '$lib/ui/Icon.svelte';
  import { MESSAGES, format, type MessageKey } from '$lib/i18n/messages';
  import { resolveLocale, type Locale } from '$lib/i18n/locales';
  import { DEFAULT_CAPACITY } from '$lib/meeting/roster';
  import type { LinkPath } from '$lib/link/path';
  import {
    入退室の知らせ,
    話の記録,
    送ってよい,
    いま時刻,
    type 会話行,
    type 出来事,
  } from '$lib/meeting/announce';
  import { 準備を出す, 画面の状態を決める, 届く先がある as 送れるか } from '$lib/meeting/stage';
  import PaneRail from '$lib/shell/PaneRail.svelte';
  import { 既定の面, 押した後の面, type 面 as 面の型 } from '$lib/shell/panes';
  import ContactsPane from '$lib/contacts/ContactsPane.svelte';
  import { 机の印, 部屋のid, type 行 as 連絡帳の行 } from '$lib/contacts/list';
  import type { 口の種類 } from '$lib/contacts/actions';
  import ChatPanel from '$lib/chat/ChatPanel.svelte';
  import { 届く先を並べる, 宛先を決める } from '$lib/chat/reach';
  import {
    その部屋の会話,
    人の部屋,
    机の部屋,
    見る部屋,
    足す as 会話に足す,
    type 部屋の会話,
  } from '$lib/chat/rooms';
  import { どう送るか, 留守中の行 } from '$lib/chat/postbox';
  import { 呼び名 } from '$lib/meeting/names';
  import { 入室の音, 退室の音, 鳴らす } from '$lib/meeting/chime';
  import {
    describeMediaFailure,
    nextAttempt,
    sendModeFor,
    送るものを言う,
    type SendMode,
  } from '$lib/webrtc/media';
  import {
    DEFAULT_PREFS,
    canBlurBackground,
    constraintsFor,
    readStored,
    toOptions,
    withBackground,
    writeStored,
    type DeviceOptions,
    type Prefs,
  } from '$lib/webrtc/devices';
  import { Call } from '$lib/webrtc/session';
  import {
    type ClosedReason,
    EVENT_CLOSED,
    EVENT_INTRODUCED,
    EVENT_JOINED,
    EVENT_LEFT,
    EVENT_SIGNAL,
    EVENT_TEXT,
    EVENT_DESK,
    EVENT_DESK_SEATS,
    deskSeats,
    connect,
    contacts,
    callContact,
    stopKnowing,
    knownKeys,
    stopAgent,
    currentRoom,
    rooms as 部屋を読む,
    lookAtRoom,
    type RoomRow,
    type ContactRow,
    hostMeeting,
    inTauri,
    invite,
    leave,
    listen,
    log,
    myKey,
    onEvent,
    remember,
    sendText,
    sendToContact,
    postbox,
    setPostbox,
    fetchPostbox,
    setMenuLocale,
    shouldOfferTo,
    type SignalPayload,
  } from '$lib/bridge';

  const locale: Locale = resolveLocale(
    typeof navigator === 'undefined' ? [] : [...navigator.languages],
  );
  const t = (key: MessageKey) => MESSAGES[locale][key];

  /** 会議キー。**宛先と割符が 1 本になっている**（D39） */
  let meetingKey = $state('');
  let received = $state('');
  /** 会議キーの有効期間。既定 10 分。**長く置くほど、渡した先が分からなくなる** */
  /**
   * 会議キーの既定の有効期間（秒）。**24 時間。**CLI と揃えてある。
   *
   * **もとは 600 秒（10 分）だった。短すぎた**（2026-09-07 のオーナー指摘
   * 「離席したり他の作業している間に切れて、再発行する手間はアホです」）。
   *
   * **守っているのは期限ではなく割符そのもの** —— 1 本 = 1 人（D12）で、
   * 一度使われたらその本人しか戻れない（D44）。
   * **短い窓が要るのは予定に紐づく会議（D43）だけで、既定ではない。**
   */
  const KEY_TTL_SECS = 60 * 60 * 24;

  let prefs = $state<Prefs>(DEFAULT_PREFS);
  let devices = $state<DeviceOptions>({ cameras: [], microphones: [] });
  let blurAvailable = $state(false);
  let localStream: MediaStream | null = $state(null);
  /** いま何を送れる状態か（**機器が無くても入れる**）。 */
  let sendMode = $state<SendMode>('none');
  /** 支度を一度でも試したか。**まだなら「受け取るだけ」と言わない。** */
  let 支度した = $state(false);
  /**
   * 入室の最中か。
   *
   * **押した手応えが無いと、人は二度押す。**二度押すと 2 本目の経路が
   * 1 本目を置き換え、1 本目が閉じて「経路が閉じました」になる
   * （2026-09-04 に実機で踏んだ）。**押せなくするだけでなく、動いていると見せる。**
   */
  let 入室中 = $state(false);

  let members = $state<Member[]>([]);
  let notice = $state('');
  /** 相手ごとの通話（**M6**）。1 本しか持たないと、3 人目で前の相手が切れる。 */
  const calls = new Map<string, Call>();
  /** 相手ごとの映像と経路。名簿の並びで出す。 */
  let remotes = $state<Array<{ key: string; stream: MediaStream | null; path: LinkPath }>>([]);
  /**
   * 会議が始まっているか。
   *
   * **始まったら、会議キーの発行と入室の欄は出さない。**出したままだと、
   * 入っているのに「会議をはじめる」を押せてしまい、**二重に主催になる**
   * （2026-09-04 に実機で踏んだ）。
   */
  const 会議中 = $derived(remotes.length > 0);
  /**
   * 机に着いている人数（同じ PC の AI）。
   *
   * **「相手が居ない」と「話し相手が 1 人も居ない」は違う。**
   * オーナーが「会議ありきのチャットじゃない」と言った所である（2026-09-06）。
   */
  let 机のAIたち = $state<string[]>([]);
  const 机の人数 = $derived(机のAIたち.length);
  /**
   * いま見ている面。**入口は連絡帳**（会議ではない）。
   *
   * **相手の操作で動かさない**（`面へ移ってよい` / D31）。移すのは人が押したときだけ。
   */
  let 面 = $state<面の型>(既定の面);
  /** 連絡帳で選んでいる相手（公開鍵か、机の印）。 */
  let 選んだ相手 = $state<string | null>(null);
  /** いま呼んでいる相手。**二度押しを止める。** */
  let 呼んでいる = $state<string | null>(null);
  /** 覚えている相手（住所を知っているかを含む）。 */
  let 覚えた = $state<ContactRow[]>([]);
  /** 自分の公開鍵。**連絡帳の「この PC」に出す。** */
  let 自分の鍵 = $state('');
  /** いま会議キーなしで入れる相手。**覚えている相手とは別の集まり。** */
  let 鍵なしで入れる = $state<string[]>([]);
  /**
   * 置いてある預かり所の宛先（**D71**）。**置いていなければ空。**
   *
   * 預かり所は任意である。置かなければ、これまでどおり
   * **相手が起動している間だけ**届く（`docs/relay.md`）。
   */
  let 預かり所 = $state<string | null>(null);
  /** 預かり所の入力欄。**置いてある宛先とは別に持つ**（打ちかけを消さない） */
  let 預かり所の下書き = $state('');
  /**
   * **留守中に言葉を置いていった相手**（**D71** / **D72**）。
   *
   * 覚えていない相手からも届く。**行が無いと、開く所が無い** ——
   * 受け取っておいて出さないのは、黙って捨てるのと同じに見える。
   */
  let 留守中に届いた = $state<string[]>([]);

  /**
   * いま打ったものが届く先。**1 対 1 か 1 対 N かは、これを見れば分かる。**
   */
  /**
   * いまの宛先（同じ PC の AI 1 つ）。**連絡帳で AI を選んでいるときだけ決まる。**
   *
   * 決まっていれば、打ったものは**その席にだけ**届く。
   */
  const 宛先 = $derived(宛先を決める(選んだ相手, 机のAIたち));

  /**
   * 公開鍵 → 呼び名。**CLI（`warifu contacts`）と同じ置き場所を読む。**
   *
   * 鍵の頭 12 文字だけでは、人にもエージェントにも見分けが付かない ——
   * 2026-09-06 に画面のチャットで実際に困った（`67R54JO7ND6P…` が誰なのか分からない）。
   */
  let 名簿 = $state<Record<string, string>>({});

  /** 選んだ相手が、いま同じ部屋に居るか。 */
  const 選んだ人は部屋に居る = $derived(
    // **自分自身を選んでも、部屋の会話のまま。**自分との会話は作らない
    !!選んだ相手 && (選んだ相手 === 自分の鍵 || remotes.some((r) => r.key === 選んだ相手)),
  );

  /**
   * 打ったものの行き先（`$lib/chat/postbox`）。
   *
   * **相手が居ないことと、預かり所が無いことを混ぜない。**
   * 預かり所があれば預けられるし、無ければ「いま居ません」で終わる。
   */
  const 送り方 = $derived(
    どう送るか({
      宛先,
      選んでいる: 選んだ相手,
      その人は部屋に居る: 選んだ人は部屋に居る,
      預かり所がある: !!預かり所,
    }),
  );

  const 届く先 = $derived(
    送り方.種類 === '預ける'
      ? // **預ける先は 1 人。**1 対 1 がその場で分かる
        [呼び名(名簿, 送り方.key)]
      : 届く先を並べる({
          会議の相手: remotes.map((r) => 呼び名(名簿, r.key)),
          机のAIたち,
          宛先,
        }),
  );

  /** 打ったものが誰かに届くか。**会議の人でも、同じ席の AI でもよい。** */
  const 届く先がある = $derived(
    // **預けられるなら、相手が起動していなくても打てる**（D71）
    送り方.種類 === '預ける' || 送れるか({ 相手: remotes.length, 机の人数 }),
  );

  /**
   * **部屋ごとの会話。**混ぜない。閉じれば消える（履歴は `issues/010`）。
   *
   * 会話が 1 本しか無いと、**相手ごとの会話に見えない** ——
   * 「1 対 1 と 1 対 N がはっきりしない」（オーナー・2026-09-08）の本体である。
   */
  let 部屋の会話たち = $state<部屋の会話>({});
  /** いま居る部屋の id（Rust 側が持っている）。 */
  let いまの部屋 = $state<string | null>(null);
  /** いま居る部屋たち。**持てても見えなければ切り替えようがない。** */
  let 部屋たち: RoomRow[] = $state([]);

  const 連絡帳の素材 = $derived({
    自分: 自分の鍵,
    机のAIたち,
    部屋たち,
    会議の相手: remotes.map((r) => r.key),
    覚えた,
    留守中に届いた,
  });
  /** いま見ている部屋。**同じ PC の AI を選んでいれば机の部屋。** */
  const 見ている = $derived(見る部屋(選んだ相手, いまの部屋, 選んだ人は部屋に居る));
  /** 画面に出す会話。**選んだ部屋のものだけ。** */
  const 会話 = $derived(その部屋の会話(部屋の会話たち, 見ている));

  /**
   * 画面の状態（`$lib/meeting/stage`）。**会議中かどうかだけでは足りない。**
   *
   * 鍵を出して待っている間と、相手が落ちた直後は `会議中` が false になる。
   * そこで準備の口を丸ごと出し直していたため、**チャットと名簿が窓の外
   * （下へ 442px）へ押し出されていた**（2026-09-06 に実測）。
   * 落ちた知らせがチャットに出るのは、まさにその瞬間である。
   */
  const 状態 = $derived(
    画面の状態を決める({ 相手: remotes.length, 会議キー: !!meetingKey, 会話: 会話.length }),
  );
  const 支度の口を出す = $derived(準備を出す(状態));

  /**
   * いま居る部屋を読み直す。
   *
   * **部屋が増える／減るのは Rust 側の出来事**なので、
   * 節目（建てた・入った・呼んだ・誰か入ってきた）で読み直す。
   */
  /** 見る部屋を移す。**見ていない部屋も生きている。** */
  async function 部屋へ移る(id: string) {
    try {
      await lookAtRoom(id);
      いまの部屋 = (await currentRoom()) ?? null;
    } catch (e) {
      notice = 読める(e);
    }
  }

  async function 部屋を読み直す() {
    if (!inTauri()) return;
    try {
      いまの部屋 = (await currentRoom()) ?? null;
      部屋たち = (await 部屋を読む()) ?? [];
    } catch (e) {
      notice = 読める(e);
    }
  }

  async function 名簿を読む() {
    if (!inTauri()) return;
    try {
      // ブラウザで開いたときは null が返る（Tauri の外）。**そこで落ちない**
      const rows = (await contacts()) ?? [];
      名簿 = Object.fromEntries(rows.map((r) => [r.key, r.label]));
      覚えた = rows;
      鍵なしで入れる = (await knownKeys()) ?? [];
    } catch (e) {
      // **握り潰さない。**名前が出ないだけで会議は続けられる
      log(`名簿を読めなかった（${読める(e)}）`);
    }
  }
  async function 名前を付ける(key: string, label: string) {
    try {
      await remember(key, label);
      await 名簿を読む();
    } catch (e) {
      notice = 読める(e);
    }
  }

  /**
   * 相手を戸口から降ろす。**次からは割符が要る。**
   *
   * 知り合いを保存した以上、取り消す口が要る（**D58**）——
   * 保存する前は、間違って開けた相手もアプリを閉じれば切れていた。
   */
  async function 戸口から降ろす(key: string) {
    try {
      await stopKnowing(key);
      await 名簿を読む();
    } catch (e) {
      notice = 読める(e);
    }
  }

  /**
   * 同じ PC の AI に「止まれ」と言う。
   *
   * **落とすしか止め方が無い状態にしない**（`issues/014`）。
   * 相手を殺すのではなく、**受けた側が自分で降りる。**
   */
  async function AIを止める(呼び方: string) {
    try {
      await stopAgent(呼び方);
    } catch (e) {
      notice = 読める(e);
    }
  }

  /** 確かめた結果、その機器が実際にあるか。**無いものに入を出さない** */
  const カメラあり = $derived(devices.cameras.length > 0);
  const マイクあり = $derived(devices.microphones.length > 0);
  /** 知らせ音を出す口。**要るときだけ作る**（作った時点で音の許可を使う環境がある）。 */
  let 音の口: AudioContext | null = null;
  function 音を出す(chime: Parameters<typeof 鳴らす>[1]) {
    try {
      音の口 ??= new AudioContext();
      鳴らす(音の口, chime);
    } catch {
      // 握り潰す理由: 音が鳴らないことを会議の失敗にしない
    }
  }
  let call: Call | null = null;
  let keyField: HTMLTextAreaElement | undefined = $state();
  let previewVideo: HTMLVideoElement | undefined = $state();

  const CAMERA_MESSAGE = {
    'camera-denied': 'camera.denied',
    'camera-missing': 'camera.missing',
    'camera-busy': 'camera.busy',
    'camera-unknown': 'camera.unknown',
  } as const;

  // ── 支度（入室前の確認） ───────────────────────────────
  // **入ってから慌てるのが一番困る。**入る前に、自分が何で映って何で喋るかを見せる。
  async function 支度する() {
    notice = '';
    支度した = true;
    // **止めるのは、入れ替えたあと。**先に止めると、通話中の相手には
    // 静止画のあと真っ黒が映る（2026-09-04 に実機で踏んだ）
    const 前のもの = localStream;
    localStream = null;

    // **段を下げながら試す。**映像と音声 → 音声だけ → 何も送らない。
    // 1 段目で止めると、カメラの無い機械が会議に入れない
    let 試す = nextAttempt(null);
    let 最後の失敗 = '';
    while (試す) {
      try {
        const c = withBackground({ ...試す, ...機器の指定(試す) }, prefs, blurAvailable);
        localStream = await navigator.mediaDevices.getUserMedia(c);
        sendMode = sendModeFor(試す);
        適用する();
        // **会議中なら、いま流れている経路の中身を入れ替える。**張り直さない
        for (const call of calls.values()) await call.replaceTracks(localStream);
        前のもの?.getTracks().forEach((tr) => tr.stop());
        if (previewVideo) previewVideo.srcObject = localStream;
        devices = toOptions(await navigator.mediaDevices.enumerateDevices());
        return;
      } catch (e) {
        最後の失敗 = t(CAMERA_MESSAGE[describeMediaFailure(e)]);
        試す = nextAttempt(試す);
      }
    }
    // **何も取れなくても入れる。**ただし理由は伏せない
    for (const call of calls.values()) await call.replaceTracks(null);
    前のもの?.getTracks().forEach((tr) => tr.stop());
    sendMode = 'none';
    notice = 最後の失敗;
  }

  /** 選んだ機器を、その段の制約へ重ねる。 */
  function 機器の指定(attempt: MediaStreamConstraints): MediaStreamConstraints {
    const base = constraintsFor(prefs);
    return {
      audio: attempt.audio === false ? false : base.audio,
      video: attempt.video === false ? false : base.video,
    };
  }

  /** 支度の値を、いま持っている映像へ反映する。 */
  function 適用する() {
    for (const tr of localStream?.getAudioTracks() ?? []) tr.enabled = prefs.micOn;
    for (const tr of localStream?.getVideoTracks() ?? []) tr.enabled = prefs.cameraOn;
    writeStored(prefs);
    for (const c of calls.values()) c.setPrefs(prefs);
  }

  $effect(() => {
    prefs = readStored();
    blurAvailable = canBlurBackground(
      typeof navigator === 'undefined'
        ? undefined
        : navigator.mediaDevices?.getSupportedConstraints(),
    );
    if (!inTauri()) {
      notice = t('browser.only');
      return;
    }
    void (async () => {
      try {
        await setMenuLocale(locale);
      } catch {
        // メニューが訳せなくても会議はできる。**止めない**
      }
      await hostMeeting(DEFAULT_CAPACITY);
      await listen();
      const me = (await myKey()) ?? '';
      自分の鍵 = me;
      await 部屋を読み直す();
      members = [{ key: me, me: true, host: true, path: 'unknown' }];
      void 名簿を読む();
      // **置いてある預かり所を、画面にも出す。**
      // 留守中の分は Rust 側が起動時に取りに行く（`postbox.rs`）ので、
      // ここで取りに行くと二重になる
      預かり所 = (await postbox()) ?? null;
      預かり所の下書き = 預かり所 ?? '';
      // **構えてから取りに行く。**渡されたものを落とさない
      await 留守中の分を取りに行く();
    })();
  });

  $effect(() => {
    const unsubs: Array<() => void> = [];
    void (async () => {
      // **窓より先に AI が着いていることがある。**知らせを待つだけだと、
      // その 1 人を数え損ねて「入ってきたら送れます」が出たままになる
      机のAIたち = (await deskSeats()) ?? [];
      unsubs.push(
        await onEvent<string>(EVENT_JOINED, async (key) => {
          log(`入った人がいる（${短く(key)}）。通話を作る`);
          // **入ってきた時に読み直す。**起動時に 1 回だけだと、
          // その後 `warifu contacts add` で付けた名前が反映されない（2026-09-06 に実測）
          void 名簿を読む();
          members = [...members, { key, path: 'unknown' }];
          // **見ていない間に誰が来たかを残す。**名簿は動くが、目を離すと分からない
          if (いまの部屋) {
            部屋の会話たち = 会話に足す(部屋の会話たち, いまの部屋, {
              ...入退室の知らせ('入室', 呼び名(名簿, key), (k, v) => format(t(`chat.${k}`), v)),
              at: いま時刻(),
            });
          }
          音を出す(入室の音);
          remotes = [...remotes, { key, stream: null, path: 'unknown' }];
          await 部屋を読み直す();
          const offering = (await shouldOfferTo(key)) ?? false;
          const call = new Call(
            offering,
            {
              onRemoteStream: (s) => {
                log(`相手の映像が届いた（${短く(key)}）`);
                相手を更新(key, { stream: s });
              },
              onPath: (p) => {
                log(`経路が変わった: ${p}（${短く(key)}）`);
                相手を更新(key, { path: p });
                members = members.map((m) => (m.key === key ? { ...m, path: p } : m));
              },
            },
            prefs,
            key,
          );
          calls.set(key, call);
          log(`offer を出す側か: ${offering}`);
          if (!支度した) await 支度する();
          // **null でも入れる**（受け取るだけ・機器が無い機械）
          log(`通話を始める（送るもの: ${sendMode}）`);
          await call.begin(localStream);
        }),
      );
      unsubs.push(
        // **教わった住所へ、自分から呼びに行く**（D41）。
        // どちらが呼ぶかは D38 と同じ規則で決まっているので、
        // 両側から呼んで 2 本張られることは無い
        await onEvent<[string, string]>(EVENT_INTRODUCED, ([key, address]) => {
          if (!address || calls.has(key)) return;
          void connect(address).catch((e) => (notice = 読める(e)));
        }),
      );
      unsubs.push(
        await onEvent<[string, string]>(EVENT_TEXT, ([key, body]) => {
          log(話の記録('受信', 短く(key), body));
          // **どの部屋あてかは Rust が振り分けている。**画面はいまの部屋へ積む
          if (いまの部屋) {
            部屋の会話たち = 会話に足す(部屋の会話たち, いまの部屋, {
              who: 呼び名(名簿, key),
              body,
              mine: false,
              at: いま時刻(),
            });
          }
        }),
      );
      unsubs.push(
        // **同じ席の AI が言ったこと。**人の発言と見分けが付く形で出す
        // **どこで動いているエージェントかを、机が刻んで渡してくる。**
        // 1 台の PC で複数のエージェントが同じ机に着くので、
        // 「この PC の AI」だけでは、どれが喋ったのか分からない（2026-09-08）
        await onEvent<[string, string, string]>(EVENT_DESK, ([呼び方, body, at]) => {
          log(話の記録('送信', 呼び方, body));
          部屋の会話たち = 会話に足す(部屋の会話たち, 机の部屋, {
            who: 呼び方,
            body,
            mine: false,
            agent: true,
            at,
          });
        }),
      );
      unsubs.push(
        await onEvent<string[]>(EVENT_DESK_SEATS, (顔ぶれ) => {
          机のAIたち = 顔ぶれ;
        }),
      );
      unsubs.push(
        await onEvent<string>(EVENT_LEFT, (key) => 片付ける(key)),
      );
      unsubs.push(
        await onEvent<SignalPayload>(EVENT_SIGNAL, (p) => {
          // **誰から来たかで振り分ける。**間違えると別の組の経路が壊れる
          const 宛先 = p.from ? calls.get(p.from) : undefined;
          log(`下ごしらえが来た: ${p.step}（${p.from ? 短く(p.from) : '差出人なし'}）${宛先 ? '' : ' ← 通話が無い'}`);
          void 宛先?.receive(p);
        }),
      );
      unsubs.push(
        await onEvent<[string, ClosedReason]>(EVENT_CLOSED, ([key, 訳]) => {
          // **落ちたのを「退出しました」と言わない。**待てば戻ると誤解させる
          notice = t(訳 === 'lost' ? 'link.lost' : 'link.closed');
          片付ける(key, 訳 === 'lost' ? '切断' : '退室');
        }),
      );
    })();
    return () => unsubs.forEach((un) => un());
  });

  /**
   * タイルの並べ方を決めるための人数（自分 ＋ 相手）。
   *
   * **10 人以上をひとまとめにする。**`data-count` を人数ぶん書くと、
   * 外枠（16）まで CSS が伸びる。**そこまで細かく分ける意味は無い**（4 列で足りる）。
   */
  const タイルの数 = $derived(remotes.length + 1 >= 10 ? 'many' : String(remotes.length + 1));

  /**
   * 預かり所を**ときどき見に行く**間隔（ミリ秒）。
   *
   * 預かり所は**渡すだけ**で、こちらへ知らせてはこない（そのほうが預かり所は何も知らずに済む）。
   * 見に行かないと、**窓を開けている間に届いた分が、次の起動まで出ない。**
   * 短くするほど早く出るが、**その回数だけ「私は生きています」を預かり所に伝える**ことになる。
   */
  const 預かり所を見る間隔 = 5 * 60 * 1000;

  $effect(() => {
    if (!預かり所) return;
    const 札 = setInterval(() => void 留守中の分を取りに行く(), 預かり所を見る間隔);
    return () => clearInterval(札);
  });

  // 窓を閉じるときに「抜けます」と告げる（相手の名簿から消えるように）。
  // **閉じる側を待たせない** — 届かなくても閉じる
  $effect(() => {
    const 閉じる前に = () => void leave();
    window.addEventListener('beforeunload', 閉じる前に);
    return () => window.removeEventListener('beforeunload', 閉じる前に);
  });

  // **画面がいま何を出しているかを、ログへ書く。**
  //
  // 画面を押せない相手（別の機械のエージェント・CI）からは、
  // **絵は見えないがログは読める。**「押して確かめる」の代わりになる。
  //
  // **落ち着いてから 1 行だけ書く。**起動直後は初期化が数段に分かれるので、
  // そのまま書くと 10 ミリ秒差で同じような行が並ぶ（2026-09-04 に指摘された）。
  // 会議中も、人が 1 人入るたびに何行も出ると肝心の行が埋もれる。
  let 直前の状態 = '';
  $effect(() => {
    const 状態 = [
      `名簿 ${members.length}/${DEFAULT_CAPACITY}`,
      `相手 ${remotes.length} 人`,
      // **机に AI が着いているかは、後から追えないと分からない。**
      // 「送れない」と言われたときに、居たのか居なかったのかが読めなくなる
      `机 ${机の人数} 人${机のAIたち.length ? `（${机のAIたち.join('・')}）` : ''}`,
      // **届く先を書く。**書かないと「誰に届くはずだったか」が後から読めない
      `部屋 ${いまの部屋 ? 短く(いまの部屋) : 'なし'}／見ている ${見ている ?? 'なし'}`,
      `届く先 ${届く先.length ? 届く先.join('・') : 'なし'}`,
      `宛先 ${宛先 ?? 'なし'}`,
      `送るもの ${送るものを言う(sendMode)}`,
      `会議キー ${meetingKey ? 'あり' : 'なし'}`,
      `経路 ${remotes.map((r) => r.path).join(',') || 'なし'}`,
      notice ? `知らせ「${notice}」` : '知らせなし',
    ].join(' / ');

    const 待つ = setTimeout(() => {
      // 落ち着いた結果が前と同じなら、書かない
      if (状態 === 直前の状態) return;
      直前の状態 = 状態;
      log(`いまの画面: ${状態}`);
    }, 250);
    return () => clearTimeout(待つ);
  });

  /** 1 人ぶんの表示を差し替える。 */
  function 相手を更新(key: string, patch: { stream?: MediaStream; path?: LinkPath }) {
    remotes = remotes.map((r) => (r.key === key ? { ...r, ...patch } : r));
  }

  /**
   * 抜けた相手を片付ける。**ほかの相手との経路には触らない**（M6）。
   *
   * **なぜ抜けたかで文言を変える。**退室は本人の意思、切断は事故である。
   */
  function 片付ける(key: string, 種類: 出来事 = '退室') {
    calls.get(key)?.close();
    calls.delete(key);
    remotes = remotes.filter((r) => r.key !== key);
    members = members.filter((m) => m.key !== key);
    if (いまの部屋) {
      部屋の会話たち = 会話に足す(部屋の会話たち, いまの部屋, {
        ...入退室の知らせ(種類, 呼び名(名簿, key), (k, v) => format(t(`chat.${k}`), v)),
        at: いま時刻(),
      });
    }
    音を出す(退室の音);
  }

  /** 鍵は長い。**先頭だけ出す**（全桁は会議キーの欄で選べる） */
  const 短く = (key: string) => (key.length > 12 ? `${key.slice(0, 12)}…` : key);

  function 読める(e: unknown): string {
    if (e && typeof e === 'object' && 'message' in e) {
      const m = String((e as { message: unknown }).message);
      // Rust 側が返す既知の理由は、**画面の言語で**出す
      return m === 'meeting.key.own' ? t('meeting.key.own') : m;
    }
    return e instanceof Error ? e.message : String(e);
  }

  async function はじめる() {
    notice = '';
    try {
      meetingKey = (await invite(KEY_TTL_SECS)) ?? '';
      // **鍵を出すと部屋ができる。**どの部屋の会話かを画面が知る必要がある
      await 部屋を読み直す();
    } catch (e) {
      notice = 読める(e);
    }
  }

  /** コピーできたことを見せる時間（ms）。押した手応えが無いと、人は二度押す。 */
  const COPIED_FOR_MS = 1600;
  let copied = $state(false);

  /**
   * 会議キーを写す。
   *
   * **`navigator.clipboard` が使えない場面がある**（安全な文脈でないとき）。
   * そのときは選択して `execCommand` へ落ちる。**黙って失敗させない。**
   */
  async function 写す() {
    try {
      await navigator.clipboard.writeText(meetingKey);
    } catch {
      keyField?.select();
      if (!document.execCommand('copy')) {
        notice = t('meeting.key.copy');
        return;
      }
    }
    copied = true;
    setTimeout(() => (copied = false), COPIED_FOR_MS);
  }

  /**
   * **相手が起動していないので、預かり所へ預ける**（**D71**）。
   *
   * **届いたとは言わない。**届くのは相手が次に起動したときであり、
   * いつになるかは分からない（`postbox.kept`）。
   */
  async function 預ける(key: string, body: string) {
    notice = '';
    try {
      await sendToContact(key, body);
    } catch (e) {
      notice = 読める(e);
      return;
    }
    log(話の記録('送信', `${短く(key)}（預かり所）`, body));
    部屋の会話たち = 会話に足す(部屋の会話たち, 人の部屋(key), {
      who: t('tile.me'),
      body,
      mine: true,
      at: いま時刻(),
    });
    notice = t('postbox.kept');
  }

  async function 話す(body: string) {
    if (!body.trim()) return;
    // **相手が居ないなら、預かり所へ。**置いていなければ、そう言って終わる（D71）
    if (送り方.種類 === '送れない') {
      notice = t('send.absent');
      return;
    }
    if (送り方.種類 === '預ける') {
      await 預ける(送り方.key, body);
      return;
    }
    try {
      await sendText(body, 宛先);
      // **中身は書かない。**長さと相手だけ（下ごしらえがバイト数を出しているのと釣り合う）
      log(話の記録('送信', 宛先 ?? (会議中 ? `${remotes.length} 人` : '机'), body));
      // **自分の言ったことも並べる。**送った側に何も残らないと、言ったか分からない
      if (見ている) {
        部屋の会話たち = 会話に足す(部屋の会話たち, 見ている, {
          who: t('tile.me'),
          body,
          mine: true,
          at: いま時刻(),
        });
      }
    } catch (e) {
      notice = 読める(e);
    }
  }

  /**
   * 連絡帳から口を押した。
   *
   * **チャットは会議へ連れて行かない**（`押した後の面('chat') === null`）。
   * 「会議ありきのチャットじゃないんです」（オーナー・2026-09-07）。
   */
  async function 連絡帳から押す(種類: 口の種類, 相手: 連絡帳の行) {
    // 机の AI は同じ席に居る。**繋ぎに行くものが無い**
    if (相手.key === 机の印) return;
    if (種類 === 'mail') return;

    // すでに繋がっているなら、繋ぎ直さない
    if (!相手.いま会議に居る) {
      呼んでいる = 相手.key;
      notice = '';
      try {
        await callContact(相手.key);
        await 部屋を読み直す();
        await 名簿を読む();
      } catch (e) {
        // **チャットは、相手が起動していなくても続けられる**（預かり所・D71）。
        // 会議はそうはいかない —— 映像も音も、いま繋がっていないと始まらない
        if (種類 !== 'chat' || !預かり所) {
          notice = 読める(e);
          return;
        }
        notice = '';
      } finally {
        呼んでいる = null;
      }
    }
    const 次 = 押した後の面(種類 === 'call' ? 'call' : 'chat');
    if (次) 面 = 次;
  }

  /**
   * 預かり所の宛先を置く／外す。**人が書く**（D71）。
   *
   * 置いた直後に**留守中の分を取りに行く。**置いた甲斐が、その場で見える。
   */
  async function 預かり所を置く(宛先: string | null) {
    notice = '';
    try {
      await setPostbox(宛先);
    } catch (e) {
      notice = 読める(e);
      return;
    }
    預かり所 = 宛先?.trim() ? 宛先.trim() : null;
    預かり所の下書き = 預かり所 ?? '';
    notice = 預かり所 ? t('postbox.saved') : t('postbox.cleared');
    if (預かり所) await 留守中の分を取りに行く();
  }

  /**
   * 留守中に届いていた分を取りに行く。
   *
   * **画面から取りに行く。**知らせ（イベント）で受け取る形にしていたときは、
   * **画面が聞き始める前に渡ってしまう**ことがあった ——
   * 預かり所は渡したら手放すので、**そのまま消える**（2026-09-08 に実物で踏んだ）。
   *
   * **その人との会話へ積む。**部屋の会話に混ぜると、
   * 3 日前の言葉が「いまの部屋で言われたこと」として並ぶ。
   */
  async function 留守中の分を取りに行く() {
    if (!預かり所) return;
    const 届いた = (await fetchPostbox()) ?? [];
    if (届いた.length === 0) return;
    for (const [key, body, 秒] of 届いた) {
      log(話の記録('受信', `${短く(key)}（留守中）`, body));
      // **開く所を出す。**覚えていない相手からも届く
      if (!留守中に届いた.includes(key)) 留守中に届いた = [...留守中に届いた, key];
      部屋の会話たち = 会話に足す(
        部屋の会話たち,
        人の部屋(key),
        留守中の行(呼び名(名簿, key), body, 秒),
      );
    }
    notice = format(t('postbox.received'), { n: String(届いた.length) });
  }

  async function 入室する() {
    if (入室中) return;
    notice = '';
    入室中 = true;
    try {
      await connect(received.trim());
      await 部屋を読み直す();
    } catch (e) {
      notice = 読める(e);
    } finally {
      入室中 = false;
    }
  }
</script>

<TitleBar
  {locale}
  status={状態 === '会議中'
    ? t('meeting.status.live')
    : 状態 === '待っている'
      ? t('meeting.status.waiting')
      : ''}
/>

<!--
  **知らせは、どの面に居ても見える所に置く。**
  会議の面の中だけに置いていたため、連絡帳で送信が断られても
  **画面には何も出ず、打った文字が黙って消えたように見えた**
  （2026-09-07 にオーナーが踏んだ「こんばんは〜ってうって送るを押したけど、消えたよ」）。
-->
{#if notice}
  <p class="notice top">{notice}</p>
{/if}

<main>
  <PaneRail {locale} いまの面={面} {状態} 選ぶ={(次) => (面 = 次)} />

  <!--
    **会議の面は畳まない。隠すだけ。**
    相手の音は `<video>` 要素から出ているので、`{#if}` で外すと
    **連絡帳へ移った瞬間に相手の声が消える**（`畳んでよい('会議') === false`）。
  -->
  <div class="pane meeting" hidden={面 !== '会議'} role="tabpanel">
  <section class="stage">
    <!--
      **知らせは映像の上。**下に置くと目に入らない —— 会議中の目線は
      帯の直下か映像の中にある（2026-09-06 の実測でここへ上げた）。
      「相手との経路が切れました」は、いちばん見落としてはいけない 1 行である。
    -->
    <div class="tiles" data-count={タイルの数}>
      <div class="tile">
        <!-- 自分の映像は音を出さない（**回り込む**） -->
        <video bind:this={previewVideo} autoplay playsinline muted></video>
        <span class="cap">
          <Icon name={prefs.cameraOn ? 'camera' : 'camera-off'} />{t('tile.me')}
          <Icon name={prefs.micOn ? 'mic' : 'mic-off'} />
        </span>
      </div>
      {#each remotes as r (r.key)}
        <div class="tile">
          <!-- svelte-ignore a11y_media_has_caption -->
          <video autoplay playsinline {@attach (el) => { (el as HTMLVideoElement).srcObject = r.stream; }}></video>
          <span class="cap">{呼び名(名簿, r.key)} <LinkBadge {locale} path={r.path} /></span>
        </div>
      {/each}
    </div>
    {#if 会議中}
      <!--
        **会議中の入切。**「カメラとマイクを確かめる」（支度）とは別物にしてある。
        支度は機器を取り直すので、会議中に押すと相手には静止画のあと真っ黒が映る
        （2026-09-04 に実機で踏んだ）。ここは track の入切だけを触る。
      -->
      <!--
        **無い機器の入切を押させない。**押せる形にしておいて何も起きないのは、
        押した人には「効かない」としか見えない
        （2026-09-06 にチャットの「送る」で同じことを言われた）。
        なぜ押せないかは、押す前に読める所へ出す。
      -->
      <div class="controls">
        <button
          type="button"
          class:off={!prefs.micOn || (支度した && !マイクあり)}
          aria-pressed={prefs.micOn && (!支度した || マイクあり)}
          disabled={支度した && !マイクあり}
          title={支度した && !マイクあり ? t('setup.mic.none') : ''}
          onclick={() => { prefs.micOn = !prefs.micOn; 適用する(); }}
        >
          <Icon name={prefs.micOn && (!支度した || マイクあり) ? 'mic' : 'mic-off'} />
          {支度した && !マイクあり ? t('setup.mic.none') : t('call.mic')}
        </button>
        <button
          type="button"
          class:off={!prefs.cameraOn || (支度した && !カメラあり)}
          aria-pressed={prefs.cameraOn && (!支度した || カメラあり)}
          disabled={支度した && !カメラあり}
          title={支度した && !カメラあり ? t('setup.camera.none') : ''}
          onclick={() => { prefs.cameraOn = !prefs.cameraOn; 適用する(); }}
        >
          <Icon name={prefs.cameraOn && (!支度した || カメラあり) ? 'camera' : 'camera-off'} />
          {支度した && !カメラあり ? t('setup.camera.none') : t('call.camera')}
        </button>
      </div>
    {/if}
  </section>

  <aside>
    <!--
      **チャットを先頭に置く。**割符はビデオ会議が中心のアプリではない ——
      「**チャットやメールを MCP を通じておこなえる OSS** です。
      **会議ありきのチャットじゃないんです**」（オーナー・2026-09-06）。

      2026-09-07、オーナーの画面でチャットが 3 枚のカードの下に埋まっていた。
      **スクロールしないと会話が見えない並びは、製品と逆である。**
      支度・会議キーは、チャットの下に置く（**使うのは最初の 1 回だけ**）。
    -->
    <!--
      **名簿とチャットは、会議前でも畳まない。**この 2 つが窓の外にあると、
      相手が落ちた知らせを目で見つけられない（2026-09-05 に実際に見つけられなかった）。
      名簿は数行しかないので、チャットの始まりを押し下げない。
      （2026-09-04 にオーナーから「場所が悪い。気づかなかった」と指摘された所である）
    -->
    <Roster
      {locale}
      {members}
      capacity={DEFAULT_CAPACITY}
      names={名簿}
      onRename={(key, label) => void 名前を付ける(key, label)}
      {机のAIたち}
    />

    <ChatPanel
      {locale}
      {会話}
      {届く先がある}
      {会議中}
      {机の人数}
      送る={(body) => void 話す(body)}
      {届く先}
    />

    {#if 支度の口を出す}
    <div class="card">
      <h2><Icon name="camera" size={18} />{t('setup.title')}</h2>
      <p class="hint">{t('setup.hint')}</p>
      <button type="button" onclick={支度する}>
        <Icon name="camera" />{t('setup.action')}
      </button>

      <!--
        **無い機器のチェックを入のままにしない。**確かめた後にカメラもマイクも
        見つからない機械で、両方に ☑ が付いたまま「カメラもマイクも無いので、
        受け取るだけで入ります」と出ていた（2026-09-05 の実測）。
        **画面が自分と食い違っている。**
        好みそのもの（`prefs`）は消さない —— 後で挿せば戻る。
      -->
      <label class="row" class:dim={支度した && !マイクあり}>
        <input
          type="checkbox"
          checked={prefs.micOn && (!支度した || マイクあり)}
          disabled={支度した && !マイクあり}
          onchange={(e) => { prefs.micOn = e.currentTarget.checked; void 適用する(); }}
        />
        <Icon name={prefs.micOn && (!支度した || マイクあり) ? 'mic' : 'mic-off'} />
        {支度した && !マイクあり ? t('setup.mic.none') : t('setup.mic')}
      </label>
      <label class="row" class:dim={支度した && !カメラあり}>
        <input
          type="checkbox"
          checked={prefs.cameraOn && (!支度した || カメラあり)}
          disabled={支度した && !カメラあり}
          onchange={(e) => { prefs.cameraOn = e.currentTarget.checked; void 適用する(); }}
        />
        <Icon name={prefs.cameraOn && (!支度した || カメラあり) ? 'camera' : 'camera-off'} />
        {支度した && !カメラあり ? t('setup.camera.none') : t('setup.camera')}
      </label>

      {#if devices.cameras.length}
        <select bind:value={prefs.cameraId} onchange={支度する}>
          {#each devices.cameras as c (c.id)}<option value={c.id}>{c.label}</option>{/each}
        </select>
      {/if}
      {#if devices.microphones.length}
        <select bind:value={prefs.micId} onchange={支度する}>
          {#each devices.microphones as m (m.id)}<option value={m.id}>{m.label}</option>{/each}
        </select>
      {/if}

      {#if blurAvailable}
        <label class="row">
          <input
            type="checkbox"
            checked={prefs.background === 'blur'}
            onchange={(e) => {
              prefs.background = e.currentTarget.checked ? 'blur' : 'none';
              void 支度する();
            }}
          />
          <Icon name="blur" />{t('setup.blur')}
        </label>
      {:else}
        <p class="hint"><Icon name="blur" />{t('setup.blur.os')}</p>
      {/if}

      {#if 支度した}
        <p class="hint">
          <Icon name={sendMode === 'both' ? 'camera' : sendMode === 'audio' ? 'mic' : 'camera-off'} />
          {t(`setup.mode.${sendMode}`)}
        </p>
      {/if}
      <p class="hint"><Icon name="headphones" />{t('setup.headphones')}</p>
    </div>

    <div class="card">
      <h2><Icon name="people" size={18} />{t('meeting.start.title')}</h2>
      <p class="hint">{t('meeting.key.hint')}</p>
      <button type="button" onclick={はじめる}>
        <Icon name="people" />{t('meeting.start.action')}
      </button>
    </div>

    <div class="card">
      <h2><Icon name="enter" size={18} />{t('meeting.join.title')}</h2>
      <p class="hint">{t('meeting.join.hint')}</p>
      <textarea bind:value={received} rows="4" placeholder="WARIFU1-…#…"></textarea>
      <button type="button" onclick={入室する} disabled={入室中 || !received.trim()}>
        <Icon name="enter" />{入室中 ? t('meeting.join.working') : t('meeting.join.action')}
      </button>
    </div>
    {/if}

    <!--
      **預かり所は任意である**（D71 / `docs/relay.md`）。
      置かなければ、これまでどおり**相手が起動している間だけ**届く。
      **割符が用意する中央ではない** —— 立てるのは導入した人である（D68）。
    -->
    <div class="card">
      <h2><Icon name="postbox" size={18} />{t('postbox.title')}</h2>
      <p class="hint">{t('postbox.hint')}</p>
      <textarea
        bind:value={預かり所の下書き}
        rows="2"
        placeholder={t('postbox.placeholder')}
      ></textarea>
      <div class="row">
        <button type="button" onclick={() => void 預かり所を置く(預かり所の下書き)}>
          {t('postbox.save')}
        </button>
        <!-- **外す口を、置く口と同じ所に出す。**置いたきり戻せない物を作らない -->
        <button
          type="button"
          class="quiet"
          disabled={!預かり所}
          onclick={() => void 預かり所を置く(null)}
        >
          {t('postbox.clear')}
        </button>
      </div>
    </div>

    <!--
      **会議キーは畳んで置く。**全文は 346〜357 文字で、開いたままだと 177px を占める。
      渡した後は要らない —— ただし **消さない**（DESIGN.md §7「QR と文字列を必ず両方出す」）。
      畳んだ状態でも **コピーする** は押せる。渡すのに全文を見る必要は無い。
    -->
    {#if meetingKey}
      <div class="card key">
        <div class="field-head">
          <span class="with-icon"><Icon name="key" />{t('meeting.key.label')}</span>
          <button type="button" class="quiet" onclick={写す}>
            <Icon name={copied ? 'check' : 'copy'} />
            {copied ? t('meeting.key.copied') : t('meeting.key.copy')}
          </button>
        </div>
        <details>
          <summary>{t('meeting.key.reveal')}</summary>
          <!-- 触れた時点で全部選ぶ。**手で端から端まで引かせない** -->
          <textarea
            bind:this={keyField}
            readonly
            rows="4"
            value={meetingKey}
            onfocus={(e) => e.currentTarget.select()}
          ></textarea>
        </details>
        <!--
          **1 本の会議キーで入れるのは 1 人だけ**（割符は「1 つの鍵 = 1 人」・D12）。
          3 人目を呼ぶなら、**もう 1 本出して、その人に渡す**（D47）。
          前の鍵は死なない —— 出した本数だけ、別々の人が入れる。

          この口を出していなかったため、**鍵を出した後は作り直せなかった**
          （2026-09-06 に D45 で入れてしまった不具合）。
        -->
        <button type="button" class="quiet" onclick={はじめる}>
          <Icon name="key" />{t('meeting.key.more')}
        </button>
        <p class="hint">{t('meeting.key.more.hint')}</p>
      </div>
    {/if}

  </aside>
  </div>

  <!-- 連絡帳。**起動して最初に出る面** -->
  {#if 面 === '連絡帳'}
    <div class="pane contacts" role="tabpanel">
      <ContactsPane
        {locale}
        素材={連絡帳の素材}
        選んでいる={選んだ相手}
        選ぶ={(key) => {
          選んだ相手 = key;
          // **部屋を選んだら、Rust 側の「いま見ている部屋」も動かす。**
          // 動かさないと、打ったものが前の部屋へ流れる
          const id = 部屋のid(key);
          if (id) void 部屋へ移る(id);
        }}
        押す={連絡帳から押す}
        呼んでいる={呼んでいる}
        名前を付ける={(key, label) => void 名前を付ける(key, label)}
        降ろす={(key) => void 戸口から降ろす(key)}
        止める={(呼び方) => void AIを止める(呼び方)}
        {鍵なしで入れる}
        預かり所がある={!!預かり所}
      />
      <!--
        **自分を選んでいる間は、会話の枠を出さない。**
        自分から自分へ話す口は無いので、置いておくと
        「これは何だろう」で終わる（オーナー・2026-09-08
        「なにもつかえないなら、あることは誤解しか生みません」）。
      -->
      {#if 選んだ相手 !== 自分の鍵}
        <ChatPanel
          {locale}
          {会話}
          {届く先がある}
          {会議中}
          {机の人数}
          送る={(body) => void 話す(body)}
          相手ごとではない={送り方.種類 !== '預ける'}
          {届く先}
        />
      {/if}
    </div>
  {/if}

  <!-- 予定。**まだ動かないことを、理由つきで出す**（§2 原則 7） -->
  {#if 面 === '予定'}
    <div class="pane schedule" role="tabpanel">
      <div class="card">
        <h2><Icon name="calendar" size={18} />{t('schedule.title')}</h2>
        <p class="hint">{t('schedule.none')}</p>
      </div>
    </div>
  {/if}
</main>

<style>
  main {
    flex: 1;
    /* **レール（面の切り替え）と、面 1 つ。**面の中身は面ごとに決める */
    display: flex;
    min-height: 0;
    overflow: hidden;
  }
  .pane {
    display: flex;
    flex: 1;
    min-height: 0;
    gap: var(--space-4);
    padding: var(--space-4);
    overflow: hidden;
  }
  /* 連絡帳は「一覧 ＋ 相手」と「会話」の 2 つ */
  .pane.contacts > :global(.card) {
    width: 340px;
    flex: none;
  }
  .pane.schedule {
    align-items: flex-start;
  }
  .pane.meeting {
    display: grid;
    grid-template-columns: 1fr 340px;
    /* **窓ごと動かさない。**動いてよいのは右の列の中だけである。
       ここを `overflow-y: auto` にしていたため、右の列が 1101px まで伸びて
       **チャットと名簿が窓の下端から 442px はみ出していた**（2026-09-06 の実測）。
       映像まで一緒に流れるので、探しに行くと相手が見えなくなる。
       min-height: 0 が無いと grid の子が縮まない */
    min-height: 0;
    overflow: hidden;
  }

  /* **隠すのであって、外すのではない。**
     `display: none` でも DOM には残るので、**相手の音は鳴り続ける。**
     `{#if}` で外すと `<video>` ごと消えて声が切れる（`畳んでよい('会議') === false`）。

     **ここが最後でなければ効かない。**`.pane[hidden]` と `.pane.meeting` は
     同じ強さなので、後に書いたほうが勝つ（2026-09-07 に実物で踏んだ ——
     会議の面が隠れず、連絡帳の横に居座っていた）。 */
  .pane[hidden] {
    display: none;
  }
  .stage {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    /* **映像を内容領域いっぱいに。**左に 266px 余っていた（2026-09-06 の実測）。
       会議中の主役は映像である。min-height:0 が無いと子が縮まない */
    min-height: 0;
  }
  /* 人数で列を変える。**1 対 1 は大きく、増えたら小さく**。
     auto-fit だけに任せると、3 人のときに 1 人だけ次の行で大きく残る */
  .tiles {
    display: grid;
    gap: var(--space-3);
    grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
    /* **残りを取って、その中で縦中央に置く。**
       下に 266px 余っていた（2026-09-06 の実測）が、**映像を縦に伸ばすことはできない** ——
       幅 699px の列で 16:9 を保つと高さは 393px に決まる。伸ばせば顔が切れる。
       **余りは下に溜めず、上下へ振り分ける。**
       枠に動きは付けない（DESIGN.md §6）—— ここは置き方であって、出入りで跳ねない */
    flex: 1;
    min-height: 0;
    align-content: center;
  }
  .tiles[data-count='3'],
  .tiles[data-count='4'] {
    grid-template-columns: repeat(2, 1fr);
  }
  .tiles[data-count='5'],
  .tiles[data-count='6'],
  .tiles[data-count='7'],
  .tiles[data-count='8'],
  .tiles[data-count='9'] {
    grid-template-columns: repeat(3, 1fr);
  }
  .tiles[data-count='many'] {
    grid-template-columns: repeat(4, 1fr);
  }
  .tile {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    min-height: 0;
  }
  video {
    width: 100%;
    max-height: 100%;
    aspect-ratio: 16 / 9;
    background: var(--bg-sunken);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    object-fit: cover;
  }
  .cap {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-size: var(--text-xs-size);
    color: var(--text-secondary);
  }
  /* **帯の直下に出す。**面をまたいで同じ場所に出る */
  .notice.top {
    margin: 0 var(--space-4);
  }
  .notice {
    margin: 0;
    padding: var(--space-3);
    border-radius: var(--radius-sm);
    background: var(--warning-bg);
    color: var(--warning-fg);
    font-size: var(--text-sm-size);
    line-height: var(--text-sm-line);
  }
  aside {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    /* **窓の外へ出さない。**溢れるならここの中だけで動く（映像は動かない） */
    min-height: 0;
    overflow-y: auto;
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
  h2 {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 0;
    font-size: var(--text-sm-size);
    line-height: var(--text-sm-line);
    font-weight: 600;
  }
  .hint {
    display: flex;
    align-items: flex-start;
    gap: 6px;
    margin: 0;
    font-size: var(--text-xs-size);
    line-height: var(--text-xs-line);
    color: var(--text-tertiary);
  }
  /* **会議中の入切。**映像のすぐ下に置く（探させない） */
  .controls {
    display: flex;
    gap: var(--space-2);
    justify-content: center;
    padding: var(--space-2) 0 0;
  }
  .controls button.off {
    opacity: 0.55;
  }
  /* **押せないものは、押せないように見せる。**見た目が押せるままだと、
     押して何も起きない側に「効かない」と受け取られる */
  .controls button:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
  .row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-size: var(--text-sm-size);
  }
  .with-icon {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  /* **会議キーは畳んで置く。**全文は 346〜357 文字で 177px を占める（DESIGN.md §7 で消せない） */
  .card.key summary {
    cursor: pointer;
    font-size: var(--text-xs-size);
    color: var(--accent);
  }
  .card.key summary:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
    border-radius: var(--radius-sm);
  }
  .card.key textarea {
    margin-top: var(--space-2);
  }
  /* 無い機器は、あることを示さない */
  .row.dim {
    color: var(--text-tertiary);
  }
  .field-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
  }
  /* 主要な操作は 1 画面に 1 つ。コピーは控えめな見た目にする（DESIGN.md §2） */
  button.quiet {
    background: transparent;
    color: var(--accent);
    border-color: var(--accent-border);
    padding: 3px var(--space-2);
    font-size: var(--text-2xs-size);
  }
  button.quiet:hover:not(:disabled) {
    background: var(--accent-subtle);
  }
  textarea,
  select {
    width: 100%;
    /* 会議キーは 1 文字の違いが意味を変える（DESIGN.md §5） */
    font-family: var(--font-mono);
    font-size: var(--text-xs-size);
    line-height: var(--text-xs-line);
    color: var(--text-primary);
    background: var(--bg-app);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: var(--space-2);
  }
  textarea {
    resize: vertical;
  }
  /* **`width: 100%` に padding と枠を足すと、その分だけ横へ溢れる。**
     右の列が窓ごとスクロールしていた頃は隠れていたが、列の中だけを動かすようにしたら
     横スクロールバーになって出てきた（2026-09-06 の実測） */
  textarea,
  select {
    box-sizing: border-box;
  }
  select {
    font-family: var(--font-ui);
  }
  textarea:focus-visible,
  select:focus-visible,
  button:focus-visible,
  input:focus-visible {
    outline: 3px solid var(--accent-subtle);
    outline-offset: 1px;
  }
  button {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    align-self: flex-start;
    padding: 5px var(--space-3);
    border-radius: var(--radius-sm);
    border: 1px solid transparent;
    background: var(--accent);
    color: var(--text-on-accent);
    font: inherit;
    font-size: var(--text-sm-size);
    font-weight: 500;
    transition: background var(--dur-fast) var(--ease);
  }
  button:hover:not(:disabled) {
    background: var(--accent-hover);
  }
  button:disabled {
    background: var(--neutral-bg);
    color: var(--text-tertiary);
  }
  @media (max-width: 860px) {
    main {
      grid-template-columns: 1fr;
    }
  }
</style>
