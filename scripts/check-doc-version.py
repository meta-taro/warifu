#!/usr/bin/env python3
"""**受け取る人が読む手順が、いまの版を指しているか**（2026-09-18）。

**2026-09-18、`docs/install*.md` が v0.1.8 のままだった**（実際は 0.1.11）。
**落とすファイルの名前まで古い**ので、**そのまま打つと見つからない。**

```
gh release download v0.1.8 -p 'warifu_*_x64-setup.exe'   ← **もう無い**
```

**人が気をつける方式は、必ず漏れる**（baseline §25 と同じ筋）。
`scripts/bump-version.py` が直すが、**直ったかを見る目はここに置く。**

**履歴の版は見ない** ——「v0.1.5 から署名済み」は、そのときの事実である。
見るのは「**いまの版が、手順のどこかに書いてあるか**」だけ。
"""

from __future__ import annotations

import pathlib
import re
import sys

for 口 in (sys.stdout, sys.stderr):
    if hasattr(口, "reconfigure"):
        口.reconfigure(encoding="utf-8", errors="replace")

設定 = pathlib.Path("apps/desktop/src-tauri/tauri.conf.json")
見つけた = re.search(r'"version": "([^"]+)"', 設定.read_text(encoding="utf-8"))
if not 見つけた:
    sys.exit(f"{設定}: version の行が読めません")
いまの版 = 見つけた.group(1)

落ちた: list[str] = []
for 名 in ("docs/install.md", "docs/install.ja.md"):
    場所 = pathlib.Path(名)
    if not 場所.exists():
        continue
    中 = 場所.read_text(encoding="utf-8")
    if いまの版 not in 中:
        # **どの版を指しているかまで出す。**「違う」だけでは直しようがない
        版たち = sorted(set(re.findall(r"\d+\.\d+\.\d+", 中)))
        落ちた.append(f"  {名}: いまの版 {いまの版} が出てきません（書いてある版: {', '.join(版たち) or 'なし'}）")

# **MCP の公式一覧に出す版**（0.1.43・`server.json`）。ずれると、一覧が別の版を指す
# （出し直しの workflow はタグと食い違えば止まるが、止まる前に気づけるようにここでも見る）
一覧 = pathlib.Path("server.json")
if 一覧.exists():
    一覧の版 = re.search(r'"version": "([^"]+)"', 一覧.read_text(encoding="utf-8"))
    if not 一覧の版 or 一覧の版.group(1) != いまの版:
        落ちた.append(
            f"  server.json: MCP の一覧に出す版が {一覧の版.group(1) if 一覧の版 else '読めない'} です（いまの版 {いまの版}）"
        )

if 落ちた:
    print("受け取る人が読む手順が、いまの版を指していません:")
    print("\n".join(落ちた))
    print("  → python3 scripts/bump-version.py <版> で直ります")
    sys.exit(1)
print(f"手順の版: {いまの版} ok")
