// 経路が付かなかったときに、**何が起きなかったのかを読めるようにする層**（issues/11 / #17）。
//
// **付かなかったという結果しか残らないのが、いちばん困る。**
// 2026-09-14、Windows の人からこう来た ——
//
//   「何を送って、相手が何を返して、どの組を試して、なぜ捨てたのかが読めません。
//     Mac Air 側は『経路が変わった: direct』が出るので付いたと分かりますが、
//     こちらは付かなかったという結果しか残りません。」
//
// **ここは純ロジック。**`RTCPeerConnection` を知らない（テストは素の文字列で書ける）。
//
// ## 住所をそのまま出さない
//
// 画面から記録へ渡すものは「短い一言だけ・中身（SDP・鍵・住所）は渡さない」と
// 決めてある（`src-tauri/src/lib.rs` の `log`）。**その線は崩さない。**
// ただし**同じ網に居るかどうか**は切り分けでいちばん効くので、
// **網は残して機械だけ伏せる**（`192.168.24.11` → `192.168.24.x`）。
// 記録は公開の issue に貼られる前提で書く。

/**
 * **記録の版。**画面（フロント）を建て直したかどうかを、記録から見分けるための印。
 *
 * 2026-09-15、Windows の人が「`組 …` が 1 行も出ない」と報告した。
 * **`候補 送る` は出ていた** —— つまり**前の版のフロントが埋まったまま**で、
 * Rust だけ建て直されていた。**こちらには見分ける手段が無く、1 往復を無駄にした。**
 *
 * **記録に出すものを増やしたら、この数を 1 つ上げる。**
 */
export const 追跡の版 = 7;

/** 候補 1 つから読み取れること。 */
export interface 候補のあらまし {
  種類: 'host' | 'srflx' | 'prflx' | 'relay' | '不明';
  手: 'udp' | 'tcp' | '不明';
  /** 住所。**機械の所は伏せてある。** */
  網: string;
  口: number | null;
  /** mDNS の名前（`….local`）で来たか。**塞がれている機械を疑う手掛かり。** */
  名前で来たか: boolean;
}

/** この層が見る統計の形。`link/path.ts` の `RtcStatLike` に住所が足りないので、ここで足す。 */
export interface 統計の行 {
  id: string;
  type: string;
  kind?: string;
  packetsSent?: number;
  packetsReceived?: number;
  /**
   * **音の積もり**（2026-09-17）。
   *
   * **本数だけでは、黙っているのか喋っているのかが分からない。**
   * `track.enabled = false` にしても、**音は「無音」として送られ続ける**
   * ——**本数は減らない。**（映像は止まるので 0 になる。音だけ違う）
   *
   * だから **`送り 音 121` は「声が 121 個ぶん出た」ではない。**
   * **無音 121 個かもしれない。**——**そこを分けるのがこれである。**
   */
  totalAudioEnergy?: number;
  state?: string;
  nominated?: boolean;
  localCandidateId?: string;
  remoteCandidateId?: string;
  candidateType?: string;
  protocol?: string;
  address?: string;
  port?: number;
}

const 種類たち = ['host', 'srflx', 'prflx', 'relay'] as const;

/**
 * 住所の**機械の所だけ**を伏せる。
 *
 * mDNS の名前は**そのまま出す** — もともと誰のものか分からない形なので、
 * 伏せても何も守らず、読める情報だけが減る。
 */
export function 伏せる(住所: string): string {
  if (住所.endsWith('.local')) return 住所;
  if (住所.includes(':')) {
    const 組 = 住所.split(':');
    return `${組[0]}:${組[1]}:…`;
  }
  const 節 = 住所.split('.');
  if (節.length !== 4) return 住所;
  return `${節[0]}.${節[1]}.${節[2]}.x`;
}

const 読めない: 候補のあらまし = {
  種類: '不明',
  手: '不明',
  網: '不明',
  口: null,
  名前で来たか: false,
};

/**
 * 候補の文字列を読む。
 *
 * **読めないものは読めないと言う。**途中まで読めたからといって、
 * 残りを埋めない（原則 7）。
 */
