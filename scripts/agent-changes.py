#!/usr/bin/env python3
"""版ごとに何が変わったかを、**git のタグから**書き出す（**D82**）。

`docs/agent/changes.md` を作る。`warifu mcp` の `changes` がこれを返す
（実行ファイルに埋め込むので、配った先に git は要らない）。

**CHANGELOG.md からは作らない。**あれは版ごとに切っていないので、
そこから抜こうとすると空になるか、関係のない所まで拾う。
**タグ間の commit の見出しが、いちばん確かな「変わったこと」である。**

    python3 scripts/agent-changes.py

**タグを打つ前に回す。**回し忘れても CI が配る実行ファイルには新しいものが入る
（`release.yml` がビルドの前にこれを回す）。
"""

from __future__ import annotations

import pathlib
import subprocess
import sys

置き場所 = pathlib.Path("docs/agent/changes.md")
# **出しすぎない。**エージェントが読む物なので、古い所まで全部返す意味は薄い
残す版の数 = 12


def 走らせる(*引数: str) -> str:
    """git を呼ぶ。**文字コードを明示する。**

    `text=True` だけでは、その機械の既定の文字コードで読む ——
    **Windows は cp1252 なので、日本語の commit 見出しで落ちる**
    （2026-09-10 に CI で踏んだ: `'charmap' codec can't decode byte 0x81`）。

    `errors="replace"` にしてあるのは、**1 文字のために全部を落とさない**ため。
    見出しが 1 つ化けても、他の版の記録は残るほうがよい。
    """
    出た = subprocess.run(
        ["git", *引数],
        capture_output=True,
        check=False,
        encoding="utf-8",
        errors="replace",
    )
    if 出た.returncode != 0:
        sys.exit(f"git {' '.join(引数)} が失敗しました: {(出た.stderr or '').strip()}")
    return (出た.stdout or "").strip()


def タグたち() -> list[str]:
    """新しい順のタグ。**日付順**にする（名前順だと alpha.9 が alpha.10 より後になる）。"""
    生 = 走らせる("tag", "--sort=-creatordate", "--list", "v*")
    return [x for x in 生.split("\n") if x]


def 見出したち(前: str | None, 今: str) -> list[str]:
    範囲 = f"{前}..{今}" if 前 else 今
    生 = 走らせる("log", "--no-merges", "--pretty=format:%s", 範囲)
    return [x for x in 生.split("\n") if x]


def いつ(タグ: str) -> str:
    return 走らせる("log", "-1", "--format=%cs", タグ)


def main() -> None:
    順 = タグたち()
    if not 順:
        置き場所.write_text(
            "# 版ごとに変わったこと\n\nまだタグがありません。\n", encoding="utf-8"
        )
        print(f"{置き場所}: タグが無いので空で書きました")
        return

    行 = [
        "# 版ごとに変わったこと",
        "",
        "**git のタグの間にある commit の見出し**をそのまま並べたもの。",
        f"新しいものから {残す版の数} 版まで。**作った文ではなく、実際に入った物の見出しである。**",
        "",
    ]
    for i, タグ in enumerate(順[:残す版の数]):
        # 1 つ古いタグとの差分。いちばん古い所は、そのタグまで全部
        前 = 順[i + 1] if i + 1 < len(順) else None
        見出し = 見出したち(前, タグ)
        行.append(f"## {タグ} — {いつ(タグ)}")
        行.append("")
        if 見出し:
            行 += [f"- {x}" for x in 見出し]
        else:
            # **空を隠さない。**「何も入っていないタグ」も事実である
            行.append("- （このタグに入った commit はありません）")
        行.append("")

    置き場所.parent.mkdir(parents=True, exist_ok=True)
    置き場所.write_text("\n".join(行), encoding="utf-8")
    print(f"{置き場所} を書いた（{min(len(順), 残す版の数)} 版 / タグは全部で {len(順)}）")


if __name__ == "__main__":
    main()
