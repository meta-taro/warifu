import { describe, expect, it } from 'vitest';
import {
  いま入っているか,
  ルームが変わったら,
  初めの同意,
  同意を置く,
  声をどうするか,
  音の玉を選ぶ,
} from './agentVoice';

describe('声の同意', () => {
  it('既定は切', () => {
    expect(いま入っているか(初めの同意, 'ROOM-A')).toBe(false);
  });

  it('入れたルームでだけ効く', () => {
    const 同意 = 同意を置く('ROOM-A', true);
    expect(いま入っているか(同意, 'ROOM-A')).toBe(true);
    expect(いま入っているか(同意, 'ROOM-B')).toBe(false);
  });

  it('ルームが無いところでは入れられない', () => {
    expect(同意を置く(null, true)).toEqual(初めの同意);
  });

  it('ルームを移ったら切に戻る', () => {
    const 同意 = 同意を置く('ROOM-A', true);
    expect(ルームが変わったら(同意, 'ROOM-B')).toEqual(初めの同意);
  });

  it('ルームを抜けたら切に戻る', () => {
    const 同意 = 同意を置く('ROOM-A', true);
    expect(ルームが変わったら(同意, null)).toEqual(初めの同意);
  });

  it('同じルームのままなら同じものを返す', () => {
    const 同意 = 同意を置く('ROOM-A', true);
    expect(ルームが変わったら(同意, 'ROOM-A')).toBe(同意);
  });
});

describe('声をどうするか', () => {
  const 会議中 = {
    同意: 同意を置く('ROOM-A', true),
    見ている: 'ROOM-A',
    映像を使う: true,
    映像がある部屋: 'ROOM-A',
    相手の数: 1,
  };

  it('入れていて会議中なら流す', () => {
    expect(声をどうするか(会議中)).toBe('play');
  });

  it('入れていなければ、会議中でも流さない', () => {
    expect(声をどうするか({ ...会議中, 同意: 初めの同意 })).toBe('not_allowed');
  });

  it('別のルームで入れた印では流さない', () => {
    expect(声をどうするか({ ...会議中, 同意: 同意を置く('ROOM-B', true) })).toBe('not_allowed');
  });

  it('映像と音を足していなければ会議が無い', () => {
    expect(声をどうするか({ ...会議中, 映像を使う: false })).toBe('no_meeting');
  });

  it('映像が別の部屋にあれば会議が無い', () => {
    expect(声をどうするか({ ...会議中, 映像がある部屋: 'ROOM-B' })).toBe('no_meeting');
  });

  it('相手が居なければ会議が無い', () => {
    expect(声をどうするか({ ...会議中, 相手の数: 0 })).toBe('no_meeting');
  });
});

describe('音の玉を選ぶ', () => {
  it('声の間は、マイクが入っていても声を送る', () => {
    expect(音の玉を選ぶ({ 声: '声', マイク: 'マイク', 音を送る: true, 映像を使う: true })).toBe('声');
  });

  it('マイクが切でも、声の間は声を送る', () => {
    expect(音の玉を選ぶ({ 声: '声', マイク: 'マイク', 音を送る: false, 映像を使う: true })).toBe('声');
  });

  it('声が終わったら、マイクへ戻す', () => {
    expect(音の玉を選ぶ({ 声: null, マイク: 'マイク', 音を送る: true, 映像を使う: true })).toBe(
      'マイク',
    );
  });

  it('声が終わったら、マイクが切なら何も送らない', () => {
    expect(音の玉を選ぶ({ 声: null, マイク: 'マイク', 音を送る: false, 映像を使う: true })).toBe(
      null,
    );
  });

  it('映像と音を足していない部屋へは、声も送らない', () => {
    expect(音の玉を選ぶ({ 声: '声', マイク: 'マイク', 音を送る: false, 映像を使う: false })).toBe(
      null,
    );
  });
});
