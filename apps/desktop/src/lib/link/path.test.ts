import { describe, expect, it } from 'vitest';
import { classifyPath, pathFromStats, type RtcStatLike } from './path';

describe('経路の分類（DESIGN.md §4.1 / issues/005 満たすこと 2）', () => {
  it('どちらも中継でなければ「直接」', () => {
    expect(classifyPath('host', 'host')).toBe('direct');
    expect(classifyPath('srflx', 'host')).toBe('direct');
    expect(classifyPath('prflx', 'srflx')).toBe('direct');
  });

  it('どちらか一方でも中継なら「中継」', () => {
    expect(classifyPath('relay', 'host')).toBe('relayed');
    expect(classifyPath('host', 'relay')).toBe('relayed');
    expect(classifyPath('relay', 'relay')).toBe('relayed');
  });

  it('分からないものを「直接」に倒さない', () => {
    // 繋がっていないことを、繋がっているように見せない（DESIGN.md §2 原則 7）
    expect(classifyPath(undefined, 'host')).toBe('unknown');
    expect(classifyPath('host', undefined)).toBe('unknown');
    expect(classifyPath('へんな値', 'host')).toBe('unknown');
  });
});

const pair = (over: Partial<RtcStatLike> = {}): RtcStatLike => ({
  id: 'P1',
  type: 'candidate-pair',
  state: 'succeeded',
  nominated: true,
  localCandidateId: 'L1',
  remoteCandidateId: 'R1',
  ...over,
});
const local = (t: string, id = 'L1'): RtcStatLike => ({ id, type: 'local-candidate', candidateType: t });
const remote = (t: string, id = 'R1'): RtcStatLike => ({ id, type: 'remote-candidate', candidateType: t });

describe('WebRTC の統計から取り出す', () => {
  it('選ばれている組から局所と相手の種別を引く', () => {
    expect(pathFromStats([pair(), local('host'), remote('srflx')])).toBe('direct');
    expect(pathFromStats([pair(), local('relay'), remote('host')])).toBe('relayed');
  });

  it('組が 1 つも無ければ「不明」', () => {
    expect(pathFromStats([])).toBe('unknown');
    expect(pathFromStats([local('host'), remote('host')])).toBe('unknown');
  });

  it('成立していない組は見ない', () => {
    const s = [pair({ state: 'failed' }), local('host'), remote('host')];
    expect(pathFromStats(s)).toBe('unknown');
  });

  it('候補が見つからなければ「不明」（組だけあっても断定しない）', () => {
    expect(pathFromStats([pair()])).toBe('unknown');
    expect(pathFromStats([pair(), local('host')])).toBe('unknown');
  });

  it('複数あるときは nominated を優先する', () => {
    const s = [
      pair({ id: 'P0', nominated: false, localCandidateId: 'L0', remoteCandidateId: 'R0' }),
      pair({ id: 'P1', nominated: true }),
      local('relay', 'L0'),
      remote('relay', 'R0'),
      local('host', 'L1'),
      remote('host', 'R1'),
    ];
    expect(pathFromStats(s)).toBe('direct');
  });
});
