// **顔を切り取って WebP にする**（**D87**）。
//
// オーナー指示（2026-09-10）——
// 「**クロップ機能を搭載してください**」「**WEBP にコンバートしてください**」
// 「**JPG も WebP も許可してください**」
//
// 入るのは JPG / PNG / WebP（webview が開ける形）。**出るのは WebP だけ。**
// 置き場所に置くのは 512×512 の 1 枚で、**64 KB を超えたら質を落として収める。**

/** 顔の一辺（px）。**置ける上限と同じ**（`warifu-vault` の `AVATAR_MAX_SIDE`）。 */
export const 顔の一辺 = 512;

/** 置ける大きさ（バイト）。**64 KiB**（`warifu-vault` の `AVATAR_MAX_BYTES`）。 */
export const 顔の上限 = 64 * 1024;

/** 切り取る枠。**元の画像の中の、正方形 1 つ。** */
export interface 切り取り {
  /** 枠の左上（元の画像の px）。 */
  x: number;
  y: number;
  /** 枠の一辺（元の画像の px）。 */
  辺: number;
}

/**
 * **枠が画像の外へ出ないように収める。**
 *
 * 出たままにすると、canvas が透明を描いて**縁が欠けた顔**になる。
 */
export function 枠を収める(枠: 切り取り, 画像の幅: number, 画像の高さ: number): 切り取り {
  const 最大 = Math.min(画像の幅, 画像の高さ);
  const 辺 = Math.max(16, Math.min(枠.辺, 最大));
  return {
    辺,
    x: Math.max(0, Math.min(枠.x, 画像の幅 - 辺)),
    y: Math.max(0, Math.min(枠.y, 画像の高さ - 辺)),
  };
}

/** 真ん中いっぱいの枠。**開いた直後の既定。** */
export function まん中の枠(画像の幅: number, 画像の高さ: number): 切り取り {
  const 辺 = Math.min(画像の幅, 画像の高さ);
  return { 辺, x: (画像の幅 - 辺) / 2, y: (画像の高さ - 辺) / 2 };
}

/**
 * 切り取って WebP にする。**64 KB に収まるまで質を落とす。**
 *
 * **収まらなければ諦めて投げる** —— 置けないものを置きに行かない
 * （Rust 側も断るが、そこで断られると理由が遠い）。
 */
export async function webpにする(
  絵: CanvasImageSource,
  枠: 切り取り,
  一辺 = 顔の一辺,
): Promise<Uint8Array> {
  const canvas = document.createElement('canvas');
  canvas.width = 一辺;
  canvas.height = 一辺;
  const 筆 = canvas.getContext('2d');
  if (!筆) throw new Error('画像を描けません');
  筆.drawImage(絵, 枠.x, 枠.y, 枠.辺, 枠.辺, 0, 0, 一辺, 一辺);

  // **質を落としながら試す。**PNG には質が無いので、WebP にする理由でもある
  for (const 質 of [0.9, 0.8, 0.7, 0.6, 0.5, 0.4, 0.3]) {
    const 塊 = await 出す(canvas, 質);
    if (塊 && 塊.size <= 顔の上限) return new Uint8Array(await 塊.arrayBuffer());
  }
  throw new Error(`64 KB に収まりませんでした`);
}

function 出す(canvas: HTMLCanvasElement, 質: number): Promise<Blob | null> {
  return new Promise((返す) => canvas.toBlob(返す, 'image/webp', 質));
}
