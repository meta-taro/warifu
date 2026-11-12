// **エージェントの専用カーソル**（11-12・tadori の `packages/ui/static/agent.js` を手本にした）。
//
// エージェントが「いまどこを指しているか・何を押したか」を、割符の画面の中だけで見せる。
// **人のマウスとキーボードは使わない（奪わない）。**押す・打つはページの中で起こす。
//
// **割符だけの決まり：人が押すものは押さない。**札の［通す］［断る］、ルームキーの［入る］、
// 発言の［送る］、秘密の欄には `data-agent-hands-off` の印がある。印のある要素とその中は、
// 押すのも打つのも断る（エージェントがカーソルで押せると、自分で自分に許しを出せてしまう）。
// パスワードの欄にも打たない。

/** 動く時間（CSS の `.agent-cursor` の transition と合わせる）。 */
const 動く時間 = 650;
const 枠の時間 = 2600;
const 一言の最短 = 5000;
const 一字ごと = 100;
const 一言の最長 = 20000;
const 離したあと = 1500;
const 隠すまで = 6000;

/** 一言を出しておく時間。秒を渡されればそれ、無ければ文の長さで決める。 */
export function 出す時間(文: string, 秒?: number): number {
  if (秒) return 秒 * 1000;
  if (!文) return 枠の時間;
  return Math.min(一言の最長, 一言の最短 + [...文].length * 一字ごと);
}

/** 画面の指示。`go`・`filter` は割符の画面に当たる仕組みが無いので受けない（席の口が断る）。 */
export type 指示 =
  | { action: 'point'; target: string; message?: string; seconds?: number }
  | { action: 'click'; target: string; message?: string; seconds?: number }
  | { action: 'type'; target: string; value: string; message?: string; seconds?: number }
  | { action: 'say'; message: string; seconds?: number };

/** 画面に出す言葉（見ている人の言葉で渡す）。 */
export type カーソルの言葉 = {
  札: string;
  閉じる: string;
  見つからない: (target: string) => string;
  押さない: (target: string) => string;
  /** 動かし終えたときの 1 文（席の口へ返す）。 */
  済んだ: (action: 指示['action'], target: string) => string;
};

/** 押せる・打てる所（ここから target の言葉で探す）。 */
const 探す所 = 'button, a, input, select, textarea, [role="tab"], [role="option"], [role="menuitem"], h1, h2, h3, label, summary';

/** **人が押すもの**の印。この中は押さない・打たない。 */
export const 押させない印 = 'data-agent-hands-off';

/** 人が押すものか（印のある要素の中・パスワードの欄）。 */
export function 人が押すものか(要素: Element): boolean {
  if (要素.closest(`[${押させない印}]`)) return true;
  return 要素 instanceof HTMLInputElement && 要素.type === 'password';
}

const 見えているか = (要素: HTMLElement) => {
  const r = 要素.getBoundingClientRect();
  return r.width > 0 && r.height > 0 && getComputedStyle(要素).visibility !== 'hidden';
};

/** 要素の呼び名（画面の文字・読み上げ名・説明・欄の見出し）。 */
const 呼び名たち = (要素: HTMLElement) =>
  [
    要素.getAttribute('aria-label'),
    要素.getAttribute('title'),
    要素.getAttribute('placeholder'),
    'labels' in 要素 ? (要素 as HTMLInputElement).labels?.[0]?.textContent : undefined,
    要素.textContent,
  ]
    .filter((s): s is string => Boolean(s))
    .map((s) => s.replace(/\s+/g, ' ').trim());

/** target の言葉で探す。同じ名前 → 頭が合う → 含む の順に、画面の上から。 */
export function 探す(target: string, 根: ParentNode = document, 見える = 見えているか): HTMLElement | undefined {
  const 欲しい = target.replace(/\s+/g, ' ').trim();
  const 候補 = [...根.querySelectorAll(探す所)].filter(
    (e): e is HTMLElement => e instanceof HTMLElement && 見える(e),
  );
  return (
    候補.find((e) => 呼び名たち(e).some((n) => n === 欲しい)) ??
    候補.find((e) => 呼び名たち(e).some((n) => n.startsWith(欲しい))) ??
    候補.find((e) => 呼び名たち(e).some((n) => n.includes(欲しい)))
  );
}

const 端 = 8;
const 間 = 10;
const しっぽの寄せ = 14;
const 挟む = (v: number, 下: number, 上: number) => Math.min(Math.max(v, 下), Math.max(下, 上));

/**
 * **吹き出しの置き場所**（tadori の `bubblePlacement` と同じ）。上 → 下 → 右 → 左の順に収まる所へ置き、
 * 窓の端から 8px 内側に収める。しっぽは指した所（カーソル）を向く。
 */
