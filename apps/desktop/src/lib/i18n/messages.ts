// 文言辞書（DOM 非依存の純データ層・DESIGN.md §9 / D35）。
//
// **`ja` が正本である。**`en` / `zh` / `ko` は AI が下書きしたもので、
// **まだ人がレビューしていない**（baseline §19 / §27 — 訳文の可否は実物を見た人が決める）。
// レビューが済むまで、この事実を消さないこと。
//
// 割符・宛先・公開鍵・会議 id は**翻訳しない**。生値は差し込み口（`{tally}` 等）で渡す。
// base32 は同じバイト列に複数の表記を許さないので、言語ごとに見た目が変わると
// 紙・口頭で伝える経路が壊れる（M1 / D35）。

import type { Locale } from './locales';

/** 文言の鍵。増やすときは 4 言語すべてに足す（テストが落ちる）。 */
export type MessageKey =
  | 'app.name'
  | 'window.minimize'
  | 'window.maximize'
  | 'window.restore'
  | 'window.close'
  | 'update.available'
  | 'update.apply'
  | 'revoke.irreversible'
  | 'revoke.confirm'
  | 'door.refused'
  | 'link.direct'
  | 'link.relayed'
  | 'link.unknown'
  | 'roster.capacity'
  | 'tile.me'
  | 'tile.peer'
  | 'meeting.start.title'
  | 'meeting.key.hint'
  | 'meeting.key.label'
  | 'meeting.start.action'
  | 'meeting.join.title'
  | 'meeting.join.hint'
  | 'meeting.join.action'
  | 'meeting.join.working'
  | 'chat.title'
  | 'chat.hint'
  | 'chat.placeholder'
  | 'chat.send'
  | 'chat.empty'
  | 'chat.joined'
  | 'chat.left'
  | 'chat.lost'
  | 'chat.nobody'
  | 'chat.agent'
  | 'chat.desk'
  | 'chat.placeholder.desk'
  | 'chat.late'
  | 'send.absent'
  | 'postbox.title'
  | 'postbox.hint'
  | 'postbox.placeholder'
  | 'postbox.save'
  | 'postbox.clear'
  | 'postbox.saved'
  | 'postbox.cleared'
  | 'postbox.kept'
  | 'postbox.received'
  | 'pane.contacts'
  | 'pane.meeting'
  | 'pane.schedule'
  | 'contacts.this'
  | 'contacts.me'
  | 'contacts.desk'
  | 'contacts.desk.none'
  | 'contacts.inmeeting'
  | 'contacts.saved'
  | 'contacts.late'
  | 'contacts.rooms'
  | 'room.members.some'
  | 'room.alone'
  | 'room.host'
  | 'contacts.empty'
  | 'contacts.pick'
  | 'contacts.key.label'
  | 'contacts.me.what'
  | 'contacts.me.share'
  | 'contacts.desk.how'
  | 'chat.shared'
  | 'chat.reach'
  | 'chat.reach.none'
  | 'contacts.presence.none'
  | 'contacts.forget'
  | 'contacts.forget.hint'
  | 'act.chat'
  | 'act.call'
  | 'act.call.working'
  | 'act.mail'
  | 'act.mail.none'
  | 'act.address.none'
  | 'act.already'
  | 'act.desk.local'
  | 'act.desk.empty'
  | 'act.desk.stop'
  | 'act.desk.stop.hint'
  | 'schedule.title'
  | 'schedule.none'
  | 'chat.placeholder.nobody'
  | 'call.mic'
  | 'call.camera'
  | 'call.controls'
  | 'meeting.key.own'
  | 'setup.title'
  | 'setup.hint'
  | 'setup.action'
  | 'setup.mic'
  | 'setup.camera'
  | 'setup.blur'
  | 'setup.blur.os'
  | 'setup.headphones'
  | 'setup.mode.both'
  | 'setup.mode.audio'
  | 'setup.mode.none'
  | 'meeting.key.copy'
  | 'meeting.key.copied'
  | 'meeting.key.reveal'
  | 'meeting.key.more'
  | 'meeting.key.more.hint'
  | 'roster.name.action'
  | 'roster.name.save'
  | 'roster.name.placeholder'
  | 'meeting.status.waiting'
  | 'meeting.status.live'
  | 'setup.mic.none'
  | 'setup.camera.none'
  | 'edit.cut'
  | 'edit.copy'
  | 'edit.paste'
  | 'edit.selectAll'
  | 'edit.pasteHint'
  | 'link.closed'
  | 'link.lost'
  | 'browser.only'
  | 'camera.denied'
  | 'camera.missing'
  | 'camera.busy'
  | 'camera.unknown';

