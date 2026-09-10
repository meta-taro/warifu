# 版ごとに変わったこと

**git のタグの間にある commit の見出し**をそのまま並べたもの。
新しいものから 12 版まで。**作った文ではなく、実際に入った物の見出しである。**

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

## v0.1.0-alpha.5 — 2026-09-08

- docs(trial): 試験導入の手順を、いまの版に合わせる
- feat(desktop): 相手が起動していなくても、預かり所ごしに届くようにする
- feat(relay): 相手が起動していない間、封を預かる所を立てられるようにする
- feat(post): 預かり所。相手が起動していない間、封を預かる（D70）
- feat(seal): 封をする。中継は運ぶだけで、開けられない（D69）
- docs: 中継の法的整理を、着手の前提条件から外す（D68）
- feat(desktop): 部屋の一覧を出す（issues/015 段 3）
- feat(desktop): 会話を部屋ごとに分ける（issues/015 段 2）
- refactor(desktop): 部屋を同時に複数持てるようにする（issues/015 段 1）
- feat: 常駐に止める口・上限・やったことの記録を足す（issues/014）
- feat(cli): 机に着いて待ち、呼ばれずに動く常駐（warifu agent・D67）
- feat(desktop): 部屋の名簿に、この PC の AI を出す
- feat(desktop): 机の中で、相手を選んで話せるようにする（D66）
- feat(mcp): 待てる口を道具として並べる（chat_wait・D65）
- docs: MCP の入れ方を warifu setup に一本化する
- feat(desktop): 画面の言葉を「部屋」にする（D64）
- feat: どこで動いているエージェントかを名乗り、線の向こうにも残す
- feat(desktop): 届いたことを窓の外へ押し出す（人が気づく所だけ・D60）
- fix(desktop): 画面から打った行が、同じ席の AI に届くようにする
- feat(desktop): 届く先を並べ、空の面と行き止まりを埋める
- fix(desktop): 呼び名を変える口を、名前の隣の鉛筆 1 つだけにする
- feat(desktop): 連絡帳から呼び名を付けられるようにし、内側の言葉を画面から消す
- feat(desktop): 連絡帳を画面の入口にする（面は 連絡帳 / 会議 / 予定）
- feat(vault): 戸口の知り合いと、最後に繋がった住所を保存する
- fix(desktop): 潰れていた会話欄を直し、チャットを右の列の先頭へ出す
- fix(desktop): 会議が無くても、同じ席の AI には話しかけられる
- fix(mcp): 届いていないのに「流しました」と返さない（D49 の再発）
- feat(mcp): 人と AI が同じ会話に着けるようにする（机）
- docs(issues): 010 を「会議が先ではない。チャットが先」に書き直す
- fix: 会議キーの既定を 10 分から 24 時間にする（D54）
- docs(issues): 013 に「エージェントが warifu を実行できない」を記録する
- docs(issues): エージェントが画面の会議に参加する件を起票する（013）
- fix(desktop): ログを決まった場所へ書き置く
- ci(bundle): develop の成果物を上げる（Windows で試す手段が無かった）
- docs(decisions): D13 の引き金が引かれたことを記録する（中継の 3 択）
- docs(status): 遅延の実測が同じ LAN の中の数字であることを訂正する
- feat(cli): 届かないのに「待っています」と言わない ＋ warifu doctor（D53）
- docs(status): 2026-09-07 の区切りを記録する
- docs(issues): 011 の入口を「ドメインを持てる個人か法人」に絞る
