import { describe, expect, it } from 'vitest';
import {
  伏せる,
  同じ網に居るか,
  候補を読む,
  候補を言い表す,
  対を言い表す,
  数えて言い表す,
  組の様子,
  送り受けを言い表す,
  映像の向き,
} from './trace';

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

describe('組の様子', () => {
  it('**組ごとに、様子と、候補が引けたかを出す。**`connected` なのに画面が unknown のとき、ここしか手掛かりが無い', () => {
    const 統計 = [
      { id: 'p1', type: 'candidate-pair', state: 'succeeded', nominated: true, localCandidateId: 'l1', remoteCandidateId: 'r9' },
      { id: 'l1', type: 'local-candidate', candidateType: 'host' },
    ];
    // r9 が統計に無い＝**相手の候補が引けない**。これだと経路を決められない
    expect(組の様子(統計)).toEqual(['組 1 succeeded 選ばれた／こちら host／あちら 引けません']);
  });

  it('両方引ければ、両方の種別を出す', () => {
    const 統計 = [
      { id: 'p1', type: 'candidate-pair', state: 'in-progress', localCandidateId: 'l1', remoteCandidateId: 'r1' },
      { id: 'l1', type: 'local-candidate', candidateType: 'host' },
      { id: 'r1', type: 'remote-candidate', candidateType: 'srflx' },
    ];
    expect(組の様子(統計)).toEqual(['組 1 in-progress／こちら host／あちら srflx']);
  });

  it('組が無ければ、空で返す（無いことを言うのは呼ぶ側）', () => {
    expect(組の様子([])).toEqual([]);
  });
});

describe('送り受けを言い表す', () => {
  it('**送っているのか、受けているのか**を数で出す', () => {
    // 2026-09-15、Windows の映像が mac に出ない。**経路は direct、文字は通る。**
    // **送っていないのか、送っているのに映らないのか**が、記録から読めなかった
    const 統計 = [
      { id: 'o1', type: 'outbound-rtp', kind: 'video', packetsSent: 0 },
      { id: 'o2', type: 'outbound-rtp', kind: 'audio', packetsSent: 132 },
      { id: 'i1', type: 'inbound-rtp', kind: 'video', packetsReceived: 480 },
      { id: 'i2', type: 'inbound-rtp', kind: 'audio', packetsReceived: 120 },
    ];
    // **音の積もりが「不明」と付く**（2026-09-17）——
    // この統計は `totalAudioEnergy` を持っていないので、**「無音」とは言えない**
    expect(送り受けを言い表す(統計)).toBe(
      '送り 映像 0 / 音 132（音の積もり 不明） ／ 受け 映像 480 / 音 120（音の積もり 不明）',
    );
  });

  it('枠が無ければ「なし」と言う（0 と混ぜない）', () => {
    // **「送る枠が無い」と「送っているが 0 個」は別の話。**混ぜると切り分けられない
    expect(送り受けを言い表す([])).toBe('送り なし ／ 受け なし');
  });

  it('片側だけ枠があるときは、その側だけ数える', () => {
    const 統計 = [{ id: 'i1', type: 'inbound-rtp', kind: 'video', packetsReceived: 7 }];
    expect(送り受けを言い表す(統計)).toBe('送り なし ／ 受け 映像 7 / 音 なし');
  });
});

describe('映像の向き（#39）', () => {
  it('**受けているのに送っていない**を、そのまま返す', () => {
    // 2026-09-16、使った人 ——「入った瞬間相手には映っているのに、
    // こっちがビデオ会議を始めるボタン押さないと見れないのはおかしい、最悪の UX です」
    const 統計 = [
      { id: 'o', type: 'outbound-rtp', kind: 'video', packetsSent: 0 },
      { id: 'i', type: 'inbound-rtp', kind: 'video', packetsReceived: 89 },
    ];
    expect(映像の向き(統計)).toEqual({ 送っている: false, 受けている: true });
  });

  it('両方流れていれば、両方 true', () => {
    const 統計 = [
      { id: 'o', type: 'outbound-rtp', kind: 'video', packetsSent: 355863 },
      { id: 'i', type: 'inbound-rtp', kind: 'video', packetsReceived: 350072 },
    ];
    expect(映像の向き(統計)).toEqual({ 送っている: true, 受けている: true });
  });

  it('**枠が無いのと 0 個を、同じに扱う**（どちらも「流れていない」）', () => {
    // 言い分けるのは `送り受けを言い表す` の仕事。**画面へ出す判断はここで丸める**
    expect(映像の向き([])).toEqual({ 送っている: false, 受けている: false });
    expect(
      映像の向き([{ id: 'o', type: 'outbound-rtp', kind: 'video', packetsSent: 0 }]),
    ).toEqual({ 送っている: false, 受けている: false });
  });

  it('音は見ない（映像の向きだけ）', () => {
    const 統計 = [{ id: 'o', type: 'outbound-rtp', kind: 'audio', packetsSent: 900 }];
    expect(映像の向き(統計)).toEqual({ 送っている: false, 受けている: false });
  });
});

