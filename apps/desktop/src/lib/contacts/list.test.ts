import { describe, expect, it } from 'vitest';

import { 連絡帳を組む, 机の印, 部屋のid, type 素材 } from './list';

const 自分 = 'AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA';
const 相手 = 'BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB';
const もう一人 = 'CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC';

const 素: 素材 = { 自分, 机のAIたち: [], 会議の相手: [], 覚えた: [] };

describe('連絡帳の並び', () => {
  it('いちばん上は「この PC」（自分と、着いているエージェント）', () => {
    // `issues/012`「この PC で会議するとき、私とあなたはセットでしょっていう」
    const [先頭] = 連絡帳を組む({ ...素, 机のAIたち: ['zumen のエージェント'] });
    expect(先頭.title).toBe('contacts.this');
    expect(先頭.行たち.map((r) => r.種類)).toEqual(['自分', 'AI']);
  });

  it('誰も着いていなければ、まとめの行を作らない', () => {
    // **「マイ PC エージェント」という 1 人は居ない**（オーナー・2026-09-08）——
    // 行にすると、それが 1 人に見える。案内は画面側で出す
    const [先頭] = 連絡帳を組む({ ...素, 机のAIたち: [] });
    expect(先頭.行たち.some((r) => r.key === 机の印)).toBe(false);
    expect(先頭.行たち.map((r) => r.種類)).toEqual(['自分']);
  });

  it('着いているエージェントを、1 つずつ行にする', () => {
    // **まとめて「この PC の AI  2」にすると、どれが着いているのか分からない**
    // （2026-09-08 オーナー指摘）
    const [先頭] = 連絡帳を組む({ ...素, 机のAIたち: ['zumen のエージェント', 'git-qa のエージェント'] });
    const ai = 先頭.行たち.filter((r) => r.種類 === 'AI');
    expect(ai.map((r) => r.name)).toEqual(['zumen のエージェント', 'git-qa のエージェント']);
    expect(ai.every((r) => r.いま会議に居る)).toBe(true);
  });

  it('着いているエージェントが居るときは、まとめの行を出さない', () => {
    // **同じものを 2 か所に出さない**
    const [先頭] = 連絡帳を組む({ ...素, 机のAIたち: ['zumen のエージェント'] });
    expect(先頭.行たち.some((r) => r.key === 机の印)).toBe(false);
  });

  it('行ごとに別の印を持つ（押し分けられる）', () => {
    const [先頭] = 連絡帳を組む({ ...素, 机のAIたち: ['a のエージェント', 'b のエージェント'] });
    const 印 = 先頭.行たち.filter((r) => r.種類 === 'AI').map((r) => r.key);
    expect(new Set(印).size).toBe(2);
  });

  it('会議に誰も居なければ、その区画そのものを出さない', () => {
    // **見出しだけが並ぶ画面にしない**
    const 区画 = 連絡帳を組む(素);
    expect(区画.map((s) => s.title)).toEqual(['contacts.this', 'contacts.saved']);
  });

  it('いま会議に居る人を、覚えている相手の中に二度出さない', () => {
    const 区画 = 連絡帳を組む({
      ...素,
      会議の相手: [相手],
      覚えた: [{ key: 相手, label: 'air', has_address: true }],
    });
    const 覚えた = 区画.find((s) => s.title === 'contacts.saved');
    expect(覚えた?.行たち).toHaveLength(0);
    expect(区画.find((s) => s.title === 'contacts.inmeeting')?.行たち).toHaveLength(1);
  });

  it('覚えていない相手は鍵の頭で出す（知っているように見せない）', () => {
    const 区画 = 連絡帳を組む({ ...素, 会議の相手: [相手] });
    const 行 = 区画.find((s) => s.title === 'contacts.inmeeting')?.行たち[0];
    expect(行?.name).toBe('BBBBBBBBBBBB…');
  });

  it('覚えている相手は呼び名の順に並ぶ', () => {
    // **読み込むたびに並びが変わらない**（毎回探させない）
    const 区画 = 連絡帳を組む({
      ...素,
      覚えた: [
        { key: もう一人, label: 'zzz', has_address: false },
        { key: 相手, label: 'aaa', has_address: true },
      ],
    });
    const 覚えた = 区画.find((s) => s.title === 'contacts.saved');
    expect(覚えた?.行たち.map((r) => r.name)).toEqual(['aaa', 'zzz']);
  });

  it('住所を覚えているかを、行が持つ', () => {
    // **これで「呼ぶ」が押せるかが決まる**
    const 区画 = 連絡帳を組む({
      ...素,
      覚えた: [{ key: 相手, label: 'air', has_address: true }],
    });
    expect(区画.find((s) => s.title === 'contacts.saved')?.行たち[0].住所を覚えている).toBe(true);
  });

  it('自分は「この PC」にだけ出て、覚えている相手には出ない', () => {
    const 区画 = 連絡帳を組む({ ...素, 覚えた: [{ key: 相手, label: 'air', has_address: false }] });
    const 覚えた = 区画.find((s) => s.title === 'contacts.saved');
    expect(覚えた?.行たち.some((r) => r.key === 自分)).toBe(false);
  });
});