export function 候補を読む(生: string): 候補のあらまし {
  const 場 = 生.trim().split(/\s+/);
  if (場.length < 8 || 場[6] !== 'typ') return 読めない;

  const 手 = 場[2]?.toLowerCase();
  const 種類 = 場[7];
  const 住所 = 場[4] ?? '';
  const 口 = Number(場[5]);
  if (手 !== 'udp' && 手 !== 'tcp') return 読めない;
  if (!(種類たち as readonly string[]).includes(種類 ?? '')) return 読めない;
  if (!Number.isFinite(口)) return 読めない;

  return {
    種類: 種類 as 候補のあらまし['種類'],
    手,
    網: 伏せる(住所),
    口,
    名前で来たか: 住所.endsWith('.local'),
  };
}

/** 候補 1 つを 1 行にする。**送ったものと来たものを同じ形で書く**（並べて読めるように）。 */
export function 候補を言い表す(向き: '送る' | '来た', 生: string): string {
  const あらまし = 候補を読む(生);
  if (あらまし.種類 === '不明') return `候補 ${向き} 読めません`;
  const 印 = あらまし.名前で来たか ? '（mDNS の名前）' : '';
  return `候補 ${向き} ${あらまし.種類} ${あらまし.手} ${あらまし.網}:${あらまし.口}${印}`;
}

function 候補の姿(行: 統計の行 | undefined): string {
  if (!行) return '不明';
  const 住所 = 行.address ? 伏せる(行.address) : '不明';
  return `${行.candidateType ?? '不明'} ${行.protocol ?? '不明'} ${住所}:${行.port ?? '不明'}`;
}

/**
 * 選ばれた組を 1 行にする。**組が無ければ黙る** —
 * 「まだ無い」を「無い」と書くと、付かなかった理由のように読める。
 */
export function 対を言い表す(統計: readonly 統計の行[]): string | null {
  const 組たち = 統計.filter((s) => s.type === 'candidate-pair' && s.state === 'succeeded');
  if (組たち.length === 0) return null;
  const 選ばれた = 組たち.find((p) => p.nominated === true) ?? 組たち[0];
  const 引く = new Map(統計.map((s) => [s.id, s]));
  const こちら = 引く.get(選ばれた.localCandidateId ?? '');
  const あちら = 引く.get(選ばれた.remoteCandidateId ?? '');
  return `選ばれた組 ${候補の姿(こちら)} ↔ ${候補の姿(あちら)}`;
}

/**
 * **選ばれた組の両端が、同じ網に居るか。**
 *
 * **ハウリングの手がかりに使う**（2026-09-17・オーナー依頼）。
 * エコー除去は**自分の出力しか知らない**ので、
 * **机の隣に居る別の端末から出た音は消せない。**
 *
 * **同じ網は「近い」の手がかりであって、確証ではない。**
 * 同じ網でも別の階に居ることはある。**だから断定せず、案内に使う。**
 * 逆に**別の網なら、机の隣に居ることはまず無い** —— そこは出さないでよい。
 *
 * 組が無ければ `null`（**「まだ分からない」を「別の網」と言わない**）。
 */
export function 同じ網に居るか(統計: readonly 統計の行[]): boolean | null {
  const 組たち = 統計.filter((s) => s.type === 'candidate-pair' && s.state === 'succeeded');
  if (組たち.length === 0) return null;
  const 選ばれた = 組たち.find((p) => p.nominated === true) ?? 組たち[0];
  const 引く = new Map(統計.map((s) => [s.id, s]));
  const こちら = 引く.get(選ばれた.localCandidateId ?? '')?.address;
  const あちら = 引く.get(選ばれた.remoteCandidateId ?? '')?.address;
  if (!こちら || !あちら) return null;
  // **名前で来たものは比べられない**（mDNS は住所を隠すのが目的）
  if (こちら.endsWith('.local') || あちら.endsWith('.local')) return null;
  return 伏せる(こちら) === 伏せる(あちら);
}

/**
 * 数だけを言う。**付かなかったときに、どこで止まったかが分かる。**
 *
 * 来た候補が 0 なら相手の下ごしらえが届いていない。
 * 組はあるのに成立が 0 なら、届いてはいるが通れていない。**この 2 つは別の話。**
 */
