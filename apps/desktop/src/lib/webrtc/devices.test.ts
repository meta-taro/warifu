import { describe, expect, it } from 'vitest';
import {
  AUDIO_PROCESSING,
  DEFAULT_PREFS,
  canBlurBackground,
  constraintsFor,
  loadPrefs,
  toOptions,
  withBackground,
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
      background: 'none',
    });
  });
});

describe('背景（ぼかし）', () => {
  it('環境が持っていれば使える', () => {
    expect(canBlurBackground({ backgroundBlur: true })).toBe(true);
  });

  it('**持っていない環境で「できる」と言わない**', () => {
    expect(canBlurBackground({ echoCancellation: true })).toBe(false);
    expect(canBlurBackground(undefined)).toBe(false);
  });

  it('持っていなければ制約に足さない（黙って無視されるより、足さない）', () => {
    const prefs: Prefs = { ...DEFAULT_PREFS, background: 'blur' };
    const c = withBackground(constraintsFor(prefs), prefs, false);
    expect(JSON.stringify(c)).not.toContain('backgroundBlur');
  });

  it('持っていて、かつ希望していれば足す', () => {
    const prefs: Prefs = { ...DEFAULT_PREFS, background: 'blur' };
    const c = withBackground(constraintsFor(prefs), prefs, true);
    expect(c.video).toMatchObject({ backgroundBlur: true });
  });

  it('希望していなければ足さない', () => {
    const c = withBackground(constraintsFor(DEFAULT_PREFS), DEFAULT_PREFS, true);
    expect(JSON.stringify(c)).not.toContain('backgroundBlur');
  });
});
