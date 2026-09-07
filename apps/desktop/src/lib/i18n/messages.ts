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
  | 'pane.contacts'
  | 'pane.meeting'
  | 'pane.schedule'
  | 'contacts.this'
  | 'contacts.me'
  | 'contacts.desk'
  | 'contacts.desk.none'
  | 'contacts.inmeeting'
  | 'contacts.saved'
  | 'contacts.empty'
  | 'contacts.pick'
  | 'contacts.key.label'
  | 'contacts.presence.none'
  | 'contacts.forget'
  | 'contacts.forget.hint'
  | 'contacts.rename.hint'
  | 'act.chat'
  | 'act.call'
  | 'act.call.working'
  | 'act.mail'
  | 'act.mail.none'
  | 'act.address.none'
  | 'act.already'
  | 'act.desk.local'
  | 'act.desk.empty'
  | 'chat.scope'
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
    'meeting.start.title': '会議をはじめる',
    'meeting.key.label': '会議キー',
    'meeting.key.hint': 'これを参加者に渡します。渡した相手だけが入室できます。24 時間で切れます。紙に書いても、読み上げても渡せます。',
    'meeting.start.action': '会議をはじめる',
    'meeting.join.title': 'もらった会議キーで入室する',
    'meeting.join.hint': 'もらった側が入室します。渡した側は待つだけです。',
    'meeting.join.action': '入室する',
    'meeting.key.own': 'これは自分の会議キーです。参加者に渡してください。',
    'link.closed': '相手が退出しました。',
    'link.lost': '相手との経路が切れました。同じ会議キーで戻ってこられます — 待っています。',
    'browser.only': 'ブラウザで開いています。会議は warifu の窓でだけ動きます。',
    'camera.denied': 'カメラとマイクが許可されていません。OS の設定で許可してください。',
    'camera.missing': 'カメラかマイクが見つかりません。',
    'camera.busy': 'ほかのアプリがカメラを使っています。',
    'meeting.key.copy': 'コピーする',
    'meeting.key.copied': 'コピーしました',
    'meeting.key.reveal': '会議キーの全文を見る',
    'meeting.key.more': '会議キーをもう 1 本出す',
    'meeting.key.more.hint': '1 本の会議キーで入れるのは 1 人だけです。3 人目を呼ぶなら、もう 1 本出してその人に渡します。前の鍵は使えたままです。',
    'roster.name.action': '名前を付ける',
    'roster.name.save': '決める',
    'roster.name.placeholder': '呼び名（例: Mac Air のエージェント）',
    'meeting.status.waiting': '相手を待っています',
    'meeting.status.live': '会議中',
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
    'meeting.join.working': '入室しています…',
    'chat.title': '文字で話す',
    'chat.hint': '会議に入っている人へ届きます。残りません — 閉じると消えます。\n',
    'chat.placeholder': '書いて Enter（改行は Shift+Enter）',
    'chat.send': '送る',
    'chat.empty': 'まだ何もありません',
    'chat.joined': '{who} が入室しました',
    'chat.left': '{who} が退室しました',
    'chat.lost': '{who} との経路が切れました',
    'chat.nobody': 'まだ誰も居ません。会議キーを渡して、入ってもらうと送れます。',
    'chat.agent': 'この PC の AI',
    'chat.desk': 'この PC の AI が居ます。会議に人が居なくても話しかけられます。',
    'chat.placeholder.desk': 'この PC の AI に話しかける',
    'pane.contacts': '連絡帳',
    'pane.meeting': '会議',
    'pane.schedule': '予定',
    'contacts.this': 'この PC',
    'contacts.me': 'あなた',
    'contacts.desk': 'この PC の AI',
    'contacts.desk.none': '机に誰も着いていません。`warifu mcp` で繋ぐと、ここに出ます。',
    'contacts.inmeeting': 'いま会議に居る人',
    'contacts.saved': '覚えている相手',
    'contacts.empty': 'まだ誰も覚えていません。会議で会った相手に呼び名を付けると、ここに残ります。',
    'contacts.pick': '相手を選ぶと、できることが出ます。',
    'contacts.key.label': '公開鍵',
    'contacts.presence.none': '相手がいま起動しているかは分かりません。呼んでみるまで分かりません。',
    'contacts.forget': '鍵なしで入れるのをやめる',
    'contacts.forget.hint': 'この相手はいま、会議キーなしで入ってこられます。やめると、次からは会議キーが要ります。',
    'contacts.rename.hint': '名前を右クリックすると、呼び名を付けられます。',
    'act.chat': 'チャットする',
    'act.call': '会議に呼ぶ',
    'act.call.working': '呼んでいます…',
    'act.mail': 'メールを送る',
    'act.mail.none': 'まだ送れません。warifu にメールを送る経路が、まだ 1 本もありません（読む口だけがあります）。',
    'act.address.none': '住所をまだ覚えていません。会議キーで一度つながると覚えます。それまでは、こちらから呼べません。',
    'act.already': 'すでにこの会議に居ます。',
    'act.desk.local': 'この PC の AI は、同じ机に着いています。会議に呼ぶ必要はありません。',
    'act.desk.empty': '机に誰も着いていません。',
    'chat.scope': 'この会話は 1 本です。会議に居る人と、この PC の AI の全員に届きます。',
    'schedule.title': '予定',
    'schedule.none': '予定の面は、まだ動きません。画面から予定表を読む口が、まだ 1 本もありません。',
    'chat.placeholder.nobody': '入ってきたら送れます',
    'call.mic': 'マイク',
    'call.camera': 'カメラ',
    'call.controls': '会議中の入切。支度の確認とは別で、いま送っているものを止めます。',
    'setup.title': '入室前のしたく',
    'setup.hint': 'いま自分が何で映って、何で喋るかを、入る前に確かめられます。',
    'setup.action': 'カメラとマイクを確かめる',
    'setup.mic': 'マイクを入にして入室する',
    'setup.camera': 'カメラを入にして入室する',
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
    'meeting.start.title': 'Start a meeting',
    'meeting.key.label': 'Meeting key',
    'meeting.key.hint': 'Give this to the person you are inviting. Only they can join. It expires in 24 hours. You can write it down or read it aloud.',
    'meeting.start.action': 'Start a meeting',
    'meeting.join.title': 'Enter with a meeting key',
    'meeting.join.hint': 'Whoever received the key enters. Whoever gave it just waits.',
    'meeting.join.action': 'Enter the meeting',
    'meeting.key.own': 'That is your own meeting key. Hand it to the people joining.',
    'link.closed': 'The other person left.',
    'link.lost': 'The connection to the other person was lost. They can come back with the same meeting key — still waiting.',
    'browser.only': 'Opened in a browser. Meetings only work inside the warifu window.',
    'camera.denied': 'Camera and microphone are not allowed. Allow them in your OS settings.',
    'camera.missing': 'No camera or microphone found.',
    'camera.busy': 'Another app is using the camera.',
    'meeting.key.copy': 'Copy',
    'meeting.key.copied': 'Copied',
    'meeting.key.reveal': 'Show the full meeting key',
    'meeting.key.more': 'Create another meeting key',
    'meeting.key.more.hint': 'One meeting key lets one person in. To invite a third person, create another key and give it to them. The earlier keys keep working.',
    'roster.name.action': 'Name',
    'roster.name.save': 'Save',
    'roster.name.placeholder': 'A name (e.g. Agent on Mac Air)',
    'meeting.status.waiting': 'Waiting for the other person',
    'meeting.status.live': 'In a meeting',
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
    'meeting.join.working': 'Entering…',
    'chat.title': 'Chat',
    'chat.hint': 'Goes to everyone in the meeting. Not saved — it disappears when you close.\n',
    'chat.placeholder': 'Type and press Enter (Shift+Enter for a new line)',
    'chat.send': 'Send',
    'chat.empty': 'Nothing yet',
    'chat.joined': '{who} joined',
    'chat.left': '{who} left',
    'chat.lost': 'The connection to {who} was lost',
    'chat.nobody': 'Nobody is here yet. Share a meeting key and wait for someone to join.',
    'chat.agent': 'AI on this computer',
    'chat.desk': 'The AI on this computer is here. You can talk to it even with nobody else in the meeting.',
    'chat.placeholder.desk': 'Talk to the AI on this computer',
    'pane.contacts': 'Contacts',
    'pane.meeting': 'Meeting',
    'pane.schedule': 'Schedule',
    'contacts.this': 'This computer',
    'contacts.me': 'You',
    'contacts.desk': 'AI on this computer',
    'contacts.desk.none': 'No agent is at the desk. Connect one with `warifu mcp` and it appears here.',
    'contacts.inmeeting': 'In the meeting now',
    'contacts.saved': 'People you remember',
    'contacts.empty': 'You have not remembered anyone yet. Name someone you met in a meeting and they stay here.',
    'contacts.pick': 'Pick someone to see what you can do.',
    'contacts.key.label': 'Public key',
    'contacts.presence.none': 'There is no way to tell whether they are running right now. You find out by calling.',
    'contacts.forget': 'Require a meeting key again',
    'contacts.forget.hint': 'Right now this person can come in without a meeting key. Turn it off and they will need one again.',
    'contacts.rename.hint': 'Right-click a name to give it a nickname.',
    'act.chat': 'Chat',
    'act.call': 'Invite to a meeting',
    'act.call.working': 'Calling…',
    'act.mail': 'Send mail',
    'act.mail.none': 'Not possible yet. warifu has no way to send mail at all (it can only read).',
    'act.address.none': 'Their whereabouts are not remembered yet. Connect once with a meeting key and it is remembered. Until then you cannot call them.',
    'act.already': 'Already in this meeting.',
    'act.desk.local': 'The AI on this computer is at the same desk. There is nothing to invite.',
    'act.desk.empty': 'No agent is at the desk.',
    'chat.scope': 'There is one conversation. It reaches everyone in the meeting and the AI on this computer.',
    'schedule.title': 'Schedule',
    'schedule.none': 'The schedule pane does not work yet. There is no way for the window to read a calendar.',
    'chat.placeholder.nobody': 'You can send once someone joins',
    'call.mic': 'Microphone',
    'call.camera': 'Camera',
    'call.controls': 'Turn off what you are sending right now. Separate from the pre-join check.',
    'setup.title': 'Before you enter',
    'setup.hint': 'Check what you look and sound like before entering.',
    'setup.action': 'Check camera and microphone',
    'setup.mic': 'Enter with the microphone on',
    'setup.camera': 'Enter with the camera on',
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
    'meeting.start.title': '开始会议',
    'meeting.key.label': '会议密钥',
    'meeting.key.hint': '把它交给参会者。只有拿到的人才能进入。24 小时后失效。可以写在纸上，也可以念给对方。',
    'meeting.start.action': '开始会议',
    'meeting.join.title': '用收到的会议密钥进入',
    'meeting.join.hint': '收到密钥的一方进入，交出的一方等待即可。',
    'meeting.join.action': '进入会议',
    'meeting.key.own': '这是你自己的会议密钥。请交给参加者。',
    'link.closed': '对方已离开。',
    'link.lost': '与对方的连接已中断。对方可以用同一个会议密钥重新进入 — 仍在等待。',
    'browser.only': '正在浏览器中打开。会议只能在 warifu 窗口里进行。',
    'camera.denied': '未允许使用摄像头和麦克风。请在系统设置中允许。',
    'camera.missing': '找不到摄像头或麦克风。',
    'camera.busy': '其他应用正在使用摄像头。',
    'meeting.key.copy': '复制',
    'meeting.key.copied': '已复制',
    'meeting.key.reveal': '查看完整会议密钥',
    'meeting.key.more': '再生成一个会议密钥',
    'meeting.key.more.hint': '一个会议密钥只能让一个人进入。要邀请第三个人，请再生成一个并交给对方。之前的密钥仍然可用。',
    'roster.name.action': '命名',
    'roster.name.save': '保存',
    'roster.name.placeholder': '名称（例：Mac Air 上的智能体）',
    'meeting.status.waiting': '正在等待对方',
    'meeting.status.live': '会议中',
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
    'chat.hint': '发送给会议中的所有人。不会保存 — 关闭后消失。\n',
    'chat.placeholder': '输入后按 Enter（换行用 Shift+Enter）',
    'chat.send': '发送',
    'chat.empty': '还没有内容',
    'chat.joined': '{who} 已加入',
    'chat.left': '{who} 已离开',
    'chat.lost': '与 {who} 的连接已中断',
    'chat.nobody': '还没有人加入。把会议密钥交给对方，等对方进入后即可发送。',
    'chat.agent': '这台电脑的 AI',
    'chat.desk': '这台电脑的 AI 在。即使会议里没有其他人，也可以对它说话。',
    'chat.placeholder.desk': '对这台电脑的 AI 说话',
    'pane.contacts': '通讯录',
    'pane.meeting': '会议',
    'pane.schedule': '日程',
    'contacts.this': '这台电脑',
    'contacts.me': '你',
    'contacts.desk': '这台电脑的 AI',
    'contacts.desk.none': '桌旁没有人。用 `warifu mcp` 连接后会出现在这里。',
    'contacts.inmeeting': '正在会议中的人',
    'contacts.saved': '记住的对方',
    'contacts.empty': '还没有记住任何人。给会议中遇到的人取个称呼，就会留在这里。',
    'contacts.pick': '选择一位，就会显示可以做的事。',
    'contacts.key.label': '公钥',
    'contacts.presence.none': '无法知道对方现在是否已启动。只有呼叫之后才知道。',
    'contacts.forget': '恢复需要会议密钥',
    'contacts.forget.hint': '现在这位不用会议密钥就能进来。取消后，下次就需要会议密钥了。',
    'contacts.rename.hint': '右键点击名字即可取一个称呼。',
    'act.chat': '聊天',
    'act.call': '邀请进会议',
    'act.call.working': '正在呼叫…',
    'act.mail': '发送邮件',
    'act.mail.none': '还发不了。warifu 目前完全没有发送邮件的通道（只有读取的口）。',
    'act.address.none': '还没有记住对方的位置。用会议密钥连接一次后就会记住。在那之前无法从这边呼叫。',
    'act.already': '已经在这个会议里了。',
    'act.desk.local': '这台电脑的 AI 就在同一张桌旁，不需要邀请进会议。',
    'act.desk.empty': '桌旁没有人。',
    'chat.scope': '这是一条会话。会送达会议里的所有人，以及这台电脑的 AI。',
    'schedule.title': '日程',
    'schedule.none': '日程还不能用。画面还没有读取日程表的口。',
    'chat.placeholder.nobody': '有人进入后即可发送',
    'call.mic': '麦克风',
    'call.camera': '摄像头',
    'call.controls': '关闭当前正在发送的内容。与入会前的确认不同。',
    'setup.title': '进入前的准备',
    'setup.hint': '进入之前，先确认自己的画面和声音。',
    'setup.action': '检查摄像头和麦克风',
    'setup.mic': '开着麦克风进入',
    'setup.camera': '开着摄像头进入',
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
    'meeting.start.title': '회의 시작하기',
    'meeting.key.label': '회의 키',
    'meeting.key.hint': '참가자에게 건네주세요. 받은 사람만 입장할 수 있습니다. 24시간 후 만료됩니다. 종이에 적어도, 읽어 주어도 전달할 수 있습니다.',
    'meeting.start.action': '회의 시작하기',
    'meeting.join.title': '받은 회의 키로 입장하기',
    'meeting.join.hint': '받은 쪽이 입장합니다. 건넨 쪽은 기다리기만 하면 됩니다.',
    'meeting.join.action': '입장하기',
    'meeting.key.own': '자신의 회의 키입니다. 참가자에게 건네주세요.',
    'link.closed': '상대방이 나갔습니다.',
    'link.lost': '상대방과의 연결이 끊어졌습니다. 같은 회의 키로 다시 들어올 수 있습니다 — 기다리는 중입니다.',
    'browser.only': '브라우저에서 열렸습니다. 회의는 warifu 창에서만 동작합니다.',
    'camera.denied': '카메라와 마이크가 허용되지 않았습니다. OS 설정에서 허용해 주세요.',
    'camera.missing': '카메라나 마이크를 찾을 수 없습니다.',
    'camera.busy': '다른 앱이 카메라를 사용 중입니다.',
    'meeting.key.copy': '복사하기',
    'meeting.key.copied': '복사했습니다',
    'meeting.key.reveal': '회의 키 전문 보기',
    'meeting.key.more': '회의 키를 하나 더 만들기',
    'meeting.key.more.hint': '회의 키 하나로 들어올 수 있는 사람은 한 명입니다. 세 번째 사람을 부르려면 하나 더 만들어 건네주세요. 이전 키는 그대로 쓸 수 있습니다.',
    'roster.name.action': '이름 붙이기',
    'roster.name.save': '저장',
    'roster.name.placeholder': '이름 (예: Mac Air 에이전트)',
    'meeting.status.waiting': '상대방을 기다리는 중',
    'meeting.status.live': '회의 중',
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
    'meeting.join.working': '입장 중…',
    'chat.title': '문자로 대화',
    'chat.hint': '회의에 있는 사람에게 전달됩니다. 남지 않습니다 — 닫으면 사라집니다.\n',
    'chat.placeholder': '입력 후 Enter (줄바꿈은 Shift+Enter)',
    'chat.send': '보내기',
    'chat.empty': '아직 아무것도 없습니다',
    'chat.joined': '{who} 님이 입장했습니다',
    'chat.left': '{who} 님이 퇴장했습니다',
    'chat.lost': '{who} 님과의 연결이 끊어졌습니다',
    'chat.nobody': '아직 아무도 없습니다. 회의 키를 건네고 상대가 들어오면 보낼 수 있습니다.',
    'chat.agent': '이 PC 의 AI',
    'chat.desk': '이 PC 의 AI 가 있습니다. 회의에 사람이 없어도 말을 걸 수 있습니다.',
    'chat.placeholder.desk': '이 PC 의 AI 에게 말을 걸기',
    'pane.contacts': '연락처',
    'pane.meeting': '회의',
    'pane.schedule': '일정',
    'contacts.this': '이 PC',
    'contacts.me': '당신',
    'contacts.desk': '이 PC 의 AI',
    'contacts.desk.none': '책상에 아무도 없습니다. `warifu mcp` 로 연결하면 여기에 나옵니다.',
    'contacts.inmeeting': '지금 회의에 있는 사람',
    'contacts.saved': '기억한 상대',
    'contacts.empty': '아직 아무도 기억하지 않았습니다. 회의에서 만난 상대에게 이름을 붙이면 여기에 남습니다.',
    'contacts.pick': '상대를 고르면 할 수 있는 일이 나옵니다.',
    'contacts.key.label': '공개키',
    'contacts.presence.none': '상대가 지금 켜져 있는지는 알 수 없습니다. 불러 봐야 알 수 있습니다.',
    'contacts.forget': '다시 회의 키를 받게 하기',
    'contacts.forget.hint': '지금 이 상대는 회의 키 없이 들어올 수 있습니다. 끄면 다음부터는 회의 키가 필요합니다.',
    'contacts.rename.hint': '이름을 오른쪽 클릭하면 호칭을 붙일 수 있습니다.',
    'act.chat': '채팅하기',
    'act.call': '회의에 부르기',
    'act.call.working': '부르는 중…',
    'act.mail': '메일 보내기',
    'act.mail.none': '아직 보낼 수 없습니다. warifu 에는 메일을 보내는 경로가 하나도 없습니다 (읽는 입구만 있습니다).',
    'act.address.none': '상대의 위치를 아직 기억하지 못했습니다. 회의 키로 한 번 연결하면 기억합니다. 그전에는 이쪽에서 부를 수 없습니다.',
    'act.already': '이미 이 회의에 있습니다.',
    'act.desk.local': '이 PC 의 AI 는 같은 책상에 있습니다. 회의에 부를 필요가 없습니다.',
    'act.desk.empty': '책상에 아무도 없습니다.',
    'chat.scope': '이 대화는 하나입니다. 회의에 있는 사람과 이 PC 의 AI 모두에게 갑니다.',
    'schedule.title': '일정',
    'schedule.none': '일정 화면은 아직 움직이지 않습니다. 화면에서 일정표를 읽는 입구가 아직 하나도 없습니다.',
    'chat.placeholder.nobody': '누군가 들어오면 보낼 수 있습니다',
    'call.mic': '마이크',
    'call.camera': '카메라',
    'call.controls': '지금 보내고 있는 것을 끕니다. 입장 전 확인과는 별개입니다.',
    'setup.title': '입장 전 준비',
    'setup.hint': '들어가기 전에 자신의 화면과 소리를 확인할 수 있습니다.',
    'setup.action': '카메라와 마이크 확인',
    'setup.mic': '마이크를 켠 채로 입장',
    'setup.camera': '카메라를 켠 채로 입장',
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
  'chat.scope',
] as const;

