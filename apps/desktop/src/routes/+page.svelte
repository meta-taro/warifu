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
  import { describeMediaFailure, nextAttempt, sendModeFor, type SendMode } from '$lib/webrtc/media';
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
    EVENT_CLOSED,
    EVENT_INTRODUCED,
    EVENT_JOINED,
    EVENT_LEFT,
    EVENT_SIGNAL,
    EVENT_TEXT,
    connect,
    hostMeeting,
    inTauri,
    invite,
    leave,
    listen,
    log,
    myKey,
    onEvent,
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
  const KEY_TTL_SECS = 600;

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
  /** 会議の中の文字。**残らない** — 閉じれば消える（保存には D2 の決着が要る）。 */
  let 会話 = $state<Array<{ who: string; body: string; mine: boolean }>>([]);
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
    localStream?.getTracks().forEach((tr) => tr.stop());
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
        if (previewVideo) previewVideo.srcObject = localStream;
        devices = toOptions(await navigator.mediaDevices.enumerateDevices());
        return;
      } catch (e) {
        最後の失敗 = t(CAMERA_MESSAGE[describeMediaFailure(e)]);
        試す = nextAttempt(試す);
      }
    }
    // **何も取れなくても入れる。**ただし理由は伏せない
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
      members = [{ name: `${短く(me)}（${t('tile.me')}）`, host: true, path: 'unknown' }];
    })();
  });

  $effect(() => {
    const unsubs: Array<() => void> = [];
    void (async () => {
      unsubs.push(
        await onEvent<string>(EVENT_JOINED, async (key) => {
          log(`入った人がいる（${短く(key)}）。通話を作る`);
          members = [...members, { name: 短く(key), path: 'unknown' }];
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
                members = members.map((m) => (m.name === 短く(key) ? { ...m, path: p } : m));
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
          会話 = [...会話, { who: 短く(key), body, mine: false }];
        }),
      );
      unsubs.push(
        await onEvent<string>(EVENT_LEFT, (key) => 片付ける(key)),
      );
      unsubs.push(
        await onEvent<SignalPayload>(EVENT_SIGNAL, (p) => {
          // **誰から来たかで振り分ける。**間違えると別の組の経路が壊れる
          const 宛先 = p.from ? calls.get(p.from) : undefined;
          log(`下ごしらえが来た: ${p.step}（${p.from ? 短く(p.from) : '差出人なし'}）${宛先 ? '' : ' ← **通話が無い**'}`);
          void 宛先?.receive(p);
        }),
      );
      unsubs.push(
        await onEvent<string>(EVENT_CLOSED, (key) => {
          notice = t('link.closed');
          片付ける(key);
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
  // 変わったときだけ書く —— 毎秒書くと、肝心の行が埋もれる
  $effect(() => {
    const 状態 = [
      `名簿 ${members.length}/${DEFAULT_CAPACITY}`,
      `相手 ${remotes.length} 人`,
      `送るもの ${sendMode}`,
      `会議キー ${meetingKey ? 'あり' : 'なし'}`,
      `経路 ${remotes.map((r) => r.path).join(',') || 'なし'}`,
      notice ? `知らせ「${notice}」` : '知らせなし',
    ].join(' / ');
    log(`いまの画面: ${状態}`);
  });

  /** 1 人ぶんの表示を差し替える。 */
  function 相手を更新(key: string, patch: { stream?: MediaStream; path?: LinkPath }) {
    remotes = remotes.map((r) => (r.key === key ? { ...r, ...patch } : r));
  }

  /** 抜けた相手を片付ける。**ほかの相手との経路には触らない**（M6）。 */
  function 片付ける(key: string) {
    calls.get(key)?.close();
    calls.delete(key);
    remotes = remotes.filter((r) => r.key !== key);
    members = members.filter((m) => m.name !== 短く(key));
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
      // **自分の言ったことも並べる。**送った側に何も残らないと、言ったか分からない
      会話 = [...会話, { who: t('tile.me'), body, mine: true }];
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
  status={format(t('roster.capacity'), { current: members.length, capacity: DEFAULT_CAPACITY })}
/>

<main>
  <section class="stage">
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
          <span class="cap">{短く(r.key)} <LinkBadge {locale} path={r.path} /></span>
        </div>
      {/each}
    </div>
    {#if notice}
      <p class="notice">{notice}</p>
    {/if}
  </section>

  <aside>
    <div class="card">
      <h2><Icon name="camera" size={18} />{t('setup.title')}</h2>
      <p class="hint">{t('setup.hint')}</p>
      <button type="button" onclick={支度する}>
        <Icon name="camera" />{t('setup.action')}
      </button>

      <label class="row">
        <input type="checkbox" bind:checked={prefs.micOn} onchange={適用する} />
        <Icon name={prefs.micOn ? 'mic' : 'mic-off'} />{t('setup.mic')}
      </label>
      <label class="row">
        <input type="checkbox" bind:checked={prefs.cameraOn} onchange={適用する} />
        <Icon name={prefs.cameraOn ? 'camera' : 'camera-off'} />{t('setup.camera')}
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
      {#if meetingKey}
        <div class="field">
          <div class="field-head">
            <span class="with-icon"><Icon name="key" />{t('meeting.key.label')}</span>
            <button type="button" class="quiet" onclick={写す}>
              <Icon name={copied ? 'check' : 'copy'} />
              {copied ? t('meeting.key.copied') : t('meeting.key.copy')}
            </button>
          </div>
          <!-- 触れた時点で全部選ぶ。**手で端から端まで引かせない** -->
          <textarea
            bind:this={keyField}
            readonly
            rows="4"
            value={meetingKey}
            onfocus={(e) => e.currentTarget.select()}
          ></textarea>
        </div>
      {/if}
    </div>

    <div class="card">
      <h2><Icon name="enter" size={18} />{t('meeting.join.title')}</h2>
      <p class="hint">{t('meeting.join.hint')}</p>
      <textarea bind:value={received} rows="4" placeholder="WARIFU1-…#…"></textarea>
      <button type="button" onclick={入室する} disabled={入室中 || !received.trim()}>
        <Icon name="enter" />{入室中 ? t('meeting.join.working') : t('meeting.join.action')}
      </button>
    </div>

    <Roster {locale} {members} capacity={DEFAULT_CAPACITY} />

    <div class="card">
      <h2><Icon name="people" size={18} />{t('chat.title')}</h2>
      <p class="hint">{t('chat.hint')}</p>
      <div class="talk">
        {#if 会話.length === 0}
          <p class="hint">{t('chat.empty')}</p>
        {/if}
        {#each 会話 as line, i (i)}
          <p class="line" class:mine={line.mine}><b>{line.who}</b>{line.body}</p>
        {/each}
      </div>
      <div class="say">
        <input
          type="text"
          bind:value={下書き}
          placeholder={t('chat.placeholder')}
          onkeydown={(e) => e.key === 'Enter' && 話す()}
        />
        <button type="button" onclick={話す} disabled={!下書き.trim()}>{t('chat.send')}</button>
      </div>
    </div>
  </aside>
</main>

<style>
  main {
    flex: 1;
    display: grid;
    grid-template-columns: 1fr 340px;
    gap: var(--space-4);
    padding: var(--space-4);
    align-items: start;
    /* **ここだけが動く。**min-height: 0 が無いと grid の子が縮まず、
       窓ごとはみ出して帯まで一緒に動く */
    min-height: 0;
    overflow-y: auto;
  }
  .stage {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }
  /* 人数で列を変える。**1 対 1 は大きく、増えたら小さく**。
     auto-fit だけに任せると、3 人のときに 1 人だけ次の行で大きく残る */
  .tiles {
    display: grid;
    gap: var(--space-3);
    grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
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
  }
  video {
    width: 100%;
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
  .talk {
    display: flex;
    flex-direction: column;
    gap: 4px;
    /* **溢れたら中で動く。**外側（画面全体）を伸ばさない */
    max-height: 220px;
    overflow-y: auto;
    padding: var(--space-2);
    background: var(--bg-app);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
  }
  .line {
    margin: 0;
    font-size: var(--text-sm-size);
    line-height: var(--text-sm-line);
    word-break: break-word;
  }
  .line b {
    margin-right: 6px;
    font-weight: 500;
    color: var(--text-tertiary);
  }
  .line.mine b {
    color: var(--accent);
  }
  .say {
    display: flex;
    gap: var(--space-2);
  }
  .say input {
    flex: 1;
    min-width: 0;
    font: inherit;
    font-size: var(--text-sm-size);
    color: var(--text-primary);
    background: var(--bg-app);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 5px var(--space-2);
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
  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: var(--text-xs-size);
    color: var(--text-secondary);
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
