import { describe, expect, it } from 'vitest';
import {
  機器が無いと言うか,
  機器をどうするか,
  機器を放すまでの秒,
  AUDIO_PROCESSING,
  DEFAULT_PREFS,
  constraintsFor,
  loadPrefs,
  toOptions,
  type DeviceLike,
  type Prefs,
} from './devices';

const d = (kind: string, id: string, label = ''): DeviceLike => ({
  kind,
  deviceId: id,
  label,
  groupId: '',
});

describe('機器の一覧（入室前に選ぶ）', () => {
  it('カメラとマイクだけを取り出す。スピーカーは混ぜない', () => {
    const list = [d('videoinput', 'cam1', '内蔵カメラ'), d('audioinput', 'mic1', '内蔵マイク'), d('audiooutput', 'spk1')];
    const o = toOptions(list);
    expect(o.cameras.map((c) => c.id)).toEqual(['cam1']);
    expect(o.microphones.map((m) => m.id)).toEqual(['mic1']);
  });

  it('**名前が空の機器を捨てない。**許可前は名前が取れない', () => {
    // 許可を出す前、ブラウザは label を空で返す。捨てると「機器が無い」に見える
    const o = toOptions([d('videoinput', 'cam1', '')]);
    expect(o.cameras).toHaveLength(1);
    expect(o.cameras[0].label).not.toBe('');
  });

  it('同じ機器を二度出さない', () => {
    const o = toOptions([d('videoinput', 'cam1', 'A'), d('videoinput', 'cam1', 'A')]);
    expect(o.cameras).toHaveLength(1);
  });
});

describe('入室前の初期設定', () => {
  it('既定は**マイクもカメラも切**（入った瞬間に映らない・喋らない）', () => {
    expect(DEFAULT_PREFS.micOn).toBe(false);
    expect(DEFAULT_PREFS.cameraOn).toBe(false);
  });

  it('選んだ機器を制約に載せる', () => {
    const prefs: Prefs = { ...DEFAULT_PREFS, cameraId: 'cam1', micId: 'mic1' };
    const c = constraintsFor(prefs);
    expect(c.video).toEqual({ deviceId: { exact: 'cam1' } });
    expect(c.audio).toMatchObject({ deviceId: { exact: 'mic1' } });
  });

  it('カメラを選んでいなければ、機器の指定をしない（既定の機器に任せる）', () => {
    expect(constraintsFor(DEFAULT_PREFS).video).toBe(true);
  });

  it('**ハウリング防止を必ず要求する**（既定に任せない）', () => {
    // 明示しないと環境によって切れる。1 台で 2 窓を開いた瞬間に鳴き始めるのがこれ
    for (const prefs of [DEFAULT_PREFS, { ...DEFAULT_PREFS, micId: 'mic1' }]) {
      expect(constraintsFor(prefs).audio).toMatchObject(AUDIO_PROCESSING);
    }
    expect(AUDIO_PROCESSING.echoCancellation).toBe(true);
    expect(AUDIO_PROCESSING.noiseSuppression).toBe(true);
    expect(AUDIO_PROCESSING.autoGainControl).toBe(true);
  });

  it('壊れた保存値を読んでも落ちない。既定へ戻す', () => {
    expect(loadPrefs('{壊れている')).toEqual(DEFAULT_PREFS);
    expect(loadPrefs(null)).toEqual(DEFAULT_PREFS);
    expect(loadPrefs('{"micOn":"はい"}')).toEqual(DEFAULT_PREFS);
  });

  it('保存されている値は読み戻す', () => {
    const saved = JSON.stringify({ micOn: true, cameraOn: false, cameraId: 'cam9', micId: null });
    expect(loadPrefs(saved)).toEqual({
      micOn: true,
      cameraOn: false,
      cameraId: 'cam9',
      micId: null,
      background: 'blur',
      blurStrength: 'medium',
    });
  });
});

