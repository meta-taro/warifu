import { describe, expect, it } from 'vitest';
import { 伏せる, 候補を読む, 候補を言い表す, 対を言い表す, 数えて言い表す } from './trace';

describe('伏せる', () => {
  it('IPv4 は、網は残して機械だけ隠す', () => {
    // **同じ網に居るかどうかが、切り分けでいちばん効く。**
    // 末尾まで出すと、公開の issue に貼ったときに機械が特定できる
    expect(伏せる('192.168.24.11')).toBe('192.168.24.x');
  });

  it('IPv6 は頭の 2 組だけ残す', () => {
    expect(伏せる('2001:db8:85a3:8d3:1319:8a2e:370:7348')).toBe('2001:db8:…');
  });

  it('mDNS の名前は、そのまま出す（もともと誰のものか分からない）', () => {
    expect(伏せる('a1b2c3d4-0000-1111-2222-333344445555.local')).toBe(
      'a1b2c3d4-0000-1111-2222-333344445555.local',
    );
  });
});

describe('候補を読む', () => {
  const 生 = 'candidate:842163049 1 udp 1686052607 192.168.24.11 58488 typ host generation 0';

  it('種類・手・網・口を取り出す', () => {
    expect(候補を読む(生)).toEqual({
      種類: 'host',
      手: 'udp',
      網: '192.168.24.x',
      口: 58488,
      名前で来たか: false,
    });
  });

  it('mDNS の候補は、名前で来たと分かる', () => {
    // **これが分かると、mDNS が塞がれている機械を疑える**
    const m = 'candidate:1 1 udp 2113937151 abcd-0000.local 51234 typ host generation 0';
    expect(候補を読む(m).名前で来たか).toBe(true);
  });

  it('読めないものは、読めないと言う（推測しない）', () => {
    expect(候補を読む('こわれています')).toEqual({
      種類: '不明',
      手: '不明',
      網: '不明',
      口: null,
      名前で来たか: false,
    });
  });
});

describe('候補を言い表す', () => {
  it('送った候補', () => {
    const 生 = 'candidate:1 1 udp 2113937151 192.168.24.11 58488 typ host generation 0';
    expect(候補を言い表す('送る', 生)).toBe('候補 送る host udp 192.168.24.x:58488');
  });

  it('来た候補', () => {
    const 生 = 'candidate:2 1 udp 1686052607 198.51.100.7 40404 typ srflx generation 0';
    expect(候補を言い表す('来た', 生)).toBe('候補 来た srflx udp 198.51.100.x:40404');
  });

  it('mDNS なら、そう書き添える', () => {
    const 生 = 'candidate:1 1 udp 2113937151 abcd-0000.local 51234 typ host generation 0';
    expect(候補を言い表す('送る', 生)).toBe(
      '候補 送る host udp abcd-0000.local:51234（mDNS の名前）',
    );
  });
});

describe('対を言い表す', () => {
  const 統計 = [
    { id: 'p1', type: 'candidate-pair', state: 'succeeded', nominated: true, localCandidateId: 'l1', remoteCandidateId: 'r1' },
    { id: 'l1', type: 'local-candidate', candidateType: 'host', protocol: 'udp', address: '192.168.24.11', port: 58488 },
    { id: 'r1', type: 'remote-candidate', candidateType: 'host', protocol: 'udp', address: '192.168.24.9', port: 51111 },
  ];

  it('選ばれた組を、両側とも出す', () => {
    expect(対を言い表す(統計)).toBe('選ばれた組 host udp 192.168.24.x:58488 ↔ host udp 192.168.24.x:51111');
  });

  it('組が無いときは何も言わない（黙る）', () => {
    expect(対を言い表す([])).toBeNull();
  });
});

describe('数えて言い表す', () => {
  it('送った数・来た数・組の様子を数える', () => {
    const 統計 = [
      { id: 'l1', type: 'local-candidate' },
      { id: 'l2', type: 'local-candidate' },
      { id: 'r1', type: 'remote-candidate' },
      { id: 'p1', type: 'candidate-pair', state: 'failed' },
      { id: 'p2', type: 'candidate-pair', state: 'in-progress' },
    ];
    expect(数えて言い表す(統計)).toBe('候補 送った 2 / 来た 1 ／ 組 2（成立 0・試し中 1・だめ 1）');
  });

  it('来た候補が 0 なら、それが分かる', () => {
    const 統計 = [{ id: 'l1', type: 'local-candidate' }];
    expect(数えて言い表す(統計)).toBe('候補 送った 1 / 来た 0 ／ 組 0（成立 0・試し中 0・だめ 0）');
  });
});
