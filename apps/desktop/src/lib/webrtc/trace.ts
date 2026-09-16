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
export const 追跡の版 = 5;

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
export function 送り受けを言い表す(統計: readonly 統計の行[]): string {
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
  return `送り ${言う('outbound-rtp')} ／ 受け ${言う('inbound-rtp')}`;
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
  const 数 = (種: 'outbound-rtp' | 'inbound-rtp'): number =>
    統計
      .filter((s) => s.type === 種 && s.kind === 'video')
      .reduce(
        (合計, s) =>
          合計 + (種 === 'outbound-rtp' ? (s.packetsSent ?? 0) : (s.packetsReceived ?? 0)),
        0,
      );
  return { 送っている: 数('outbound-rtp') > 0, 受けている: 数('inbound-rtp') > 0 };
}
