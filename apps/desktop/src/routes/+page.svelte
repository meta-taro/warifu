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
  let 机の人数 = $state(0);
  /** 打ったものが誰かに届くか。**会議の人でも、同じ席の AI でもよい。** */
  const 届く先がある = $derived(送れるか({ 相手: remotes.length, 机の人数 }));
  /** 会議の中の文字。**残らない** — 閉じれば消える（保存には D2 の決着が要る）。 */
  let 会話 = $state<会話行[]>([]);

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
   * 公開鍵 → 呼び名。**CLI（`warifu contacts`）と同じ置き場所を読む。**
   *
   * 鍵の頭 12 文字だけでは、人にもエージェントにも見分けが付かない ——
   * 2026-09-06 に画面のチャットで実際に困った（`67R54JO7ND6P…` が誰なのか分からない）。
   */
  let 名簿 = $state<Record<string, string>>({});
  async function 名簿を読む() {
    if (!inTauri()) return;
    try {
      // ブラウザで開いたときは null が返る（Tauri の外）。**そこで落ちない**
      const rows = (await contacts()) ?? [];
      名簿 = Object.fromEntries(rows.map((r) => [r.key, r.label]));
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
  let 下書き = $state('');
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
      members = [{ key: me, me: true, host: true, path: 'unknown' }];
      void 名簿を読む();
    })();
  });

  $effect(() => {
    const unsubs: Array<() => void> = [];
    void (async () => {
      // **窓より先に AI が着いていることがある。**知らせを待つだけだと、
      // その 1 人を数え損ねて「入ってきたら送れます」が出たままになる
      机の人数 = (await deskSeats()) ?? 0;
      unsubs.push(
        await onEvent<string>(EVENT_JOINED, async (key) => {
          log(`入った人がいる（${短く(key)}）。通話を作る`);
          // **入ってきた時に読み直す。**起動時に 1 回だけだと、
          // その後 `warifu contacts add` で付けた名前が反映されない（2026-09-06 に実測）
          void 名簿を読む();
          members = [...members, { key, path: 'unknown' }];
          // **見ていない間に誰が来たかを残す。**名簿は動くが、目を離すと分からない
          会話 = [
            ...会話,
            {
              ...入退室の知らせ('入室', 呼び名(名簿, key), (k, v) => format(t(`chat.${k}`), v)),
              at: いま時刻(),
            },
          ];
          音を出す(入室の音);
          remotes = [...remotes, { key, stream: null, path: 'unknown' }];
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
          会話 = [...会話, { who: 呼び名(名簿, key), body, mine: false, at: いま時刻() }];
        }),
      );
      unsubs.push(
        // **同じ席の AI が言ったこと。**人の発言と見分けが付く形で出す
        await onEvent<[string, string, string]>(EVENT_DESK, ([key, body, at]) => {
          log(話の記録('送信', 短く(key), body));
          会話 = [...会話, { who: t('chat.agent'), body, mine: false, agent: true, at }];
        }),
      );
      unsubs.push(
        await onEvent<number>(EVENT_DESK_SEATS, (数) => {
          机の人数 = 数;
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
      `机 ${机の人数} 人`,
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
    会話 = [
      ...会話,
      {
        ...入退室の知らせ(種類, 呼び名(名簿, key), (k, v) => format(t(`chat.${k}`), v)),
        at: いま時刻(),
      },
    ];
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

  async function 話す() {
    const body = 下書き.trim();
    if (!body) return;
    try {
      await sendText(body);
      // **中身は書かない。**長さと相手だけ（下ごしらえがバイト数を出しているのと釣り合う）
      log(話の記録('送信', 会議中 ? `${remotes.length} 人` : '机', body));
      // **自分の言ったことも並べる。**送った側に何も残らないと、言ったか分からない
      会話 = [...会話, { who: t('tile.me'), body, mine: true, at: いま時刻() }];
      下書き = '';
    } catch (e) {
      notice = 読める(e);
    }
  }

  async function 入室する() {
    if (入室中) return;
    notice = '';
    入室中 = true;
    try {
      await connect(received.trim());
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

<main>
  <section class="stage">
    <!--
      **知らせは映像の上。**下に置くと目に入らない —— 会議中の目線は
      帯の直下か映像の中にある（2026-09-06 の実測でここへ上げた）。
      「相手との経路が切れました」は、いちばん見落としてはいけない 1 行である。
    -->
    {#if notice}
      <p class="notice">{notice}</p>
    {/if}
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
    />

    <div class="card chat" class:live={状態 !== '会議前'}>
      <h2><Icon name="people" size={18} />{t('chat.title')}</h2>
      <p class="hint">{t('chat.hint')}</p>
      <div class="talk">
        {#if 会話.length === 0}
          <p class="hint">{t('chat.empty')}</p>
        {/if}
        {#each 会話 as line, i (i)}
          <!-- **いつの発言かを出す。**無いと、あとから読み返せない -->
          <p class="line" class:mine={line.mine} class:system={line.system} class:agent={line.agent}>
            {#if line.at}<span class="at">{line.at}</span>{/if}{#if !line.system}<b>{line.who}</b
              >{/if}{line.body}
          </p>
        {/each}
      </div>
      <!--
        **相手が居ないときは押させない。**押せる形にしておいて「まだ誰も居ません」と
        返すのは、**押した人には「効かない」としか見えない**
        （2026-09-06 にオーナーから「チャット送るボタンきかないよ」と報告された）。
        **打ち込みは残す** —— 先に書いておいて、入ってきたら送りたいことがある。
      -->
      {#if !届く先がある}
        <p class="hint">{t('chat.nobody')}</p>
      {:else if !会議中}
        <!-- **会議に人は居ないが、同じ席の AI は居る。**話しかけられる -->
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
            : 机の人数 > 0
              ? t('chat.placeholder.desk')
              : t('chat.placeholder.nobody')}
          onkeydown={(e) => {
            if (送ってよい(e)) {
              e.preventDefault();
              void 話す();
            }
          }}
        ></textarea>
        <button type="button" onclick={話す} disabled={!届く先がある || !下書き.trim()}>
          {t('chat.send')}
        </button>
      </div>
    </div>

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
</main>

<style>
  main {
    flex: 1;
    display: grid;
    grid-template-columns: 1fr 340px;
    gap: var(--space-4);
    padding: var(--space-4);
    /* **窓ごと動かさない。**動いてよいのは右の列の中だけである。
       ここを `overflow-y: auto` にしていたため、右の列が 1101px まで伸びて
       **チャットと名簿が窓の下端から 442px はみ出していた**（2026-09-06 の実測）。
       映像まで一緒に流れるので、探しに行くと相手が見えなくなる。
       min-height: 0 が無いと grid の子が縮まない */
    min-height: 0;
    overflow: hidden;
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
  .card.chat {
    /* **残りを取る。**打ち込み欄は底に固定され、行が増えても動かない */
    flex: 1;
    /* **見出し・案内・打ち込み欄で 160px はほぼ埋まる。**
       埋まった残りが会話欄になるので、160 だと会話欄が数 px に潰れた
       （2026-09-07・オーナーの画面で「まだ何もありません」が切れていた）。
       会話欄の min-height と足し合わせた高さにする */
    min-height: 320px;
  }
  .talk {
    display: flex;
    flex-direction: column;
    gap: 4px;
    /* **溢れたら中で動く。**外側（画面全体）を伸ばさない */
    flex: 1;
    /* **空でも読める高さを持つ。**0 だと、親に余りが無いときに潰れて
       「まだ何もありません」の 1 行すら切れる（縦のつまみだけが出る）。
       ここは会話を読む場所であって、入力欄ではない —— 潰れた見た目にしない */
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
  .line.system {
    color: var(--text-tertiary);
    font-style: italic;
    text-align: center;
  }
  /* 会議中はチャットを広く取る。**下に小さく置くと気づかれない** */
  .chat.live .talk {
    max-height: 420px;
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
  /* **同じ席の AI。**人の発言と一目で見分けが付く必要がある
     （見分けが付かないと、人が言っていないことを人が言ったと読まれる） */
  .line.agent b {
    color: var(--text-secondary);
    font-weight: 600;
  }
  /* **1 行から始めて、打った分だけ伸びる。**伸びすぎない（会話が見えなくなる） */
  .say textarea {
    flex: 1;
    min-width: 0;
    min-height: 34px;
    max-height: 120px;
    resize: none;
    font-family: var(--font-ui);
    font-size: var(--text-sm-size);
    line-height: var(--text-sm-line);
    color: var(--text-primary);
    background: var(--bg-app);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 6px var(--space-2);
  }
  .say {
    display: flex;
    gap: var(--space-2);
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