describe('部屋の一覧', () => {
  it('居る部屋を並べる', () => {
    // **持てても見えなければ切り替えようがない**（2026-09-08）
    const 区画 = 連絡帳を組む({
      ...素,
      部屋たち: [{ id: 'ROOM1AAAAAAAAAAAAAAAA', members: 2, host: true }],
    });
    const 部屋 = 区画.find((s) => s.title === 'contacts.rooms');
    expect(部屋?.行たち).toHaveLength(1);
    expect(部屋?.行たち[0].種類).toBe('部屋');
  });

  it('名前の無いルームは、生の id で呼ばない', () => {
    // **生の id を人に見せない**（DESIGN §10-A）。
    // 「HPXQEPXFKFMA…」では、どのルームか分からない
    const 区画 = 連絡帳を組む({
      自分: 'ME',
      机のAIたち: [],
      会議の相手: [],
      覚えた: [],
      部屋たち: [
        { id: 'ROOM1AAAAAAAAAAAAAAAA', members: 3, host: true },
        { id: 'ROOM2BBBBBBBBBBBBBBBB', members: 1, host: false },
      ],
    });
    const 行たち = 区画.find((s) => s.title === 'contacts.rooms')?.行たち ?? [];
    expect(行たち.map((r) => r.name)).toEqual(['room.nth:1:3', 'room.nth:2:1']);
  });

  it('名前を付けたルームは、その名前で呼ぶ', () => {
    const 区画 = 連絡帳を組む({
      自分: 'ME',
      机のAIたち: [],
      会議の相手: [],
      覚えた: [],
      部屋たち: [{ id: 'ROOM1AAAAAAAAAAAAAAAA', members: 2, host: true }],
      部屋の名前: { ROOM1AAAAAAAAAAAAAAAA: '週次' },
    });
    const 行たち = 区画.find((s) => s.title === 'contacts.rooms')?.行たち ?? [];
    expect(行たち[0].name).toBe('週次');
  });

  it('部屋が無ければ、その区画そのものを出さない', () => {
    // **空の見出しを並べない**
    expect(連絡帳を組む(素).some((s) => s.title === 'contacts.rooms')).toBe(false);
  });

  it('自分しか居ない部屋は、繋がっている扱いにしない', () => {
    const 区画 = 連絡帳を組む({
      ...素,
      部屋たち: [{ id: 'ROOM1AAAAAAAAAAAAAAAA', members: 1, host: true }],
    });
    expect(区画.find((s) => s.title === 'contacts.rooms')?.行たち[0].いま会議に居る).toBe(false);
  });

  it('部屋の印から id を取り出せる', () => {
    expect(部屋のid('room:ROOM1')).toBe('ROOM1');
    expect(部屋のid('desk:zumen の AI')).toBeNull();
  });
});

describe('留守中に届いた相手', () => {
  it('覚えていない相手でも、開く所を出す', () => {
    // **受け取っておいて出さないのは、黙って捨てるのと同じに見える**
    const 区画 = 連絡帳を組む({
      自分: 'ME',
      机のAIたち: [],
      会議の相手: [],
      覚えた: [],
      留守中に届いた: ['XYZ'],
    });
    const 留守 = 区画.find((s) => s.title === 'contacts.late');
    expect(留守?.行たち.map((r) => r.key)).toEqual(['XYZ']);
    // **居場所は知らない。**預かり所ごしに届いただけ
    expect(留守?.行たち[0].住所を覚えている).toBe(false);
  });

  it('覚えている相手は、二度出さない', () => {
    const 区画 = 連絡帳を組む({
      自分: 'ME',
      机のAIたち: [],
      会議の相手: [],
      覚えた: [{ key: 'ABC', label: '田中', has_address: true }],
      留守中に届いた: ['ABC'],
    });
    expect(区画.find((s) => s.title === 'contacts.late')).toBeUndefined();
  });

  it('届いていなければ、区画そのものを出さない', () => {
    const 区画 = 連絡帳を組む({ 自分: 'ME', 机のAIたち: [], 会議の相手: [], 覚えた: [] });
    expect(区画.find((s) => s.title === 'contacts.late')).toBeUndefined();
  });
});

