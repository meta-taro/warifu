import { describe, expect, it } from 'vitest';
import { 席の顔の画, 相手の顔の鍵, 行の顔の画 } from './peer-face';
import { 場所の帳の鍵 } from './name';

const 鍵A = 'AAAAAAAAAAAABBBBBBBBBBBBCCCCCCCCCCCCDDDDDDDDDDDDEEEE';
const 鍵B = 'AAAAAAAAAAAAZZZZZZZZZZZZCCCCCCCCCCCCDDDDDDDDDDDDEEEE';

describe('相手の顔', () => {
  it('人は鍵そのまま・席は鍵と場所で引く', () => {
    expect(相手の顔の鍵(鍵A, '')).toBe(鍵A);
    expect(相手の顔の鍵(鍵A, 'docs')).toBe(`${鍵A}\ndocs`);
  });

  it('この端末と相手の人の顔は、種でそのまま引く', () => {
    const 顔の画 = { me: 'blob:me', 'desk:docs のエージェント': 'blob:desk', [鍵A]: 'blob:a' };
    expect(行の顔の画('desk:docs のエージェント', 顔の画, {})).toBe('blob:desk');
    expect(行の顔の画(鍵A, 顔の画, {})).toBe('blob:a');
    expect(行の顔の画(鍵B, 顔の画, {})).toBeNull();
  });

  it('相手の席は、話し手の場所の帳で場所を引いてから引く（名乗り直しても同じ顔）', () => {
    const 顔の画 = { [相手の顔の鍵(鍵A, 'docs')]: 'blob:seat' };
    const 帳 = {
      [場所の帳の鍵(鍵A, '資料くん（docs）')]: 'docs',
      [場所の帳の鍵(鍵A, '新しい名')]: 'docs',
    };
    expect(行の顔の画('AAAAAAAAAAAABB…（資料くん（docs））', 顔の画, 帳)).toBe('blob:seat');
    expect(行の顔の画('AAAAAAAAAAAABB…（新しい名）', 顔の画, 帳)).toBe('blob:seat');
  });

  it('場所が届いていない・頭が 2 台に当たるときは、取り違えないよう出さない', () => {
    const 顔の画 = { [相手の顔の鍵(鍵A, 'docs')]: 'blob:a', [相手の顔の鍵(鍵B, 'docs')]: 'blob:b' };
    expect(行の顔の画('AAAAAAAAAAAABB…（知らない札）', 顔の画, {})).toBeNull();
    const 帳 = { [場所の帳の鍵(鍵A, '札')]: 'docs', [場所の帳の鍵(鍵B, '札')]: 'docs' };
    expect(行の顔の画('AAAAAAAAAAAA…（札）', 顔の画, 帳)).toBeNull();
  });
});

describe('名簿の席の顔（このルームに入っている）', () => {
  const 鍵 = 'A'.repeat(52);
  it('この機械の席は desk:<場所> で引く', () => {
    expect(席の顔の画(鍵, 'docs', true, { 'desk:docs': 'blob:1' }, {})).toBe('blob:1');
  });
  it('相手の機械の席は、名札から場所を引いてから引く', () => {
    const 帳 = { [場所の帳の鍵(鍵, '連動（docs）')]: 'docs' };
    const 画 = { [相手の顔の鍵(鍵, 'docs')]: 'blob:2' };
    expect(席の顔の画(鍵, '連動（docs）', false, 画, 帳)).toBe('blob:2');
  });
  it('場所が届いていなければ出さない（取り違えない）', () => {
    expect(席の顔の画(鍵, '連動（docs）', false, { [相手の顔の鍵(鍵, 'docs')]: 'blob:2' }, {})).toBeNull();
  });
});
