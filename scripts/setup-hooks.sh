#!/usr/bin/env bash
# commit 前のゲートを有効にする（product-baseline §5）。
#
# git hooks は clone ごとに設定が要るので、**clone したら 1 回これを走らせる。**
set -euo pipefail

cd "$(dirname "$0")/.."
git config core.hooksPath .githooks

echo "core.hooksPath = $(git config core.hooksPath)"
echo "有効にしました。次の commit から .githooks/pre-commit が走ります。"

# **ゲートと同じ見方で確かめる。**pre-commit は ~/.cargo/bin を自分で足すので、
# ここでも足してから見る（足さずに見ると、ゲートは走るのに「飛ばされます」と嘘を言う）
[ -d "$HOME/.cargo/bin" ] && PATH="$HOME/.cargo/bin:$PATH"
if ! command -v cargo >/dev/null 2>&1; then
  printf '\033[33m%s\033[0m\n' "注意: cargo が見つかりません。Rust のゲートは飛ばされます（CI が最終ゲート）。"
fi
