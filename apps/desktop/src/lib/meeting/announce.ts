// 会議の出来事を、人へ知らせる形にする。
//
// **名簿が動くだけでは足りない。**画面を見ていない間に誰が来たのかが分からない。
// チャット欄は人が見ている所なので、そこへ 1 行残す。

/**
 * 打ち込みを送ってよいか（Enter が押されたとき）。
 *
 * **日本語入力の変換確定の Enter で送らない。**
 * 2026-09-06 にオーナーが実際に踏んだ ——「まって、」「あと」「エンターで」が
 * **変換のたびに別々の発言として飛んだ。**
 *
 * `isComposing` は変換中に true になる。`keyCode === 229` は、
 * 古い WebView が `isComposing` を出さないときの保険である。
 *
 * **Shift / Option（Alt）を押しながらの Enter は改行**（2026-09-06 のオーナー要望
 * 「**改行できたらもっといいかな。シフトエンターとか？オルトエンターとか？**」）。
 * どちらも受けるのは、**人によって指が覚えている組み合わせが違う**ためである。
 */
export function 送ってよい(e: {
  key: string;
  isComposing?: boolean;
  keyCode?: number;
  shiftKey?: boolean;
  altKey?: boolean;
}): boolean {
  if (e.key !== 'Enter') return false;
  if (e.isComposing || e.keyCode === 229) return false;
  // **改行したいときは送らない**
  return !e.shiftKey && !e.altKey;
}

/** チャット欄に並ぶ 1 行。 */
export type 会話行 = {
  /** 誰の発言か。 */
  who: string;
  /** 中身。 */
  body: string;
  /** 自分の発言か。 */
  mine: boolean;
  /** 人の発言ではなく、会議からの知らせか。 */
  system?: true;
  /**
   * いつの発言か（`HH:MM`）。
   *
   * **無いと、あとから読み返せない。**2026-09-06 にオーナーが
   * 「**何時に投稿したかわからないです**」と言った所である。
   */
  at?: string;
};

/** いまの時刻を `HH:MM` で。**秒は出さない** —— 会話に秒の精度は要らない。 */
export function いま時刻(now: Date = new Date()): string {
  const 二桁 = (n: number) => String(n).padStart(2, '0');
  return `${二桁(now.getHours())}:${二桁(now.getMinutes())}`;
}

/**
 * 知らせの種類。
 *
 * **`退室` と `切断` を混ぜない。**退室は本人の意思、切断は事故である。
 * 人は前者を待たないが、後者は待つ —— そして**この会議キーでは戻れない**（割符は一度きり・D12）。
 * 2026-09-04 に実物で食い違っていた（帯は「経路が切れました」、チャット欄は「退室しました」）。
 */
export type 出来事 = '入室' | '退室' | '切断';

const 文言の鍵 = { 入室: 'joined', 退室: 'left', 切断: 'lost' } as const;

/**
 * 入退室を、チャット欄の 1 行にする。
 *
 * 文言は呼ぶ側が訳す（`t` を渡す）。**この層は言語を知らない。**
 */
export function 入退室の知らせ(
  種類: 出来事,
  who: string,
  t: (key: 'joined' | 'left' | 'lost', values: { who: string }) => string,
): 会話行 {
  return {
    who: '',
    body: t(文言の鍵[種類], { who }),
    mine: false,
    system: true,
  };
}

/**
 * チャットの往復をログへ書く言い方。
 *
 * **中身を書かない。**書くと会議の中身が `warifu.log` に残る。
 * 出すのは**長さと相手だけ** —— 下ごしらえ（SDP / ICE）がバイト数を出しているのと
 * 釣り合いを取る（2026-09-04 に別の機械の担当から「文字だけ何も出ない」と指摘された）。
 */
export function 話の記録(向き: '送信' | '受信', 相手: string, body: string): string {
  const 助詞 = 向き === '送信' ? 'へ' : 'から';
  return `${向き}: 文字（${[...body].length} 文字）を ${相手} ${助詞}`;
}
