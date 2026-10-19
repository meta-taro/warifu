// チャットで送る画像を、**送れる大きさまで縮める。**
//
// 割符のチャットは P2P で、相手の機械へ直接届く。原寸の写真（数 MB）をそのまま流すと、
// 相手の回線と画面を埋める。だから**画面で縮めてから送る**：
//
// - 長い辺を 1600px まで
// - JPEG で書き直す（画質を下げながら、上限に収まるまで）
// - 上限は 512 KiB（Rust 側の `IMAGE_MAX` と同じ数。向こうでも数える）
//
// **小さくて描ける型のものは、そのまま送る**（書き直すと PNG の図が滲む）。

/** 送れる大きさの上限（バイト）。`warifu_meeting::IMAGE_MAX` と同じ。 */
export const 画像の上限 = 512 * 1024;
/** 長い辺の上限（px）。 */
export const 長い辺 = 1600;
/** そのまま送れる型。**SVG は入れない**（中に script を持てる）。 */
export const 送れる型 = ['image/jpeg', 'image/png', 'image/webp', 'image/gif'] as const;
/** 書き直すときに試す画質（高い順）。 */
export const 画質の段 = [0.85, 0.75, 0.6, 0.45] as const;

/** 長い辺が `上限` に収まる寸法。**大きくはしない。** */
export function 縮めた寸法(幅: number, 高さ: number, 上限 = 長い辺): { 幅: number; 高さ: number } {
  const 長い = Math.max(幅, 高さ);
  if (長い <= 上限) return { 幅, 高さ };
  const 倍 = 上限 / 長い;
  return { 幅: Math.max(1, Math.round(幅 * 倍)), 高さ: Math.max(1, Math.round(高さ * 倍)) };
}

/** 書き直さずに、そのまま送ってよいか。 */
export function そのまま送れるか(型: string, バイト: number, 幅: number, 高さ: number): boolean {
  return (
    (送れる型 as readonly string[]).includes(型) &&
    バイト <= 画像の上限 &&
    Math.max(幅, 高さ) <= 長い辺
  );
}

/** 縮めた画像。 */
export interface 縮めた画像 {
  型: string;
  中身: Uint8Array;
}

/**
 * 画像を送れる大きさにする。**収まらなければ投げる**（呼ぶ側が人に言う）。
 *
 * 透ける PNG は白地に置いてから JPEG にする（黒く潰れないように）。
 */
export async function 画像を縮める(元: Blob): Promise<縮めた画像> {
  const 絵 = await createImageBitmap(元);
  try {
    if (そのまま送れるか(元.type, 元.size, 絵.width, 絵.height)) {
      return { 型: 元.type, 中身: new Uint8Array(await 元.arrayBuffer()) };
    }
    let 寸法 = 縮めた寸法(絵.width, 絵.height);
    // **画質で収まらなければ、寸法も下げる。**3 回で諦める
    for (let 回 = 0; 回 < 3; 回++) {
      for (const 画質 of 画質の段) {
        const 出来 = await JPEGで描く(絵, 寸法.幅, 寸法.高さ, 画質);
        if (出来.size <= 画像の上限) {
          return { 型: 'image/jpeg', 中身: new Uint8Array(await 出来.arrayBuffer()) };
        }
      }
      寸法 = 縮めた寸法(寸法.幅, 寸法.高さ, Math.round(Math.max(寸法.幅, 寸法.高さ) * 0.7));
    }
    throw new Error('画像を小さくできませんでした');
  } finally {
    絵.close();
  }
}

async function JPEGで描く(絵: ImageBitmap, 幅: number, 高さ: number, 画質: number): Promise<Blob> {
  const 紙 = document.createElement('canvas');
  紙.width = 幅;
  紙.height = 高さ;
  const 筆 = 紙.getContext('2d');
  if (!筆) throw new Error('画像を描けませんでした');
  筆.fillStyle = '#ffffff';
  筆.fillRect(0, 0, 幅, 高さ);
  筆.drawImage(絵, 0, 0, 幅, 高さ);
  return await new Promise<Blob>((ok, ng) =>
    紙.toBlob((b) => (b ? ok(b) : ng(new Error('画像を描けませんでした'))), 'image/jpeg', 画質),
  );
}

/** 貼り付け・落とした物の中から、画像を 1 つ拾う。**無ければ null。** */
export function 画像を拾う(物たち: ArrayLike<{ kind?: string; type: string; getAsFile?: () => File | null }> | null | undefined): File | null {
  if (!物たち) return null;
  for (let i = 0; i < 物たち.length; i++) {
    const 物 = 物たち[i];
    if (!物.type.startsWith('image/')) continue;
    if (物 instanceof File) return 物;
    const f = 物.getAsFile?.();
    if (f) return f;
  }
  return null;
}
