#!/usr/bin/env python3
"""warifu のアイコンを作る。

**割符**（わりふ）— 二つに割った札。片割れが合うことで、相手が確かにその相手だと分かる。
その「割れ目が合う」ことだけを描く。鍵も錠も盾も出さない（それは割符の意味ではない）。

- 地: 角丸の四角に**カフェオレ**（ミルク多め）。**飲み物の色**であって、木の色ではない
- 印: **木の板**（濃い木肌）。縦長で、一枚が割れて二つの片になっている
- **地と板で役割を分ける。**地＝カフェオレ、板＝木。ここを両方とも木の色にすると、
  背景まで木に見えて「板の上に板」になる（一度そうなった）
- 割れ目は縦のギザギザ。**左右がぴったり噛み合う**（片方をずらして描いていない）
- 木目は**薄く縦に**。**128px 未満では描かない**（小さい時は汚れにしか見えない）

**アクセント色（`--accent` = #5b5bd6）は使っていない。**
オーナー指示（2026-09-03「モカ、カフェオレみたい。木の板をイメージ」）による。
割符は木の札なので、道具の色ではなく**物の色**を採る。

**32px で読めることが要件。**歯は 2 つ、割れ目は太い。
細かいギザギザにすると、小さい時に潰れて「ただの白い四角」か「ノイズ」に戻る
（3 歯・細い割れ目で試して、実際にそうなった）。

依存を足さないため、PNG は zlib と struct だけで書く（Pillow を入れない）。
`.icns` は macOS の `iconutil`、`.ico` は PNG を並べた最小の容器を自分で組む。

    python3 scripts/make-icon.py
"""

from __future__ import annotations

import math
import pathlib
import struct
import subprocess
import zlib

# カフェオレ（地・飲み物の色）と木の板（札）。**ここを勝手に変えない**（DESIGN.md §4-A）
# 地と板の明暗差が割れ目の読みやすさになる。片方を動かしたら GAP も見直すこと
GROUND = (0xCD, 0xB2, 0x95)
MARK = (0x7B, 0x5A, 0x3E)
# 木目。札よりわずかに濃いだけ。強くすると板ではなく縞模様になる
GRAIN = (0x6B, 0x4C, 0x33)

# 角丸（アイコンの一辺に対する比）
CORNER = 0.22
# 印の大きさ（同）
# 板らしく縦長にする（割符は棒状の札を縦に割る）
MARK_W, MARK_H = 0.50, 0.66
MARK_R = 0.045  # 札の角丸
# 割れ目の幅と、ギザギザの歯の数
GAP = 0.095
TEETH = 2
# 歯の振れ幅（横方向）
SWING = 0.105

# 木目を描き始める大きさ。これ未満では汚れにしか見えない
GRAIN_FROM = 128
# 木目の位置（札の幅に対する比）と太さ
GRAIN_AT = (0.30, 0.72)
GRAIN_W = 0.012

SAMPLES = 4


def rounded(x: float, y: float, cx: float, cy: float, w: float, h: float, r: float) -> bool:
    """中心 (cx, cy)・幅 w・高さ h・角丸 r の角丸長方形の内側か。"""
    dx = abs(x - cx) - (w / 2 - r)
    dy = abs(y - cy) - (h / 2 - r)
    if dx <= 0 or dy <= 0:
        return abs(x - cx) <= w / 2 and abs(y - cy) <= h / 2
    return dx * dx + dy * dy <= r * r


def split_at(y: float) -> float:
    """高さ y における割れ目の中心（0.5 を基準に左右へ振れる）。

    三角波で往復させる。**上下の端で必ず 0.5 に戻す**ので、
    札の外形は左右対称のまま保たれる。
    """
    top = 0.5 - MARK_H / 2
    t = (y - top) / MARK_H  # 0..1
    t = min(max(t, 0.0), 1.0)
    phase = (t * TEETH) % 1.0
    # 0 → +1 → 0 → -1 → 0 の三角波
    wave = 4 * abs(phase - 0.5) - 1
    return 0.5 + wave * SWING


def inside_mark(x: float, y: float) -> bool:
    """白い印（割れた札）の内側か。"""
    if not rounded(x, y, 0.5, 0.5, MARK_W, MARK_H, MARK_R):
        return False
    # 割れ目のぶんだけ抜く。**両側が同じ線で抜かれる = 噛み合う**
    return abs(x - split_at(y)) > GAP / 2


def inside_grain(x: float, y: float) -> bool:
    """木目の線の上か。**札の内側にしか引かない。**"""
    if not inside_mark(x, y):
        return False
    left = 0.5 - MARK_W / 2
    for at in GRAIN_AT:
        # わずかに蛇行させる。まっすぐだと木ではなく罫線に見える。
        # **滑らかに**振ること — 行ごとに飛ばすと、木目ではなく破線（ミシン目）になる
        wobble = 0.008 * math.sin(y * 9.0)
        if abs(x - (left + MARK_W * at + wobble)) < GRAIN_W / 2:
            return True
    return False


