import { describe, expect, it } from 'vitest';
import { compareVersions, isNewer, updateStateFrom } from './version';

describe('版の比較', () => {
  it('数値として比べる。文字列順では比べない', () => {
    expect(compareVersions('0.10.0', '0.9.0')).toBe(1);
    expect(compareVersions('1.0.0', '1.0.0')).toBe(0);
    expect(compareVersions('1.0.0', '1.0.1')).toBe(-1);
  });

  it('桁数が違っても比べられる', () => {
    expect(compareVersions('1.2', '1.2.0')).toBe(0);
    expect(compareVersions('2', '1.9.9')).toBe(1);
  });

  it('読めない版は新しいと判定しない', () => {
    expect(isNewer('ばーじょん', '1.0.0')).toBe(false);
    expect(isNewer('', '1.0.0')).toBe(false);
  });
});

describe('更新の状態（DESIGN.md §10 / D36）', () => {
  it('署名が確かめられていない更新は、あることにしない', () => {
    const s = updateStateFrom({ current: '0.2.0', found: '0.3.0', signatureVerified: false });
    expect(s.kind).toBe('none');
  });

  it('署名が確かめられて、かつ新しいときだけ知らせる', () => {
    const s = updateStateFrom({ current: '0.2.0', found: '0.3.0', signatureVerified: true });
    expect(s).toEqual({ kind: 'available', version: '0.3.0' });
  });

  it('同じ版・古い版では知らせない', () => {
    expect(updateStateFrom({ current: '0.3.0', found: '0.3.0', signatureVerified: true }).kind).toBe('none');
    expect(updateStateFrom({ current: '0.3.0', found: '0.2.9', signatureVerified: true }).kind).toBe('none');
  });

  it('見つからなければ知らせない', () => {
    expect(updateStateFrom({ current: '0.2.0', found: null, signatureVerified: true }).kind).toBe('none');
  });

  it('署名検証を省く入口が無い（省略すると通らない）', () => {
    // @ts-expect-error signatureVerified は必須。省ける形にすると、そこが抜け道になる
    const s = updateStateFrom({ current: '0.2.0', found: '0.3.0' });
    expect(s.kind).toBe('none');
  });
});
