# 版ごとに変わったこと

**git のタグの間にある commit の見出し**をそのまま並べたもの。
新しいものから 12 版まで。**作った文ではなく、実際に入った物の見出しである。**

## v0.1.0-alpha.17 — 2026-09-10

- feat(mcp): エージェントに、自分が何かを説明する口（about / changes・D82）
- fix(release): シェルの変数名に日本語を使っていた

## v0.1.0-alpha.16 — 2026-09-10

- feat(desktop): 自動アップデートと、版の表示（D81）
- docs: alpha.15 が最初の「配れる配布物」になったことを書く

## v0.1.0-alpha.15 — 2026-09-09

- fix(release): 署名の検証が、一度も検証していなかった

## v0.1.0-alpha.14 — 2026-09-09

- docs(signing): 配布物に署名する手順を書く（D80）

## v0.1.0-alpha.13 — 2026-09-09

- docs(install): リンクは一度アプリを開かないと効かないことを書く
- feat(desktop): 渡すのはリンク 1 本にする（warifu:// と QR・D79）
- docs: alpha.12 に --relay が入ったことを書く

## v0.1.0-alpha.12 — 2026-09-09

- feat(net,cli): 中継を --relay で選べるようにする（D78・オーナー判断）
- docs(status): alpha.10 / alpha.11 を出したことと、その中身を記録する

## v0.1.0-alpha.11 — 2026-09-09

- fix(release): 配布物を平らにして渡す

## v0.1.0-alpha.10 — 2026-09-09

- fix(release,docs): 2 台目の Mac に CLI が届いていなかった
- fix(desktop): 同じ席が、連絡帳と会話で別の名前で出ていた（D77）
- fix(desktop,cli): 2 台で名乗りを交わして見つけた 3 つを直す

## v0.1.0-alpha.9 — 2026-09-09

- docs(status): 私自身が机に着けた理由と、今日の 2 件を記録する
- fix(desktop): 押しても何も起きない「チャットする」を消す
- docs(trial): 今日入れたものを試験導入の手順へ反映する
- docs(mcp): 待たずに反応する経路（warifu agent）を書く

## v0.1.0-alpha.8 — 2026-09-09

- docs(status): エージェントが Issue を投げ、AI が直す回り方を記録する
- fix(mcp): 待っている最中に机が閉じたら、繋ぎ直して待ち続ける（issues/2）
- feat(mcp): いつ席に着いたかを返す（届いていない／着く前だった・issues/4）
- feat(mcp): 断るときに何が要るかを言い、版を名乗る（issues/4）
- feat(desk): 既読を数える（届いたと読んだを分ける・D76）
- feat(desktop): 部屋に名前・相手に覚え書き・名乗らない席は着けない

## v0.1.0-alpha.7 — 2026-09-08

- docs(status): 2026-09-08 夜の進捗を記録する
- fix(setup): warifu setup が入れ直せなかった
- fix(desktop): 名乗らない席が 2 つ着いても、1 行に潰れないようにする
- feat(profile): 顔を差し替えられるようにする（PNG・落として置く）
- feat(cli): warifu post —— 預かり所を画面なしで確かめる
- docs: プロフィールと呼び方の変更を、文書へ反映する

## v0.1.0-alpha.6 — 2026-09-08

- feat(meeting): 名乗りを相手へ配る（呼び名が勝つ形で）
- feat(profile): エージェント自身にも名乗らせ、席と名乗りを両方出す
- feat(desktop): プロフィール（名前・短い紹介・顔）を、人にも各エージェントにも
- feat(desktop): メニューバーからテーマを切り替えられるようにする
- feat(desktop): 画面の色をアイコンの色調にし、ダークを実際に出るようにする
- fix(desktop): 机の発言を、この PC の人にも届いたと数える
