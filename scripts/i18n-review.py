#!/usr/bin/env python3
"""訳文レビューのシートを作り直す（`docs/i18n-review.tsv`）。

**鍵を足したら、これを回してシートを作り直すこと。**
作り直さないと、レビューの穴（訳されていない鍵）が見えないまま残る。

    python3 scripts/i18n-review.py

**判定と「直した訳」の欄は空のまま出す**（baseline §19）。
**実物を見た人が記入する。AI は代筆しない。**
"""

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SRC = ROOT / "apps/desktop/src/lib/i18n/messages.ts"
OUT = ROOT / "docs/i18n-review.tsv"
LOCALES = ["en", "ja", "zh", "ko"]  # DESIGN.md §9 の並び


def 読む() -> tuple[dict[str, dict[str, str]], set[str], dict[str, str]]:
    src = SRC.read_text(encoding="utf-8")
    body = src[src.index("export const MESSAGES") :]

    出 = {}
    for lang in LOCALES:
        i = body.index(f"\n  {lang}: {{")
        j = body.index("\n  },", i)
        出[lang] = dict(re.findall(r"'([^']+)':\s*'((?:[^'\\]|\\.)*)'", body[i:j]))

    数 = {l: len(d) for l, d in 出.items()}
    if len(set(数.values())) != 1:
        sys.exit(f"言語ごとに鍵の数が違う: {数}")

    塊 = re.search(r"export const CRITICAL_KEYS[^\[]*\[(.*?)\] as const;", src, re.S)
    事故 = set(re.findall(r"'([^']+)'", 塊.group(1))) if 塊 else set()

    注記 = {}
    nb = src[src.index("export const TRANSLATOR_NOTES") :]
    for m in re.finditer(r"'([^']+)':\s*\n?\s*((?:'(?:[^'\\]|\\.)*'\s*\+?\s*)+),", nb):
        注記[m.group(1)] = "".join(re.findall(r"'((?:[^'\\]|\\.)*)'", m.group(2)))

    return 出, 事故, 注記


def 書く() -> None:
    出, 事故, 注記 = 読む()
    頭 = ["鍵", *LOCALES, "事故になるか", "翻訳者への注記", "判定", "直した訳（ja/en/zh/ko）"]
    行 = ["\t".join(頭)]
    for k in sorted(出["en"]):
        行.append(
            "\t".join(
                [
                    k,
                    *[出[l][k] for l in LOCALES],
                    "★" if k in 事故 else "",
                    注記.get(k, "").replace("\t", " "),
                    "",  # 判定 —— **人が書く**
                    "",  # 直した訳 —— **人が書く**
                ]
            )
        )
    OUT.write_text("\n".join(行) + "\n", encoding="utf-8")
    print(f"{OUT.relative_to(ROOT)} を書いた（{len(行) - 1} 鍵 / 事故になるもの {len(事故)}）")


if __name__ == "__main__":
    書く()
