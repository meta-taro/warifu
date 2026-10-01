#!/usr/bin/env bash
# **公開してはいけないものが、リポジトリの木に入っていないか**を見る（2026-10-01）。
#
# 公開リポジトリの履歴に、内部の記録（会話・決定や進捗のメモ・社内の呼び名・網の詳細）が
# 入っていた。消すにはリポジトリを作り直すしかなかった。**人が気をつける形では止まらない**ので、
# commit・push・CI の 3 か所で、機械が止める。
#
#   scripts/check-public-tree.sh            # いまの作業の木（index）を見る（commit の前）
#   scripts/check-public-tree.sh <rev>      # その版の木を見る（push の前・CI）
#
# 見つけたら 1 で終わる。**見つけた語の中身は出さない**（CI の記録は公開される）。
# 何に当たったかは、規則の番号とファイルの場所で言う。
#
# 内部の記録の置き場所は、公開しない別のリポジトリ warifu-notes である（CLAUDE.md）。

set -u

rev="${1:-}"

# **置いてはいけない場所**（正規表現・パスの頭から）
DENY_PATHS=(
  '^\.claude/decisions\.md$'
  '^\.claude/project-status\.md$'
  '^\.claude/roadmap\.md$'
  '^\.claude/issues/'
  '^\.claude/pdca/'
  '^\.claude/archive/'
  '^\.claude/rules/踏んだ落とし穴\.md$'
  '^\.claude/rules/product-baseline\.full\.md$'
  '^\.claude/templates/design-system/'
  '^docs/原案/'
  '^docs/test-specs/'
)

# **書いてはいけない語**（そのままの文字で探す）。
# 会話の引用の印・人や席の呼び名・社内の呼び名・網の詳細。
DENY_WORDS=(
  'オーナー'
  # 「席」は製品の言葉でもある（机の席）。**機械や人の席の呼び名だけ**を止める
  'mini の席'
  'Air の席'
  'ASUS の席'
  'Zenbook の席'
  'sshboard の席'
  'warifu(zenbook)'
  '連動くん'
  'めたたろ'
  '田中'
  'Asahi'
  'RTX810'
  '固定 IP'
  '固定IP'
  'Dokokade'
  'SNZK24Z6S4'
  '使った人 ——'
)

# 規則を書いているファイル自身は見ない（語の一覧そのものが入っているため）
SELF='^(scripts/check-public-tree\.sh|\.github/scripts/oss-privacy-check\.sh)$'

fail=0
note() { printf '%s\n' "$*" >&2; }

if [ -n "$rev" ]; then
  files="$(git ls-tree -r --name-only "$rev")"
else
  files="$(git ls-files --cached)"
fi

# --- 1. 置いてはいけない場所 ---------------------------------------------------
i=0
for pat in "${DENY_PATHS[@]}"; do
  i=$((i + 1))
  hits="$(printf '%s\n' "$files" | grep -E -- "$pat" || true)"
  if [ -n "$hits" ]; then
    while IFS= read -r f; do
      note "NG [path #$i] $f : 内部の記録の置き場所です（warifu-notes へ）"
    done <<< "$hits"
    fail=1
  fi
done

# --- 2. 書いてはいけない語 -----------------------------------------------------
i=0
for w in "${DENY_WORDS[@]}"; do
  i=$((i + 1))
  if [ -n "$rev" ]; then
    hits="$(git grep -l -F -e "$w" "$rev" -- 2>/dev/null | sed "s#^$rev:##" || true)"
  else
    hits="$(git grep -l -F --cached -e "$w" -- 2>/dev/null || true)"
  fi
  hits="$(printf '%s\n' "$hits" | grep -Ev -- "$SELF" | sed '/^$/d' || true)"
  if [ -n "$hits" ]; then
    while IFS= read -r f; do
      note "NG [word #$i] $f : 公開の場に書かない語に当たりました（語は出しません。規則は scripts/check-public-tree.sh）"
    done <<< "$hits"
    fail=1
  fi
done

if [ "$fail" -ne 0 ]; then
  note ""
  note "公開してはいけないものが見つかりました。"
  note "  - 会話・決定や進捗のメモ・実測の記録は warifu-notes に置く"
  note "  - 人や席の呼び名・社内の呼び名・網の詳細は、書かずに事実と理由だけを書く"
  note "  - **--no-verify で外さない。**誤検知なら、規則（この script）を直す commit を先に出す"
  exit 1
fi
note "OK 公開してはいけないものは見つかりませんでした"
