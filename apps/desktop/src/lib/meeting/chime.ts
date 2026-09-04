// 入退室の知らせ音。
//
// **音のファイルを持たない。**その場で作る。
// 参照だけ足してアップロードを忘れると、ビルドもテストも通ったまま実行時にだけ壊れる
// （baseline §23）。**持っていないものは、忘れようがない。**
//
// 短くする。会話の最中に鳴るので、長いと人の話をさえぎる。

/** 1 つぶんの音。 */
export type Tone = {
  /** 高さ（Hz）。 */
  hz: number;
  /** 長さ（ミリ秒）。 */
  ms: number;
};

/** 知らせ音。 */
export type Chime = {
  /** 順に鳴らす音。 */
  tones: readonly Tone[];
};

/** 入ってきた。**上がる**（人が増える）。 */
export const 入室の音: Chime = { tones: [{ hz: 660, ms: 90 }, { hz: 880, ms: 130 }] };

/** 出ていった。**下がる**（人が減る）。聞いただけで向きが分かる。 */
export const 退室の音: Chime = { tones: [{ hz: 660, ms: 90 }, { hz: 440, ms: 130 }] };

/** ぶつからない程度の小ささ。会議の音を邪魔しない。 */
const 音量 = 0.06;

/**
 * 鳴らす。
 *
 * **鳴らせなくても落とさない。**音が出ないことは、会議に入れないことではない
 * （機器が無くても会議に入れるようにしてある `nextAttempt` と同じ構え）。
 */
export function 鳴らす(ctx: AudioContext | null, chime: Chime): void {
  if (!ctx) return;
  try {
    let at = ctx.currentTime;
    for (const tone of chime.tones) {
      const 長さ = tone.ms / 1000;
      const osc = ctx.createOscillator();
      const gain = ctx.createGain();
      osc.frequency.value = tone.hz;
      // 切れ際を落とす。**急に切ると「プツッ」と鳴る**
      gain.gain.setValueAtTime(音量, at);
      gain.gain.linearRampToValueAtTime(0, at + 長さ);
      osc.connect(gain);
      gain.connect(ctx.destination);
      osc.start(at);
      osc.stop(at + 長さ);
      at += 長さ;
    }
  } catch {
    // 握り潰す理由: 音が鳴らないことを会議の失敗にしない。
    // 鳴らない環境（音声出力が無い・自動再生が止められている）は普通にある
  }
}
