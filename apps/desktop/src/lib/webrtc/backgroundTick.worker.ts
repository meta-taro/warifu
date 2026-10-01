// 背景を隠す流れの拍子（**#40**）。
//
// **窓が隠れていても刻む。**画面側の setTimeout / requestAnimationFrame は、
// 窓が隠れる（最小化・ほかの窓の下）と間引かれ、相手へ送る映像が止まる。
// worker の時計は間引かれにくいので、拍子だけをここで刻む。

const scope = globalThis as unknown as {
  onmessage: ((e: MessageEvent<number>) => void) | null;
  postMessage: (m: number) => void;
};

let timer: ReturnType<typeof setInterval> | null = null;

// 受け取った数（ミリ秒）ごとに 1 度知らせる。0 以下で止める
scope.onmessage = (e) => {
  if (timer !== null) clearInterval(timer);
  timer = null;
  const ms = Number(e.data);
  if (ms > 0) timer = setInterval(() => scope.postMessage(0), ms);
};
