// 予定の並べ方と読み方。**判断はここに置く**（`.svelte` は出すだけ）。
//
// オーナー判断（2026-09-07）——「**予定は割符の中に持つ**（TSV で保存し、
// `.ics` で出し入れ）。外のカレンダー API とは繋がない。」
//
// **時刻は Unix 秒で持ち、画面で地元の時計に直す。**
// 置き場所に地元の言い方を書くと、機械を移したときに読めなくなる。

/** 予定 1 つ（Rust から来る形）。 */
export interface 予定 {
  /** 始まり（Unix 秒）。 */
  start: number;
  /** 終わり（Unix 秒）。 */
  end: number;
  title: string;
  note: string;
}

/**
 * これからの予定だけ。**終わっていないもの**を、始まりの早い順に。
 *
 * **終わった予定を上に出さない** —— 人が見たいのは次に来るものである。
 */
export function これから(予定たち: readonly 予定[], いま: number): 予定[] {
  return 予定たち.filter((一つ) => 一つ.end > いま).sort((a, b) => a.start - b.start);
}

/** 終わった予定（新しい順）。**消さない** —— 何をしたかは残る。 */
export function 終わったもの(予定たち: readonly 予定[], いま: number): 予定[] {
  return 予定たち.filter((一つ) => 一つ.end <= いま).sort((a, b) => b.start - a.start);
}

/** いま進んでいる予定か。 */
export function いま進んでいる(一つ: 予定, いま: number): boolean {
  return 一つ.start <= いま && いま < 一つ.end;
}

/**
 * 画面の入力（`YYYY-MM-DD` と `HH:MM`）を Unix 秒に直す。
 *
 * **地元の時計で読む** —— 人が書いたのは手元の時刻である。
 * 読めなければ `null`（**0 にしない**。1970 年の予定が入る）。
 */
export function 秒にする(日: string, 時刻: string): number | null {
  const 日の形 = /^(\d{4})-(\d{2})-(\d{2})$/.exec(日.trim());
  const 時の形 = /^(\d{1,2}):(\d{2})$/.exec(時刻.trim());
  if (!日の形 || !時の形) return null;
  const [, 年, 月, 日にち] = 日の形;
  const [, 時, 分] = 時の形;
  const h = Number(時);
  const m = Number(分);
  if (h > 23 || m > 59) return null;
  const d = new Date(Number(年), Number(月) - 1, Number(日にち), h, m, 0, 0);
  if (Number.isNaN(d.getTime())) return null;
  // **月や日が繰り上がったら、書いた日ではない**（2 月 30 日など）
  if (d.getMonth() !== Number(月) - 1 || d.getDate() !== Number(日にち)) return null;
  return Math.floor(d.getTime() / 1000);
}

/** Unix 秒を `YYYY-MM-DD` に。**地元の時計で。** */
export function 日にする(秒: number): string {
  const d = new Date(秒 * 1000);
  const 二桁 = (n: number) => String(n).padStart(2, '0');
  return `${d.getFullYear()}-${二桁(d.getMonth() + 1)}-${二桁(d.getDate())}`;
}

/** Unix 秒を `HH:MM` に。**地元の時計で。** */
export function 時刻にする(秒: number): string {
  const d = new Date(秒 * 1000);
  const 二桁 = (n: number) => String(n).padStart(2, '0');
  return `${二桁(d.getHours())}:${二桁(d.getMinutes())}`;
}

/** 何分ぶんか。 */
export function 長さの分(一つ: 予定): number {
  return Math.max(0, Math.round((一つ.end - 一つ.start) / 60));
}