describe('同じ網に居るか（ハウリングの手がかり）', () => {
  const 候補 = (id: string, address: string) => ({
    id,
    type: 'local-candidate' as const,
    address,
  });
  const 組 = (l: string, r: string) => ({
    id: 'p1',
    type: 'candidate-pair' as const,
    state: 'succeeded',
    nominated: true,
    localCandidateId: l,
    remoteCandidateId: r,
  });

  it('同じ網なら、そう言う', () => {
    // **机の隣に別の端末が在る形。**エコー除去では消せない
    const 統計 = [
      組('a', 'b'),
      候補('a', '192.168.24.11'),
      { ...候補('b', '192.168.24.16'), type: 'remote-candidate' as const },
    ];
    expect(同じ網に居るか(統計 as never)).toBe(true);
  });

  it('別の網なら、そう言う', () => {
    const 統計 = [
      組('a', 'b'),
      候補('a', '192.168.24.11'),
      { ...候補('b', '10.0.5.9'), type: 'remote-candidate' as const },
    ];
    expect(同じ網に居るか(統計 as never)).toBe(false);
  });

  it('組が無ければ null —— 「まだ分からない」を「別の網」と言わない', () => {
    expect(同じ網に居るか([])).toBe(null);
  });

  it('mDNS の名前で来たら null —— 比べられないものを比べない', () => {
    const 統計 = [
      組('a', 'b'),
      候補('a', 'f1b2ba37-e746-4426-ad1a-f7762754b545.local'),
      { ...候補('b', '192.168.24.16'), type: 'remote-candidate' as const },
    ];
    expect(同じ網に居るか(統計 as never)).toBe(null);
  });

  it('片方の候補が引けなければ null', () => {
    expect(同じ網に居るか([組('a', 'b'), 候補('a', '192.168.24.11')] as never)).toBe(null);
  });
});

describe('音の積もり（本数では黙っているか分からない）', () => {
  const 音 = (向き: 'outbound-rtp' | 'inbound-rtp', 本数: number, 積もり?: number) => ({
    id: `a-${向き}`,
    type: 向き,
    kind: 'audio',
    packetsSent: 本数,
    packetsReceived: 本数,
    totalAudioEnergy: 積もり,
  });

  it('**無音なら、そう言う**（本数が出ていても）', () => {
    // **2026-09-17 の本題。**`track.enabled = false` でも音は無音として送られ続け、
    // **本数は減らない。**`送り 音 121` を見て「まだ漏れている」と読みかけた
    const 出た = 送り受けを言い表す([音('outbound-rtp', 121, 0)] as never);
    expect(出た).toContain('音 121');
    expect(出た).toContain('無音');
  });

  it('声が出ていれば、積もりが出る', () => {
    const 出た = 送り受けを言い表す([音('outbound-rtp', 121, 0.0034)] as never);
    expect(出た).toContain('音の積もり');
    expect(出た).not.toContain('無音');
  });

  it('音の行が無ければ、積もりも言わない', () => {
    // **「無い」を「0」と言わない**（`なし` と `0` を分けたのと同じ筋）
    const 出た = 送り受けを言い表す([
      { id: 'v', type: 'outbound-rtp', kind: 'video', packetsSent: 5 },
    ] as never);
    expect(出た).not.toContain('音の積もり');
  });

  it('**出していない版には「不明」と言う。「無音」と言わない**', () => {
    // 古い版は `totalAudioEnergy` を出さない。
    // **「出していない」を「0」と読むと、喋っているのに「無音」と書くことになる。**
    // （`なし` と `0` を分けたのと、まったく同じ理由）
    const 出た = 送り受けを言い表す([音('outbound-rtp', 121, undefined)] as never);
    expect(出た).toContain('音 121');
    expect(出た).toContain('不明');
    expect(出た).not.toContain('無音');
  });
});

describe('手元の様子（`なし` の意味を言い分ける）', () => {
  const 音 = { id: 'a', type: 'outbound-rtp', kind: 'audio', packetsSent: 5, totalAudioEnergy: 0.1 };

  it('**掴んでいるのに外してあるなら、そう言う**', () => {
    // **2026-09-18・ASUS の指摘。**`replaceTrack(null)` は行ごと消すので、
    // **「枠が無い」と「外してある」が同じ `なし` になる。**
    // **外から音量計で測らせていた** —— 道具が自分で言う
    const 出た = 送り受けを言い表す([], { 掴んでいる: true, 外してある: true });
    expect(出た).toContain('送り なし');
    expect(出た).toContain('外してある');
  });

  it('掴んでいないなら、そう言う', () => {
    const 出た = 送り受けを言い表す([], { 掴んでいる: false, 外してある: false });
    expect(出た).toContain('機器を掴んでいない');
    expect(出た).not.toContain('外してある');
  });

  it('掴んでいて付いているなら、そう言う', () => {
    const 出た = 送り受けを言い表す([音] as never, { 掴んでいる: true, 外してある: false });
    expect(出た).toContain('送り手に付いている');
  });

  it('手元を渡さなければ、何も足さない（古い呼び方を壊さない）', () => {
    const 出た = 送り受けを言い表す([音] as never);
    expect(出た).not.toContain('【');
  });
});
