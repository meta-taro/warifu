// **この機械のエージェントの声を鳴らす**（#50）—— 副作用の層。
//
// 決めるところは `meeting/agentVoice.ts` に置いてある。ここは鳴らすだけ。
//
//   WAV（机が OS の読み上げで作った）
//     → AudioContext.decodeAudioData
//     → MediaStreamAudioDestinationNode（会議の音の送り手へ差し替える）
//     → ctx.destination（この PC の人にも聞こえるように）

/** 鳴らす間に使う手。**通話へ声を重ねる・外す**のは呼ぶ側が渡す。 */
export type 声の手 = {
  /** 声のトラックを通話へ重ねる。`null` で外す。 */
  重ねる(声: MediaStreamTrack | null): Promise<void>;
  /** 鳴らし始めた。 */
  始めた(): void;
};

/**
 * **鳴らす場**。人が声を入れたとき（押したとき）に作っておく。
 *
 * 押していない時に作ると、WebView が音を止めたまま始めることがある。
 */
export function 声の場を作る(): AudioContext {
  return new AudioContext();
}

/**
 * **1 回鳴らす。**鳴らし終えたら、必ず声を外す（マイクか無しに戻す）。
 *
 * 解けない・鳴らせないときは投げる（呼ぶ側が「失敗」として机へ返す）。
 */
export async function 声を鳴らす(場: AudioContext, wav: ArrayBuffer, 手: 声の手): Promise<void> {
  if (場.state === 'suspended') await 場.resume();
  const 音 = await 場.decodeAudioData(wav);
  const 行き先 = 場.createMediaStreamDestination();
  const 源 = 場.createBufferSource();
  源.buffer = 音;
  源.connect(行き先);
  // **この PC の人にも聞かせる。**自分のエージェントが何を言ったかを、人が知らないままにしない
  源.connect(場.destination);
  const [声] = 行き先.stream.getAudioTracks();
  try {
    await 手.重ねる(声 ?? null);
    手.始めた();
    await new Promise<void>((終わり) => {
      源.onended = () => 終わり();
      源.start();
    });
  } finally {
    // **外し損ねると、マイクが戻らない。**失敗しても必ず外す
    await 手.重ねる(null);
    声?.stop();
    源.disconnect();
  }
}