export const MESSAGES: Record<Locale, Record<MessageKey, string>> = {
  ja: {
    'app.name': 'warifu',
    'window.minimize': '最小化',
    'window.maximize': '最大化',
    'window.restore': '元のサイズに戻す',
    'window.close': '閉じる',
    'update.available': '更新あり — {version}',
    'update.apply': '再起動して更新する',
    'revoke.irreversible': 'この失効は取り消せません。',
    'revoke.confirm': '{device} を失効させる',
    'door.refused': '断りました。',
    'link.direct': '直接',
    'link.relayed': '中継',
    'link.unknown': '不明',
    'roster.capacity': '{current} / {capacity}',
    'tile.me': '自分',
    'tile.peer': '相手',
    'meeting.start.title': '部屋をつくる',
    'meeting.key.label': '部屋の鍵',
    'meeting.key.hint': 'これを入ってほしい人に渡します。渡した相手だけが入れます。24 時間で切れます。紙に書いても、読み上げても渡せます。',
    'meeting.start.action': '部屋をつくる',
    'meeting.join.title': 'もらった鍵で部屋に入る',
    'meeting.join.hint': 'もらった側が入ります。渡した側は待つだけです。',
    'meeting.join.action': '部屋に入る',
    'meeting.key.own': 'これは自分の部屋の鍵です。入ってほしい人に渡してください。',
    'link.closed': '相手が退出しました。',
    'link.lost': '相手との経路が切れました。同じ鍵で戻ってこられます — 待っています。',
    'browser.only': 'ブラウザで開いています。部屋は warifu の窓でだけ動きます。',
    'camera.denied': 'カメラとマイクが許可されていません。OS の設定で許可してください。',
    'camera.missing': 'カメラかマイクが見つかりません。',
    'camera.busy': 'ほかのアプリがカメラを使っています。',
    'meeting.key.copy': 'コピーする',
    'meeting.key.copied': 'コピーしました',
    'meeting.key.reveal': '鍵の全文を見る',
    'meeting.key.more': '鍵をもう 1 本出す',
    'meeting.key.more.hint': '1 本の鍵で入れるのは 1 人だけです。もう 1 人入れるなら、もう 1 本出してその人に渡します。前の鍵は使えたままです。',
    'roster.name.action': '名前を付ける',
    'roster.name.save': '決める',
    'roster.name.placeholder': '呼び名（例: Mac Air のエージェント）',
    'meeting.status.waiting': '相手を待っています',
    'meeting.status.live': 'つながっています',
    'setup.mic.none': 'マイクが見つかりません',
    'setup.camera.none': 'カメラが見つかりません',
    'edit.cut': '切り取る',
    'edit.copy': 'コピー',
    'edit.paste': '貼り付け',
    'edit.selectAll': 'すべてを選択',
    'edit.pasteHint': '貼り付けは ⌘V を使ってください。',
    'setup.mode.both': '映像と音声を送ります',
    'setup.mode.audio': 'カメラが無いので、音声だけ送ります',
    'setup.mode.none': 'カメラもマイクも無いので、受け取るだけで入ります',
    'meeting.join.working': '入っています…',
    'chat.title': '文字で話す',
    'chat.hint': '同じ部屋に居る人へ届きます。残りません — 閉じると消えます。',
    'chat.placeholder': '書いて Enter（改行は Shift+Enter）',
    'chat.send': '送る',
    'chat.empty': 'まだ何もありません',
    'chat.joined': '{who} が入りました',
    'chat.left': '{who} が退室しました',
    'chat.lost': '{who} との経路が切れました',
    'chat.nobody': 'まだ誰も居ません。鍵を渡して、入ってもらうと送れます。',
    'chat.agent': 'この PC の AI',
    'chat.desk': 'この PC の AI が居ます。部屋に人が居なくても話しかけられます。',
    'chat.placeholder.desk': 'この PC の AI に話しかける',
    'chat.late': '留守中',
    'send.absent': 'いま居ません。預かり所を置くと、留守中でも届きます。',
    'postbox.title': '預かり所（任意）',
    'postbox.hint': '相手が起動していない間、封を預かる所です。中身は読めません。置かなければ、相手が起動している間だけ届きます。',
    'postbox.placeholder': '預かり所の宛先を貼り付ける',
    'postbox.save': '置く',
    'postbox.clear': '外す',
    'postbox.saved': '預かり所を置きました。',
    'postbox.cleared': '預かり所を外しました。',
    'postbox.kept': 'いま居ないので、預かり所へ預けました。相手が起動したときに届きます。',
    'postbox.received': '留守中に届いていた分が {n} 通ありました。',
    'pane.contacts': '連絡帳',
    'pane.meeting': '部屋',
    'pane.schedule': '予定',
    'contacts.this': 'この PC',
    'contacts.me': 'あなた',
    'contacts.desk': 'この PC の AI',
    'contacts.desk.none': '机に誰も着いていません。`warifu mcp` で繋ぐと、ここに出ます。',
    'contacts.inmeeting': 'いま同じ部屋に居る人',
    'contacts.saved': '覚えている相手',
    'contacts.late': '留守中に届いた相手',
    'contacts.rooms': 'いま居る部屋',
    'room.members.some': '人が居ます。',
    'room.alone': 'まだ自分だけです。鍵を渡すと入ってもらえます。',
    'room.host': 'あなたが建てた部屋です',
    'contacts.empty': 'まだ誰も覚えていません。部屋で会った相手に呼び名を付けると、ここに残ります。',
    'contacts.pick': '相手を選ぶと、できることが出ます。',
    'contacts.key.label': '公開鍵',
    'contacts.me.what': 'これがこの PC のあなたです。閉じても同じ人でいられます。',
    'contacts.me.share': 'この公開鍵は、相手に見せて構いません。これだけでは誰も入ってこられません（入るには鍵が要ります）。',
    'contacts.desk.how': '机に着かせるには、この PC のエージェントの設定に warifu の口を書いて、エージェントを立て直します。手順は docs/mcp.md にあります。',
    'chat.shared': 'ここは、選んだ相手だけの会話ではありません。',
    'chat.reach': '届く先 {who}',
    'chat.reach.none': '届く先はまだありません。部屋に人が入るか、この PC の AI が着くと出ます。',
    'contacts.presence.none': '相手がいま起動しているかは分かりません。呼んでみるまで分かりません。',
    'contacts.forget': '鍵なしで入れるのをやめる',
    'contacts.forget.hint': 'この相手はいま、鍵なしで入ってこられます。やめると、次からは鍵が要ります。',
    'act.chat': 'チャットする',
    'act.call': '部屋に入れる',
    'act.call.working': '呼んでいます…',
    'act.mail': 'メールを送る',
    'act.mail.none': 'まだ送れません。warifu にメールを送る経路が、まだ 1 本もありません（読む口だけがあります）。',
    'act.address.none': '住所をまだ覚えていません。鍵で一度つながると覚えます。それまでは、こちらから呼べません。',
    'act.already': 'すでに同じ部屋に居ます。',
    'act.desk.local': 'この PC の AI は、はじめから同じ部屋に居ます。入れる必要はありません。',
    'act.desk.empty': '机に誰も着いていません。',
    'act.desk.stop': 'この AI を止める',
    'act.desk.stop.hint': '「止まれ」と伝えます。相手を殺すのではなく、受けた側が自分で降ります。もう一度動かすには、そちらで立ち上げ直してください。',
    'schedule.title': '予定',
    'schedule.none': '予定の面は、まだ動きません。画面から予定表を読む口が、まだ 1 本もありません。',
    'chat.placeholder.nobody': '入ってきたら送れます',
    'call.mic': 'マイク',
    'call.camera': 'カメラ',
    'call.controls': '通話の入切。支度の確認とは別で、いま送っているものを止めます。',
    'setup.title': '入る前のしたく',
    'setup.hint': 'いま自分が何で映って、何で喋るかを、入る前に確かめられます。',
    'setup.action': 'カメラとマイクを確かめる',
    'setup.mic': 'マイクを入にして入る',
    'setup.camera': 'カメラを入にして入る',
    'setup.blur': '背景をぼかす',
    'setup.blur.os': 'この環境では、アプリから背景をぼかせません。macOS ならコントロールセンターのビデオエフェクトが使えます。',
    'setup.headphones': '同じ部屋で 2 台を鳴らすと、エコー除去では消せません。ヘッドフォンを使ってください。',
    'camera.unknown': 'カメラを使えませんでした。',
  },
  en: {
    'app.name': 'warifu',
    'window.minimize': 'Minimize',
    'window.maximize': 'Maximize',
    'window.restore': 'Restore',
    'window.close': 'Close',
    'update.available': 'Update available — {version}',
    'update.apply': 'Restart and update',
    'revoke.irreversible': 'This revocation cannot be undone.',
    'revoke.confirm': 'Revoke {device}',
    'door.refused': 'Refused.',
    'link.direct': 'Direct',
    'link.relayed': 'Relayed',
    'link.unknown': 'Unknown',
    'roster.capacity': '{current} / {capacity}',
    'tile.me': 'You',
    'tile.peer': 'Them',
    'meeting.start.title': 'Make a room',
    'meeting.key.label': 'Room key',
    'meeting.key.hint': 'Hand this to the person you want in. Only they can come in. It expires in 24 hours. Writing it down or reading it aloud both work.',
    'meeting.start.action': 'Make a room',
    'meeting.join.title': 'Come in with a key you were given',
    'meeting.join.hint': 'The one who was given the key comes in. The one who gave it just waits.',
    'meeting.join.action': 'Come in',
    'meeting.key.own': 'This is your own room key. Hand it to the person you want in.',
    'link.closed': 'The other person left.',
    'link.lost': 'The route to them broke. They can come back with the same key — waiting.',
    'browser.only': 'This is open in a browser. Rooms only work in the warifu window.',
    'camera.denied': 'Camera and microphone are not allowed. Allow them in your OS settings.',
    'camera.missing': 'No camera or microphone found.',
    'camera.busy': 'Another app is using the camera.',
    'meeting.key.copy': 'Copy',
    'meeting.key.copied': 'Copied',
    'meeting.key.reveal': 'Show the whole key',
    'meeting.key.more': 'Hand out one more key',
    'meeting.key.more.hint': 'One key lets exactly one person in. To let another person in, hand out one more key. The earlier key keeps working.',
    'roster.name.action': 'Name',
    'roster.name.save': 'Save',
    'roster.name.placeholder': 'A name (e.g. Agent on Mac Air)',
    'meeting.status.waiting': 'Waiting for the other person',
    'meeting.status.live': 'Connected',
    'setup.mic.none': 'No microphone found',
    'setup.camera.none': 'No camera found',
    'edit.cut': 'Cut',
    'edit.copy': 'Copy',
    'edit.paste': 'Paste',
    'edit.selectAll': 'Select All',
    'edit.pasteHint': 'Use Cmd+V to paste.',
    'setup.mode.both': 'Sending video and audio',
    'setup.mode.audio': 'No camera, so sending audio only',
    'setup.mode.none': 'No camera or microphone, so joining to receive only',
    'meeting.join.working': 'Coming in…',
    'chat.title': 'Chat',
    'chat.hint': 'It reaches everyone in the room. Nothing is kept — it goes when you close.',
    'chat.placeholder': 'Type and press Enter (Shift+Enter for a new line)',
    'chat.send': 'Send',
    'chat.empty': 'Nothing yet',
    'chat.joined': '{who} came in',
    'chat.left': '{who} left',
    'chat.lost': 'The connection to {who} was lost',
    'chat.nobody': 'Nobody is here yet. Hand out a key and wait for someone to come in.',
    'chat.agent': 'AI on this computer',
    'chat.desk': 'The AI on this computer is here. You can talk to it even with nobody else in the room.',
    'chat.placeholder.desk': 'Talk to the AI on this computer',
    'chat.late': 'while away',
    'send.absent': 'Not here right now. Set up a mailbox and messages will reach them later.',
    'postbox.title': 'Mailbox (optional)',
    'postbox.hint': 'A place that holds sealed messages while the other person is not running. It cannot read them. Without one, messages only arrive while the other person is running.',
    'postbox.placeholder': 'Paste the mailbox address',
    'postbox.save': 'Set',
    'postbox.clear': 'Remove',
    'postbox.saved': 'Mailbox set.',
    'postbox.cleared': 'Mailbox removed.',
    'postbox.kept': 'They are not here, so it was left at the mailbox. It arrives when they start up.',
    'postbox.received': '{n} message(s) had arrived while you were away.',
    'pane.contacts': 'Contacts',
    'pane.meeting': 'Room',
    'pane.schedule': 'Schedule',
    'contacts.this': 'This computer',
    'contacts.me': 'You',
    'contacts.desk': 'AI on this computer',
    'contacts.desk.none': 'No agent is at the desk. Connect one with `warifu mcp` and it appears here.',
    'contacts.inmeeting': 'In the room now',
    'contacts.saved': 'People you remember',
    'contacts.late': 'Arrived while you were away',
    'contacts.rooms': 'Rooms you are in',
    'room.members.some': 'People are here.',
    'room.alone': 'Just you so far. Hand out a key and someone can come in.',
    'room.host': 'You made this room',
    'contacts.empty': 'You have not remembered anyone yet. Name someone you met in a room and they stay here.',
    'contacts.pick': 'Pick someone to see what you can do.',
    'contacts.key.label': 'Public key',
    'contacts.me.what': 'This is you on this computer. You stay the same person after closing it.',
    'contacts.me.share': 'You can show this public key to anyone. On its own it lets nobody in — coming in needs a key.',
    'contacts.desk.how': 'To seat an agent, write the warifu entry into the agent settings on this computer and restart the agent. The steps are in docs/mcp.md.',
    'chat.shared': 'This is not a conversation with the person you picked.',
    'chat.reach': 'Goes to {who}',
    'chat.reach.none': 'It goes nowhere yet. Someone coming into the room, or the AI on this computer, shows up here.',
    'contacts.presence.none': 'There is no way to tell whether they are running right now. You find out by calling.',
    'contacts.forget': 'Require a meeting key again',
    'contacts.forget.hint': 'Right now this person can come in without a key. Turn it off and they will need one again.',
    'act.chat': 'Chat',
    'act.call': 'Bring into the room',
    'act.call.working': 'Calling…',
    'act.mail': 'Send mail',
    'act.mail.none': 'Not possible yet. warifu has no way to send mail at all (it can only read).',
    'act.address.none': 'Their whereabouts are not remembered yet. Connect once with a key and it is remembered. Until then you cannot call them.',
    'act.already': 'Already in the same room.',
    'act.desk.local': 'The AI on this computer is in the room from the start. There is nothing to bring in.',
    'act.desk.empty': 'No agent is at the desk.',
    'act.desk.stop': 'Stop this AI',
    'act.desk.stop.hint': 'It is told to stop. Nothing is killed — the other side steps down on its own. To run it again, start it there.',
    'schedule.title': 'Schedule',
    'schedule.none': 'The schedule pane does not work yet. There is no way for the window to read a calendar.',
    'chat.placeholder.nobody': 'You can send once someone joins',
    'call.mic': 'Microphone',
    'call.camera': 'Camera',
    'call.controls': 'Turn the call on and off. Separate from checking your gear — this stops what you are sending now.',
    'setup.title': 'Before you come in',
    'setup.hint': 'Check what you look and sound like before entering.',
    'setup.action': 'Check camera and microphone',
    'setup.mic': 'Come in with the microphone on',
    'setup.camera': 'Come in with the camera on',
    'setup.blur': 'Blur my background',
    'setup.blur.os': 'This environment cannot blur backgrounds from the app. On macOS, use Video Effects in Control Center.',
    'setup.headphones': 'Two devices in one room will echo no matter what. Use headphones.',
    'camera.unknown': 'Could not use the camera.',
  },
  zh: {
    'app.name': 'warifu',
    'window.minimize': '最小化',
    'window.maximize': '最大化',
    'window.restore': '还原',
    'window.close': '关闭',
    'update.available': '有可用更新 — {version}',
    'update.apply': '重启并更新',
    'revoke.irreversible': '此吊销无法撤销。',
    'revoke.confirm': '吊销 {device}',
    'door.refused': '已拒绝。',
    'link.direct': '直连',
    'link.relayed': '中继',
    'link.unknown': '未知',
    'roster.capacity': '{current} / {capacity}',
    'tile.me': '自己',
    'tile.peer': '对方',
    'meeting.start.title': '建一个房间',
    'meeting.key.label': '房间密钥',
    'meeting.key.hint': '把它交给你想让进来的人。只有拿到的人能进来。24 小时后失效。写在纸上、口头念出来都可以。',
    'meeting.start.action': '建一个房间',
    'meeting.join.title': '用拿到的密钥进入房间',
    'meeting.join.hint': '拿到密钥的一方进来。交出去的一方只需等待。',
    'meeting.join.action': '进入房间',
    'meeting.key.own': '这是你自己房间的密钥。请交给你想让进来的人。',
    'link.closed': '对方已离开。',
    'link.lost': '与对方的连接断了。用同一把密钥可以回来 —— 正在等待。',
    'browser.only': '你在浏览器里打开了。房间只在 warifu 的窗口里工作。',
    'camera.denied': '未允许使用摄像头和麦克风。请在系统设置中允许。',
    'camera.missing': '找不到摄像头或麦克风。',
    'camera.busy': '其他应用正在使用摄像头。',
    'meeting.key.copy': '复制',
    'meeting.key.copied': '已复制',
    'meeting.key.reveal': '查看密钥全文',
    'meeting.key.more': '再发一把密钥',
    'meeting.key.more.hint': '一把密钥只能让一个人进来。要再让一个人进来，就再发一把交给他。之前的密钥仍然有效。',
    'roster.name.action': '命名',
    'roster.name.save': '保存',
    'roster.name.placeholder': '名称（例：Mac Air 上的智能体）',
    'meeting.status.waiting': '正在等待对方',
    'meeting.status.live': '已连接',
    'setup.mic.none': '未找到麦克风',
    'setup.camera.none': '未找到摄像头',
    'edit.cut': '剪切',
    'edit.copy': '复制',
    'edit.paste': '粘贴',
    'edit.selectAll': '全选',
    'edit.pasteHint': '请使用 Cmd+V 粘贴。',
    'setup.mode.both': '将发送视频和音频',
    'setup.mode.audio': '没有摄像头，只发送音频',
    'setup.mode.none': '没有摄像头和麦克风，仅接收方式加入',
    'meeting.join.working': '正在进入…',
    'chat.title': '文字聊天',
    'chat.hint': '会送达房间里的所有人。不会保留 —— 关掉就没了。',
    'chat.placeholder': '输入后按 Enter（换行用 Shift+Enter）',
    'chat.send': '发送',
    'chat.empty': '还没有内容',
    'chat.joined': '{who} 进来了',
    'chat.left': '{who} 已离开',
    'chat.lost': '与 {who} 的连接已中断',
    'chat.nobody': '还没有人进来。把密钥交给对方，等对方进来后即可发送。',
    'chat.agent': '这台电脑的 AI',
    'chat.desk': '这台电脑的 AI 在。即使房间里没有其他人，也可以对它说话。',
    'chat.placeholder.desk': '对这台电脑的 AI 说话',
    'chat.late': '离线期间',
    'send.absent': '对方现在不在。设置寄存处后，离线期间也能送达。',
    'postbox.title': '寄存处（可选）',
    'postbox.hint': '在对方没有启动时，代为保管密封内容的地方。它读不到内容。不设置的话，只有对方启动时才能送达。',
    'postbox.placeholder': '粘贴寄存处地址',
    'postbox.save': '设置',
    'postbox.clear': '移除',
    'postbox.saved': '已设置寄存处。',
    'postbox.cleared': '已移除寄存处。',
    'postbox.kept': '对方不在，已寄存。对方启动时会送到。',
    'postbox.received': '离线期间收到了 {n} 条。',
    'pane.contacts': '通讯录',
    'pane.meeting': '房间',
    'pane.schedule': '日程',
    'contacts.this': '这台电脑',
    'contacts.me': '你',
    'contacts.desk': '这台电脑的 AI',
    'contacts.desk.none': '桌旁没有人。用 `warifu mcp` 连接后会出现在这里。',
    'contacts.inmeeting': '现在同一个房间里的人',
    'contacts.saved': '记住的对方',
    'contacts.late': '离线期间来信的人',
    'contacts.rooms': '你所在的房间',
    'room.members.some': '有人在。',
    'room.alone': '目前只有你。把密钥交出去，对方就能进来。',
    'room.host': '这个房间由你建立',
    'contacts.empty': '还没有记住任何人。给房间里遇到的人取个称呼，就会留在这里。',
    'contacts.pick': '选择一位，就会显示可以做的事。',
    'contacts.key.label': '公钥',
    'contacts.me.what': '这是这台电脑上的你。关掉之后仍然是同一个人。',
    'contacts.me.share': '这个公钥可以给对方看。仅凭它谁也进不来（进来需要密钥）。',
    'contacts.desk.how': '要让 AI 坐到桌旁，请在这台电脑的代理设置里写入 warifu 的入口，然后重启代理。步骤见 docs/mcp.md。',
    'chat.shared': '这里不是只和所选对方的会话。',
    'chat.reach': '送达 {who}',
    'chat.reach.none': '目前送不到任何人。有人进入房间，或这台电脑的 AI 到位后，就会显示在这里。',
    'contacts.presence.none': '无法知道对方现在是否已启动。只有呼叫之后才知道。',
    'contacts.forget': '恢复需要会议密钥',
    'contacts.forget.hint': '现在这位不用密钥就能进来。取消后，下次就需要密钥了。',
    'act.chat': '聊天',
    'act.call': '请进房间',
    'act.call.working': '正在呼叫…',
    'act.mail': '发送邮件',
    'act.mail.none': '还发不了。warifu 目前完全没有发送邮件的通道（只有读取的口）。',
    'act.address.none': '还没有记住对方的位置。用密钥连接一次后就会记住。在那之前无法从这边呼叫。',
    'act.already': '已经在同一个房间里了。',
    'act.desk.local': '这台电脑的 AI 从一开始就在同一个房间里，不需要请进来。',
    'act.desk.empty': '桌旁没有人。',
    'act.desk.stop': '让这个 AI 停下',
    'act.desk.stop.hint': '会传达「停下」。不是杀掉对方，而是收到的一方自己退出。要再次运行，请在那边重新启动。',
    'schedule.title': '日程',
    'schedule.none': '日程还不能用。画面还没有读取日程表的口。',
    'chat.placeholder.nobody': '有人进入后即可发送',
    'call.mic': '麦克风',
    'call.camera': '摄像头',
    'call.controls': '通话的开关。与设备确认不同，这会停止你现在正在发送的内容。',
    'setup.title': '进来之前的准备',
    'setup.hint': '进入之前，先确认自己的画面和声音。',
    'setup.action': '检查摄像头和麦克风',
    'setup.mic': '开着麦克风进来',
    'setup.camera': '开着摄像头进来',
    'setup.blur': '虚化背景',
    'setup.blur.os': '此环境无法由应用虚化背景。macOS 可使用控制中心的视频效果。',
    'setup.headphones': '同一房间里的两台设备一定会啸叫，回声消除也无法解决。请使用耳机。',
    'camera.unknown': '无法使用摄像头。',
  },
  ko: {
    'app.name': 'warifu',
    'window.minimize': '최소화',
    'window.maximize': '최대화',
    'window.restore': '이전 크기로',
    'window.close': '닫기',
    'update.available': '업데이트 있음 — {version}',
    'update.apply': '다시 시작하고 업데이트',
    'revoke.irreversible': '이 해지는 되돌릴 수 없습니다.',
    'revoke.confirm': '{device} 해지',
    'door.refused': '거절했습니다.',
    'link.direct': '직접',
    'link.relayed': '중계',
    'link.unknown': '알 수 없음',
    'roster.capacity': '{current} / {capacity}',
    'tile.me': '나',
    'tile.peer': '상대',
    'meeting.start.title': '방 만들기',
    'meeting.key.label': '방 열쇠',
    'meeting.key.hint': '들어오게 하고 싶은 사람에게 건넵니다. 건넨 상대만 들어올 수 있습니다. 24 시간이면 끊깁니다. 종이에 적어도, 읽어 줘도 건넬 수 있습니다.',
    'meeting.start.action': '방 만들기',
    'meeting.join.title': '받은 열쇠로 방에 들어가기',
    'meeting.join.hint': '받은 쪽이 들어갑니다. 건넨 쪽은 기다리기만 하면 됩니다.',
    'meeting.join.action': '방에 들어가기',
    'meeting.key.own': '이것은 자기 방의 열쇠입니다. 들어오게 하고 싶은 사람에게 건네주세요.',
    'link.closed': '상대방이 나갔습니다.',
    'link.lost': '상대와의 경로가 끊겼습니다. 같은 열쇠로 돌아올 수 있습니다 — 기다리고 있습니다.',
    'browser.only': '브라우저에서 열려 있습니다. 방은 warifu 창에서만 움직입니다.',
    'camera.denied': '카메라와 마이크가 허용되지 않았습니다. OS 설정에서 허용해 주세요.',
    'camera.missing': '카메라나 마이크를 찾을 수 없습니다.',
    'camera.busy': '다른 앱이 카메라를 사용 중입니다.',
    'meeting.key.copy': '복사하기',
    'meeting.key.copied': '복사했습니다',
    'meeting.key.reveal': '열쇠 전체 보기',
    'meeting.key.more': '열쇠를 한 개 더 내기',
    'meeting.key.more.hint': '열쇠 하나로 들어올 수 있는 사람은 한 명뿐입니다. 한 명 더 들이려면 한 개 더 내어 그 사람에게 건넵니다. 앞의 열쇠는 그대로 쓸 수 있습니다.',
    'roster.name.action': '이름 붙이기',
    'roster.name.save': '저장',
    'roster.name.placeholder': '이름 (예: Mac Air 에이전트)',
    'meeting.status.waiting': '상대방을 기다리는 중',
    'meeting.status.live': '연결되어 있습니다',
    'setup.mic.none': '마이크를 찾을 수 없습니다',
    'setup.camera.none': '카메라를 찾을 수 없습니다',
    'edit.cut': '오려두기',
    'edit.copy': '복사하기',
    'edit.paste': '붙여넣기',
    'edit.selectAll': '전체 선택',
    'edit.pasteHint': '붙여넣기는 Cmd+V를 사용하세요.',
    'setup.mode.both': '영상과 음성을 보냅니다',
    'setup.mode.audio': '카메라가 없어 음성만 보냅니다',
    'setup.mode.none': '카메라도 마이크도 없어 받기만 하며 참여합니다',
    'meeting.join.working': '들어가는 중…',
    'chat.title': '문자로 대화',
    'chat.hint': '같은 방에 있는 사람에게 갑니다. 남지 않습니다 — 닫으면 사라집니다.',
    'chat.placeholder': '입력 후 Enter (줄바꿈은 Shift+Enter)',
    'chat.send': '보내기',
    'chat.empty': '아직 아무것도 없습니다',
    'chat.joined': '{who} 가 들어왔습니다',
    'chat.left': '{who} 님이 퇴장했습니다',
    'chat.lost': '{who} 님과의 연결이 끊어졌습니다',
    'chat.nobody': '아직 아무도 없습니다. 열쇠를 건네고 상대가 들어오면 보낼 수 있습니다.',
    'chat.agent': '이 PC 의 AI',
    'chat.desk': '이 PC 의 AI 가 있습니다. 방에 사람이 없어도 말을 걸 수 있습니다.',
    'chat.placeholder.desk': '이 PC 의 AI 에게 말을 걸기',
    'chat.late': '부재 중',
    'send.absent': '지금 없습니다. 보관소를 두면 부재 중에도 전달됩니다.',
    'postbox.title': '보관소 (선택)',
    'postbox.hint': '상대가 실행 중이 아닐 때 봉한 내용을 맡아 두는 곳입니다. 내용은 읽을 수 없습니다. 두지 않으면 상대가 실행 중일 때만 전달됩니다.',
    'postbox.placeholder': '보관소 주소를 붙여넣기',
    'postbox.save': '두기',
    'postbox.clear': '내리기',
    'postbox.saved': '보관소를 두었습니다.',
    'postbox.cleared': '보관소를 내렸습니다.',
    'postbox.kept': '지금 없어서 보관소에 맡겼습니다. 상대가 실행하면 전달됩니다.',
    'postbox.received': '부재 중에 {n} 통이 와 있었습니다.',
    'pane.contacts': '연락처',
    'pane.meeting': '방',
    'pane.schedule': '일정',
    'contacts.this': '이 PC',
    'contacts.me': '당신',
    'contacts.desk': '이 PC 의 AI',
    'contacts.desk.none': '책상에 아무도 없습니다. `warifu mcp` 로 연결하면 여기에 나옵니다.',
    'contacts.inmeeting': '지금 같은 방에 있는 사람',
    'contacts.saved': '기억한 상대',
    'contacts.late': '부재 중에 온 상대',
    'contacts.rooms': '지금 있는 방',
    'room.members.some': '사람이 있습니다.',
    'room.alone': '아직 자신뿐입니다. 열쇠를 건네면 들어올 수 있습니다.',
    'room.host': '당신이 만든 방입니다',
    'contacts.empty': '아직 아무도 기억하지 않았습니다. 방에서 만난 상대에게 이름을 붙이면 여기에 남습니다.',
    'contacts.pick': '상대를 고르면 할 수 있는 일이 나옵니다.',
    'contacts.key.label': '공개키',
    'contacts.me.what': '이것이 이 PC 의 당신입니다. 닫아도 같은 사람으로 있습니다.',
    'contacts.me.share': '이 공개키는 상대에게 보여도 됩니다. 이것만으로는 아무도 들어올 수 없습니다 (들어오려면 열쇠가 필요합니다).',
    'contacts.desk.how': '책상에 앉히려면 이 PC 의 에이전트 설정에 warifu 입구를 쓰고 에이전트를 다시 시작합니다. 순서는 docs/mcp.md 에 있습니다.',
    'chat.shared': '여기는 고른 상대만의 대화가 아닙니다.',
    'chat.reach': '가는 곳 {who}',
    'chat.reach.none': '아직 갈 곳이 없습니다. 방에 사람이 들어오거나 이 PC 의 AI 가 자리에 앉으면 여기에 나옵니다.',
    'contacts.presence.none': '상대가 지금 켜져 있는지는 알 수 없습니다. 불러 봐야 알 수 있습니다.',
    'contacts.forget': '다시 회의 키를 받게 하기',
    'contacts.forget.hint': '지금 이 상대는 열쇠 없이 들어올 수 있습니다. 끄면 다음부터는 열쇠가 필요합니다.',
    'act.chat': '채팅하기',
    'act.call': '방에 들이기',
    'act.call.working': '부르는 중…',
    'act.mail': '메일 보내기',
    'act.mail.none': '아직 보낼 수 없습니다. warifu 에는 메일을 보내는 경로가 하나도 없습니다 (읽는 입구만 있습니다).',
    'act.address.none': '상대의 위치를 아직 기억하지 못했습니다. 열쇠로 한 번 연결하면 기억합니다. 그전에는 이쪽에서 부를 수 없습니다.',
    'act.already': '이미 같은 방에 있습니다.',
    'act.desk.local': '이 PC 의 AI 는 처음부터 같은 방에 있습니다. 들일 필요가 없습니다.',
    'act.desk.empty': '책상에 아무도 없습니다.',
    'act.desk.stop': '이 AI 를 멈추기',
    'act.desk.stop.hint': '「멈추라」고 전합니다. 상대를 죽이는 것이 아니라, 받은 쪽이 스스로 내려옵니다. 다시 움직이려면 그쪽에서 다시 시작해 주세요.',
    'schedule.title': '일정',
    'schedule.none': '일정 화면은 아직 움직이지 않습니다. 화면에서 일정표를 읽는 입구가 아직 하나도 없습니다.',
    'chat.placeholder.nobody': '누군가 들어오면 보낼 수 있습니다',
    'call.mic': '마이크',
    'call.camera': '카메라',
    'call.controls': '통화 켜고 끄기. 기기 확인과는 다르며, 지금 보내고 있는 것을 멈춥니다.',
    'setup.title': '들어가기 전 준비',
    'setup.hint': '들어가기 전에 자신의 화면과 소리를 확인할 수 있습니다.',
    'setup.action': '카메라와 마이크 확인',
    'setup.mic': '마이크를 켜고 들어가기',
    'setup.camera': '카메라를 켜고 들어가기',
    'setup.blur': '배경 흐리게',
    'setup.blur.os': '이 환경에서는 앱이 배경을 흐리게 할 수 없습니다. macOS라면 제어 센터의 비디오 효과를 사용하세요.',
    'setup.headphones': '같은 방에서 두 대를 켜면 에코 제거로도 막을 수 없습니다. 헤드폰을 사용하세요.',
    'camera.unknown': '카메라를 사용할 수 없습니다.',
  },
};

