import { describe, expect, it, vi } from 'vitest';

import { 通話を置く } from './shelf';

describe('通話の棚', () => {
  it('同じ相手の別の通話が棚にあれば、閉じてから置く（閉じずに上書きしない）', () => {
    // 11-12、入り直しが重なると、先の通話が閉じられないまま残り、UDP の口が溜まっていた
    const 棚 = new Map<string, { close: () => void }>();
    const 先 = { close: vi.fn() };
    const 後 = { close: vi.fn() };
    通話を置く(棚, 'A', 先);
    通話を置く(棚, 'A', 後);
    expect(先.close).toHaveBeenCalledTimes(1);
    expect(後.close).not.toHaveBeenCalled();
    expect(棚.get('A')).toBe(後);
    通話を置く(棚, 'A', 後);
    expect(後.close).not.toHaveBeenCalled();
  });
});
