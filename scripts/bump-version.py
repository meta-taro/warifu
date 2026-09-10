#!/usr/bin/env python3
"""版を上げる（**D85**）。

    python3 scripts/bump-version.py 0.1.1

**版が上がらないと、自動アップデートは一度も降りない。**
`latest.json` に書く版はタグから作るが、**アプリが名乗る版はここから来る** ——
`0.1.0` のままだと「同じか古い」と判定されて、更新が出ない
（semver では `0.1.0-alpha.19` は `0.1.0` より**古い**）。

書き換える所は 3 つ。**1 か所でも取り残すと、どこかで食い違う。**

    Cargo.toml                       [workspace.package] version
    apps/desktop/src-tauri/Cargo.toml   package version
    apps/desktop/src-tauri/tauri.conf.json  version

`Cargo.lock` も一緒に直す（`--locked` の CI で弾かれるため）。
"""

from __future__ import annotations

import pathlib
import re
import subprocess
import sys

for 口 in (sys.stdout, sys.stderr):
    if hasattr(口, "reconfigure"):
        口.reconfigure(encoding="utf-8", errors="replace")


def 置き換える(場所: str, 型: str, 新しい: str) -> None:
    p = pathlib.Path(場所)
    元 = p.read_text(encoding="utf-8")
    直した, 数 = re.subn(型, 新しい, 元, count=1)
    if 数 != 1:
        sys.exit(f"{場所}: 版の行が見つかりません（{型}）")
    p.write_text(直した, encoding="utf-8")
    print(f"{場所} を直した")


def main() -> None:
    if len(sys.argv) != 2:
        sys.exit("使い方: python3 scripts/bump-version.py <新しい版>（例 0.1.1）")
    新しい版 = sys.argv[1].lstrip("vV")
    if not re.fullmatch(r"\d+\.\d+\.\d+", 新しい版):
        # **前置き（-alpha.1）を受けない。**受けると semver で「古い」判定になり、
        # 更新が降りない —— まさにこれで踏んだ
        sys.exit(f"数だけの版にしてください（例 0.1.1）。受けたもの: {新しい版}")

    置き換える("Cargo.toml", r'(?m)^version = "[^"]+"', f'version = "{新しい版}"')
    置き換える(
        "apps/desktop/src-tauri/Cargo.toml",
        r'(?m)^version = "[^"]+"',
        f'version = "{新しい版}"',
    )
    置き換える(
        "apps/desktop/src-tauri/tauri.conf.json",
        r'"version": "[^"]+"',
        f'"version": "{新しい版}"',
    )

    # **Cargo.lock も直す。**CI は `--locked` なので、置いていくと弾かれる
    出た = subprocess.run(
        ["cargo", "update", "-w", "--offline"],
        capture_output=True,
        check=False,
        encoding="utf-8",
        errors="replace",
    )
    if 出た.returncode != 0:
        print("Cargo.lock を直せませんでした（手元で cargo check を回してください）")
        print((出た.stderr or "").strip()[:300])
    else:
        print("Cargo.lock も直した")

    print()
    print(f"次は: git commit → git tag v{新しい版} → git push origin v{新しい版}")


if __name__ == "__main__":
    main()
