#!/usr/bin/env bash
# **出す前の関門**（2026-10-24・0.1.40）。タグを打つ前に、人がこれを 1 回通す。
#
#   scripts/release-gate.sh            # Cargo.toml の版で見る
#   scripts/release-gate.sh 0.1.40     # 版を指定して見る
#   WARIFU_GATE_SKIP_TESTS=1 scripts/release-gate.sh   # 試験を飛ばして筋だけ見る（タグは打てないと出る）
#
# 見るもの（どれか 1 つでも落ちたら 1 で終わる・落ちた理由を全部出す）：
#   1. 試験が全部通る（workspace・画面側 Rust・画面の型・vitest）
#   2. CHANGELOG にその版の節（`## vX.Y.Z`）がある。「作業中」の印が残っていない
#   3. 互換が切れうる場所（Notice の形・机の線の形・CLI の引数）が前のタグから動いているなら、
#      CHANGELOG のその節に「互換」の語がある（切れないなら「互換：切れない」と 1 行書く）
#   4. 配布の文書が、その版の数字になっている（check-doc-version.py）
#   5. 公開してはいけない語が無い（check-public-words.sh）
#   6. 作業の木がきれい
#
# **これは人を介す所を減らすためのもので、押し通すためのものではない。**
# 落ちたら直してから打つ。`--no-verify` に当たるものは無い。
# シェルの変数名は ASCII だけ（非公開の記録「踏んだ落とし穴」1）。

set -u
cd "$(dirname "$0")/.."

version="${1:-}"
if [ -z "${version}" ]; then
  version="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)"
fi
if [ -z "${version}" ]; then
  echo "関門: Cargo.toml から版が読めません" >&2
  exit 1
fi

failed=0
fail() {
  echo "  x $1"
  failed=1
}
pass() {
  echo "  o $1"
}
section() {
  # CHANGELOG のその版の節だけを出す（次の「## v」の手前まで）
  awk -v v="## v${version}" '
    index($0, v) == 1 { on = 1; print; next }
    on && /^## v/ { exit }
    on { print }
  ' CHANGELOG.md
}

echo "出す前の関門（v${version}）"

# 0. 版が bump されているか（引数で版を言われたとき。Cargo.toml と違えば、まだ上げていない）
toml_version="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)"
if [ "${toml_version}" = "${version}" ]; then
  pass "Cargo.toml は v${version}"
else
  fail "Cargo.toml は v${toml_version} のまま（scripts/bump-version.py ${version} で上げる）"
fi

# 6. 作業の木
if [ -n "$(git status --porcelain)" ]; then
  fail "作業の木に commit していない変更があります"
else
  pass "作業の木はきれい"
fi

# 2. CHANGELOG の節
if grep -q "^## v${version}" CHANGELOG.md; then
  if section | grep -q "作業中"; then
    fail "CHANGELOG の v${version} の節に「作業中」が残っています（日付を入れて外す）"
  else
    pass "CHANGELOG に v${version} の節がある"
  fi
else
  fail "CHANGELOG に「## v${version}」の節がありません"
fi

# 3. 互換が切れうる場所
prev_tag="$(git describe --tags --abbrev=0 --match 'v*' 2>/dev/null || true)"
if [ -n "${prev_tag}" ]; then
  changed="$(git diff --name-only "${prev_tag}"..HEAD -- \
    crates/warifu-meeting/src crates/warifu-desk/src/line.rs crates/warifu-cli/src/main.rs 2>/dev/null || true)"
  if [ -n "${changed}" ]; then
    if section | grep -q "互換"; then
      pass "線の形が ${prev_tag} から動いている。CHANGELOG に「互換」の記述がある"
    else
      fail "線の形が ${prev_tag} から動いています（$(echo "${changed}" | tr '\n' ' ')）。CHANGELOG の v${version} の節に「互換」を書いてください"
    fi
  else
    pass "線の形は ${prev_tag} から動いていない"
  fi
else
  pass "前のタグが無い（互換の比較を飛ばす）"
fi

# 4. 配布の文書
if python3 scripts/check-doc-version.py >/dev/null 2>&1; then
  pass "配布の文書は Cargo.toml の版と合う"
else
  fail "配布の文書の版が合いません（scripts/check-doc-version.py）"
fi

# 5. 公開してはいけない語
if scripts/check-public-words.sh >/dev/null 2>&1; then
  pass "公開してはいけない語は無い"
else
  fail "公開してはいけない語があります（scripts/check-public-words.sh）"
fi

# 1. 試験
if [ "${WARIFU_GATE_SKIP_TESTS:-}" = "1" ]; then
  fail "試験を飛ばしました（WARIFU_GATE_SKIP_TESTS=1）。タグは打てません"
else
  echo "  … 試験（数分かかります）"
  if cargo test --workspace --quiet >/dev/null 2>&1; then
    pass "cargo test（workspace）"
  else
    fail "cargo test（workspace）が落ちました"
  fi
  if cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --quiet >/dev/null 2>&1; then
    pass "cargo test（画面側 Rust）"
  else
    fail "cargo test（画面側 Rust）が落ちました"
  fi
  if pnpm --filter @warifu/desktop check >/dev/null 2>&1; then
    pass "画面の型"
  else
    fail "画面の型（svelte-check）が落ちました"
  fi
  if pnpm --filter @warifu/desktop test >/dev/null 2>&1; then
    pass "vitest"
  else
    fail "vitest が落ちました"
  fi
fi

if [ "${failed}" -ne 0 ]; then
  echo "関門: 通りませんでした。直してからタグを打ってください"
  exit 1
fi
echo "関門: 通りました。タグ v${version} を打てます（打つのは人）"
