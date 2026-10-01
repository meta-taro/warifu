#!/usr/bin/env python3
"""**GitHub Actions の段の順番を検める。**

    python3 scripts/check-step-order.py

**まだ無い値を見る `if:` は、黙って false になる。**段ごと飛ばされ、
しかも job は success になる —— **飛んだことが誰にも見えない。**

2026-09-12 に踏んだ（`v0.1.3`）。`release.yml` の「CLI に署名する」が

    if: steps.creds.outputs.ready == 'true'

と書いてあるのに、`id: creds` の段は**その 60 行あとにあった。**
条件は空文字と比べられて false になり、**署名の段は一度も走らないまま**
ad-hoc の CLI が 2 版続けて配られた（D96 は効いていなかった）。

`actionlint` は入れていないので**ここで見る。**見るのは 1 つだけ ——
**`steps.<id>.outputs` を、その `id:` を定義する段より前で読んでいないか。**
"""

from __future__ import annotations

import pathlib
import re
import sys

for 口 in (sys.stdout, sys.stderr):
    if hasattr(口, "reconfigure"):
        口.reconfigure(encoding="utf-8", errors="replace")

# `jobs:` の直下（2 字下げ）にある鍵が job の名前
JOB = re.compile(r"^  ([A-Za-z0-9_-]+):\s*$")
# job の直下（4 字下げ）の鍵。`steps:` に入ったかどうかを見るため
JOB鍵 = re.compile(r"^    ([A-Za-z0-9_-]+):")
ID = re.compile(r"^\s*id:\s*([A-Za-z0-9_-]+)\s*$")
参照 = re.compile(r"steps\.([A-Za-z0-9_-]+)\.outputs")


def 検める(道: pathlib.Path) -> list[str]:
    """**まだ定義されていない `id` を読んでいる行**を返す。"""
    苦情: list[str] = []
    jobsの中 = False
    段の中 = False
    job = ""
    済み: set[str] = set()
    for 番, 行 in enumerate(道.read_text(encoding="utf-8").splitlines(), 1):
        if 行.rstrip() == "jobs:":
            jobsの中 = True
            continue
        if not jobsの中:
            continue
        if m := JOB.match(行):
            job, 済み, 段の中 = m.group(1), set(), False
            continue
        # **見るのは段の中だけ。**job の `environment.url` が
        # `steps.deploy.outputs.page_url` を読むのは Pages の定石で、
        # これは段が全部終わってから評価される（飛ばされる話ではない）
        if m := JOB鍵.match(行):
            段の中 = m.group(1) == "steps"
            continue
        if not 段の中:
            continue
        # **定義を先に数える。**同じ行に両方は書けないので順番は問わない
        if m := ID.match(行):
            済み.add(m.group(1))
            continue
        for 名 in 参照.findall(行):
            if 名 not in 済み:
                苦情.append(
                    f"{道}:{番}: job `{job}` が、まだ定義されていない "
                    f"`steps.{名}.outputs` を読んでいます\n    {行.strip()}"
                )
    return 苦情


def main() -> None:
    場所 = pathlib.Path(".github/workflows")
    if not 場所.is_dir():
        sys.exit(f"{場所} が見つかりません（リポジトリの根で実行してください）")
    苦情: list[str] = []
    数 = 0
    for 道 in sorted(場所.glob("*.yml")) + sorted(場所.glob("*.yaml")):
        数 += 1
        苦情 += 検める(道)
    if 苦情:
        print("段の順番が壊れています:", file=sys.stderr)
        for ひとつ in 苦情:
            print(f"  {ひとつ}", file=sys.stderr)
        print(
            "\n**`id:` を定義する段を、それを読む段より前へ動かしてください。**",
            file=sys.stderr,
        )
        sys.exit(1)
    print(f"段の順番: ok（{数} 個の workflow を見ました）")


if __name__ == "__main__":
    main()
