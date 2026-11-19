import { describe, expect, it } from 'vitest';

import { 残す形, 戻す, 時刻の札, 送った時刻の欄 } from './history';
import type { 会話行 } from '$lib/meeting/announce';

const 行 = (body: string, 足し: Partial<会話行> = {}): 会話行 => ({
  who: 'A',
  body,
  mine: false,
  ...足し,
});

describe('残す形', () => {
  it('画像は残さず、画像があったことだけを残す', () => {
    // `blob:` の URL は立ち上げ直すと指す先が無い。指したまま残すと、壊れた絵が出る
    const 残す = 残す形({ R: [行('', { 画像: 'blob:abc' }), 行('文字')] });
    expect(残す.R[0].画像).toBeUndefined();
    expect(残す.R[0].画像は残していない).toBe(true);
    expect(残す.R[1]).toEqual(行('文字'));
  });

  it('ルームの並びも行の並びも変えない', () => {
    const 残す = 残す形({ R: [行('1'), 行('2')], S: [行('3')] });
    expect(Object.keys(残す)).toEqual(['R', 'S']);
    expect(残す.R.map((r) => r.body)).toEqual(['1', '2']);
  });
});

describe('戻す', () => {
  it('残した行は、立ち上げてから届いた行より前に並ぶ', () => {
    // 読み終わる前に届いた発言を、残した分で潰さない
    const 戻した = 戻す({ R: [行('昨日')] }, { R: [行('いま')] });
    expect(戻した.R.map((r) => r.body)).toEqual(['昨日', 'いま']);
  });

  it('いまにしか無いルームも、残した分にしか無いルームも持つ', () => {
    const 戻した = 戻す({ R: [行('1')] }, { S: [行('2')] });
    expect(Object.keys(戻した).sort()).toEqual(['R', 'S']);
  });

  it('形の合わない行は捨て、合う行は戻す', () => {
    // 古い版や壊れた控えで、画面ごと止めない
    const 戻した = 戻す(
      { R: [行('良い'), { who: 1 }, null, 'x'], S: '配列でない' } as unknown,
      {},
    );
    expect(戻した.R.map((r) => r.body)).toEqual(['良い']);
    expect(戻した.S).toBeUndefined();
  });

  it('控えが無い・読めないときは、いまのまま', () => {
    const いま = { R: [行('いま')] };
    expect(戻す(null, いま)).toEqual(いま);
    expect(戻す('壊れた', いま)).toEqual(いま);
  });
});

describe('時刻の札', () => {
  const 今 = new Date(2026, 10, 4, 21, 0);

  it('今日の行は時刻だけ', () => {
    expect(時刻の札(行('x', { at: '20:15', 時: new Date(2026, 10, 4, 20, 15).getTime() }), 今)).toBe(
      '20:15',
    );
  });

  it('別の日の行は日付を添える', () => {
    // 立ち上げ直して読み返すと、昨日の 20:15 と今日の 20:15 が見分けられない
    expect(時刻の札(行('x', { at: '20:15', 時: new Date(2026, 10, 3, 20, 15).getTime() }), 今)).toBe(
      '11/3 20:15',
    );
  });

  it('いつの行か分からなければ、元の時刻のまま', () => {
    expect(時刻の札(行('x', { at: '09:00' }), 今)).toBe('09:00');
    expect(時刻の札(行('x'), 今)).toBeUndefined();
  });
});

describe('送った時刻の欄（0.1.53）', () => {
  it('送った時刻があれば、その時刻で出し、日付のために時も持つ', () => {
    const 送った = new Date(2026, 10, 19, 10, 20).getTime();
    expect(送った時刻の欄(送った, new Date(2026, 10, 19, 10, 29))).toEqual({ at: '10:20', 時: 送った });
  });

  it('古い版の相手（時刻なし）は、受け取った時刻で出す', () => {
    expect(送った時刻の欄(null, new Date(2026, 10, 19, 10, 29))).toEqual({ at: '10:29' });
    expect(送った時刻の欄(undefined, new Date(2026, 10, 19, 10, 29))).toEqual({ at: '10:29' });
  });

  it('前の日に送られた行は、日付つきで出る', () => {
    const 欄 = 送った時刻の欄(new Date(2026, 10, 18, 23, 50).getTime());
    expect(時刻の札({ ...行('昨日の発言'), ...欄 }, new Date(2026, 10, 19, 0, 5))).toBe('11/18 23:50');
  });
});
