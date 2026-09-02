<script lang="ts">
  // 会議の画面（M5-c2）。**判断はここに置かない** — 規則は lib 側の純ロジックが持つ。
  import TitleBar from '$lib/window/TitleBar.svelte';
  import Roster, { type Member } from '$lib/meeting/Roster.svelte';
  import LinkBadge from '$lib/link/LinkBadge.svelte';
  import { MESSAGES, format } from '$lib/i18n/messages';
  import { resolveLocale, type Locale } from '$lib/i18n/locales';
  import { DEFAULT_CAPACITY } from '$lib/meeting/roster';
  import type { LinkPath } from '$lib/link/path';
  import type { MediaFailure } from '$lib/webrtc/media';
  import { Call } from '$lib/webrtc/session';
  import {
    EVENT_CLOSED,
    EVENT_JOINED,
    EVENT_LEFT,
    EVENT_SIGNAL,
    connect,
    hostMeeting,
    inTauri,
    listen,
    myAddress,
    myKey,
    onEvent,
    shouldOfferTo,
    type SignalPayload,
  } from '$lib/bridge';

  const locale: Locale = resolveLocale(
    typeof navigator === 'undefined' ? [] : [...navigator.languages],
  );
  const t = MESSAGES[locale];

  let address = $state('');
  let peerAddress = $state('');
  let path = $state<LinkPath>('unknown');
  let members = $state<Member[]>([]);
  let notice = $state('');
  let call: Call | null = null;
  let localVideo: HTMLVideoElement | undefined = $state();
  let remoteVideo: HTMLVideoElement | undefined = $state();

  const 断られた: Record<MediaFailure, string> = {
    'camera-denied': 'カメラとマイクの使用が許可されていません。OS の設定で許可してください。',
    'camera-missing': 'カメラかマイクが見つかりません。',
    'camera-busy': '他のアプリがカメラを使っています。',
    'camera-unknown': 'カメラを使えませんでした。',
  };

  $effect(() => {
    if (!inTauri()) {
      notice = 'ブラウザで開いています。経路は Tauri の中でしか動きません。';
      return;
    }
    void (async () => {
      address = (await myAddress()) ?? '';
      await hostMeeting(DEFAULT_CAPACITY);
      await listen();
      const me = (await myKey()) ?? '';
      members = [{ name: 短く(me) + '（自分）', host: true, path: 'unknown' }];
    })();
  });

  $effect(() => {
    const unsubs: Array<() => void> = [];
    void (async () => {
      unsubs.push(
        await onEvent<string>(EVENT_JOINED, async (key) => {
          members = [...members, { name: 短く(key), path: 'unknown' }];
          const offering = (await shouldOfferTo(key)) ?? false;
          call = new Call(offering, {
            onLocalStream: (s) => localVideo && (localVideo.srcObject = s),
            onRemoteStream: (s) => remoteVideo && (remoteVideo.srcObject = s),
            onPath: (p) => (path = p),
            onMediaFailure: (r) => (notice = 断られた[r]),
          });
          await call.begin();
        }),
      );
      unsubs.push(
        await onEvent<string>(EVENT_LEFT, (key) => {
          members = members.filter((m) => m.name !== 短く(key));
        }),
      );
      unsubs.push(
        await onEvent<SignalPayload>(EVENT_SIGNAL, (payload) => {
          void call?.receive(payload);
        }),
      );
      unsubs.push(
        await onEvent<void>(EVENT_CLOSED, () => {
          notice = '経路が閉じました。';
          path = 'unknown';
          call?.close();
          call = null;
        }),
      );
    })();
    return () => unsubs.forEach((un) => un());
  });

  /** 鍵は長い。**先頭だけ出して、全桁は選べる所に置く**（§5 は等幅を求めている） */
  const 短く = (key: string) => (key.length > 12 ? `${key.slice(0, 12)}…` : key);

  async function つなぐ() {
    notice = '';
    try {
      await connect(peerAddress.trim());
    } catch (e) {
      notice = e instanceof Error ? e.message : String(e);
    }
  }
</script>

<TitleBar
  {locale}
  status={format(t['roster.capacity'], { current: members.length, capacity: DEFAULT_CAPACITY })}
/>

<main>
  <section class="stage">
    <div class="tiles">
      <div class="tile">
        <!-- 自分の映像は音を出さない（回り込む） -->
        <video bind:this={localVideo} autoplay playsinline muted></video>
        <span class="cap">自分</span>
      </div>
      <div class="tile">
        <video bind:this={remoteVideo} autoplay playsinline></video>
        <span class="cap">相手 <LinkBadge {locale} {path} /></span>
      </div>
    </div>
    {#if notice}
      <p class="notice">{notice}</p>
    {/if}
  </section>

  <aside>
    <div class="card">
      <h2>自分の宛先</h2>
      <p class="hint">これを相手へ渡す。紙でも口頭でも成立する。</p>
      <textarea readonly rows="3" value={address}></textarea>
    </div>

    <div class="card">
      <h2>相手の宛先</h2>
      <p class="hint">受け取った側が繋ぐ。渡した側は待つだけでよい。</p>
      <textarea bind:value={peerAddress} rows="3" placeholder="WARIFU1-…"></textarea>
      <button type="button" onclick={つなぐ} disabled={!peerAddress.trim()}>繋ぐ</button>
    </div>

    <Roster {locale} {members} capacity={DEFAULT_CAPACITY} />
  </aside>
</main>

<style>
  main {
    flex: 1;
    display: grid;
    grid-template-columns: 1fr 320px;
    gap: var(--space-4);
    padding: var(--space-4);
    align-items: start;
    min-height: 0;
  }
  .stage {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }
  .tiles {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
    gap: var(--space-3);
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
    margin: 0;
    font-size: var(--text-sm-size);
    line-height: var(--text-sm-line);
    font-weight: 600;
  }
  .hint {
    margin: 0;
    font-size: var(--text-xs-size);
    line-height: var(--text-xs-line);
    color: var(--text-tertiary);
  }
  textarea {
    width: 100%;
    resize: vertical;
    /* 宛先は 1 文字の違いが意味を変える（DESIGN.md §5） */
    font-family: var(--font-mono);
    font-size: var(--text-xs-size);
    line-height: var(--text-xs-line);
    color: var(--text-primary);
    background: var(--bg-app);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: var(--space-2);
  }
  textarea:focus-visible,
  button:focus-visible {
    outline: 3px solid var(--accent-subtle);
    outline-offset: 1px;
  }
  button {
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
