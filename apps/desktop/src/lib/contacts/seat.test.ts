import { describe, expect, it } from 'vitest';
import { エージェントの名札 } from './seat';

describe('エージェントの名札', () => {
  it('名乗っていれば、名前とどこのエージェントかを並べる', () => {
    expect(エージェントの名札('zumen のエージェント', '図面くん')).toBe('図面くん（zumen）');
  });

  it('名乗っていなければ、エージェントそのままにする', () => {
    expect(エージェントの名札('zumen のエージェント', undefined)).toBe('zumen のエージェント');
  });

  it('名前が空なら、エージェントそのままにする', () => {
    expect(エージェントの名札('zumen のエージェント', '')).toBe('zumen のエージェント');
  });

  it('どこのエージェントかは落とさない', () => {
    // 落とすと zumen と git-qa の見分けが付かない（D75）
    expect(エージェントの名札('git-qa のエージェント', '図面くん')).toContain('git-qa');
  });

  it('「のエージェント」で終わらない呼び方でも壊れない', () => {
    expect(エージェントの名札('名乗りのないエージェント', undefined)).toBe('名乗りのないエージェント');
  });
});