def coverage(px: int, py: int, size: int, fn) -> float:
    hit = 0
    for sy in range(SAMPLES):
        for sx in range(SAMPLES):
            x = (px + (sx + 0.5) / SAMPLES) / size
            y = (py + (sy + 0.5) / SAMPLES) / size
            if fn(x, y):
                hit += 1
    return hit / (SAMPLES * SAMPLES)


def rgba_rows(size: int) -> bytes:
    out = bytearray()
    for py in range(size):
        out.append(0)  # フィルタ種別 None
        for px in range(size):
            ground = coverage(px, py, size, lambda x, y: rounded(x, y, 0.5, 0.5, 1.0, 1.0, CORNER))
            if ground == 0.0:
                out += bytes((0, 0, 0, 0))
                continue
            mark = coverage(px, py, size, inside_mark)
            grain = coverage(px, py, size, inside_grain) if size >= GRAIN_FROM else 0.0
            # 地 → 札 → 木目 の順に重ねる
            r = round(GROUND[0] * (1 - mark) + MARK[0] * mark)
            g = round(GROUND[1] * (1 - mark) + MARK[1] * mark)
            b = round(GROUND[2] * (1 - mark) + MARK[2] * mark)
            r = round(r * (1 - grain) + GRAIN[0] * grain)
            g = round(g * (1 - grain) + GRAIN[1] * grain)
            b = round(b * (1 - grain) + GRAIN[2] * grain)
            out += bytes((r, g, b, round(ground * 255)))
    return bytes(out)


def chunk(kind: bytes, data: bytes) -> bytes:
    body = kind + data
    return struct.pack(">I", len(data)) + body + struct.pack(">I", zlib.crc32(body))


def png_bytes(size: int) -> bytes:
    header = struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0)  # 8bit RGBA
    return (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", header)
        + chunk(b"IDAT", zlib.compress(rgba_rows(size), 9))
        + chunk(b"IEND", b"")
    )


def ico_bytes(pngs: dict[int, bytes]) -> bytes:
    """PNG を並べた ICO。Vista 以降は PNG のまま入れられる。"""
    sizes = sorted(pngs)
    header = struct.pack("<HHH", 0, 1, len(sizes))
    offset = 6 + 16 * len(sizes)
    entries, blobs = b"", b""
    for s in sizes:
        data = pngs[s]
        # 256 は 0 と書く決まり
        entries += struct.pack(
            "<BBBBHHII", 0 if s >= 256 else s, 0 if s >= 256 else s, 0, 0, 1, 32, len(data), offset
        )
        blobs += data
        offset += len(data)
    return header + entries + blobs


def main() -> None:
    root = pathlib.Path(__file__).resolve().parent.parent
    icons = root / "apps/desktop/src-tauri/icons"
    icons.mkdir(parents=True, exist_ok=True)

    sizes = [16, 32, 48, 64, 128, 256, 512, 1024]
    pngs = {s: png_bytes(s) for s in sizes}

    for name, s in [
        ("32x32.png", 32),
        ("128x128.png", 128),
        ("128x128@2x.png", 256),
        ("icon.png", 512),
    ]:
        (icons / name).write_bytes(pngs[s])
        print(f"{name} ({len(pngs[s])} bytes)")

    (icons / "icon.ico").write_bytes(ico_bytes({s: pngs[s] for s in (16, 32, 48, 64, 128, 256)}))
    print("icon.ico")

    # .icns は macOS の iconutil に任せる（自前で組むと壊れ方が分かりにくい）
    iconset = icons / "icon.iconset"
    iconset.mkdir(exist_ok=True)
    for name, s in [
        ("icon_16x16.png", 16),
        ("icon_16x16@2x.png", 32),
        ("icon_32x32.png", 32),
        ("icon_32x32@2x.png", 64),
        ("icon_128x128.png", 128),
        ("icon_128x128@2x.png", 256),
        ("icon_256x256.png", 256),
        ("icon_256x256@2x.png", 512),
        ("icon_512x512.png", 512),
        ("icon_512x512@2x.png", 1024),
    ]:
        (iconset / name).write_bytes(pngs[s])
    try:
        subprocess.run(
            ["iconutil", "-c", "icns", str(iconset), "-o", str(icons / "icon.icns")], check=True
        )
        print("icon.icns")
    except (OSError, subprocess.CalledProcessError) as e:
        # **握り潰さない。**macOS 以外では作れないので、その事実を出す
        print(f"icon.icns は作れませんでした（macOS の iconutil が要ります）: {e}")
    for f in iconset.iterdir():
        f.unlink()
    iconset.rmdir()


if __name__ == "__main__":
    main()
