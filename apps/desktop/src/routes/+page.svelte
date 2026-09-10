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
  import CropDialog from '$lib/contacts/CropDialog.svelte';
  import { 机の印, 部屋のid, type 行 as 連絡帳の行 } from '$lib/contacts/list';
  import type { 口の種類 } from '$lib/contacts/actions';
  import ChatPanel from '$lib/chat/ChatPanel.svelte';
  import { 届く先を並べる, 宛先を決める } from '$lib/chat/reach';
  import { 席の名札 } from '$lib/contacts/seat';
  import { getVersion } from '@tauri-apps/api/app';
  import { 入れ替える, 確かめる, 立て直す } from '$lib/update/check';
  import { 進み具合, type 新しい版 } from '$lib/update/notice';
  import { 当てる色, 覚える鍵, 読み取る, type テーマ } from '$lib/window/theme';
  import {
    その部屋の会話,
    人の部屋,
    机と部屋へ足す,
    席へ足す,
    机の部屋,
    見る部屋,
    足す as 会話に足す,
    type 部屋の会話,
  } from '$lib/chat/rooms';
  import { どう送るか, 留守中の行 } from '$lib/chat/postbox';
  import { 呼び名 } from '$lib/meeting/names';
  import { 渡してあるか, 足す as 鍵を足す, type 出した鍵 } from '$lib/meeting/handout';
  import { 呼ぶ名 } from '$lib/contacts/claimed';
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
    EVENT_THEME,
    EVENT_LINK,
    EVENT_CHECK_UPDATE,
    roomLink,
    roomQr,
    EVENT_PROFILES,
    EVENT_CLAIMED,
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
    profiles,
    setProfile,
    rememberNote,
    setAvatarBytes,
    readImage,
    clearAvatar,
    avatarBytes,
    type ProfileRow,
    fetchPostbox,
    setMenuLocale,
    shouldOfferTo,
    type SignalPayload,
  } from '$lib/bridge';

  const locale: Locale = resolveLocale(
    typeof navigator === 'undefined' ? [] : [...navigator.languages],
  );
  const t = (key: MessageKey) => MESSAGES[locale][key];

  /** 出した鍵 1 本ぶん（**D84**）。**渡す口ごと**に持つ。 */

  /**
   * **出した鍵ぜんぶ**（**D84**・オーナー指摘 2026-09-10
   * 「鍵が一人 1 本なら、10 人呼ぶ時どうするんですか？」）。
   *
   * **1 本ずつ上書きしていた。**前の鍵は生きているのに画面から消えるので、
   * **写す前にもう 1 本出すと、前の鍵は二度と取り出せなかった。**
   * 出した本数ぶん並べて持つ。
   */
  let 鍵たち = $state<出した鍵[]>([]);
  /** **何人ぶん出すか。**CLI の `--keys` と同じことを画面でもできるようにする。 */
  let 何人ぶん = $state(1);
  /** どの鍵の何を写したか（`0:鍵` / `0:リンク`）。押した手応えを出すため。 */
  let 写した印 = $state('');

  /**
   * **いちばん新しい鍵。**画面の状態（会議前／待っている）の判定はこれを見る。
   *
   * 1 本でも出していれば「待っている」である。
   */
  const meetingKey = $derived(鍵たち.at(-1)?.鍵 ?? '');
  /** いま動いている版。**「最新です」と言うときに添える。** */
  let 版 = $state('');

  /**
   * **見つかった更新**（**D81**）。無ければ `null`。
   *
   * **黙って入れ替えない。**何が変わるかを見せてから、人が押す。
   */
  let 更新 = $state<新しい版 | null>(null);
  /** 更新の口の様子（確認中・落とし中・済んだ）。 */
  let 更新の様子 = $state<'休み' | '確認中' | '落とし中' | '済んだ'>('休み');
  /** 落とした割合。**分からなければ `null`**（総量を教えてこない置き場所がある）。 */
  let 落とし割合 = $state<number | null>(null);

  /**
   * **受け取ったリンクの鍵**（**D79**）。**入るかどうかはまだ決まっていない。**
   *
   * 届いた URL は他人が作れる。**押しただけで部屋へ入る作りにしない。**
   */
  let 誘われた鍵 = $state('');
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
  /**
   * いま選んでいるテーマ（**OS に合わせる / ライト / ダーク**）。
   *
   * **描く前に当てるのは `app.html` の頭のスクリプト。**ここが持つのは
   * 「メニューで選び直したあと」だけである。
   */
  let テーマの選び = $state<テーマ>('auto');

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
   * この端末のプロフィール（**人と、マイ PC エージェント**）。
   *
   * **書き換えられるのはこの端末の持ち主だけ**（`profile.rs`）。
   */
  let 名乗りたち = $state<ProfileRow[]>([]);
  /** 差し替えた顔（`who` → 画面に出せる URL）。**閉じれば消える。** */
  let 顔の画たち = $state<Record<string, string>>({});
  /**
   * 部屋に付けた名前（部屋 id → 名前）。**閉じれば消える。**
   *
   * 部屋 id はその場限りのものなので、置き場所へ書くと
   * **使い終わった名前が溜まっていく。**
   */
  let 部屋の名前たち = $state<Record<string, string>>({});
  /**
   * **一度でも人が部屋に入ったか。**
   *
   * **チャットの行数で数えない**（2026-09-09 に実物で踏んだ）——
   * 机のエージェントと 1 行話しただけで「待っている」になり、
   * **`部屋をつくる` が消えて、以後だれも呼べなくなっていた。**
   */
  let 人が入った = $state(false);
  /**
   * **相手が名乗ったもの**（**D75**）。公開鍵 → 名前と紹介。
   *
   * **本人確認ではない。**こちらが付けた呼び名があれば、そちらが勝つ（D46）。
   * 閉じれば消える（覚えるのは呼び名だけ —— 名乗りは相手の都合で変わる）。
   */
  let 相手の名乗り = $state<Record<string, { 名前: string; 紹介: string }>>({});

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

  /**
   * 席の名乗り（`zumen のエージェント`）→ その席が書いた名前。
   *
   * **識別は名乗りのまま、表示だけ名札にする**（**D77**）——
   * 名札で識別すると、名前を書き換えた瞬間に宛先が外れる。
   */
  const 席の名前たち = $derived(
    Object.fromEntries(
      名乗りたち
        .filter((p) => p.who.startsWith(机の印) && p.name)
        .map((p) => [p.who.slice(机の印.length), p.name]),
    ),
  );

  const 届く先 = $derived(
    送り方.種類 === '預ける'
      ? // **預ける先は 1 人。**1 対 1 がその場で分かる
        [呼び名(名簿, 送り方.key)]
      : 届く先を並べる({
          会議の相手: remotes.map((r) => 呼び名(名簿, r.key)),
          机のAIたち,
          宛先,
        // **会話に出る名前と揃える。**連絡帳では「かくにん係」、
        // 届く先では「kakunin のエージェント」だと、**2 人に見える**（D77）
        }).map((名) => 席の名札(名, 席の名前たち[名])),
  );

  /** 打ったものが誰かに届くか。**会議の人でも、同じ席のエージェント でもよい。** */
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
    // **立ち上げていない席も並べる。**エージェントはそれぞれが 1 人であり、
    // 消えると、その 1 人ぶんの名乗りが編集できなくなる（2026-09-08）
    名乗りのある席: 名乗りたち
      .filter((p) => p.who.startsWith(机の印))
      .map((p) => p.who.slice(机の印.length)),
    部屋たち,
    部屋の名前: 部屋の名前たち,
    会議の相手: remotes.map((r) => r.key),
    覚えた,
    留守中に届いた,
  });
  /**
   * 画面に出す相手の名前。
   *
   * **こちらが付けた呼び名が最優先**（**D46**）。無ければ**本人の名乗り**、
   * それも無ければ鍵の頭。**会話の行も、連絡帳と同じ呼び方にする** ——
   * 揃っていないと、同じ人が 2 つの名前で出る（2026-09-09 に実物で見た）。
   */
  /**
   * 名簿（画面に出す名前）。**本人の名乗りも混ぜたもの。**
   *
   * **呼ぶ名の決め方を 2 か所に置かない** —— 置くと、同じ人が
   * 会話と名簿で別の名前で出る（2026-09-09 に実物で見た）。
   */
  const 画面での名簿 = $derived(
    Object.fromEntries(
      members
        .map((m) => m.key)
        .concat(Object.keys(相手の名乗り))
        .map((key) => [key, 画面での名(key)]),
    ),
  );

  function 画面での名(key: string): string {
    const 既定 = 呼び名(名簿, key);
    // `呼び名` は覚えていなければ鍵の頭（末尾が `…`）を返す
    const 呼んでいる名 = 既定.endsWith('…') ? undefined : 既定;
    return 呼ぶ名(呼んでいる名, 相手の名乗り[key], 既定);
  }

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
    画面の状態を決める({ 相手: remotes.length, 会議キー: !!meetingKey, 人が入った }),
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
        // **メニューには、いま選んでいるテーマの印も付ける。**
        // 覚えているのは画面側なので、こちらから渡す
        await setMenuLocale(locale, テーマの選び);
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
      名乗りたち = (await profiles()) ?? [];
      await 顔を読み直す();
      // **覚えているテーマを、画面の状態にも持つ**（当てるのは app.html が済ませている）
      テーマの選び = 読み取る(localStorage.getItem(覚える鍵));
      // **置いてある預かり所を、画面にも出す。**
      // 留守中の分は Rust 側が起動時に取りに行く（`postbox.rs`）ので、
      // ここで取りに行くと二重になる
      預かり所 = (await postbox()) ?? null;
      預かり所の下書き = 預かり所 ?? '';
      // **構えてから取りに行く。**渡されたものを落とさない
      await 留守中の分を取りに行く();
      // **いま動いている版**（画面の帯にも出している）
      版 = await getVersion().catch(() => '');
      // **起動時に一度だけ、黙って確かめる**（**D81**）。
      // 無かったことは言わない —— 毎回「最新です」と出るのはうるさい。
      // **繋がらなくても起動を止めない**
      void 更新を確かめる(true);
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
          // **人が入った印。**ここから先は会議前へ戻さない
          人が入った = true;
          // **見ていない間に誰が来たかを残す。**名簿は動くが、目を離すと分からない。
          // **机と部屋の両方へ**（`issues/1`）—— 入退室も、どちらの入口でも読める
          部屋の会話たち = 机と部屋へ足す(部屋の会話たち, いまの部屋, {
            ...入退室の知らせ('入室', 画面での名(key), (k, v) => format(t(`chat.${k}`), v)),
            at: いま時刻(),
          });
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
          // **机と部屋の両方へ積む**（`issues/1`）。
          // 片方にしか積まないと、もう片方を見ている人には何も見えない
          部屋の会話たち = 机と部屋へ足す(部屋の会話たち, いまの部屋, {
            who: 画面での名(key),
            body,
            mine: false,
            at: いま時刻(),
          });
        }),
      );
      unsubs.push(
        // **同じ席のエージェント が言ったこと。**人の発言と見分けが付く形で出す
        // **どこで動いているエージェントかを、机が刻んで渡してくる。**
        // 1 台の PC で複数のエージェントが同じ机に着くので、
        // 「マイ PC エージェント」だけでは、どれが喋ったのか分からない（2026-09-08）
        await onEvent<[string, string, string]>(EVENT_DESK, ([呼び方, body, at]) => {
          log(話の記録('送信', 呼び方, body));
          // **机と部屋の両方へ積む**（`issues/1`「部屋の画面にエージェントの
          // 発言が出ない」）。**人が置き去りになったことに、人が気づけない**
          const その行 = { who: 呼び方, body, mine: false, agent: true as const, at };
          部屋の会話たち = 机と部屋へ足す(部屋の会話たち, いまの部屋, その行);
          // **その席との 1 対 1 にも積む**（**D86**）。
          // **席の名札で来る**ので、席そのもの（`zumen のエージェント`）に戻す
          部屋の会話たち = 席へ足す(部屋の会話たち, 席に戻す(呼び方), その行);
        }),
      );
      unsubs.push(
        await onEvent<string[]>(EVENT_DESK_SEATS, (顔ぶれ) => {
          机のAIたち = 顔ぶれ;
        }),
      );
      unsubs.push(
        // **メニューでテーマを選んだ。**当てて、覚えて、印を付け直す
        await onEvent<string>(EVENT_THEME, (選び) => void テーマを選ぶ(読み取る(選び))),
      );
      unsubs.push(
        // **メニューから「更新を確認」を押した**（**D81**）。
        // **無かったことも言う** —— 押して何も起きないと、押せたのか分からない
        await onEvent<void>(EVENT_CHECK_UPDATE, () => void 更新を確かめる()),
      );
      unsubs.push(
        // **`warifu://join/…` を押された**（**D79**）。
        // **ここでは入らない。**届いた URL は他人が作れるので、人に尋ねる
        await onEvent<string>(EVENT_LINK, (鍵) => {
          誘われた鍵 = 鍵;
          log(`リンクで誘われました（${鍵.length} 文字）`);
        }),
      );
      unsubs.push(
        // **机に着いたエージェントが、自分で名乗った。**画面にもすぐ出す
        await onEvent<void>(EVENT_PROFILES, () => {
          void profiles().then(async (面々) => {
            名乗りたち = 面々 ?? [];
            await 顔を読み直す();
          });
        }),
      );
      unsubs.push(
        // **相手が名乗った。**覚えはしない（呼び名だけを覚える・D46）
        await onEvent<[string, string, string]>(EVENT_CLAIMED, ([key, 名前, 紹介]) => {
          相手の名乗り = { ...相手の名乗り, [key]: { 名前, 紹介 } };
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
    部屋の会話たち = 机と部屋へ足す(部屋の会話たち, いまの部屋, {
      ...入退室の知らせ(種類, 画面での名(key), (k, v) => format(t(`chat.${k}`), v)),
      at: いま時刻(),
    });
    音を出す(退室の音);
  }

  /**
   * **名札から席そのものへ戻す**（**D86**）。
   *
   * 机は `名前（名乗り）` の形で渡してくる（`profile::席の名札`・D77）。
   * 席ごとの会話は**席そのもの**（`zumen のエージェント`）で引いているので、戻す。
   *
   * **名乗りは括弧の中にある。**名前を書いていない席は、名札が席そのままである。
   */
  function 席に戻す(名札: string): string {
    const m = /（([^（）]+)）$/.exec(名札);
    return m ? `${m[1]} のエージェント` : 名札;
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

  /**
   * **人数ぶんの鍵を出す**（**D84**）。
   *
   * オーナー指摘（2026-09-10）——
   * 「**鍵が一人 1 本なら、10 人呼ぶ時どうするんですか？
   * ひとりずつに鍵を主は配布するんですか？**」
   *
   * **はい、1 人に 1 本である**（D12）。だから**人数ぶん一度に出せる**ようにする。
   * CLI には `--keys` があったのに、画面には無かった（10 回押すしかなかった）。
   *
   * **1 本ずつ出すのも同じ道を通る**（`何本 = 1`）。
   */
  async function はじめる(何本 = 1) {
    notice = '';
    const 本数 = Math.max(1, Math.min(何本, 出せる本数));
    try {
      for (let i = 0; i < 本数; i += 1) {
        const 鍵 = (await invite(KEY_TTL_SECS)) ?? '';
        if (!鍵) continue;
        鍵たち = 鍵を足す(鍵たち, { 鍵, ...(await 渡す形にする(鍵)) });
      }
      // **鍵を出すと部屋ができる。**どの部屋の会話かを画面が知る必要がある
      await 部屋を読み直す();
    } catch (e) {
      notice = 読める(e);
    }
  }

  /**
   * 鍵から**渡せる形**を作る（**D79**）。
   *
   * オーナー指摘（2026-09-09）——
   * 「**どんな手順で相手（知り合いの人間）とつながれるかわかりません**」
   * 「**たとえば QR とか URL スキーマで相手におくれるとか**」
   *
   * **鍵を貼り付けさせない。**リンク 1 本か、QR を見せるだけにする。
   */
  async function 渡す形にする(鍵: string): Promise<{ リンク: string; qr: string }> {
    try {
      return {
        リンク: (await roomLink(鍵)) ?? '',
        qr: (await roomQr(鍵)) ?? '',
      };
    } catch (e) {
      // **黙って消さない。**QR が作れなくても、鍵そのものは渡せる
      log(`リンクを作れませんでした（${読める(e)}）`);
      return { リンク: '', qr: '' };
    }
  }

  /** 写す。**押した手応えを、その 1 本の所に出す。** */
  async function 写し取る(印: string, 中身: string) {
    try {
      await navigator.clipboard.writeText(中身);
    } catch {
      notice = t('meeting.key.copy');
      return;
    }
    写した印 = 印;
    setTimeout(() => {
      if (写した印 === 印) 写した印 = '';
    }, COPIED_FOR_MS);
  }

  /**
   * **誘われたリンクで入る**（**D79**）。**人が押したときだけ。**
   */
  async function 誘いに乗る() {
    const 鍵 = 誘われた鍵;
    誘われた鍵 = '';
    if (!鍵) return;
    received = 鍵;
    await 入室する();
  }

  /**
   * **更新を確かめる**（**D81**）。
   *
   * **無かったことも言う。**「確認する」を押して何も起きないと、
   * 押せていないのか、更新が無いのか分からない。
   */
  async function 更新を確かめる(黙って = false) {
    if (更新の様子 === '確認中' || 更新の様子 === '落とし中') return;
    更新の様子 = '確認中';
    try {
      const 見つけた = await 確かめる();
      更新 = 見つけた;
      更新の様子 = '休み';
      // 起動直後の確認では、無いことをわざわざ言わない（うるさい）
      if (!見つけた && !黙って) {
        notice = format(t('update.none'), { version: 版 });
      }
    } catch (e) {
      更新の様子 = '休み';
      // **繋がらないことは、よくある。**起動を止めない・黙って捨てない
      log(`更新を確かめられませんでした（${読める(e)}）`);
      if (!黙って) notice = format(t('update.failed'), { why: 読める(e) });
    }
  }

  /** 落として入れ替える。**立て直すのは人が押したとき。** */
  async function 更新を入れる() {
    if (更新の様子 === '落とし中') return;
    更新の様子 = '落とし中';
    落とし割合 = null;
    try {
      await 入れ替える((状態) => {
        落とし割合 = 進み具合(状態.落とした, 状態.全部);
        if (状態.済んだ) 更新の様子 = '済んだ';
      });
      更新の様子 = '済んだ';
    } catch (e) {
      更新の様子 = '休み';
      notice = format(t('update.failed'), { why: 読める(e) });
    }
  }

  /** コピーできたことを見せる時間（ms）。押した手応えが無いと、人は二度押す。 */
  const COPIED_FOR_MS = 1600;

  /**
   * **出せる本数の上限。**定員から自分の 1 人を引いたぶん。
   *
   * **定員より多く出しても入れない。**出せる形にすると、
   * 「渡したのに入れない」が起きる（CLI の `--keys` と同じ上限）。
   */
  const 出せる本数 = DEFAULT_CAPACITY - 1;

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
      const 私の行 = {
        who: t('tile.me'),
        body,
        mine: true,
        at: いま時刻(),
      };
      if (送り方.種類 === '机') {
        // **その席にしか届いていない**（**D86**）。
        // 部屋にも机ぜんぶにも積まない —— **届いていない所に出すのは嘘である**
        部屋の会話たち = 席へ足す(部屋の会話たち, 送り方.宛先, 私の行);
      } else {
        // **机と部屋の両方へ積む**（`issues/1`）。自分の行だけが部屋に残り、
        // エージェントの行が机にしか無い、という食い違いをなくす
        部屋の会話たち = 机と部屋へ足す(部屋の会話たち, いまの部屋, 私の行);
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
    // **押せる札は 2 枚だけ**（`contacts/actions.ts`）。
    // チャットは行を選んだ時点で開いていて、予定はまだ作っていない
    if (種類 === 'chat' || 種類 === 'calendar') return;

    // **グループチャットは、鍵を出す所へ移るだけ。**
    // ここで勝手にルームを建てない —— 何人ぶん出すかは人が決める（D84）
    if (種類 === 'group') {
      const 次 = 押した後の面('group');
      if (次) 面 = 次;
      return;
    }

    // 机のエージェントは同じ席に居る。**繋ぎに行くものが無い**
    if (相手.key === 机の印) return;

    // すでに繋がっているなら、繋ぎ直さない
    if (!相手.いま会議に居る) {
      呼んでいる = 相手.key;
      notice = '';
      try {
        await callContact(相手.key);
        await 部屋を読み直す();
        await 名簿を読む();
      } catch (e) {
        notice = 読める(e);
        return;
      } finally {
        呼んでいる = null;
      }
    }
    const 次 = 押した後の面('call');
    if (次) 面 = 次;
  }

  /** いま渡そうとしている 1 本（幕に出す）。**渡し終わっても消さない。** */
  let 渡す一本 = $state<出した鍵 | null>(null);

  /**
   * **その人ぶんの鍵を 1 本出して、渡す形で見せる**（**D84** の本題）。
   *
   * オーナー ——「**手動だとミスります。いかにアプリ側で普段はよしなに
   * 裏側でそれをやるかどうかです**」。だから**出すときに宛先を書く。**
   * 人に「何本目を誰に渡したか」を数えさせない。
   */
  async function その人に鍵を渡す(相手: 連絡帳の行) {
    notice = '';
    try {
      const 鍵 = (await invite(KEY_TTL_SECS)) ?? '';
      if (!鍵) return;
      const 本: 出した鍵 = {
        鍵,
        ...(await 渡す形にする(鍵)),
        宛先: { key: 相手.key, 名: 画面での名(相手.key) || 相手.name },
      };
      鍵たち = 鍵を足す(鍵たち, 本);
      渡す一本 = 本;
      // **鍵を出すと部屋ができる。**どの部屋の会話かを画面が知る必要がある
      await 部屋を読み直す();
    } catch (e) {
      notice = 読める(e);
    }
  }

  /**
   * そのルームを抜ける。
   *
   * **抜ける口が画面に無かった**（窓を閉じるときだけ内部で抜けていた）——
   * オーナー・2026-09-10「たとえば、ルームぬけれるの？ みたいなところとかね」。
   *
   * `leave` は**いま見ている部屋**を抜けるので、先にそこへ移してから告げる。
   * 抜けたあとは**選択を外す** —— 無くなった行を選んだままにしない。
   */
  async function ルームを抜ける(id: string) {
    try {
      await 部屋へ移る(id);
      await leave();
      await 部屋を読み直す();
      await 名簿を読む();
      選んだ相手 = null;
    } catch (e) {
      notice = 読める(e);
    }
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
        留守中の行(画面での名(key), body, 秒),
      );
    }
    notice = format(t('postbox.received'), { n: String(届いた.length) });
  }

  /**
   * テーマを選び直す。
   *
   * **当てる・覚える・印を付け直す**の 3 つを 1 か所でやる。
   * 分けると、**メニューの印だけが前のまま残る**（どれが効いているか読めなくなる）。
   */
  async function テーマを選ぶ(選び: テーマ) {
    テーマの選び = 選び;
    const 暗いか = window.matchMedia('(prefers-color-scheme: dark)').matches;
    document.documentElement.dataset.theme = 当てる色(選び, 暗いか);
    try {
      localStorage.setItem(覚える鍵, 選び);
    } catch {
      // **覚えられなくても、いまの見た目は変える。**次の起動で OS に戻るだけ
    }
    try {
      await setMenuLocale(locale, 選び);
    } catch {
      // 印が付け直せなくても、色は変わっている。**止めない**
    }
  }

  /**
   * プロフィールを書く（**この端末の人と、マイ PC エージェント**）。
   *
   * **名乗りは本人確認にしない**（D46）。相手が付けた呼び名があれば、そちらが勝つ。
   */
  /**
   * **画像を落として、顔を差し替える。**
   *
   * 落とせる先は「いま選んでいる行」である ——
   * **選んでいなければ何もしない**（どこへ入るのか分からないものを受け取らない）。
   *
   * `@tauri-apps/api` の口を使う（**外の道具を足さない**）。
   */
  $effect(() => {
    if (!inTauri()) return;
    let 外す: (() => void) | undefined;
    void (async () => {
      const { getCurrentWebview } = await import('@tauri-apps/api/webview');
      外す = await getCurrentWebview().onDragDropEvent((e) => {
        if (e.payload.type === 'over') {
          落とせる = 顔を差し替えられる先 !== null;
          return;
        }
        落とせる = false;
        if (e.payload.type !== 'drop') return;
        const 先 = 顔を差し替えられる先;
        const 場所 = e.payload.paths[0];
        if (!先 || !場所) return;
        // **落とした瞬間に置かない**（**D87**）。
        // どこが顔になるかを見せてから、人が決める
        void 切り取りを開く(先, 場所);
      });
    })();
    return () => 外す?.();
  });

  /** いま顔を差し替えられる相手（`me` か `desk:◯◯`）。**選んでいなければ `null`。** */
  /** **Esc で渡す幕を閉じる。**ほかの幕と同じ振る舞いにする（DESIGN §10-A） */
  function 幕を閉じる(e: KeyboardEvent) {
    if (e.key === 'Escape') 渡す一本 = null;
  }

  const 顔を差し替えられる先 = $derived.by(() => {
    if (!選んだ相手) return null;
    if (選んだ相手 === 自分の鍵) return 'me';
    if (選んだ相手.startsWith(机の印) && 選んだ相手 !== 机の印) return 選んだ相手;
    return null;
  });
  /** いま落とせる所にドラッグしているか。 */
  let 落とせる = $state(false);

  /** いま切り取っている画像（**D87**）。**決めるまで置かない。** */
  let 切り取り中 = $state<{ 先: string; 画像: string } | null>(null);

  /**
   * 落とした画像を開いて、**切り取る画面を出す**（**D87**）。
   *
   * **Rust に読ませてから blob にする** —— webview から
   * 好きな場所を読ませない（置き場所は 0700 の中にある）。
   */
  async function 切り取りを開く(先: string, 場所: string) {
    notice = '';
    try {
      const 生 = await readImage(場所);
      const 塊 = new Blob([new Uint8Array(生 ?? [])]);
      切り取り中 = { 先, 画像: URL.createObjectURL(塊) };
    } catch (e) {
      notice = 読める(e);
    }
  }

  /** 切り取りを閉じる。**blob を手放す**（持ったままにすると溜まる）。 */
  function 切り取りを閉じる() {
    if (切り取り中) URL.revokeObjectURL(切り取り中.画像);
    切り取り中 = null;
  }

  /** 切り取った WebP を顔として置く（**D87**）。 */
  async function 切り取ったものを置く(bytes: Uint8Array) {
    const 先 = 切り取り中?.先;
    切り取りを閉じる();
    if (!先) return;
    notice = '';
    try {
      await setAvatarBytes(先, bytes);
      名乗りたち = (await profiles()) ?? [];
      await 顔を読み直す();
    } catch (e) {
      notice = 読める(e);
    }
  }

  /**
   * 顔を差し替える／既定へ戻す。
   *
   * **落としたファイルをそのまま指さない。**Rust 側が置き場所へ写してから指す ——
   * 指したままにすると、**消えた・入れ替わったファイル**を指すことになる。
   */
  async function 顔を差し替える(who: string, 場所: string | null) {
    notice = '';
    try {
      // **置くのは切り取ったあと**（**D87**）。ここへ来るのは「既定へ戻す」だけ
      if (場所) await 切り取りを開く(who, 場所);
      else await clearAvatar(who);
      名乗りたち = (await profiles()) ?? [];
      await 顔を読み直す();
    } catch (e) {
      notice = 読める(e);
    }
  }

  /**
   * 置いてある顔を読み直して、画面に出せる形にする。
   *
   * **画面から置き場所を辿らせない**（0700 の中にある）。Rust がバイト列で渡す。
   */
  async function 顔を読み直す() {
    const 新しい: Record<string, string> = {};
    for (const p of 名乗りたち) {
      if (!p.avatar) continue;
      const 中身 = await avatarBytes(p.who);
      if (!中身) continue;
      新しい[p.who] = URL.createObjectURL(new Blob([new Uint8Array(中身)], { type: 'image/png' }));
    }
    // **前の URL は手放す。**放っておくと、差し替えるたびに溜まる
    for (const url of Object.values(顔の画たち)) URL.revokeObjectURL(url);
    顔の画たち = 新しい;
  }

  /**
   * **こちらが書いた覚え書き**を残す（**相手の名乗りとは別**）。
   *
   * 「どの機械の、何をするエージェントか」を、人が自分の言葉で残す所である。
   */
  async function 覚え書きを書く(key: string, note: string) {
    notice = '';
    try {
      await rememberNote(key, note);
      await 名簿を読む();
    } catch (e) {
      notice = 読める(e);
    }
  }

  /**
   * **部屋に名前を付ける**（オーナー・2026-09-08「どの部屋？って人間はなります」）。
   *
   * **画面の中だけで持つ。**部屋 id はその場限りのものなので、
   * 置き場所へ書くと**使い終わった名前が溜まっていく**。
   */
  function 部屋に名前を付ける(id: string, 名前: string) {
    const 次 = { ...部屋の名前たち };
    if (名前.trim()) 次[id] = 名前.trim();
    else delete 次[id];
    部屋の名前たち = 次;
  }

  async function 名乗りを書く(who: string, name: string, bio: string) {
    notice = '';
    try {
      await setProfile(who, name, bio);
      名乗りたち = (await profiles()) ?? [];
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
<!--
  **更新があった**（**D81**・オーナー指示 2026-09-10
  「自動アップデートと、アップデート内容確認できるやつ」）。

  **黙って入れ替えない。**何が変わるかを見せてから、人が押す。
  更新は**アプリを差し替える**操作なので、押した人が中身を知っていること。
-->
{#if 更新}
  <div class="updated" role="status">
    <p class="what">{format(t('update.available'), { version: 更新.版 })}</p>
    <!-- **何が変わったか。**これが「確認できるやつ」の中身である -->
    <details open>
      <summary>{t('update.notes')}</summary>
      <pre class="notes">{更新.中身 || t('update.notes.none')}</pre>
    </details>
    {#if 更新の様子 === '落とし中'}
      <p class="hint">
        {format(t('update.downloading'), { percent: 落とし割合 ?? '…' })}
      </p>
    {:else if 更新の様子 === '済んだ'}
      <p class="hint">{t('update.installed')}</p>
    {/if}
    <div class="tail">
      {#if 更新の様子 === '済んだ'}
        <button type="button" onclick={() => void 立て直す()}>{t('update.apply')}</button>
      {:else}
        <button
          type="button"
          disabled={更新の様子 === '落とし中'}
          onclick={() => void 更新を入れる()}
        >
          {更新の様子 === '落とし中' ? t('update.checking') : t('update.apply')}
        </button>
      {/if}
      <button type="button" class="quiet" onclick={() => (更新 = null)}>
        {t('update.later')}
      </button>
    </div>
  </div>
{/if}

<!--
  **リンクで誘われた**（**D79**）。**押しただけでは入らない。**
  届いた URL は他人が作れる ——「開いたら実行」を作らないための確認である。
-->
<!-- **落とした画像を切り取る**（**D87**）。決めるまで置かない -->
{#if 切り取り中}
  <CropDialog
    {locale}
    画像={切り取り中.画像}
    決めた={(bytes) => void 切り取ったものを置く(bytes)}
    やめた={切り取りを閉じる}
  />
{/if}

{#if 誘われた鍵}
  <div class="invited" role="alertdialog" aria-live="polite">
    <p class="what">{t('link.invited')}</p>
    <p class="hint">{t('link.invited.hint')}</p>
    <div class="tail">
      <button type="button" onclick={() => void 誘いに乗る()}>{t('link.invited.enter')}</button>
      <button type="button" class="quiet" onclick={() => (誘われた鍵 = '')}>
        {t('link.invited.no')}
      </button>
    </div>
  </div>
{/if}

{#if notice}
  <p class="notice top">{notice}</p>
{/if}

<svelte:window onkeydown={幕を閉じる} />

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
      names={画面での名簿}
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
      <!--
        **最初から人数ぶん出せる**（**D84**）。1 人なら 1 本で、
        10 人なら 10 本 —— **1 本につき 1 人**なので、そこは足せない
      -->
      <div class="issue">
        <label>
          {t('meeting.key.howmany')}
          <input type="number" min="1" max={出せる本数} bind:value={何人ぶん} />
        </label>
        <button type="button" onclick={() => void はじめる(何人ぶん)}>
          <Icon name="people" />{t('meeting.start.action')}
        </button>
      </div>
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
    <!--
      **出した鍵を、出した本数ぶん並べる**（**D84**・オーナー指摘 2026-09-10
      「鍵が一人 1 本なら、10 人呼ぶ時どうするんですか？」）。

      **1 本ずつ上書きしていた。**前の鍵は生きているのに画面から消えるので、
      **写す前にもう 1 本出すと、前の鍵は二度と取り出せなかった。**
    -->
    {#if 鍵たち.length > 0}
      <div class="card key">
        <div class="field-head">
          <span class="with-icon"><Icon name="key" />{t('meeting.key.label')}</span>
          <span class="hint">{format(t('meeting.key.count'), { n: 鍵たち.length })}</span>
        </div>
        <p class="hint">{t('meeting.key.each')}</p>

        {#each 鍵たち as 本, i (本.鍵)}
          <div class="one-key">
            <div class="field-head">
              <span class="nth">
                {format(t('meeting.key.nth'), { n: i + 1 })}
                <!-- **誰に渡したかを、その 1 本のそばに出す**（人に数えさせない・D84） -->
                {#if 本.宛先}
                  <span class="for">{format(t('key.hand.for'), { name: 本.宛先.名 })}</span>
                {/if}
              </span>
              <span class="tail">
                {#if 本.リンク}
                  <button
                    type="button"
                    class="quiet"
                    onclick={() => void 写し取る(`${i}:リンク`, 本.リンク)}
                  >
                    <Icon name={写した印 === `${i}:リンク` ? 'check' : 'link'} />
                    {写した印 === `${i}:リンク` ? t('meeting.key.copied') : t('meeting.link.copy')}
                  </button>
                {/if}
                <button
                  type="button"
                  class="quiet"
                  onclick={() => void 写し取る(`${i}:鍵`, 本.鍵)}
                >
                  <Icon name={写した印 === `${i}:鍵` ? 'check' : 'copy'} />
                  {写した印 === `${i}:鍵` ? t('meeting.key.copied') : t('meeting.key.copy')}
                </button>
              </span>
            </div>
            {#if 本.qr}
              <details>
                <summary>{t('meeting.qr.reveal')}</summary>
                <!-- **目の前の相手に読ませる用。**画像ではなく SVG（テーマに合う） -->
                <div class="qr">{@html 本.qr}</div>
              </details>
            {/if}
            <details>
              <summary>{t('meeting.key.reveal')}</summary>
              <!-- 触れた時点で全部選ぶ。**手で端から端まで引かせない** -->
              <textarea readonly rows="3" value={本.鍵} onfocus={(e) => e.currentTarget.select()}
              ></textarea>
            </details>
          </div>
        {/each}

        <p class="hint">{t('meeting.link.hint')}</p>

        <!--
          **1 本の会議キーで入れるのは 1 人だけ**（割符は「1 つの鍵 = 1 人」・D12）。
          もう 1 人呼ぶなら、**もう 1 本出して、その人に渡す**（D47）。
          前の鍵は死なない —— 出した本数だけ、別々の人が入れる。
        -->
        <div class="issue">
          <label>
            {t('meeting.key.howmany')}
            <input type="number" min="1" max={出せる本数} bind:value={何人ぶん} />
          </label>
          <button type="button" class="quiet" onclick={() => void はじめる(何人ぶん)}>
            <Icon name="key" />{t('meeting.key.more')}
          </button>
        </div>
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
          if (!key) return;
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
        プロフィール={名乗りたち}
        顔の画={顔の画たち}
        顔を差し替える={(who, 場所) => void 顔を差し替える(who, 場所)}
        名乗られたもの={相手の名乗り}
        名乗りを書く={(who, name, bio) => void 名乗りを書く(who, name, bio)}
        覚え書きを書く={(key, note) => void 覚え書きを書く(key, note)}
        部屋に名前を付ける={部屋に名前を付ける}
        ルームを抜ける={(id) => void ルームを抜ける(id)}
        鍵を渡す={(相手) => void その人に鍵を渡す(相手)}
        鍵を渡してあるか={(key) => 渡してあるか(鍵たち, key)}
      />
      <!--
        **自分を選んでいる間は、会話の枠を出さない。**
        自分から自分へ話す口は無いので、置いておくと
        「これは何だろう」で終わる（オーナー・2026-09-08
        「なにもつかえないなら、あることは誤解しか生みません」）。
      -->
      <!--
        **誰も選んでいないときは出さない**（オーナー指示 2026-09-10
        「そのとおりです。いらないから消す」）。
        全員あての口をここに置いていたが、**選んでいないのに打てる所**は
        「これは誰に届くのか」で終わる。全員あては「部屋」の面にある。
      -->
      {#if 選んだ相手 && 選んだ相手 !== 自分の鍵}
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

  <!--
    **渡す 1 本。**出したらすぐ、渡せる形（リンク・QR・文字）で見せる（**D84**）。
    幕はほかと同じ（まんなか・透過の黒）
  -->
  {#if 渡す一本}
    <div class="幕" role="dialog" aria-modal="true" aria-label={t('key.hand')}>
      <div class="箱">
        <p class="what">
          {format(t('key.hand.title'), { name: 渡す一本.宛先?.名 ?? '' })}
        </p>
        <p class="hint">{format(t('key.hand.hint'), { name: 渡す一本.宛先?.名 ?? '' })}</p>
        <div class="row">
          {#if 渡す一本.リンク}
            <button type="button" onclick={() => void 写し取る('渡す:リンク', 渡す一本?.リンク ?? '')}>
              <Icon name={写した印 === '渡す:リンク' ? 'check' : 'link'} />
              {写した印 === '渡す:リンク' ? t('meeting.key.copied') : t('meeting.link.copy')}
            </button>
          {/if}
          <button type="button" class="quiet" onclick={() => void 写し取る('渡す:鍵', 渡す一本?.鍵 ?? '')}>
            <Icon name={写した印 === '渡す:鍵' ? 'check' : 'copy'} />
            {写した印 === '渡す:鍵' ? t('meeting.key.copied') : t('meeting.key.copy')}
          </button>
        </div>
        {#if 渡す一本.qr}
          <!-- **目の前の相手に読ませる用。**畳まずに出す（渡すために開いた幕である） -->
          <div class="qr">{@html 渡す一本.qr}</div>
        {/if}
        <p class="hint">{t('meeting.link.hint')}</p>
        <div class="tail">
          <button type="button" class="quiet" onclick={() => (渡す一本 = null)}>
            {t('help.close')}
          </button>
        </div>
      </div>
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
  /* **誰に渡したか**の札。番号のすぐ隣に置く（D84） */
  .one-key .for {
    margin-left: 8px;
    padding: 1px 8px;
    font-size: var(--text-2xs-size);
    font-weight: 400;
    color: var(--accent);
    background: var(--accent-subtle);
    border-radius: var(--radius-full);
  }

  /* **鍵 1 本ぶんの区画**（D84）。本ごとに渡す口を持つ */
  .one-key {
    margin-top: 0.5rem;
    padding: 0.5rem 0.6rem;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
  }

  .one-key .nth {
    font-weight: 600;
    font-size: 0.85rem;
  }

  .one-key .tail {
    display: flex;
    gap: 0.35rem;
  }

  /* **何人ぶん出すか**。数と押す所を横に並べる */
  .issue {
    display: flex;
    align-items: end;
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .issue label {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
    font-size: 0.8rem;
    color: var(--text-secondary);
  }

  .issue input {
    width: 4.5rem;
    font-variant-numeric: tabular-nums;
  }

  /* **更新の知らせ**（D81）。誘いの確認と同じ強さで出す */
  .updated {
    margin: 0.5rem 0.75rem 0;
    padding: 0.75rem 0.9rem;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--bg-sunken);
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }

  .updated .what {
    margin: 0;
    font-weight: 600;
  }

  .updated .tail {
    display: flex;
    gap: 0.5rem;
  }

  /* 更新の中身は**書かれたまま**出す（箇条書きの改行を潰さない） */
  .notes {
    margin: 0.4rem 0 0;
    max-height: 9rem;
    overflow: auto;
    white-space: pre-wrap;
    font-family: inherit;
    font-size: 0.82rem;
    line-height: 1.5;
    color: var(--text-secondary);
  }

  /*
    **リンクで誘われたときの確認**（D79）。
    知らせ（notice）より強く出す —— これは押すかどうかを決める所である。
  */
  .invited {
    margin: 0.5rem 0.75rem 0;
    padding: 0.75rem 0.9rem;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--bg-sunken);
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }

  .invited .what {
    margin: 0;
    font-weight: 600;
  }

  .invited .tail {
    display: flex;
    gap: 0.5rem;
  }

  /*
    QR は**紙のように**出す。**地の白は SVG の側が持っている**
    （静穏帯ごと白でないと、読み取り機が拾えない）。
    ここでは大きさだけを決める。
  */
  .qr {
    margin: 0.5rem 0;
    width: fit-content;
    border-radius: var(--radius-sm);
    overflow: hidden;
  }

  .qr :global(svg) {
    display: block;
    width: 168px;
    height: 168px;
  }

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
    width: 300px;
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
