import { describe, expect, it } from 'vitest';
import {
  describeMediaFailure,
  ICE_SERVERS,
  mediaConstraints,
  nextAttempt,
  sendModeFor,
  shouldSendVideo,
} from './media';

describe('カメラとマイクの求め方（M5-c2）', () => {
  it('映像と音声の両方を求める', () => {
    expect(mediaConstraints()).toEqual({ audio: true, video: true });
  });

  it('音声だけで始められる（映像は後から足す）', () => {
    // D29「始まりは音声だけ」と同じ構え。測る前に映像を出さない
    expect(mediaConstraints({ video: false })).toEqual({ audio: true, video: false });
  });

  it('**外部の STUN / TURN を既定で使わない**', () => {
    // issues/005 満たすこと 4: シグナリングを外部インフラに頼らない。
    // 経路の当ても同じで、既定で他人のサーバへ自分の存在を知らせない
    expect(ICE_SERVERS).toEqual([]);
  });
});

describe('断られた理由を、人の言葉にする', () => {
  it('拒否は「許可されていない」と伝える', () => {
    const e = new DOMException('denied', 'NotAllowedError');
    expect(describeMediaFailure(e)).toBe('camera-denied');
  });

  it('機器が無いときは、拒否と区別する', () => {
    const e = new DOMException('none', 'NotFoundError');
    expect(describeMediaFailure(e)).toBe('camera-missing');
  });

  it('他のアプリが掴んでいるときも分ける', () => {
    const e = new DOMException('busy', 'NotReadableError');
    expect(describeMediaFailure(e)).toBe('camera-busy');
  });

  it('**分からない失敗を、分かったことにしない**', () => {
    expect(describeMediaFailure(new Error('なにか'))).toBe('camera-unknown');
    expect(describeMediaFailure(undefined)).toBe('camera-unknown');
  });
});

describe('測る前に映像を出さない（D29「始まりは音声だけ」）', () => {
  it('経路が分かるまで、映像は流さない', () => {
    expect(shouldSendVideo('unknown')).toBe(false);
  });

  it('直接でも中継でも、**測れたら**流す', () => {
    // 中継だから流さない、ではない。中継は異常ではない（DESIGN.md §4.1）
    expect(shouldSendVideo('direct')).toBe(true);
    expect(shouldSendVideo('relayed')).toBe(true);
  });
});

describe('機器が無いときの入り方（受け取るだけ）', () => {
  it('まず映像と音声、次に音声だけ、最後は何も送らない', () => {
    expect(nextAttempt(null)).toEqual({ audio: true, video: true });
    expect(nextAttempt({ audio: true, video: true })).toEqual({ audio: true, video: false });
    expect(nextAttempt({ audio: true, video: false })).toBeNull();
  });

  it('**何も送らない状態でも会議には入れる**', () => {
    // カメラもマイクも無い機械（会場の画面・見るだけの人）が入れないと、
    // 「機器が無い＝参加できない」になる
    expect(sendModeFor(null)).toBe('none');
    expect(sendModeFor({ audio: true, video: false })).toBe('audio');
    expect(sendModeFor({ audio: true, video: true })).toBe('both');
  });
});