describe('背景（**D126**）', () => {
  it('**既定は隠す**（ぼかし・中くらい）', () => {
    expect(DEFAULT_PREFS.background).toBe('blur');
    expect(DEFAULT_PREFS.blurStrength).toBe('medium');
  });

  it('前の版の `none` は隠す側へ読み替える（既定値であって、選んだものではない）', () => {
    const saved = JSON.stringify({ micOn: false, cameraOn: true, background: 'none' });
    expect(loadPrefs(saved).background).toBe('blur');
  });

  it('**明示の `off` は残す**（見せたい人の選択を戻さない）', () => {
    const saved = JSON.stringify({ micOn: false, cameraOn: true, background: 'off' });
    expect(loadPrefs(saved).background).toBe('off');
  });

  it('画像と度合いは読み戻す。知らない値は既定へ', () => {
    const ok = JSON.stringify({
      micOn: false,
      cameraOn: true,
      background: 'image',
      blurStrength: 'strong',
    });
    expect(loadPrefs(ok)).toMatchObject({ background: 'image', blurStrength: 'strong' });
    const bad = JSON.stringify({ micOn: false, cameraOn: true, background: 'mirror', blurStrength: 9 });
    expect(loadPrefs(bad)).toMatchObject({ background: 'blur', blurStrength: 'medium' });
  });

  it('背景の希望は、カメラの制約に混ぜない（隠すのは画面の中の模型）', () => {
    const prefs: Prefs = { ...DEFAULT_PREFS, background: 'blur' };
    expect(JSON.stringify(constraintsFor(prefs))).not.toContain('backgroundBlur');
  });
});

describe('機器が無いと言うか（許可を待っている間は言わない）', () => {
  // **2026-09-28、Windows で、WebView2 の許可の窓が出ている間に
  // 「カメラが見つかりません／マイクが見つかりません」と出た**。
  // 機器の一覧は、許可が下りたあとに初めて読み直すので、**待っている間は空**である。
  // **空を「無い」と読まない。**
  it('許可を待っている間は、無いと言わない', () => {
    expect(機器が無いと言うか({ 支度した: true, 許可を待っている: true, 本数: 0 })).toBe(false);
  });

  it('許可が下りて（または断られて）一覧が空なら、無いと言う', () => {
    expect(機器が無いと言うか({ 支度した: true, 許可を待っている: false, 本数: 0 })).toBe(true);
  });

  it('1 つでもあれば言わない／支度の前も言わない', () => {
    expect(機器が無いと言うか({ 支度した: true, 許可を待っている: false, 本数: 1 })).toBe(false);
    expect(機器が無いと言うか({ 支度した: false, 許可を待っている: false, 本数: 0 })).toBe(false);
  });
});

describe('カメラ・マイクを切ったまましばらく経ったら放す（#44）', () => {
  // **2026-09-28、［カメラ］を切ってあるのに 38 分つかみっぱなしだった**（Windows）。
  // 一瞬切ってすぐ入れ直す使い方もある。**すぐ入れる人は待たせず、切ったままなら放す**
  it('切で、つかんでいれば、しばらく待ってから放す', () => {
    expect(機器をどうするか({ 入: false, 掴んでいる: true, 放してある: false })).toBe('待って放す');
  });

  it('入に戻したとき、放してあれば、つかみ直す', () => {
    expect(機器をどうするか({ 入: true, 掴んでいる: false, 放してある: true })).toBe('つかみ直す');
  });

  it('入でつかんでいる／支度の前（つかんでも放してもいない）は、何もしない', () => {
    expect(機器をどうするか({ 入: true, 掴んでいる: true, 放してある: false })).toBe('そのまま');
    expect(機器をどうするか({ 入: false, 掴んでいる: false, 放してある: false })).toBe('そのまま');
    expect(機器をどうするか({ 入: true, 掴んでいる: false, 放してある: false })).toBe('そのまま');
  });

  it('**マイクも同じ判断**（ミュートしたままなら放す・2026-09-28）', () => {
    // 同じ関数をマイクにも使う。**カメラとマイクで行儀を揃える**
    expect(機器をどうするか({ 入: false, 掴んでいる: true, 放してある: false })).toBe('待って放す');
  });

  it('**すぐ入れ直す人を待たせない長さ**（数秒では放さない）', () => {
    expect(機器を放すまでの秒).toBeGreaterThanOrEqual(30);
  });
});
