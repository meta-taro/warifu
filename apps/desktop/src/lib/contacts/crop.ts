// **顔を切り取って WebP にする**（**D87**）。
//
// オーナー指示（2026-09-10）——
// 「**クロップ機能を搭載してください**」「**WEBP にコンバートしてください**」
// 「**JPG も WebP も許可してください**」
//
// 入るのは JPG / PNG / WebP（webview が開ける形）。**出るのは WebP か JPEG** ——
// WebP を出せない webview があるので、そのときは JPEG へ落ちる（2026-09-10 に踏んだ）。
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

/**
 * **枠を、見せる窓いっぱいに映す倍率。**
 *
 * ここを「画像の短辺 → 窓」で固定していたのが不具合だった（2026-09-10・
 * オーナー指摘「**クロップうまくできません**」）——
 * **枠を小さくしても絵が大きくならない**ので、いつも真ん中の正方形が出るだけだった。
 *
 * 映るのは**枠の中身だけ**である。だから倍率は枠で決まる。
 */
export function 見せる倍率(見せる辺: number, 枠の辺: number): number {
  if (!(見せる辺 > 0) || !(枠の辺 > 0)) return 1;
  return 見せる辺 / 枠の辺;
}

/** つまめる下限（元の画像の px）。**これ以上寄ると、伸ばした粗さしか出ない。** */
export function 寄れる下限(画像の幅: number, 画像の高さ: number): number {
  const 最大 = Math.min(画像の幅, 画像の高さ);
  if (最大 <= 0) return 16;
  // 短辺の 1/8 か 64px の大きい方。**ただし画像より大きくはしない**
  return Math.min(最大, Math.max(64, Math.round(最大 / 8)));
}

/** 真ん中いっぱいの枠。**開いた直後の既定。** */
export function まん中の枠(画像の幅: number, 画像の高さ: number): 切り取り {
  const 辺 = Math.min(画像の幅, 画像の高さ);
  return { 辺, x: (画像の幅 - 辺) / 2, y: (画像の高さ - 辺) / 2 };
}

/** 出した 1 枚。**canvas でも、試験の作りものでも同じ形。** */
export interface 出した絵 {
  /** 実際に出てきた形（`image/webp` など）。**頼んだ形とは違うことがある。** */
  型: string;
  大きさ: number;
  中身: () => Promise<Uint8Array>;
}

/** 絵を出す口。**形・質・一辺を渡すと 1 枚返す**（出せなければ `null`）。 */
export type 絵を出す口 = (型: string, 質: number, 一辺: number) => Promise<出した絵 | null>;

/** 試す質。**良い方から落としていく。** */
export const 質の段: readonly number[] = [0.9, 0.8, 0.7, 0.6, 0.5, 0.4, 0.3];

/** 試す一辺。**質で足りなければ、小さくする。** */
export const 辺の段: readonly number[] = [顔の一辺, 384, 256];

/**
 * **64 KB に収まる 1 枚を出す。**
 *
 * オーナー・2026-09-10 ——「**クロップですが、64 KB に収まりませんでしたとでて
 * 保存できません。**」
 *
 * 原因は、**頼んだ形が出てくると決めつけていた**ことである ——
 * `canvas.toBlob` は、**出せない形を頼まれると黙って PNG を返す。**
 * 写真の 512px PNG は 64 KB に入らないので、**質を 7 段落としても必ず失敗していた。**
 *
 * だから **出てきた形を見る。**WebP でなければ JPEG に切り替える
 * （どちらも `warifu-vault` が受け取る）。
 */
export async function 収まるまで出す(
  出す: 絵を出す口,
  上限 = 顔の上限,
): Promise<{ 型: string; 中身: Uint8Array }> {
  let 最後に見たもの: 出した絵 | null = null;

  for (const 形 of ['image/webp', 'image/jpeg']) {
    let この形は出せる = true;
    for (const 一辺 of 辺の段) {
      for (const 質 of 質の段) {
        const 出た = await 出す(形, 質, 一辺);
        if (!出た) continue;
        // **頼んだ形が返ってきていないなら、この形は出せない。**粘らずに次の形へ
        if (出た.型 !== 形) {
          この形は出せる = false;
          最後に見たもの = 出た;
          break;
        }
        最後に見たもの = 出た;
        if (出た.大きさ <= 上限) return { 型: 出た.型, 中身: await 出た.中身() };
      }
      if (!この形は出せる) break;
    }
  }

  // **「収まりませんでした」だけでは、次に何をすればよいか分からない**
  const 実際 = 最後に見たもの
    ? `${最後に見たもの.型} / ${最後に見たもの.大きさ} バイト`
    : '1 枚も出せませんでした';
  throw new Error(`${Math.round(上限 / 1024)} KB に収まりませんでした（${実際}）`);
}

/**
 * 切り取って、置ける 1 枚にする。
 *
 * **出るのは WebP か JPEG。**webview が WebP を出せるなら WebP、
 * 出せないなら JPEG になる（`収まるまで出す` が見分ける）。
 */
export async function webpにする(
  絵: CanvasImageSource,
  枠: 切り取り,
  一辺 = 顔の一辺,
): Promise<Uint8Array> {
  const 口 = canvasの口(絵, 枠);
  const { 中身 } = await 収まるまで出す((形, 質, 辺) => 口(形, 質, Math.min(辺, 一辺)));
  return 中身;
}

/** canvas に描いて出す口を作る。**描くのはここだけ。** */
export function canvasの口(絵: CanvasImageSource, 枠: 切り取り): 絵を出す口 {
  return async (形, 質, 一辺) => {
    const canvas = document.createElement('canvas');
    canvas.width = 一辺;
    canvas.height = 一辺;
    const 筆 = canvas.getContext('2d');
    if (!筆) throw new Error('画像を描けません');
    // **JPEG は透明を持てない。**黒く沈まないよう、白を敷いてから描く
    筆.fillStyle = '#ffffff';
    筆.fillRect(0, 0, 一辺, 一辺);
    筆.drawImage(絵, 枠.x, 枠.y, 枠.辺, 枠.辺, 0, 0, 一辺, 一辺);

    const 塊 = await new Promise<Blob | null>((返す) => canvas.toBlob(返す, 形, 質));
    if (!塊) return null;
    return {
      型: 塊.type,
      大きさ: 塊.size,
      中身: async () => new Uint8Array(await 塊.arrayBuffer()),
    };
  };
}