export function 吹き出しの置き場所(
  的: { left: number; top: number; right: number; bottom: number },
  錨: { x: number; y: number },
  大きさ: { width: number; height: number },
  窓: { width: number; height: number },
): { left: number; top: number; side: 'above' | 'below' | 'right' | 'left'; tail: number } {
  const 上に入る = 的.top - 間 - 大きさ.height >= 端;
  const 下に入る = 的.bottom + 間 + 大きさ.height <= 窓.height - 端;
  if (上に入る || 下に入る) {
    const left = 挟む(錨.x - しっぽの寄せ, 端, 窓.width - 端 - 大きさ.width);
    const top = 上に入る ? 的.top - 間 - 大きさ.height : 的.bottom + 間;
    return { left, top, side: 上に入る ? 'above' : 'below', tail: 挟む(錨.x - left, しっぽの寄せ, 大きさ.width - しっぽの寄せ) };
  }
  const 右に入る = 的.right + 間 + 大きさ.width <= 窓.width - 端;
  const 左に入る = 的.left - 間 - 大きさ.width >= 端;
  const top = 挟む(錨.y - 大きさ.height / 2, 端, 窓.height - 端 - 大きさ.height);
  const tail = 挟む(錨.y - top, しっぽの寄せ, 大きさ.height - しっぽの寄せ);
  if (右に入る || !左に入る) {
    return { left: 挟む(的.right + 間, 端, 窓.width - 端 - 大きさ.width), top, side: 'right', tail };
  }
  return { left: 的.left - 間 - 大きさ.width, top, side: 'left', tail };
}

const 待つ = (ms: number) => new Promise((done) => setTimeout(done, ms));