/** 翻訳者への注記。**訳文と一緒に渡す。** */
export const TRANSLATOR_NOTES: Partial<Record<MessageKey, string>> = {
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
  'chat.scope':
    '「1 本の会話」＝ 会議に居る全員と、この PC の AI に届く、という意味。' +
    '「この相手にだけ届きます」「非公開の会話です」と読める訳にしないこと。' +
    '個別に届くと誤解した人は、見られたくないものを書く。',
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
    '「渡した相手だけが入室できる」は仕組みの説明であって、安全の保証ではない。' +
    '「安全です」と読める訳にしないこと — **会議キーを他人に見られたら、その人が入室できる。**',
  'link.lost':
    '**切れたが、戻ってこられる**という意味（D44）。戻れるのは**さっきまで入っていた同じ相手だけ**で、' +
    '会議キーが別人に渡っても意味は無い（一回性＝D12 は崩していない）。' +
    '「誰でも入れるようになりました」と読める訳にしないこと。' +
    'また「自動で再接続します」とも読ませないこと — **戻るのは相手の操作**であり、こちらは待っているだけである。',
};

/** 差し込み口を埋める。無い鍵はそのまま残す（黙って空にしない）。 */
export function format(template: string, values: Record<string, string | number>): string {
  return template.replace(/\{(\w+)\}/g, (whole, name: string) =>
    name in values ? String(values[name]) : whole,
  );
}