export function 数えて言い表す(統計: readonly 統計の行[]): string {
  const 送った = 統計.filter((s) => s.type === 'local-candidate').length;
  const 来た = 統計.filter((s) => s.type === 'remote-candidate').length;
  const 組たち = 統計.filter((s) => s.type === 'candidate-pair');
  const 成立 = 組たち.filter((p) => p.state === 'succeeded').length;
  const だめ = 組たち.filter((p) => p.state === 'failed').length;
  const 試し中 = 組たち.length - 成立 - だめ;
  return `候補 送った ${送った} / 来た ${来た} ／ 組 ${組たち.length}（成立 ${成立}・試し中 ${試し中}・だめ ${だめ}）`;
}

/**
 * 組を 1 つずつ言い表す。**候補が引けたかどうかまで出す。**
 *
 * 2026-09-14、Windows で **`経路の具合 connected` が出ているのに、画面は `unknown` のまま**
 * という形を踏んだ。`pathFromStats` は**組の両側の候補を引けたときだけ**経路を決めるので、
 * **片側が引けないと、繋がっていても `unknown` になる。**
 * どちらだったのかは、**引けたかどうかを書き出さないと分からない。**
 */
export function 組の様子(統計: readonly 統計の行[]): string[] {
  const 引く = new Map(統計.map((s) => [s.id, s]));
  return 統計
    .filter((s) => s.type === 'candidate-pair')
    .map((組, i) => {
      const こちら = 引く.get(組.localCandidateId ?? '')?.candidateType ?? '引けません';
      const あちら = 引く.get(組.remoteCandidateId ?? '')?.candidateType ?? '引けません';
      const 選 = 組.nominated === true ? ' 選ばれた' : '';
      return `組 ${i + 1} ${組.state ?? '不明'}${選}／こちら ${こちら}／あちら ${あちら}`;
    });
}

/**
 * **送っているのか、受けているのか**を数で出す。
 *
 * 2026-09-15、Windows の映像が mac に出ない。**経路は `direct`、文字は通る。**
 * それでも**送っていないのか、送っているのに映らないのか**が、記録から読めなかった。
 *
 * **「枠が無い」と「枠はあるが 0 個」は別の話。**
 * 前者は**送るものを持っていない**（カメラを掴めていない・受け取り専用で張った）、
 * 後者は**送ろうとして出ていない**（経路・符号化の話）。**混ぜると切り分けられない。**
 */
