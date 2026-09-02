#!/usr/bin/env python3
"""仮のアプリアイコンを作る（**ロゴではない**）。

Tauri の `generate_context!` は `icons/icon.png` が無いとコンパイルできない。
一方 **ロゴは人が決める領域**であり（product-baseline §11・DESIGN.md）、
AI が意匠を作ると、それがそのまま既成事実になる。

そこで置くのは「意匠を含まないもの」— 承認済みのアクセント色（DESIGN.md §3.1 の
`--accent` = #5b5bd6）で塗った角丸の四角だけ。**印も文字も入れない。**
人がロゴを決めた時点で、このスクリプトごと捨てる。

依存を足さないため、PNG は zlib と struct だけで書く（Pillow を入れない）。

    python3 scripts/make-placeholder-icon.py
"""

from __future__ import annotations

import pathlib
import struct
import zlib

# DESIGN.md §3.1 --accent（ライト）。ここを勝手に変えない
ACCENT = (0x5B, 0x5B, 0xD6)
SIZE = 512
# 角丸。--radius-lg の比率をアイコンの大きさへ引き伸ばした値
RADIUS = SIZE * 0.22
# 縁のギザつきを抑えるための多重サンプリング
SAMPLES = 4


def coverage(px: int, py: int) -> float:
    """角丸の内側にどれだけ入っているかを 0.0〜1.0 で返す。"""
    hit = 0
    for sy in range(SAMPLES):
        for sx in range(SAMPLES):
            x = px + (sx + 0.5) / SAMPLES
            y = py + (sy + 0.5) / SAMPLES
            # 角の円の中心へ寄せる。辺の上では距離が 0 になる
            dx = max(RADIUS - x, 0.0, x - (SIZE - RADIUS))
            dy = max(RADIUS - y, 0.0, y - (SIZE - RADIUS))
            if dx * dx + dy * dy <= RADIUS * RADIUS:
                hit += 1
    return hit / (SAMPLES * SAMPLES)


def rgba_rows() -> bytes:
    r, g, b = ACCENT
    out = bytearray()
    for y in range(SIZE):
        out.append(0)  # PNG のフィルタ種別（None）
        for x in range(SIZE):
            a = round(coverage(x, y) * 255)
            out += bytes((r, g, b, a))
    return bytes(out)


def chunk(kind: bytes, data: bytes) -> bytes:
    body = kind + data
    return struct.pack(">I", len(data)) + body + struct.pack(">I", zlib.crc32(body))


def png(path: pathlib.Path) -> None:
    header = struct.pack(">IIBBBBB", SIZE, SIZE, 8, 6, 0, 0, 0)  # 8bit RGBA
    blob = (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", header)
        + chunk(b"IDAT", zlib.compress(rgba_rows(), 9))
        + chunk(b"IEND", b"")
    )
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(blob)
    print(f"{path} ({len(blob)} bytes)")


if __name__ == "__main__":
    root = pathlib.Path(__file__).resolve().parent.parent
    png(root / "apps/desktop/src-tauri/icons/icon.png")
