import { describe, expect, it } from 'vitest';
import type { 会話行 } from '$lib/meeting/announce';
import { 参加者を並べる } from './participants';

const 私 = 'MEAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA';
const 主 = 'HOSTBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB';
const 客 = 'GUESTCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC';

const 話 = (顔の種: string, 足す: Partial<会話行> = {}): 会話行 => ({
  who: '',
  body: 'やあ',
  mine: false,
  顔の種,
  ...足す,
});

describe('参加者を並べる', () => {
  it('主催を先頭に、次に自分、あとは名簿の順で並べる', () => {
    const 並び = 参加者を並べる([客, 私, 主], 主, 私, [], []);
    expect(並び.map((p) => [p.key, p.主催, p.自分])).toEqual([
      [主, true, false],
      [私, false, true],
      [客, false, false],
    ]);
  });

  it('自分が主催なら、自分が先頭', () => {
    const 並び = 参加者を並べる([客, 私], 私, 私, [], []);
    expect(並び[0]).toMatchObject({ key: 私, 主催: true, 自分: true });
  });

  it('自分の下には、この機械のエージェントを並べる', () => {
    const 並び = 参加者を並べる([私, 主], 主, 私, ['docs', 'zumen'], []);
    expect(並び.find((p) => p.自分)?.エージェントたち).toEqual(['docs', 'zumen']);
  });

  it('相手の下には、この部屋で話したエージェントを、名札から拾って並べる', () => {
    const 並び = 参加者を並べる([私, 主], 主, 私, [], [
      話('HOSTBBBBBBBB…（受付係 のエージェント）'),
      話('HOSTBBBBBBBB…（連動 のエージェント（docs））'),
      話('HOSTBBBBBBBB…（受付係 のエージェント）'), // 2 度目は足さない
      話(主), // 人の発言は数えない
    ]);
    expect(並び.find((p) => p.key === 主)?.エージェントたち).toEqual([
      '受付係 のエージェント',
      '連動 のエージェント（docs）',
    ]);
  });

  it('名簿に居ない機械の名札や、知らせの行は拾わない', () => {
    const 並び = 参加者を並べる([私, 主], 主, 私, [], [
      話('ZZZZZZZZZZZZ…（よそ のエージェント）'),
      話('HOSTBBBBBBBB…（知らせ）', { system: true }),
    ]);
    expect(並び.flatMap((p) => p.エージェントたち)).toEqual([]);
  });

  it('名簿が空なら、誰も並べない', () => {
    expect(参加者を並べる([], null, 私, ['docs'], [])).toEqual([]);
  });
});
