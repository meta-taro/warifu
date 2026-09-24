#!/usr/bin/env python3
"""**画面を触ったなら、実物を見たことを commit へ書く**（2026-09-24・オーナー指示）。

オーナー ——

    **タブ切り替えはいいけど、見てくれチェックが抜けていると思うので、
    ワークフロー見直してください。**

**規則には書いてあった**（`CLAUDE.md`「人に見せる前に、同じ状態を自分で作って見る」）。
**工程には入っていなかった。**——だから同じ指摘を 3 回させた
（v0.1.14 / v0.1.15 / v0.1.16・`gh issue 41`）。

# 何を見るか

**画面の見た目に関わるファイル**（`.svelte` / `.css`）が commit に入っているなら、
**message に「見た:」の行が要る。**

    見た: 入る前と入ったあとの両方を撮った。打つ欄が枠つきで出る

**「見たこと」を機械は確かめられない。**確かめられるのは**書いたかどうか**だけである。
それでも工程に置く意味がある ——

1. **撮る前に commit できない**ので、**撮る手が順番に入る**
2. **何を見たかが記録に残る**（あとから「どの状態で見たのか」を追える）
3. **書けないときは、見ていない**（自分で気づく）

**`--no-verify` で外さない**（baseline §18）。**見ていないなら、そう書く** ——

    見た: 見ていない（文言だけの直しで、配置は触っていない）

# 何を見ないか

**中身だけの変更は見なくてよい**（`.ts` の純ロジック・Rust・文書）。
**`.svelte` でも、`<style>` を触っていないなら**……までは見ない。
**触った行で判断すると、判断そのものが外れる**ので、**拡張子で切る。**
"""

from __future__ import annotations

import subprocess
import sys

for 口 in (sys.stdout, sys.stderr):
    if hasattr(口, "reconfigure"):
        口.reconfigure(encoding="utf-8", errors="replace")

見る拡張子 = (".svelte", ".css")
見る場所 = "apps/desktop/src/"
印 = "見た:"


def 触った画面のファイル() -> list[str]:
    出 = subprocess.run(
        ["git", "diff", "--cached", "--name-only", "--diff-filter=ACMR"],
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        check=False,
    )
    return [
        名
        for 名 in 出.stdout.splitlines()
        if 名.startswith(見る場所) and 名.endswith(見る拡張子)
    ]


def main() -> int:
    if len(sys.argv) < 2:
        print("使い方: check-screen-seen.py <commit message のファイル>", file=sys.stderr)
        return 2
    触った = 触った画面のファイル()
    if not 触った:
        return 0
    try:
        with open(sys.argv[1], encoding="utf-8", errors="replace") as f:
            本文 = f.read()
    except OSError as e:
        print(f"message を読めません: {e}", file=sys.stderr)
        return 2
    # **注記の行は数えない**（`#` で始まる行は git が捨てる）
    行たち = [l.strip() for l in 本文.splitlines() if not l.startswith("#")]
    if any(l.startswith(印) for l in 行たち):
        return 0
    print("画面のファイルを触っているのに、**見たことが書いてありません**。", file=sys.stderr)
    for 名 in 触った[:8]:
        print(f"    {名}", file=sys.stderr)
    if len(触った) > 8:
        print(f"    …ほか {len(触った) - 8} 件", file=sys.stderr)
    print("", file=sys.stderr)
    print("**実物を撮ってから、message に 1 行足してください。**", file=sys.stderr)
    print("    見た: 入る前と入ったあとの両方を撮った。打つ欄が枠つきで出る", file=sys.stderr)
    print("", file=sys.stderr)
    print("**見ていないなら、そう書く** ——", file=sys.stderr)
    print("    見た: 見ていない（文言だけの直しで、配置は触っていない）", file=sys.stderr)
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