/**
 * **誤訳すると人の行動が変わる文言。**
 *
 * ここに入れた鍵は、`TRANSLATOR_NOTES` に注記が無いとテストが落ちる。
 * 機械翻訳をそのまま採らない（D35）。
 */
export const CRITICAL_KEYS: readonly MessageKey[] = [
  'revoke.irreversible',
  'revoke.confirm',
  'door.refused',
  'meeting.key.hint',
  'link.lost',
  'act.mail.none',
  'act.address.none',
  'chat.shared',
  'send.absent',
  'postbox.kept',
] as const;

/** 翻訳者への注記。**訳文と一緒に渡す。** */
export const TRANSLATOR_NOTES: Partial<Record<MessageKey, string>> = {
  'send.absent':
    '**まだ送れていない。**「送信しました」「あとで届きます」と読める訳にしないこと。' +
    '預かり所を置いていない状態なので、打った言葉はどこにも残っていない。' +
    '「預かり所を置くと」は条件であって、約束ではない。',
  'postbox.kept':
    '**相手にはまだ届いていない。**「送信しました」「配達済み」と読める訳にしないこと。' +
    '届くのは相手が次に起動したときであり、いつになるかは分からない。' +
    'また「保存しました」（＝こちらの手元に残した）とも読ませないこと —— 預けた先は別の機械である。',
  'act.mail.none':
    '「まだ送れません」は「いま経路が無い」という事実である。' +
    '「送信できませんでした」（＝送ろうとして失敗した）と読める訳にしないこと。' +
    'この画面でいちばん重い事故は、送ったつもりで送られていないことである。' +
    'また「準備中です」「近日対応」のような、待てば来ると読める訳にもしないこと。',
  'act.address.none':
    '覚えていないのは**こちら側**である。' +
    '「その相手は見つかりません」「拒否されました」と読める訳にしないこと。' +
    '相手のせいにすると、人は相手に確認しに行く。' +
    'また「自動でつながります」とも読ませないこと —— つなぐのは人の操作である。',
  'chat.shared':
    '**会話は 1 本しか無い。**選んだ相手の隣に並んでいても、その人だけに届くのではない。' +
    '「この相手にだけ届きます」「非公開の会話です」「ダイレクトメッセージ」と' +
    '読める訳にしないこと。**個別に届くと誤解した人は、見られたくないものを書く。**',
  'revoke.irreversible':
    '「取り消せない」は事実であって、丁寧な警告ではない。' +
    '「後で戻せます」「元に戻すこともできます」と読める訳にしないこと。' +
    '戻せると誤解させると、人は軽く押す（D12）。',
  'revoke.confirm':
    '実行を促す文言。ここで「試す」「確認する」に寄せると、押した先が最終であることが伝わらない。',
  'door.refused':
    '**既に断り終えた**という完了の意味。「保留しています」「確認中です」と読める訳にしないこと。' +
    '待てば通ると誤解すると、来ていない相手を待ち続ける（D31）。',
  'meeting.key.hint':
    '「渡した相手だけが入れる」は仕組みの説明であって、安全の保証ではない。' +
    '「安全です」と読める訳にしないこと — **部屋の鍵を他人に見られたら、その人が入れる。**',
  'link.lost':
    '**切れたが、戻ってこられる**という意味（D44）。戻れるのは**さっきまで入っていた同じ相手だけ**で、' +
    '部屋の鍵が別人に渡っても意味は無い（一回性＝D12 は崩していない）。' +
    '「誰でも入れるようになりました」と読める訳にしないこと。' +
    'また「自動で再接続します」とも読ませないこと — **戻るのは相手の操作**であり、こちらは待っているだけである。',
};

/** 差し込み口を埋める。無い鍵はそのまま残す（黙って空にしない）。 */
export function format(template: string, values: Record<string, string | number>): string {
  return template.replace(/\{(\w+)\}/g, (whole, name: string) =>
    name in values ? String(values[name]) : whole,
  );
}
