import { describe, expect, it } from 'vitest';
import { describeMediaFailure, ICE_SERVERS, mediaConstraints } from './media';

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
