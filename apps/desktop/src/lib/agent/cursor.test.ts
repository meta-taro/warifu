// @vitest-environment jsdom
//
// **エージェントの専用カーソル**（11-12・tadori の「エージェントの専用カーソル」の試験と同じ 5 つ＋割符の決まり）。
// 人のマウスは動かさない（ページの中で押す）。人が押すもの（札の［通す］など）は押さない。
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { カーソルを作る, 吹き出しの置き場所, 探す, 出す時間, 人が押すものか, 押させない印 } from './cursor';

const 言葉 = {
  札: 'エージェント',
  閉じる: '閉じる',
  見つからない: (t: string) => `「${t}」が見つかりません`,
  押さない: (t: string) => `「${t}」は人が押すものなので押しません`,
  済んだ: (a: string, t: string) => `${a}:${t}`,
};
const いつも見える = () => true;

beforeEach(() => {
  vi.useFakeTimers();
  document.body.innerHTML = '';
});
afterEach(() => vi.useRealTimers());

const 動かし切る = async (p: Promise<string>) => {
  await vi.advanceTimersByTimeAsync(1000);
  return p;
};

describe('エージェントの専用カーソル', () => {
  it('1. point で、指した要素にカーソルと枠と吹き出しが出る（押さない）', async () => {
    document.body.innerHTML = '<button>読み直す</button>';
    const 押した = vi.fn();
    document.querySelector('button')!.addEventListener('click', 押した);
    const カ = カーソルを作る(言葉, () => {}, いつも見える);
    await 動かし切る(カ.積む({ action: 'point', target: '読み直す', message: 'ここで読み直せます' }));
    expect(document.querySelector<HTMLElement>('.agent-cursor')!.hidden).toBe(false);
    expect(document.querySelector<HTMLElement>('.agent-box')!.hidden).toBe(false);
    expect(document.querySelector('.agent-bubble-text')!.textContent).toBe('ここで読み直せます');
    expect(押した).not.toHaveBeenCalled();
  });

  it('2. click で、その要素がページの中で押される（人のマウスは動かさない）', async () => {
    document.body.innerHTML = '<button>読み直す</button>';
    const 押した = vi.fn();
    document.querySelector('button')!.addEventListener('click', 押した);
    const カ = カーソルを作る(言葉, () => {}, いつも見える);
    expect(await 動かし切る(カ.積む({ action: 'click', target: '読み直す' }))).toBe('click:読み直す');
    expect(押した).toHaveBeenCalledTimes(1);
    // 押したのはページの中の出来事で、OS のポインタを動かす道はこの部品に無い
    expect(押した.mock.calls[0][0]).toBeInstanceOf(MouseEvent);
  });

  it('3. 窓の四隅を指しても、吹き出しは窓の 8px 内側に収まる', () => {
    const 窓 = { width: 800, height: 600 };
    const 大きさ = { width: 280, height: 60 };
    const 四隅 = [
      { x: 2, y: 2 },
      { x: 798, y: 2 },
      { x: 2, y: 598 },
      { x: 798, y: 598 },
    ];
    for (const 錨 of 四隅) {
      const 的 = { left: 錨.x - 2, top: 錨.y - 2, right: 錨.x + 2, bottom: 錨.y + 2 };
      const 置き = 吹き出しの置き場所(的, 錨, 大きさ, 窓);
      expect(置き.left).toBeGreaterThanOrEqual(8);
      expect(置き.top).toBeGreaterThanOrEqual(8);
      expect(置き.left + 大きさ.width).toBeLessThanOrEqual(窓.width - 8);
      expect(置き.top + 大きさ.height).toBeLessThanOrEqual(窓.height - 8);
    }
  });

  it('4. 見つからない target では何も押さず、「見つかりません」と出す', async () => {
    document.body.innerHTML = '<button>読み直す</button>';
    const 押した = vi.fn();
    document.querySelector('button')!.addEventListener('click', 押した);
    const 知らせ: string[] = [];
    const カ = カーソルを作る(言葉, (s) => 知らせ.push(s), いつも見える);
    await 動かし切る(カ.積む({ action: 'click', target: '無いボタン' }));
    expect(押した).not.toHaveBeenCalled();
    expect(知らせ).toEqual(['「無いボタン」が見つかりません']);
  });

  it('割符の決まり：札の［通す］など、人が押すものは押さない・打たない（指すのは構わない）', async () => {
    document.body.innerHTML = `<div ${押させない印}><button>通す</button></div><input type="password" aria-label="秘密" />`;
    const 押した = vi.fn();
    document.querySelector('button')!.addEventListener('click', 押した);
    const 知らせ: string[] = [];
    const カ = カーソルを作る(言葉, (s) => 知らせ.push(s), いつも見える);
    await 動かし切る(カ.積む({ action: 'click', target: '通す' }));
    await 動かし切る(カ.積む({ action: 'type', target: '秘密', value: 'abc' }));
    await 動かし切る(カ.積む({ action: 'point', target: '通す' }));
    expect(押した).not.toHaveBeenCalled();
    expect(document.querySelector<HTMLInputElement>('input')!.value).toBe('');
    expect(知らせ).toEqual(['「通す」は人が押すものなので押しません', '「秘密」は人が押すものなので押しません']);
  });

  it('type は欄に打って input を起こす（人が打ったのと同じに画面が動く）', async () => {
    document.body.innerHTML = '<textarea placeholder="メッセージ"></textarea>';
    const 打った = vi.fn();
    document.querySelector('textarea')!.addEventListener('input', 打った);
    const カ = カーソルを作る(言葉, () => {}, いつも見える);
    await 動かし切る(カ.積む({ action: 'type', target: 'メッセージ', value: 'こんにちは' }));
    expect(document.querySelector('textarea')!.value).toBe('こんにちは');
    expect(打った).toHaveBeenCalled();
  });

  it('探すのは 同じ名前 → 頭が合う → 含む の順、見えていないものは探さない', () => {
    document.body.innerHTML = '<button>読み直すまで</button><button>読み直す</button><button hidden-x>隠れ</button>';
    expect(探す('読み直す', document, いつも見える)?.textContent).toBe('読み直す');
    expect(探す('読み', document, いつも見える)?.textContent).toBe('読み直すまで');
    expect(探す('隠れ', document, (e) => !e.hasAttribute('hidden-x'))).toBeUndefined();
  });

  it('一言の時間は、秒の指定か、文の長さ（5 秒〜20 秒）', () => {
    expect(出す時間('', undefined)).toBe(2600);
    expect(出す時間('あ'.repeat(10))).toBe(6000);
    expect(出す時間('あ'.repeat(500))).toBe(20000);
    expect(出す時間('あ', 3)).toBe(3000);
    expect(人が押すものか(document.createElement('button'))).toBe(false);
  });
});