export function 送り受けを言い表す(
  統計: readonly 統計の行[],
  /**
   * **手元の様子**（2026-09-18・ASUS の指摘）。
   *
   * **`なし` が 2 つの意味を持ってしまった。**
   *
   * ```text
   * v0.1.9 まで  なし = **枠が無い**（支度を通っていない）
   * v0.1.10 から なし = **枠が無い** か **送り手から外してある**
   * ```
   *
   * `replaceTrack(null)` は**送り手から track を外す**ので、
   * **`outbound-rtp` の行そのものが消える。**だから両方 `なし` になる。
   *
   * **外から音量計で測らせていた**（ASUS が `msedgewebview2` の録音の最大を測った）。
   * **道具が自分で言えるようにする。**
   */
  手元?: { 掴んでいる: boolean; 外してある: boolean },
): string {
  // **音の積もりを、本数の隣に置く**（2026-09-17）。
  //
  // **本数だけでは判定できない。**`track.enabled = false` でも
  // **音は「無音」として送られ続ける**ので、**本数は減らない。**
  // 2026-09-17、`送り 音 121` を見て「まだ漏れている」と読みかけた ——
  // **無音 121 個だった可能性**を、本数では切り分けられない。
  //
  // **`なし` と `0` を分けたのと、同じ話が 1 段深い所にもあった**
  // （あちらは ASUS のエージェントの指摘で分けた）。
  const 積もり = (向き: 'outbound-rtp' | 'inbound-rtp'): string => {
    const 行たち = 統計.filter((s) => s.type === 向き && s.kind === 'audio');
    if (行たち.length === 0) return '';
    // **出していない相手には、何も言わない。**
    // 古い版は `totalAudioEnergy` を出さない —— **「出していない」を「0」と言うと、
    // 喋っているのに「無音」と書くことになる。**
    // （`なし` と `0` を分けたのと、まったく同じ理由）
    const 出ている = 行たち.filter((s) => s.totalAudioEnergy !== undefined);
    if (出ている.length === 0) return '（音の積もり 不明）';
    const 合計 = 出ている.reduce((和, s) => 和 + (s.totalAudioEnergy ?? 0), 0);
    // **桁を落とさない。**無音は 0 に極めて近い値になるので、丸めると 0 と区別できない
    if (合計 === 0) return '（音の積もり 0・**無音**）';
    return `（音の積もり ${合計.toExponential(2)}）`;
  };
  const 数 = (種: string, 向き: 'outbound-rtp' | 'inbound-rtp'): number | null => {
    const 行たち = 統計.filter((s) => s.type === 向き && s.kind === 種);
    if (行たち.length === 0) return null;
    return 行たち.reduce(
      (合計, s) => 合計 + (向き === 'outbound-rtp' ? (s.packetsSent ?? 0) : (s.packetsReceived ?? 0)),
      0,
    );
  };
  const 言う = (向き: 'outbound-rtp' | 'inbound-rtp'): string => {
    const 映像 = 数('video', 向き);
    const 音 = 数('audio', 向き);
    if (映像 === null && 音 === null) return 'なし';
    return `映像 ${映像 ?? 'なし'} / 音 ${音 ?? 'なし'}`;
  };
  // **`なし` の意味を、その場で言い分ける**（上の注記）
  const 手元の言い方 = (): string => {
    if (!手元) return '';
    if (!手元.掴んでいる) return '【機器を掴んでいない】';
    return 手元.外してある ? '【掴んでいるが、送り手から外してある】' : '【掴んでいて、送り手に付いている】';
  };
  return (
    `送り ${言う('outbound-rtp')}${積もり('outbound-rtp')}${手元の言い方()}` +
    ` ／ 受け ${言う('inbound-rtp')}${積もり('inbound-rtp')}`
  );
}

/** 映像が、どちらへ流れているか。 */
export interface 向き {
  送っている: boolean;
  受けている: boolean;
}

/**
 * **映像の向き**（**#39**・2026-09-16）。
 *
 * 使った人 ——「**入った瞬間相手には映っているのに、こっちがビデオ会議を始める
 * ボタン押さないと見れないのはおかしい、最悪の UX です。**」
 *
 * **片方だけ流れているのが、いちばん悪い。**画面が「つながっています」としか言わないので、
 * **人は「壊れている」と読む**（実際そう読まれた）。
 * **画面に言わせるために、向きを取り出す。**
 *
 * **ここでは「枠が無い」と「0 個」を同じに扱う** —— どちらも**流れていない**。
 * 言い分けるのは [`送り受けを言い表す`] の仕事（切り分け用）で、
 * **人に見せる判断は、ここで丸める。**
 */
export function 映像の向き(統計: readonly 統計の行[]): 向き {
  return 向きを数える(統計, 'video');
}

/**
 * **音がどちらへ流れているか。**
 *
 * **2026-09-17 に、これが要ると分かった。**映像の向きだけ見ていて、
 * **音が押す前から流れていたのを見落としていた**（Mac Air の実測 `送り 映像 0 / 音 90`）。
 *
 * **ハウリングの案内にも使う** —— 自分が黙っていれば、こちらからは回らない。
 */
export function 音の向き(統計: readonly 統計の行[]): 向き {
  return 向きを数える(統計, 'audio');
}

function 向きを数える(統計: readonly 統計の行[], 種別: 'video' | 'audio'): 向き {
  const 数 = (種: 'outbound-rtp' | 'inbound-rtp'): number =>
    統計
      .filter((s) => s.type === 種 && s.kind === 種別)
      .reduce(
        (合計, s) =>
          合計 + (種 === 'outbound-rtp' ? (s.packetsSent ?? 0) : (s.packetsReceived ?? 0)),
        0,
      );
  return { 送っている: 数('outbound-rtp') > 0, 受けている: 数('inbound-rtp') > 0 };
}
