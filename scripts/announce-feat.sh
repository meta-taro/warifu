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
# 置いていなければ push を止める。部屋は決まっていて流すのに失敗したとき（画面が閉じている等）は、
# 止めずに大きく言う（割符の画面が閉じている機械からも push できるように）。
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

# 入る版：送る範囲で版が上がっていれば、その版。上がっていなければ、いまの版の次
read_ver() { git show "$1:Cargo.toml" 2>/dev/null | sed -n 's/^version = "\([0-9.]*\)"/\1/p' | head -1; }
new_ver="$(read_ver "$new")"
case "$old" in *[!0]*) old_ver="$(read_ver "$old")" ;; *) old_ver="" ;; esac
if [ -n "$old_ver" ] && [ "$new_ver" != "$old_ver" ]; then
  next="$new_ver"
else
  next="$(printf '%s' "$new_ver" | awk -F. '{ printf "%d.%d.%d", $1, $2, $3 + 1 }')"
fi

# **部屋が決まっていなければ push を止める。**黙って通すと、新しい機械では知らせが出ないまま develop に入る
if [ -z "$room" ]; then
  echo "  知らせる部屋が決まっていないので、push を止めました。次を打ってから push し直してください:" >&2
  echo "    git config warifu.announce-room <部屋の id か頭>   （scripts/setup-hooks.sh も聞きます）" >&2
  printf '%s\n' "$feats" | sed 's/^/    知らせる commit: /' >&2
  exit 1
fi

if ! command -v warifu >/dev/null 2>&1; then
  echo "  部屋へ知らせていません（warifu が見つからない）。手で知らせてください" >&2
  exit 0
fi

while IFS=$'\t' read -r sha subject; do
  body="develop に入りました（${sha}）：${subject#*: } —— **入る版 ${next}**（まだ出していません。出たら知らせます）"
  if out="$(warifu room say "$room" --as "割符の開発（自動の知らせ）" "$body" 2>&1)"; then
    echo "  部屋へ知らせました: ${out}"
  else
    echo "  部屋へ知らせられませんでした（${out}）。手で知らせてください: ${sha} ${subject}" >&2
  fi
done <<EOF
$feats
EOF
exit 0