describe('この PC のエージェント', () => {
  it('それぞれが 1 人として並ぶ', () => {
    // **まとめて「この PC の AI」1 行にしない**（オーナー・2026-09-08）
    const 区画 = 連絡帳を組む({
      自分: 'ME',
      机のAIたち: ['zumen のエージェント', 'git-qa のエージェント'],
      会議の相手: [],
      覚えた: [],
    });
    const このPC = 区画.find((s) => s.title === 'contacts.this');
    expect(このPC?.行たち.map((r) => r.name)).toEqual(['contacts.me', 'zumen のエージェント', 'git-qa のエージェント']);
  });

  it('立ち上げていない席も、名乗りがあれば残る', () => {
    // 消えると、その 1 人ぶんの名乗りが編集できなくなる
    const 区画 = 連絡帳を組む({
      自分: 'ME',
      机のAIたち: ['zumen のエージェント'],
      名乗りのある席: ['zumen のエージェント', 'git-qa のエージェント'],
      会議の相手: [],
      覚えた: [],
    });
    const 行たち = 区画.find((s) => s.title === 'contacts.this')?.行たち ?? [];
    expect(行たち.map((r) => r.name)).toEqual(['contacts.me', 'zumen のエージェント', 'git-qa のエージェント']);
    // **着いているかどうかは分ける**
    expect(行たち.find((r) => r.name === 'zumen のエージェント')?.いま会議に居る).toBe(true);
    expect(行たち.find((r) => r.name === 'git-qa のエージェント')?.いま会議に居る).toBe(false);
  });

  it('同じ席を二度出さない', () => {
    const 区画 = 連絡帳を組む({
      自分: 'ME',
      机のAIたち: ['zumen のエージェント'],
      名乗りのある席: ['zumen のエージェント'],
      会議の相手: [],
      覚えた: [],
    });
    const 行たち = 区画.find((s) => s.title === 'contacts.this')?.行たち ?? [];
    expect(行たち.filter((r) => r.name === 'zumen のエージェント').length).toBe(1);
  });

  it('誰も着いていなければ、人の行を出さない', () => {
    // **「マイ PC エージェント」という 1 人は居ない。**
    // 行にすると、それが 1 人に見える（オーナー・2026-09-08）
    const 区画 = 連絡帳を組む({ 自分: 'ME', 机のAIたち: [], 会議の相手: [], 覚えた: [] });
    const 行たち = 区画.find((s) => s.title === 'contacts.this')?.行たち ?? [];
    expect(行たち.map((r) => r.name)).toEqual(['contacts.me']);
  });
});

describe('部屋の名前', () => {
  it('付けた名前で呼ぶ', () => {
    // **「どの部屋？」と人が思う**（オーナー・2026-09-08）
    const 区画 = 連絡帳を組む({
      自分: 'ME',
      机のAIたち: [],
      会議の相手: [],
      覚えた: [],
      部屋たち: [{ id: 'ROOM-ABCDEF', members: 2, host: true }],
      部屋の名前: { 'ROOM-ABCDEF': '朝会' },
    });
    const 部屋 = 区画.find((s) => s.title === 'contacts.rooms');
    expect(部屋?.行たち[0].name).toBe('朝会');
  });

  it('付いていなければ、数えて呼ぶ（生の id は出さない）', () => {
    // **生の id を人に見せない**（DESIGN §10-A）。
    // それまでは id の頭（`ROOMABCD…`）を出していたが、**どのルームか分からない**
    const 区画 = 連絡帳を組む({
      自分: 'ME',
      机のAIたち: [],
      会議の相手: [],
      覚えた: [],
      部屋たち: [{ id: 'ROOMABCDEFGHIJKL', members: 1, host: true }],
    });
    const 部屋 = 区画.find((s) => s.title === 'contacts.rooms');
    expect(部屋?.行たち[0].name).toBe('room.nth:1:1');
    expect(部屋?.行たち[0].name).not.toContain('ROOM');
  });
});
