// **グレーの丸を、緑にする道筋。**
//
// オーナー指摘（2026-09-10）——「**いまの UI で人は、どう操作すると想定するか。
// そのためにはどんなデザインや機能がいるか。という視点で UX あげてほしい**」
//
// いちばん詰まる所がここだった —— **切れているエージェントを、画面から繋げない。**
// 人は丸がグレーだと分かるが、**緑にする道が画面のどこにも無い**（`warifu mcp` を
// 知っている人だけが繋げる）。だから**叩くものを画面が出す。**
//
// **割符が代わりに叩かない。**別の端末のエージェントもあるし、
// 何を許すかは人が書く（**D56** —— 割符が自動で札を出さない）。

/** 既定で許す口。**会話と、自分の席の名乗りだけ**（受信箱も予定表も許さない）。 */
export const 既定の札 = ['chat.send', 'chat.read', 'profile.write'] as const;

/** 割符の口を足すコマンド（Claude Code）。 */
export function 口を足す(実体 = 'warifu'): string {
  const 札 = 既定の札.map((一つ) => `--allow ${一つ}`).join(' ');
  return `claude mcp add warifu --scope user -- ${実体} mcp ${札}`;
}

/**
 * 待たずに反応させるコマンド（`warifu agent`）。
 *
 * **届いた行を命令の標準入力へ渡して起こす** —— `chat_wait` と違って、
 * 待っている間そのエージェントを縛らない。
 *
 * **席の名前をそのまま入れない**（`'` で囲んだ命令の中に入るため）——
 * 使えない字を落として、無ければ `agent` にする。
 */
export function 起こす(席: string, 実体 = 'warifu'): string {
  const 安全 = 席.replace(/[^A-Za-z0-9_-]/g, '') || 'agent';
  return `${実体} agent --as ${安全} --on 'claude -p'`;
}

/**
 * `.app` から入れた人向けの実体。
 *
 * **PATH に無いことがある**（dmg で入れただけなら通っていない）ので、
 * その場所も出せるようにしておく。
 */
export const macの実体 = '/Applications/warifu.app/Contents/MacOS/warifu-cli';
