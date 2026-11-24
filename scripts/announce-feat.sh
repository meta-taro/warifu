#!/usr/bin/env bash
# **機能を足す commit が develop へ出たら、部屋へ「入る版・口の名前」を知らせる**（0.1.56）。
#
# develop に入っていても版が出ていなければ、使う側の席からは「無い」と同じに見える。
# 席が言い忘れても出るように、push の前（.githooks/pre-push）から呼ぶ。
#
#   scripts/announce-feat.sh <古い sha> <新しい sha>
#
# 流す部屋は、この clone の git config に置く（部屋の id は公開のリポに書かない）:
#   git config warifu.announce-room <部屋の id か頭>
# 置いていなければ、知らせずにその旨だけ出す。**知らせられなくても push は止めない**
# （割符の画面が閉じている機械からも push できるように）。
set -u

old="${1:-}"
new="${2:-}"
if [ -z "$new" ]; then
  echo "使い方: scripts/announce-feat.sh <古い sha> <新しい sha>" >&2
  exit 2
fi

here="$(git rev-parse --show-toplevel)"
room="$(git config --get warifu.announce-room || true)"

# 範囲の feat の commit（古い順）。新しい枝なら、その sha だけを見る
case "$old" in
  *[!0]*) range="$old..$new" ;;
  *) range="$new -1" ;;
esac
# shellcheck disable=SC2086
feats="$(git log --reverse --format='%h%x09%s' $range | grep -E $'\t''feat(\([^)]*\))?!?:' || true)"
[ -z "$feats" ] && exit 0

# 入る版 ＝ いまの版の次（版を上げる commit は chore なので、ここに来るのは上げる前）
cur="$(sed -n 's/^version = "\([0-9.]*\)"/\1/p' "$here/Cargo.toml" | head -1)"
next="$(printf '%s' "$cur" | awk -F. '{ printf "%d.%d.%d", $1, $2, $3 + 1 }')"

if [ -z "$room" ]; then
  echo "  部屋へ知らせていません（git config warifu.announce-room が無い）。手で知らせてください:" >&2
  printf '%s\n' "$feats" | sed 's/^/    /' >&2
  exit 0
fi

if ! command -v warifu >/dev/null 2>&1; then
  echo "  部屋へ知らせていません（warifu が見つからない）。手で知らせてください" >&2
  exit 0
fi

while IFS=$'\t' read -r sha subject; do
  body="develop に入りました（${sha}）：${subject#*: } —— **入る版 ${next}**（まだ出していません。出たら知らせます）"
  if out="$(warifu room say "$room" --as "割符の開発（自動の知らせ）" "$body" 2>&1)"; then
    echo "  部屋へ知らせました: $out"
  else
    echo "  部屋へ知らせられませんでした（$out）。手で知らせてください: $sha $subject" >&2
  fi
done <<EOF
$feats
EOF
exit 0
