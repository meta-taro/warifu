#!/usr/bin/env python3
"""訳文レビューのシートを作り直す（`docs/i18n-review.tsv`）。

**鍵を足したら、これを回してシートを作り直すこと。**
作り直さないと、レビューの穴（訳されていない鍵）が見えないまま残る。

    python3 scripts/i18n-review.py

**判定と「直した訳」の欄は空のまま出す**（baseline §19）。
**実物を見た人が記入する。AI は代筆しない。**

**すでに書かれている判定は、鍵ごとに引き継ぐ。**
作り直すたびに消えると、**人が書いたものを、こちらの都合で捨てることになる**
（baseline §27「AI が代筆・要約しない」の裏返し —— 消すのも同じこと）。
"""

import re
import sys
from pathlib import Path

# **出す所に文字コードを書く**（`.claude/rules/踏んだ落とし穴.md` §2）。
# 書かないと、Windows の既定（cp1252）で print が落ちる。
for 口 in (sys.stdout, sys.stderr):
    if hasattr(口, "reconfigure"):
        口.reconfigure(encoding="utf-8", errors="replace")

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


def すでに書かれたもの() -> dict[str, tuple[str, str]]:
    """いまのシートから、**人が書いた 2 列だけ**を鍵ごとに拾う。"""
    if not OUT.exists():
        return {}
    行たち = OUT.read_text(encoding="utf-8").splitlines()
    if not 行たち:
        return {}
    頭 = 行たち[0].split("\t")
    try:
        判 = 頭.index("判定")
    except ValueError:
        return {}
    直 = 判 + 1
    拾った = {}
    for 行 in 行たち[1:]:
        欄 = 行.split("\t")
        if len(欄) <= 判:
            continue
        判定 = 欄[判] if 判 < len(欄) else ""
        直し = 欄[直] if 直 < len(欄) else ""
        if 判定 or 直し:
            拾った[欄[0]] = (判定, 直し)
    return 拾った


def 書く() -> None:
    出, 事故, 注記 = 読む()
    前の = すでに書かれたもの()
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
                    前の.get(k, ("", ""))[0],  # 判定 —— **人が書く。引き継ぐ**
                    前の.get(k, ("", ""))[1],  # 直した訳 —— **人が書く。引き継ぐ**
                ]
            )
        )
    OUT.write_text("\n".join(行) + "\n", encoding="utf-8")
    引き継いだ = sum(1 for k in 出["en"] if k in 前の)
    消えた = sorted(set(前の) - set(出["en"]))
    print(
        f"{OUT.relative_to(ROOT)} を書いた（{len(行) - 1} 鍵 / 事故になるもの {len(事故)}"
        f" / 人の記入を引き継いだ {引き継いだ}）"
    )
    if 消えた:
        # **黙って捨てない。**鍵が消えたときだけ、書かれていたものを画面へ出す
        print("**辞書から消えた鍵に、人の記入がありました:**", ", ".join(消えた))


if __name__ == "__main__":
    書く()
