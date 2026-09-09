import { describe, expect, it } from 'vitest';
import { 席の名札 } from './seat';

describe('席の名札', () => {
  it('名乗っていれば、名前とどこの席かを並べる', () => {
    expect(席の名札('zumen のエージェント', '図面くん')).toBe('図面くん（zumen）');
  });

  it('名乗っていなければ、席そのままにする', () => {
    expect(席の名札('zumen のエージェント', undefined)).toBe('zumen のエージェント');
  });

  it('名前が空なら、席そのままにする', () => {
    expect(席の名札('zumen のエージェント', '')).toBe('zumen のエージェント');
  });

  it('どこの席かは落とさない', () => {
    // 落とすと zumen と git-qa の見分けが付かない（D75）
    expect(席の名札('git-qa のエージェント', '図面くん')).toContain('git-qa');
  });

  it('「のエージェント」で終わらない呼び方でも壊れない', () => {
    expect(席の名札('名乗りのないエージェント', undefined)).toBe('名乗りのないエージェント');
  });
});
