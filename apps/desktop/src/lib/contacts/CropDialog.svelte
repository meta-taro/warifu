<script lang="ts">
  // **落とした画像を切り取る**（**D87**・オーナー指示 2026-09-10）。
  //
  // **落とした瞬間に置かない。**どこが顔になるかを見せてから、人が決める。
  // 出るのは 512×512 の WebP 1 枚だけ（64 KB に収める）。

  import { MESSAGES, type MessageKey } from '../i18n/messages';
  import type { Locale } from '../i18n/locales';
  import { まん中の枠, 寄れる下限, 枠を収める, webpにする, 見せる倍率, type 切り取り } from './crop';

  interface Props {
    locale: Locale;
    /** 落とした画像（blob URL）。 */
    画像: string;
    /** 決めた（WebP のバイト列）。 */
    決めた: (bytes: Uint8Array) => void;
    /** やめた。 */
    やめた: () => void;
  }
  let { locale, 画像, 決めた, やめた }: Props = $props();
  const t = (key: MessageKey) => MESSAGES[locale][key];

  /** 画面に出す枠の一辺（px）。**元の画像の大きさとは別。** */
  const 見せる辺 = 320;

  let 絵: HTMLImageElement | undefined = $state();
  let 幅 = $state(0);
  let 高さ = $state(0);
  let 枠 = $state<切り取り>({ x: 0, y: 0, 辺: 0 });
  let 掴んでいる = $state(false);
  let 断り = $state('');
  /** 最後に見た指の位置。**`movementX` に頼らない**（webview で 0 のことがある）。 */
  let 直前 = { x: 0, y: 0 };

  /**
   * 元の画像 → 画面の倍率。
   *
   * **枠の中身だけが窓に映る。**だから倍率は枠で決まる
   * （短辺で固定していたのが 2026-09-10 の不具合）。
   */
  const 倍率 = $derived(見せる倍率(見せる辺, 枠.辺));

  /** つまめる範囲。**16px まで寄らせない**（伸ばした粗さしか出ない）。 */
  const 下限 = $derived(寄れる下限(幅, 高さ));
  const 上限 = $derived(Math.max(下限, Math.min(幅, 高さ) || 下限));

  function 読めた() {
    if (!絵) return;
    幅 = 絵.naturalWidth;
    高さ = 絵.naturalHeight;
    枠 = まん中の枠(幅, 高さ);
  }

  function 動かす(dx: number, dy: number) {
    枠 = 枠を収める({ ...枠, x: 枠.x - dx / 倍率, y: 枠.y - dy / 倍率 }, 幅, 高さ);
  }

  function 大きさを変える(値: number) {
    // **真ん中を保って伸び縮みさせる。**端を軸にすると、絵が逃げていく
    const 中x = 枠.x + 枠.辺 / 2;
    const 中y = 枠.y + 枠.辺 / 2;
    枠 = 枠を収める({ 辺: 値, x: 中x - 値 / 2, y: 中y - 値 / 2 }, 幅, 高さ);
  }

  async function 決める() {
    if (!絵) return;
    断り = '';
    try {
      決めた(await webpにする(絵, 枠));
    } catch (e) {
      断り = e instanceof Error ? e.message : String(e);
    }
  }
</script>

<div class="幕" role="dialog" aria-modal="true" aria-label={t('crop.title')}>
  <div class="箱">
    <p class="what">{t('crop.title')}</p>
    <p class="hint">{t('crop.hint')}</p>

    <!--
      **丸で見せる。**画面に出るのは丸い顔なので、四角で切ると
      「切ったはずの所が出ていない」になる
    -->
    <!-- 掴んで動かす所。**役割を書く**（読み上げにも出る） -->
    <div
      class="窓"
      role="application"
      aria-label={t('crop.hint')}
      style="width:{見せる辺}px;height:{見せる辺}px"
      onpointerdown={(e) => {
        掴んでいる = true;
        直前 = { x: e.clientX, y: e.clientY };
        e.currentTarget.setPointerCapture(e.pointerId);
      }}
      onpointerup={() => (掴んでいる = false)}
      onpointercancel={() => (掴んでいる = false)}
      onpointermove={(e) => {
        if (!掴んでいる) return;
        動かす(e.clientX - 直前.x, e.clientY - 直前.y);
        直前 = { x: e.clientX, y: e.clientY };
      }}
    >
      <img
        bind:this={絵}
        src={画像}
        alt=""
        onload={読めた}
        style="
          width:{幅 * 倍率}px;
          height:{高さ * 倍率}px;
          left:{-枠.x * 倍率}px;
          top:{-枠.y * 倍率}px;
        "
      />
      <div class="丸"></div>
    </div>

    <label class="ざっくり">
      {t('crop.zoom')}
      <!-- **右へ動かすと寄る。**棒の値は枠の辺の裏返しである -->
      <input
        type="range"
        min={下限}
        max={上限}
        value={下限 + 上限 - 枠.辺}
        oninput={(e) => 大きさを変える(下限 + 上限 - Number(e.currentTarget.value))}
      />
    </label>

    {#if 断り}<p class="why">{断り}</p>{/if}

    <div class="tail">
      <button type="button" onclick={() => void 決める()}>{t('crop.apply')}</button>
      <button type="button" class="quiet" onclick={やめた}>{t('profile.cancel')}</button>
    </div>
  </div>
</div>

<style>
  .幕 {
    position: fixed;
    inset: 0;
    z-index: 50;
    display: grid;
    place-items: center;
    background: color-mix(in oklab, var(--bg-app) 70%, transparent);
    backdrop-filter: blur(2px);
  }

  .箱 {
    background: var(--bg-elevated);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-lg);
    padding: var(--space-4);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    max-width: 90vw;
  }

  .what {
    margin: 0;
    font-weight: 600;
  }

  /* **掴んで動かす所。**はみ出した絵は隠す */
  .窓 {
    position: relative;
    overflow: hidden;
    border-radius: var(--radius-md);
    background: var(--bg-sunken);
    cursor: grab;
    touch-action: none;
  }

  .窓 img {
    position: absolute;
    max-width: none;
    user-select: none;
    -webkit-user-drag: none;
  }

  /* **丸く出ることを、切る前に見せる** */
  .丸 {
    position: absolute;
    inset: 0;
    border-radius: 50%;
    box-shadow: 0 0 0 9999px color-mix(in oklab, var(--bg-app) 60%, transparent);
    pointer-events: none;
  }

  .ざっくり {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-size: 0.82rem;
    color: var(--text-secondary);
  }

  .ざっくり input {
    flex: 1;
  }

  .tail {
    display: flex;
    gap: var(--space-2);
  }

  .why {
    margin: 0;
    font-size: 0.82rem;
    color: var(--danger, var(--text-secondary));
  }
</style>
