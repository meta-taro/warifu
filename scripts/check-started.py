#!/usr/bin/env python3
"""**機能を足す commit には、着手を部屋へ出したことを書く**（0.1.56）。

使う側（ほかの席）が待っている口を作っても、作っていることを知らせなければ、
使う側からは「無い」と同じになる。develop に入っていても、版が出るまで誰も知らなかった。
決まりとして伝えてあっても、手順のままでは忘れた日に抜ける。だから commit で止める。

# 何を見るか

件名が `feat` で始まる commit（`feat:` / `feat(…):`）の message に、「着手:」の行が要る。

    着手: #4091            ← 部屋へ着手（口の名前と入る見込みの版）を出した発言の番号
    着手: 頼まれていない（自分で気づいた直し。入ったら部屋へ知らせる）

番号が本当にその発言かは、機械には確かめられない。確かめられるのは書いたかどうかだけである。
それでも、書けないときは出していないと自分で気づける。**`--no-verify` で外さない。**
"""

from __future__ import annotations

import re
import sys

for 口 in (sys.stdout, sys.stderr):
    if hasattr(口, "reconfigure"):
        口.reconfigure(encoding="utf-8", errors="replace")

機能の件名 = re.compile(r"^feat(\([^)]*\))?!?:")
着手の行 = re.compile(r"^着手:\s*(#\d+|頼まれていない\S*.*)$")


def 見る(本文: str) -> str | None:
    """足りなければ、その理由（人が読む文）を返す。足りていれば None。"""
    行たち = [l.strip() for l in 本文.splitlines() if not l.startswith("#")]
    行たち = [l for l in 行たち if l]
    if not 行たち or not 機能の件名.match(行たち[0]):
        return None
    if any(着手の行.match(l) for l in 行たち[1:]):
        return None
    if any(l.startswith("着手:") for l in 行たち[1:]):
        return "「着手:」の後ろは、部屋の発言の番号（#4091 の形）か「頼まれていない（訳）」にしてください"
    return (
        "機能を足す commit（feat）には「着手:」の行が要ります。\n"
        "  着手: #<部屋へ着手・口の名前・入る見込みの版を出した発言の番号>\n"
        "  着手: 頼まれていない（訳）"
    )


def main() -> int:
    if len(sys.argv) < 2:
        print("使い方: check-started.py <commit message のファイル>", file=sys.stderr)
        return 2
    try:
        with open(sys.argv[1], encoding="utf-8", errors="replace") as f:
            本文 = f.read()
    except OSError as e:
        print(f"message を読めません: {e}", file=sys.stderr)
        return 2
    訳 = 見る(本文)
    if 訳 is None:
        return 0
    print(訳, file=sys.stderr)
    return 1


def 自分を試す() -> None:
    assert 見る("fix: 直す\n") is None
    assert 見る("docs: 書く\n") is None
    assert 見る("feat: 口を足す\n") is not None
    assert 見る("feat(mcp): 口を足す\n\n本文\n") is not None
    assert 見る("feat: 口を足す\n\n着手: #4091\n") is None
    assert 見る("feat!: 口を足す\n\n着手: 頼まれていない（自分で気づいた）\n") is None
    assert 見る("feat: 口を足す\n\n着手: 4091\n") is not None
    # 注記の行（git が捨てる）は数えない
    assert 見る("feat: 口を足す\n# 着手: #1\n") is not None
    # 件名でない行の feat は見ない
    assert 見る("chore: 版\n\nfeat: と書いた本文\n") is None


if __name__ == "__main__":
    if sys.argv[1:] == ["--self-test"]:
        自分を試す()
        print("ok")
        sys.exit(0)
    sys.exit(main())