/** **カーソル 1 つ分。**指示は届いた順に 1 つずつ動かす（前の動きが終わってから次へ）。 */
/** `見える` は試験のため（jsdom は大きさを持たない）。既定は、大きさと visibility で見る。 */
export function カーソルを作る(
  言葉: カーソルの言葉,
  知らせる: (文: string) => void,
  見える: (要素: HTMLElement) => boolean = 見えているか,
) {
  let 矢印: HTMLDivElement | null = null;
  let 枠: HTMLDivElement | null = null;
  let 吹き出し: HTMLDivElement | null = null;
  let 消す: ReturnType<typeof setTimeout> | undefined;
  let 隠す: ReturnType<typeof setTimeout> | undefined;
  let 乗せている = false;
  let 列 = Promise.resolve();

  function 用意する() {
    if (矢印 && 枠 && 吹き出し) return { 矢印, 枠, 吹き出し };
    矢印 = document.createElement('div');
    矢印.className = 'agent-cursor';
    矢印.hidden = true;
    矢印.setAttribute('aria-hidden', 'true');
    矢印.innerHTML =
      '<svg viewBox="0 0 24 24"><path d="M4 2l15 9.5-6.6 1.4L16 20.5l-3 1.3-3.5-7.6L4 18z" fill="#5e6ad2" stroke="#fff" stroke-width="1.4" stroke-linejoin="round"/></svg><span class="tag"></span>';
    札を替える(言葉.札);
    枠 = document.createElement('div');
    枠.className = 'agent-box';
    枠.hidden = true;
    吹き出し = document.createElement('div');
    吹き出し.className = 'agent-bubble';
    吹き出し.hidden = true;
    吹き出し.setAttribute('role', 'status');
    const 文 = document.createElement('span');
    文.className = 'agent-bubble-text';
    const 閉じる = document.createElement('button');
    閉じる.type = 'button';
    閉じる.className = 'agent-bubble-close';
    閉じる.textContent = '×';
    閉じる.setAttribute('aria-label', 言葉.閉じる);
    閉じる.title = 言葉.閉じる;
    吹き出し.append(文, 閉じる);
    // 押すと消える。マウスを乗せている間は消さない（読み終わるまで待つ）
    吹き出し.addEventListener('click', () => すぐ消す());
    吹き出し.addEventListener('mouseenter', () => {
      乗せている = true;
      clearTimeout(消す);
    });
    吹き出し.addEventListener('mouseleave', () => {
      乗せている = false;
      あとで消す(離したあと);
    });
    document.body.append(矢印, 枠, 吹き出し);
    return { 矢印, 枠, 吹き出し };
  }

  function 札を替える(文: string) {
    const 札 = 矢印?.querySelector('.tag');
    if (札) 札.textContent = 文;
  }

  function すぐ消す() {
    clearTimeout(消す);
    乗せている = false;
    if (枠) 枠.hidden = true;
    if (吹き出し) 吹き出し.hidden = true;
  }

  function あとで消す(ms = 枠の時間) {
    clearTimeout(消す);
    clearTimeout(隠す);
    消す = setTimeout(() => {
      if (!乗せている) すぐ消す();
    }, ms);
    隠す = setTimeout(
      () => {
        if (!矢印 || 乗せている) return;
        矢印.hidden = true;
        札を替える(言葉.札);
      },
      Math.max(隠すまで, ms + 1000),
    );
  }

  function 吹き出しを出す(的: DOMRect, 錨: { x: number; y: number }, 文: string) {
    const { 吹き出し: 泡 } = 用意する();
    const 字 = 泡.querySelector('.agent-bubble-text');
    if (字) 字.textContent = 文;
    泡.hidden = !文;
    if (!文) return;
    Object.assign(泡.style, { left: '0px', top: '0px' });
    const 大 = 泡.getBoundingClientRect();
    const 置き = 吹き出しの置き場所(的, 錨, { width: 大.width, height: 大.height }, { width: innerWidth, height: innerHeight });
    泡.dataset.side = 置き.side;
    Object.assign(泡.style, { left: `${Math.round(置き.left)}px`, top: `${Math.round(置き.top)}px` });
    泡.style.setProperty('--tail', `${Math.round(置き.tail)}px`);
  }

  async function 動かす(要素: HTMLElement, 文?: string) {
    const { 矢印: 矢, 枠: 囲い } = 用意する();
    const 前 = 要素.getBoundingClientRect();
    const 見えている = 前.top >= 0 && 前.bottom <= innerHeight;
    要素.scrollIntoView?.({ block: 見えている ? 'nearest' : 'center', inline: 'nearest' });
    const r = 要素.getBoundingClientRect();
    矢.hidden = false;
    const 錨 = { x: Math.round(r.left + Math.min(r.width / 2, 40)), y: Math.round(r.top + r.height / 2) };
    矢.style.transform = `translate(${錨.x}px, ${錨.y}px)`;
    囲い.hidden = false;
    Object.assign(囲い.style, {
      left: `${r.left - 4}px`,
      top: `${r.top - 4}px`,
      width: `${r.width + 8}px`,
      height: `${r.height + 8}px`,
    });
    吹き出しを出す(r, 錨, 文 ?? '');
    await 待つ(動く時間);
  }

  /** 1 つ動かし、結果の 1 文を返す（席の口へそのまま返る）。 */
  async function 一つ動かす(指し: 指示): Promise<string> {
    if (指し.action === 'say') {
      const { 矢印: 矢 } = 用意する();
      矢.hidden = false;
      札を替える(指し.message);
      あとで消す(出す時間(指し.message, 指し.seconds));
      return 言葉.済んだ('say', '');
    }
    const 要素 = 探す(指し.target, document, 見える);
    if (!要素) {
      知らせる(言葉.見つからない(指し.target));
      return 言葉.見つからない(指し.target);
    }
    // **人が押すものは、押さない・打たない**（指して見せるのは構わない）
    if (指し.action !== 'point' && 人が押すものか(要素)) {
      await 動かす(要素, 言葉.押さない(指し.target));
      知らせる(言葉.押さない(指し.target));
      あとで消す(出す時間(言葉.押さない(指し.target)));
      return 言葉.押さない(指し.target);
    }
    await 動かす(要素, 指し.message);
    if (指し.action === 'click') {
      const 矢 = 矢印;
      if (矢) {
        矢.classList.remove('click');
        void 矢.offsetWidth;
        矢.classList.add('click');
      }
      // ページの中で押す（人のマウスは動かさない）。mousedown も送り、メニューなどが人の操作と同じに動くように
      要素.dispatchEvent(new MouseEvent('mousedown', { bubbles: true }));
      要素.click();
    }
    if (
      指し.action === 'type' &&
      (要素 instanceof HTMLInputElement || 要素 instanceof HTMLTextAreaElement || 要素 instanceof HTMLSelectElement)
    ) {
      要素.focus();
      要素.value = 指し.value;
      要素.dispatchEvent(new Event('input', { bubbles: true }));
      要素.dispatchEvent(new Event('change', { bubbles: true }));
    }
    あとで消す(出す時間(指し.message ?? '', 指し.seconds));
    return 言葉.済んだ(指し.action, 指し.target);
  }

  return {
    /** 指示を 1 つ積む。届いた順に動かし、結果の 1 文を返す。 */
    積む(指し: 指示): Promise<string> {
      const 結果 = 列.then(() => 一つ動かす(指し)).catch((e: unknown) => {
        知らせる(String(e));
        return String(e);
      });
      列 = 結果.then(() => undefined);
      return 結果;
    },
  };
}
