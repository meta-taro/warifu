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
  | 'update.check'
  | 'update.checking'
  | 'update.none'
  | 'update.failed'
  | 'update.notes'
  | 'update.notes.none'
  | 'update.downloading'
  | 'update.installed'
  | 'update.later'
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
  | 'meeting.key.count'
  | 'meeting.key.each'
  | 'meeting.key.nth'
  | 'key.hand'
  | 'key.hand.title'
  | 'key.hand.hint'
  | 'key.hand.for'
  | 'key.hand.again'
  | 'meeting.key.howmany'
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
  | 'mark.sent'
  | 'mark.sent.hint'
  | 'mark.kept'
  | 'mark.kept.hint'
  | 'mark.none'
  | 'mark.none.hint'
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
  | 'contacts.claimed'
  | 'contacts.where'
  | 'contacts.rooms'
  | 'room.members.some'
  | 'room.alone'
  | 'room.invite'
  | 'room.invite.title'
  | 'room.invite.hint'
  | 'room.host'
  | 'contacts.empty'
  | 'contacts.pick'
  | 'home.title'
  | 'home.lead'
  | 'home.now'
  | 'home.seats'
  | 'home.rooms'
  | 'home.people'
  | 'home.rooms.n'
  | 'home.contacts'
  | 'home.postbox'
  | 'home.postbox.on'
  | 'home.postbox.off'
  | 'home.chat'
  | 'home.group'
  | 'home.call'
  | 'home.calendar'
  | 'home.next'
  | 'home.next.hint'
  | 'contacts.key.label'
  | 'contacts.me.what'
  | 'contacts.me.share'
  | 'meeting.link.label'
  | 'meeting.link.copy'
  | 'meeting.copy.which'
  | 'meeting.link.hint'
  | 'meeting.qr.reveal'
  | 'meeting.qr.hint'
  | 'link.invited'
  | 'link.invited.hint'
  | 'link.invited.enter'
  | 'link.invited.no'
  | 'contacts.me.key.what'
  | 'howto.title'
  | 'howto.1'
  | 'howto.2'
  | 'howto.3'
  | 'howto.4'
  | 'howto.key'
  | 'contacts.me.copy'
  | 'contacts.me.copied'
  | 'contacts.me.name'
  | 'profile.edit'
  | 'profile.name'
  | 'profile.bio'
  | 'profile.save'
  | 'profile.cancel'
  | 'profile.none'
  | 'profile.ai.who'
  | 'profile.ai.hint'
  | 'profile.face'
  | 'profile.face.drop'
  | 'crop.title'
  | 'crop.hint'
  | 'crop.zoom'
  | 'crop.apply'
  | 'profile.face.clear'
  | 'face.big.open'
  | 'face.clear.confirm'
  | 'face.clear.confirm.hint'
  | 'face.clear.do'
  | 'contacts.note'
  | 'contacts.note.hint'
  | 'room.name'
  | 'room.nth'
  | 'room.leave'
  | 'room.leave.confirm'
  | 'room.leave.hint'
  | 'room.leave.do'
  | 'room.name.hint'
  | 'contacts.desk.how'
  | 'connect.open'
  | 'connect.title'
  | 'connect.lead'
  | 'connect.step1'
  | 'connect.step2'
  | 'connect.step3'
  | 'connect.wake'
  | 'connect.paste'
  | 'connect.paste.hint'
  | 'connect.paste.body'
  | 'connect.copy'
  | 'connect.copied'
  | 'connect.docs'
  | 'chat.shared'
  | 'chat.reach'
  | 'chat.reach.none'
  | 'contacts.presence.none'
  | 'reunion.title'
  | 'reunion.call'
  | 'reunion.wait'
  | 'reunion.key'
  | 'reunion.room'
  | 'reunion.name'
  | 'presence.on'
  | 'presence.off'
  | 'help.open'
  | 'help.close'
  | 'help.this.title'
  | 'help.this.me'
  | 'help.this.on'
  | 'help.this.off'
  | 'help.this.others'
  | 'contacts.forget'
  | 'contacts.forget.hint'
  | 'act.chat'
  | 'act.chat.live'
  | 'act.chat.postbox'
  | 'act.chat.desk'
  | 'act.chat.desk.none'
  | 'act.group'
  | 'act.group.what'
  | 'act.group.action'
  | 'act.group.desk.none'
  | 'act.call'
  | 'act.call.net'
  | 'act.call.action'
  | 'act.call.desk.none'
  | 'act.call.working'
  | 'act.calendar'
  | 'act.calendar.none'
  | 'act.state.ok'
  | 'act.state.wait'
  | 'act.state.no'
  | 'act.address.none'
  | 'act.already'
  | 'act.desk.local'
  | 'act.desk.empty'
  | 'act.desk.stop'
  | 'act.desk.stop.hint'
  | 'schedule.title'
  | 'schedule.none'
  | 'schedule.add'
  | 'schedule.date'
  | 'schedule.time'
  | 'schedule.minutes'
  | 'schedule.what'
  | 'schedule.note'
  | 'schedule.save'
  | 'schedule.empty'
  | 'schedule.past'
  | 'schedule.now'
  | 'schedule.remove'
  | 'schedule.remove.confirm'
  | 'schedule.remove.hint'
  | 'schedule.mine'
  | 'schedule.bad'
  | 'chat.placeholder.nobody'
  | 'call.mic'
  | 'call.camera'
  | 'call.controls'
  | 'meeting.key.own'
  | 'video.start'
  | 'video.stop'
  | 'video.title'
  | 'video.hint'
  | 'video.off.hint'
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
    'update.check': '更新を確認する',
    'update.checking': '確認しています…',
    'update.none': 'いまが最新です（{version}）。',
    'update.failed': '確認できませんでした（{why}）。',
    'update.notes': '何が変わったか',
    'update.notes.none': '書かれていません。',
    'update.downloading': '落としています… {percent}%',
    'update.installed': '入れ替えました。立て直すと新しいほうになります。',
    'update.later': 'あとで',
    'revoke.irreversible': 'この失効は取り消せません。',
    'revoke.confirm': '{device} を失効させる',
    'door.refused': '断りました。',
    'link.direct': '直接',
    'link.relayed': '中継',
    'link.unknown': '不明',
    'roster.capacity': '{current} / {capacity}',
    'tile.me': '自分',
    'tile.peer': '相手',
    'meeting.start.title': 'ルームをつくる',
    'meeting.key.label': 'ルームキー',
    'meeting.key.count': '{n} 本出しました',
    'meeting.key.each': '1 本につき 1 人です。呼ぶ人ごとに、別のルームキーを渡してください。',
    'meeting.key.nth': '{n} 本目',
    'key.hand': 'この人にルームキーを渡す',
    'key.hand.title': '{name} に渡す 1 本',
    'key.hand.hint': 'これは {name} ぶんの 1 本です。1 本につき 1 人しか入れません。リンクか QR、またはルームキーの文字を、別の手段（対面・電話・ほかのチャット）で渡してください。',
    'key.hand.for': '{name} へ渡した',
    'key.hand.again': 'この人にはもう 1 本渡してあります。渡し損じたときは、もう 1 本出せます（前の 1 本も生きています）。',
    'meeting.key.howmany': '何人ぶん',
    'meeting.key.hint': 'これを入ってほしい人に渡します。渡した相手だけが入れます。24 時間で切れます。紙に書いても、読み上げても渡せます。',
    'meeting.start.action': 'ルームをつくる',
    'meeting.join.title': 'もらったルームキーでルームに入る',
    'meeting.join.hint': 'もらった側が入ります。渡した側は待つだけです。',
    'meeting.join.action': 'ルームに入る',
    'meeting.key.own': 'これは自分のルームキーです。入ってほしい人に渡してください。',
    'link.closed': '相手が退出しました。',
    'link.lost': '相手との経路が切れました。同じルームキーで戻ってこられます — 待っています。',
    'browser.only': 'ブラウザで開いています。ルームは warifu の窓でだけ動きます。',
    'camera.denied': 'カメラとマイクが許可されていません。OS の設定で許可してください。',
    'camera.missing': 'カメラかマイクが見つかりません。',
    'camera.busy': 'ほかのアプリがカメラを使っています。',
    'meeting.key.copy': 'ルームキーをコピー',
    'meeting.key.copied': 'コピーしました',
    'meeting.key.reveal': 'ルームキーの全文を見る',
    'meeting.key.more': 'ルームキーをもう 1 本出す',
    'meeting.key.more.hint': '1 本のルームキーで入れるのは 1 人だけです。もう 1 人入れるなら、もう 1 本出してその人に渡します。前のルームキーは使えたままです。',
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
    'chat.hint': '同じルームに入っている人へ届きます。残りません — 閉じると消えます。',
    'chat.placeholder': '書いて Enter（改行は Shift+Enter）',
    'chat.send': '送る',
    'chat.empty': 'まだ何もありません',
    'chat.joined': '{who} が入りました',
    'chat.left': '{who} が退室しました',
    'chat.lost': '{who} との経路が切れました',
    'chat.nobody': 'ルームにはまだ誰も入っていません。ルームキーを渡して、入ってもらうと送れます。',
    'chat.agent': 'マイ PC エージェント',
    'chat.desk': 'この PC のエージェントがつながっています。ルームに人が入っていなくても話しかけられます。',
    'chat.placeholder.desk': 'マイ PC エージェントに話しかける',
    'chat.late': '留守中',
    'send.absent': 'いま届きません。留守中の受け取りを設定すると、留守でも届きます。',
    'postbox.title': '留守中の受け取り（任意）',
    'postbox.hint': 'あなたが起動していない間、届いた言葉を封のまま預かってもらう所です。中身は預かる側にも読めません。設定しなければ、あなたが起動している間だけ届きます。',
    'postbox.placeholder': '預かってもらう相手の宛先を貼り付ける',
    'postbox.save': '設定する',
    'postbox.clear': '外す',
    'postbox.saved': '留守中の受け取りを設定しました。',
    'postbox.cleared': '留守中の受け取りを外しました。',
    'postbox.kept': 'いま相手に届かないので、相手の受け取り先へ預けました。相手が起動したときに届きます。',
    'mark.sent': '送信済み',
    'mark.sent.hint': '相手の機械まで届きました。読んだかどうかは分かりません（既読を集める機械が無いためです）。',
    'mark.kept': '受信待ち',
    'mark.kept.hint': '相手が受け取れるようになったら届きます。それまでは受け取り先が封のまま預かっています。',
    'mark.none': '送れていません',
    'mark.none.hint': '経路も受け取り先も無いので、どこにも残っていません。',
    'postbox.received': '留守中に届いていた分が {n} 通ありました。',
    'pane.contacts': '連絡帳',
    'pane.meeting': 'ルーム',
    'pane.schedule': '予定',
    'contacts.this': 'この PC',
    'contacts.me': 'あなた',
    'contacts.desk': 'マイ PC エージェント',
    'contacts.desk.none': 'このエージェントは、いま繋がっていません。',
    'contacts.inmeeting': 'いま同じルームの人',
    'contacts.saved': '連絡先',
    'contacts.late': '留守中に届いた相手',
    'contacts.claimed': '本人は「{name}」と名乗っています。',
    'contacts.where': '話しかけるときは、会話の欄に打ちます。',
    'contacts.rooms': 'ルーム',
    'room.members.some': '人が入っています。',
    'room.alone': 'まだ自分だけです。ルームキーを渡すと入ってもらえます。',
    'room.invite': 'このルームに人を呼ぶ',
    'room.invite.title': 'このルームに渡す 1 本',
    'room.invite.hint': 'この 1 本で 1 人が入れます。リンクか QR、またはルームキーの文字を、呼びたい相手に渡してください。もう 1 人呼ぶなら、もう 1 本出します。',
    'room.host': 'あなたが作ったルームです',
    'contacts.empty': 'まだ誰も覚えていません。ルームで会った相手に呼び名を付けると、ここに残ります。',
    'contacts.pick': '相手を選ぶと、できることが出ます。',
    'home.title': 'できること',
    'home.lead': '相手を選ぶと、その相手にできることが出ます。左の一覧から選んでください。',
    'home.now': 'いまの様子',
    'home.seats': 'この PC のエージェント',
    'home.rooms': 'ルーム',
    'home.people': '{n} 人',
    'home.rooms.n': '{n} 室（{m} 人）',
    'home.contacts': '連絡先',
    'home.postbox': '留守中の受け取り',
    'home.postbox.on': '設定済み',
    'home.postbox.off': '未設定',
    'home.chat': '相手を選ぶと、その人とだけの会話になります。',
    'home.group': 'ルームを作って、人数ぶんのルームキーを渡します。1 本につき 1 人です。',
    'home.call': '同じ網なら繋がります。別の網は --relay が要りますが、繋がったことをまだ一度も見ていません。',
    'home.calendar': '自分の予定を書いて置けます。相手の予定は見えません（渡るのは空いている枠だけです）。',
    'home.next': 'はじめの一歩',
    'home.next.hint': 'まだ誰もつながっていません。左の「この PC」のあなたの行を押すと、ルームキーの渡し方が出ます。',
    'contacts.key.label': '公開鍵（あなたの ID）',
    'contacts.me.what': 'これがこの PC のあなたです。閉じても同じ人でいられます。',
    'contacts.me.share': 'この公開鍵は、相手に見せて構いません。これだけでは誰も入ってこられません（入るにはルームキーが要ります）。',
    'meeting.link.label': '入ってもらうリンク',
    'meeting.link.copy': 'リンクをコピー',
    'meeting.copy.which': 'ふだんは「リンクをコピー」で渡します。相手が押すだけで入れます。リンクが使えない相手（CLI・紙・リンクを剥がすチャット）には、「ルームキーをコピー」で文字を渡してください。',
    'meeting.link.hint': 'これを相手に送ります。相手が押すと割符が開いて、入るかどうかを尋ねます（勝手には入りません）。相手にも割符が入っている必要があります。',
    'meeting.qr.reveal': 'QR で見せる',
    'meeting.qr.hint': '目の前の相手に読ませるときはこちら。読み取ると同じリンクになります。',
    'link.invited': 'リンクでルームに誘われています。入りますか？',
    'link.invited.hint': 'このリンクは誰でも作れます。心当たりのない誘いには入らないでください。',
    'link.invited.enter': '入る',
    'link.invited.no': '入らない',
    'contacts.me.key.what': '公開鍵は「あなたが誰か」を表す名前です。これで相手とつながるのではありません。相手の連絡帳に載せてもらうときや、受け取り先の名簿に書いてもらうときに渡します。',
    'howto.title': '知り合いとつながる手順',
    'howto.1': '「ルーム」を開いて［ルームをつくる］を押す',
    'howto.2': '出てきたルームキー（またはリンク）を、相手に渡す',
    'howto.3': '相手が受け取って入ると、つながります',
    'howto.4': 'つながったら呼び名を付ける。次からは連絡帳から呼べます（ルームキーは要りません）',
    'howto.key': 'この手順で渡すのは「ルームキー」です。上の公開鍵ではありません。',
    'contacts.me.copy': '公開鍵（あなたの ID）をコピー',
    'contacts.me.copied': 'コピーしました',
    'contacts.me.name': 'ここで名乗った名前は、相手の画面にも出ます。ただし相手が呼び名を付けていれば、そちらが優先されます（名乗った名前は誰でも真似できるためです）。',
    'profile.edit': 'プロフィールを書く',
    'profile.name': '名前',
    'profile.bio': '短い紹介',
    'profile.save': '決める',
    'profile.cancel': 'やめる',
    'profile.none': 'まだ書いていません。',
    'profile.ai.who': 'マイ PC エージェント',
    'profile.ai.hint': '名前と紹介は、あなたと、このエージェント自身が書けます。ほかのエージェントは書き換えられません。',
    'profile.face': '顔',
    'profile.face.drop': 'アバター画像に画像をドラッグアンドドロップすると、切り取ってから差し替えられます（PNG・JPG・WebP）。',
    'crop.title': '顔にする所を選ぶ',
    'crop.hint': 'つまんで動かし、大きさを変えられます。丸の中が顔になります。',
    'crop.zoom': '大きさ',
    'crop.apply': 'これにする',
    'profile.face.clear': '初期アバターに戻す',
    'face.big.open': '顔を大きく見る',
    'face.clear.confirm': '初期アバターに戻しますか？',
    'face.clear.confirm.hint': '差し替えた画像は消えます。もう一度落とせば、入れ直せます。',
    'face.clear.do': '戻す',
    'contacts.note': '覚え書き',
    'contacts.note.hint': 'この相手が「どの機械の、何をする人（エージェント）か」を、自分の言葉で書けます。相手には送りません。',
    'room.name': 'ルームの名前',
    'room.nth': 'ルーム {n}（{m} 人）',
    'room.leave': 'このルームを抜ける',
    'room.leave.confirm': 'このルームを抜けますか？',
    'room.leave.hint': '抜けたことは、このルームの全員に伝わります。ここの会話は残りません。入り直すには、ルームキーをもう一度もらってください。',
    'room.leave.do': '抜ける',
    'room.name.hint': '名前はこの画面の中だけです（閉じると消えます）。相手には送りません。',
    'contacts.desk.how': '繋ぐには、そのエージェントの設定に warifu を足して、立ち上げ直します（手順は docs/mcp.md）。',
    'connect.open': 'つなぎ方を出す',
    'connect.title': 'このエージェントをつなぐ',
    'connect.lead': 'これは、そのエージェントの端末で叩くものです（動いている Claude の中ではありません）。叩いて設定を書いたら、エージェントを立ち上げ直してください —— 割符の口は起動時に読まれます。',
    'connect.step1': '1. 割符の口を、そのエージェントの設定に足す',
    'connect.step2': '2. エージェントを立ち上げ直す（設定は起動時に読まれます）',
    'connect.step3': '3. これで丸が緑になり、打った言葉が届きます',
    'connect.wake': '待たずに反応させる（届いた言葉で起こす）',
    'connect.paste': 'もう動いているエージェントに貼る',
    'connect.paste.hint': 'すでに立ち上げてあるエージェントには、下の文をそのまま貼って頼めます（この PC のエージェント向け）。設定を書いても、立ち上げ直すまでは口が開きません。',
    'connect.paste.body': '割符（warifu）に繋がってください。手順は 2 つです。1) 端末で次を実行: {cmd} 2) 書けたら、私を立ち上げ直すようにこの PC の持ち主へ伝えてください（MCP の口は起動時に読まれます）。立ち上げ直したあと、warifu の chat_read で届いた発言を読めます。',
    'connect.copy': 'コピー',
    'connect.copied': '写しました',
    'connect.docs': '許す動作は人が書きます（既定は拒否）。詳しくは docs/mcp.md。',
    'chat.shared': 'ここは、選んだ相手だけの会話ではありません。',
    'chat.reach': '届く先 {who}',
    'chat.reach.none': '届く先はまだありません。ルームに人が入るか、この PC のエージェントがつながると出ます。',
    'contacts.presence.none': '相手がいま起動しているかは分かりません。呼んでみるまで分かりません。',
    'reunion.title': '次に会うとき',
    'reunion.call': '連絡先から呼べます（ルームキーは要りません）。ただし相手の側でも覚えていないと、呼んでも通りません。',
    'reunion.wait': 'こちらからは呼べません（居場所を知りません）。相手から呼んでもらえば通ります。',
    'reunion.key': '初回と同じで、ルームキーを渡し合います。次からルームキーなしにしたいなら、名前の隣の鉛筆で呼び名を付けてください。',
    'reunion.room': 'ルームに名前を付けておくと、次に会うとき「どのルームの話か」がお互いに分かります。',
    'reunion.name': '呼び名を付ける',
    'presence.on': 'つながっています',
    'presence.off': '切れています',
    'help.open': '説明を出す',
    'help.close': '閉じる',
    'help.this.title': 'この PC の行',
    'help.this.me': 'あなた —— この PC の持ち主です。いつも出ます（顔の右下はいつも緑）。',
    'help.this.on': '緑の丸（顔の右下）—— いまこの PC の口につながっているエージェントです。打てば届きます。',
    'help.this.off': 'グレーの丸 —— 以前つながって名乗りが残っているエージェントです。打っても届きません。そちらで立ち上げ直すと、緑に戻ります。',
    'help.this.others': '下の「連絡先」には丸を出しません。相手が起動しているかは、こちらから分からないためです。',
    'contacts.forget': 'ルームキーなしで入れるのをやめる',
    'contacts.forget.hint': 'この相手はいま、ルームキーなしで入ってこられます。やめると、次からはルームキーが要ります。',
    'act.chat': 'チャット',
    'act.chat.live': '繋がっている間だけ届きます。相手が起動していないときは、打ったものはどこにも残りません。留守中の受け取りを設定すると、留守のときも預けられます。',
    'act.chat.postbox': 'この相手とだけの会話です。相手が留守のときは、受け取り先へ封のまま預けます（7 日）。',
    'act.chat.desk': 'この PC のエージェントとだけの会話です。会話の欄に打つと、そのエージェントにだけ届きます。',
    'act.chat.desk.none': 'いま繋がっていないので、打っても届きません。',
    'act.group': 'グループチャット',
    'act.group.what': 'ルームを作って、この相手を招きます。押すと、人数ぶんのルームキーを出す所へ移ります。',
    'act.group.action': 'ルームを作る',
    'act.group.desk.none': '繋がっていないエージェントは、ルームにも呼べません。',
    'act.call': 'ビデオ会議',
    'act.call.net': '同じ網なら、そのまま繋がります。別の網は --relay が要りますが、繋がったことをまだ一度も見ていません。',
    'act.call.action': 'ビデオ会議に呼ぶ',
    'act.call.desk.none': 'つながっていません。つながれば、はじめから同じ会話に入っています。',
    'act.call.working': '呼んでいます…',
    'act.calendar': '予定',
    'act.calendar.none': '画面から予定表を読む口が、まだ 1 本もありません。空いている枠も出せません。',
    'act.state.ok': 'できる',
    'act.state.wait': '条件つき',
    'act.state.no': 'まだできない',
    'act.address.none': '住所をまだ覚えていません。ルームキーで一度つながると覚えます。それまでは、こちらから呼べません。',
    'act.already': 'すでに同じルームです。',
    'act.desk.local': 'この PC のエージェントは、はじめから同じルームに入っています。呼ぶ必要はありません。',
    'act.desk.empty': 'この PC のエージェントは、まだつながっていません。',
    'act.desk.stop': 'このエージェントを止める',
    'act.desk.stop.hint': '「止まれ」と伝えます。相手を殺すのではなく、受けた側が自分で降ります。もう一度動かすには、そちらで立ち上げ直してください。',
    'schedule.title': '予定',
    'schedule.none': '予定の面は、まだ動きません。画面から予定表を読む口が、まだ 1 本もありません。',
    'schedule.add': '予定を書く',
    'schedule.date': '日',
    'schedule.time': '始まり',
    'schedule.minutes': '長さ（分）',
    'schedule.what': '何の予定か',
    'schedule.note': '覚え書き（自分だけ）',
    'schedule.save': '置く',
    'schedule.empty': 'まだ予定はありません。下に書いて［置く］を押すと、ここに並びます。',
    'schedule.past': '終わった予定',
    'schedule.now': 'いま進んでいます',
    'schedule.remove': '消す',
    'schedule.remove.confirm': 'この予定を消しますか？',
    'schedule.remove.hint': '消すと戻せません。覚え書きも一緒に消えます。',
    'schedule.mine': 'ここに置いた予定は、この機械の中だけにあります。相手には渡りません（相手に渡るのは、空いている枠だけです）。',
    'schedule.bad': '日と時刻を、この形で書いてください（日は 2026-09-11、始まりは 9:30）。',
    'chat.placeholder.nobody': '入ってきたら送れます',
    'call.mic': 'マイク',
    'call.camera': 'カメラ',
    'call.controls': '通話の入切。支度の確認とは別で、いま送っているものを止めます。',
    'video.start': 'ビデオ会議を始める',
    'video.stop': 'ビデオ会議をやめる',
    'video.title': 'ビデオ会議',
    'video.hint': 'いまのルームに、映像と音を足します。文字のやりとりはそのまま続きます。',
    'video.off.hint': 'このルームは、いま文字だけです。カメラもマイクも使っていません。',
    'setup.title': '入る前のしたく',
    'setup.hint': 'いま自分が何で映って、何で喋るかを、入る前に確かめられます。',
    'setup.action': 'カメラとマイクを確かめる',
    'setup.mic': 'マイクを入にして入る',
    'setup.camera': 'カメラを入にして入る',
    'setup.blur': '背景をぼかす',
    'setup.blur.os': 'この環境では、アプリから背景をぼかせません。macOS ならコントロールセンターのビデオエフェクトが使えます。',
    'setup.headphones': '同じ室内で 2 台を鳴らすと、エコー除去では消せません。ヘッドフォンを使ってください。',
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
    'update.check': 'Check for updates',
    'update.checking': 'Checking…',
    'update.none': 'You are on the latest ({version}).',
    'update.failed': 'Could not check ({why}).',
    'update.notes': 'What changed',
    'update.notes.none': 'Nothing written.',
    'update.downloading': 'Downloading… {percent}%',
    'update.installed': 'Installed. Restart to run the new one.',
    'update.later': 'Later',
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
    'meeting.key.count': '{n} issued',
    'meeting.key.each': 'One key lets one person in. Hand each person their own key.',
    'meeting.key.nth': 'Key {n}',
    'key.hand': 'Hand a room key to this person',
    'key.hand.title': 'One key for {name}',
    'key.hand.hint': 'This is one room key for {name}. One key lets in one person only. Hand over the link, the QR or the room key text by another route (in person, by phone, in another chat).',
    'key.hand.for': 'handed to {name}',
    'key.hand.again': 'A key has already been handed to this person. If it went astray you can issue another (the earlier one still works).',
    'meeting.key.howmany': 'How many',
    'meeting.key.hint': 'Hand this to the person you want in. Only they can come in. It expires in 24 hours. Writing it down or reading it aloud both work.',
    'meeting.start.action': 'Make a room',
    'meeting.join.title': 'Come in with a room key you were given',
    'meeting.join.hint': 'The one who was given the room key comes in. The one who gave it just waits.',
    'meeting.join.action': 'Come in',
    'meeting.key.own': 'This is your own room key. Hand it to the person you want in.',
    'link.closed': 'The other person left.',
    'link.lost': 'The route to them broke. They can come back with the same key — waiting.',
    'browser.only': 'This is open in a browser. Rooms only work in the warifu window.',
    'camera.denied': 'Camera and microphone are not allowed. Allow them in your OS settings.',
    'camera.missing': 'No camera or microphone found.',
    'camera.busy': 'Another app is using the camera.',
    'meeting.key.copy': 'Copy the room key',
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
    'chat.nobody': 'Nobody is here yet. Hand out a room key and wait for someone to come in.',
    'chat.agent': 'My PC agents',
    'chat.desk': 'An agent on this computer is connected. You can talk to it even with nobody else in the room.',
    'chat.placeholder.desk': 'Talk to an agent on this computer',
    'chat.late': 'while away',
    'send.absent': 'Not here right now. Set up a mailbox and messages will reach them later.',
    'postbox.title': 'Receiving while away (optional)',
    'postbox.hint': 'A place that holds what arrives, sealed, while you are not running. Whoever holds it cannot read it. Without it, messages only arrive while you are running.',
    'postbox.placeholder': 'Paste the address of the holder',
    'postbox.save': 'Set up',
    'postbox.clear': 'Remove',
    'postbox.saved': 'Mailbox set.',
    'postbox.cleared': 'Mailbox removed.',
    'postbox.kept': 'They are not here, so it was left at the mailbox. It arrives when they start up.',
    'mark.sent': 'Sent',
    'mark.sent.hint': 'Handed to their machine. Whether they read it is unknown (there is no machine collecting read receipts).',
    'mark.kept': 'Waiting to be received',
    'mark.kept.hint': 'It arrives once they can receive it. Until then the poste restante holds it sealed.',
    'mark.none': 'Not sent',
    'mark.none.hint': 'There was no route and no poste restante, so it is kept nowhere.',
    'postbox.received': '{n} message(s) had arrived while you were away.',
    'pane.contacts': 'Contacts',
    'pane.meeting': 'Room',
    'pane.schedule': 'Schedule',
    'contacts.this': 'This computer',
    'contacts.me': 'You',
    'contacts.desk': 'My PC agents',
    'contacts.desk.none': 'No agent on this computer is connected. Connect one with `warifu mcp` and it appears here.',
    'contacts.inmeeting': 'In the room now',
    'contacts.saved': 'Contacts',
    'contacts.late': 'Arrived while you were away',
    'contacts.claimed': 'They call themselves “{name}”.',
    'contacts.where': 'To talk to them, type in the conversation box.',
    'contacts.rooms': 'Rooms you are in',
    'room.members.some': 'People are here.',
    'room.alone': 'Just you so far. Hand out a room key and someone can come in.',
    'room.invite': 'Invite someone to this room',
    'room.invite.title': 'One key for this room',
    'room.invite.hint': 'This one room key lets one person in. Hand the link, the QR or the room key text to the person you want. To invite another person, issue another room key.',
    'room.host': 'You made this room',
    'contacts.empty': 'Nobody yet. Give someone you met in a room a name and they stay here.',
    'contacts.pick': 'Pick someone to see what you can do.',
    'home.title': 'What you can do',
    'home.lead': 'Pick someone and what you can do with them appears here. Choose from the list on the left.',
    'home.now': 'Right now',
    'home.seats': 'Agents on this computer',
    'home.rooms': 'Rooms you are in',
    'home.people': '{n} people',
    'home.rooms.n': '{n} rooms ({m} people)',
    'home.contacts': 'Contacts',
    'home.postbox': 'Receiving while away',
    'home.postbox.on': 'Set up',
    'home.postbox.off': 'Not set up',
    'home.chat': 'Pick someone and the conversation is with that person only.',
    'home.group': 'Create a room and hand out one room key per person. One key, one person.',
    'home.call': 'On the same network it connects. A different network needs --relay, and we have not once seen that connect.',
    'home.calendar': 'You can write and keep your own appointments. You cannot see theirs (only free slots are ever handed over).',
    'home.next': 'First step',
    'home.next.hint': 'Nobody here yet. Press your own row under “This computer” on the left to see how to hand over a room key.',
    'contacts.key.label': 'Public key (your ID)',
    'contacts.me.what': 'This is you on this computer. You stay the same person after closing it.',
    'contacts.me.share': 'You can show this public key to anyone. On its own it lets nobody in — coming in needs a room key.',
    'meeting.link.label': 'Link to let them in',
    'meeting.link.copy': 'Copy the link',
    'meeting.copy.which': 'Normally hand over the link — they just press it. When a link will not work (CLI, paper, chats that strip links), copy the room key text instead.',
    'meeting.link.hint': 'Send this to the other person. When they open it, warifu asks them whether to enter (it never enters on its own). They need warifu installed too.',
    'meeting.qr.reveal': 'Show as a QR code',
    'meeting.qr.hint': 'Use this when the other person is in front of you. Scanning it gives the same link.',
    'link.invited': 'A link is inviting you into a room. Enter?',
    'link.invited.hint': 'Anyone can make such a link. Do not enter an invitation you were not expecting.',
    'link.invited.enter': 'Enter',
    'link.invited.no': 'Do not enter',
    'contacts.me.key.what': 'The public key is the name that says who you are. It is not what connects you. You hand it over when someone adds you to their contacts, or when a postbox keeper adds you to their allow list.',
    'howto.title': 'How to connect with someone you know',
    'howto.1': 'Open "Room" and press [Create a room]',
    'howto.2': 'Hand the room key (or the link) to the other person',
    'howto.3': 'They take it and enter, and you are connected',
    'howto.4': 'Once connected, give them a name. From then on you can call them from Contacts (no room key needed)',
    'howto.key': 'What you hand over here is the room key, not the public key above.',
    'contacts.me.copy': 'Copy public key',
    'contacts.me.copied': 'Copied',
    'contacts.me.name': 'The name you set here is shown to the people you talk to. If they have given you a nickname of their own, theirs wins — a declared name can be copied by anyone.',
    'profile.edit': 'Edit profile',
    'profile.name': 'Name',
    'profile.bio': 'Short bio',
    'profile.save': 'Save',
    'profile.cancel': 'Cancel',
    'profile.none': 'Nothing written yet.',
    'profile.ai.who': 'My PC agents',
    'profile.ai.hint': 'Written by the owner of this computer, or by the agent in that seat. No agent can write another seat’s profile. Which seat it is — where it runs — is set by a person at launch.',
    'profile.face': 'Face',
    'profile.face.drop': 'Drag and drop an image onto the avatar to crop it and replace the face (PNG, JPG or WebP).',
    'crop.title': 'Choose the part to use',
    'crop.hint': 'Drag to move and change the size. What is inside the circle becomes the face.',
    'crop.zoom': 'Size',
    'crop.apply': 'Use this',
    'profile.face.clear': 'Back to the initial avatar',
    'face.big.open': 'View the face larger',
    'face.clear.confirm': 'Go back to the initial avatar?',
    'face.clear.confirm.hint': 'The image you set is deleted. Drop one again to put it back.',
    'face.clear.do': 'Go back',
    'contacts.note': 'Your note',
    'contacts.note.hint': 'Write in your own words which machine this is and what they (or the agent) do. It is not sent to them.',
    'room.name': 'Room name',
    'room.nth': 'Room {n} ({m} people)',
    'room.leave': 'Leave this room',
    'room.leave.confirm': 'Leave this room?',
    'room.leave.hint': 'Everyone in the room is told you left. The conversation here is not kept. To come back you need a room key again.',
    'room.leave.do': 'Leave',
    'room.name.hint': 'The name lives only in this window (it goes away when you close it). It is not sent to anyone.',
    'contacts.desk.how': 'To connect an agent, write the warifu entry into the agent settings on this computer and restart the agent. The steps are in docs/mcp.md.',
    'connect.open': 'Show how to connect',
    'connect.title': 'Connect this agent',
    'connect.lead': 'Run these in a terminal on that agent’s machine (not inside a running Claude session). After the settings are written, restart the agent — the warifu port is read at startup.',
    'connect.step1': '1. Add the warifu port to that agent’s settings',
    'connect.step2': '2. Restart the agent (settings are read at startup)',
    'connect.step3': '3. The dot turns green and what you type reaches it',
    'connect.wake': 'React without waiting (woken by what arrives)',
    'connect.paste': 'Paste into an agent that is already running',
    'connect.paste.hint': 'For an agent that is already up, paste the text below and ask it (agents on this computer). Writing the settings is not enough — the port opens only after a restart.',
    'connect.paste.body': 'Please connect to warifu. Two steps. 1) Run this in a terminal: {cmd} 2) Once written, ask the owner of this computer to restart you (the MCP port is read at startup). After the restart you can read what arrived with warifu’s chat_read.',
    'connect.copy': 'Copy',
    'connect.copied': 'Copied',
    'connect.docs': 'You write which actions are allowed (denied by default). See docs/mcp.md.',
    'chat.shared': 'This is not a conversation with the person you picked.',
    'chat.reach': 'Goes to {who}',
    'chat.reach.none': 'It goes nowhere yet. Someone coming into the room, or an agent on this computer, shows up here.',
    'contacts.presence.none': 'There is no way to tell whether they are running right now. You find out by calling.',
    'reunion.title': 'Next time you meet',
    'reunion.call': 'You can call them from Contacts (no room key needed). It only goes through if they remember you too.',
    'reunion.wait': 'You cannot call them (their whereabouts are unknown). If they call you, it goes through.',
    'reunion.key': 'Same as the first time: you hand each other a room key. To skip the room key next time, give them a name with the pencil beside their name.',
    'reunion.room': 'Naming the room means both of you can tell which room it was next time.',
    'reunion.name': 'Give them a name',
    'presence.on': 'Connected',
    'presence.off': 'Disconnected',
    'help.open': 'Show explanation',
    'help.close': 'Close',
    'help.this.title': 'Rows under “This computer”',
    'help.this.me': 'You — the owner of this computer. Always listed (the badge stays green).',
    'help.this.on': 'Green dot (bottom right of the face) — an agent attached to this computer right now. What you type reaches it.',
    'help.this.off': 'Grey dot — an agent that connected before and whose profile remains. What you type does not reach it. Start it again on its side and the dot turns green.',
    'help.this.others': 'No dot is shown for “People remembered” below. Whether they are running cannot be known from here.',
    'contacts.forget': 'Require a meeting key again',
    'contacts.forget.hint': 'Right now this person can come in without a room key. Turn it off and they will need one again.',
    'act.chat': 'Chat',
    'act.chat.live': 'Messages arrive only while the two of you are connected. If they are not running, what you type is kept nowhere. Set up a poste restante and it can be left for them.',
    'act.chat.postbox': 'A conversation with this person only. While they are away it is left sealed at the poste restante (7 days).',
    'act.chat.desk': 'A conversation with the agent on this computer only. Type in the conversation box and only that agent receives it.',
    'act.chat.desk.none': 'They are not connected right now, so what you type will not reach them.',
    'act.group': 'Group chat',
    'act.group.what': 'Create a room and invite them. Pressing this takes you to where keys are issued — one per person you invite.',
    'act.group.action': 'Create a room',
    'act.group.desk.none': 'An agent that is not connected cannot be invited into a room either.',
    'act.call': 'Video meeting',
    'act.call.net': 'On the same network it connects as it is. A different network needs --relay, and we have not once seen that connect.',
    'act.call.action': 'Call into a meeting',
    'act.call.desk.none': 'Not connected. Once connected, it is in the same conversation from the start.',
    'act.call.working': 'Calling…',
    'act.calendar': 'Schedule',
    'act.calendar.none': 'There is no way yet for this screen to read a calendar. It cannot show free slots either.',
    'act.state.ok': 'Works',
    'act.state.wait': 'Conditions apply',
    'act.state.no': 'Not yet',
    'act.address.none': 'Their whereabouts are not remembered yet. Connect once with a room key and it is remembered. Until then you cannot call them.',
    'act.already': 'Already in the same room.',
    'act.desk.local': 'Agents on this computer are in the same room from the start. There is nothing to bring in.',
    'act.desk.empty': 'No agent on this computer is connected.',
    'act.desk.stop': 'Stop this agent',
    'act.desk.stop.hint': 'It is told to stop. Nothing is killed — the other side steps down on its own. To run it again, start it there.',
    'schedule.title': 'Schedule',
    'schedule.none': 'The schedule pane does not work yet. There is no way for the window to read a calendar.',
    'schedule.add': 'Write an appointment',
    'schedule.date': 'Date',
    'schedule.time': 'Start',
    'schedule.minutes': 'Length (min)',
    'schedule.what': 'What it is',
    'schedule.note': 'Note (yours only)',
    'schedule.save': 'Save',
    'schedule.empty': 'No appointments yet. Write one below and press Save to see it here.',
    'schedule.past': 'Past appointments',
    'schedule.now': 'Happening now',
    'schedule.remove': 'Delete',
    'schedule.remove.confirm': 'Delete this appointment?',
    'schedule.remove.hint': 'It cannot be brought back. The note goes with it.',
    'schedule.mine': 'What you put here stays on this machine. It is not handed to anyone (only free slots are).',
    'schedule.bad': 'Write the date and time in this shape (date 2026-09-11, start 9:30).',
    'chat.placeholder.nobody': 'You can send once someone joins',
    'call.mic': 'Microphone',
    'call.camera': 'Camera',
    'call.controls': 'Turn the call on and off. Separate from checking your gear — this stops what you are sending now.',
    'video.start': 'Start a video meeting',
    'video.stop': 'Stop the video meeting',
    'video.title': 'Video meeting',
    'video.hint': 'Adds video and sound to the room you are in. Typing keeps working as before.',
    'video.off.hint': 'This room is text only right now. Neither camera nor microphone is in use.',
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
    'update.check': '检查更新',
    'update.checking': '检查中…',
    'update.none': '已是最新（{version}）。',
    'update.failed': '无法检查（{why}）。',
    'update.notes': '有哪些变化',
    'update.notes.none': '没有写。',
    'update.downloading': '下载中… {percent}%',
    'update.installed': '已安装。重新启动后使用新版本。',
    'update.later': '稍后',
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
    'meeting.key.count': '已发 {n} 把',
    'meeting.key.each': '一把 room key只能让一个人进入。请给每个人各自的room key。',
    'meeting.key.nth': '第 {n} 把',
    'key.hand': '给这个人一把 room key',
    'key.hand.title': '给 {name} 的一把 room key',
    'key.hand.hint': '这是给 {name} 的一把 room key。一把 room key只能让一个人进来。请通过其他方式（当面、电话、别的聊天）把链接、二维码或room key 文字交给对方。',
    'key.hand.for': '已交给 {name}',
    'key.hand.again': '已经给这个人交过一把 room key。如果没交到，可以再出一把（之前那把仍然有效）。',
    'meeting.key.howmany': '几人份',
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
    'meeting.key.copy': '复制 room key',
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
    'chat.agent': '我的电脑代理',
    'chat.desk': '这台电脑的智能体已连上。即使 room 里没有其他人，也可以对它说话。',
    'chat.placeholder.desk': '对这台电脑的代理说话',
    'chat.late': '离线期间',
    'send.absent': '对方现在不在。设置寄存处后，离线期间也能送达。',
    'postbox.title': '不在时的接收（可选）',
    'postbox.hint': '在你没有启动的期间，把送到的话封着代为保管的地方。保管方也读不到内容。不设置的话，只有你启动着的时候才会送到。',
    'postbox.placeholder': '粘贴代为保管方的地址',
    'postbox.save': '设置',
    'postbox.clear': '取消',
    'postbox.saved': '已设置寄存处。',
    'postbox.cleared': '已移除寄存处。',
    'postbox.kept': '对方不在，已寄存。对方启动时会送到。',
    'mark.sent': '已发送',
    'mark.sent.hint': '已交到对方的机器。是否读过无法知道（没有收集已读的机器）。',
    'mark.kept': '等待接收',
    'mark.kept.hint': '等对方能够接收时就会送到。在那之前由寄存处封着保管。',
    'mark.none': '未发送',
    'mark.none.hint': '既没有通路也没有寄存处，所以没有留在任何地方。',
    'postbox.received': '离线期间收到了 {n} 条。',
    'pane.contacts': '通讯录',
    'pane.meeting': '房间',
    'pane.schedule': '日程',
    'contacts.this': '这台电脑',
    'contacts.me': '你',
    'contacts.desk': '我的电脑代理',
    'contacts.desk.none': '这台电脑还没有连上智能体。用 `warifu mcp` 连接后会出现在这里。',
    'contacts.inmeeting': '现在同一个房间里的人',
    'contacts.saved': '联系人',
    'contacts.late': '离线期间来信的人',
    'contacts.claimed': '对方自称“{name}”。',
    'contacts.where': '要跟对方说话，请在会话框里输入。',
    'contacts.rooms': '你所在的房间',
    'room.members.some': '有人在。',
    'room.alone': '目前只有你。把密钥交出去，对方就能进来。',
    'room.invite': '邀请人进这个 room',
    'room.invite.title': '给这个 room 的一把 room key',
    'room.invite.hint': '这一把 room key可以让一个人进来。请把链接、二维码或room key 文字交给你想邀请的人。要再邀请一个人，就再出一把。',
    'room.host': '这个房间由你建立',
    'contacts.empty': '还没有记住任何人。给在 room 里遇到的对象起个称呼，就会留在这里。',
    'contacts.pick': '选择一位，就会显示可以做的事。',
    'home.title': '可以做的事',
    'home.lead': '选择一位对象，就会显示可以对他做的事。请从左边的列表里选。',
    'home.now': '现在的状况',
    'home.seats': '这台电脑的智能体',
    'home.rooms': '所在的 room',
    'home.people': '{n} 人',
    'home.rooms.n': '{n} 间（{m} 人）',
    'home.contacts': '联系人',
    'home.postbox': '不在时的接收',
    'home.postbox.on': '已设置',
    'home.postbox.off': '未设置',
    'home.chat': '选择一位对象，就变成只和那个人的会话。',
    'home.group': '建一个 room，按人数发room key。一把 room key一个人。',
    'home.call': '同一个网络里可以连上。不同网络需要 --relay，但我们还没有见过它真的连上过。',
    'home.calendar': '可以写下并保存自己的日程。看不到对方的日程（交给对方的只有空闲时段）。',
    'home.next': '第一步',
    'home.next.hint': '还没有任何人。按左边「这台电脑」里你自己的那一行，就会显示room key的交付方法。',
    'contacts.key.label': '公钥',
    'contacts.me.what': '这是这台电脑上的你。关掉之后仍然是同一个人。',
    'contacts.me.share': '这个公钥可以给对方看。仅凭它谁也进不来（进来需要密钥）。',
    'meeting.link.label': '让对方进入的链接',
    'meeting.link.copy': '复制链接',
    'meeting.copy.which': '平时用「复制链接」交给对方，对方按一下就能进来。链接不可用时（CLI、纸面、会剥掉链接的聊天），请用「复制 room key」交出文字。',
    'meeting.link.hint': '把它发给对方。对方点开后，割符会询问是否进入（不会擅自进入）。对方也需要装有割符。',
    'meeting.qr.reveal': '显示二维码',
    'meeting.qr.hint': '对方就在眼前时用这个。扫出来是同一个链接。',
    'link.invited': '有链接邀请你进入房间。要进入吗？',
    'link.invited.hint': '这样的链接谁都能做。没有印象的邀请，请不要进入。',
    'link.invited.enter': '进入',
    'link.invited.no': '不进入',
    'contacts.me.key.what': '公钥是表示「你是谁」的名字。它不是用来建立连接的。对方把你加入通讯录时，或保管处的管理者把你写进名单时，才需要交给对方。',
    'howto.title': '与熟人建立连接的步骤',
    'howto.1': '打开「房间」，按下［创建房间］',
    'howto.2': '把出现的room key（或链接）交给对方',
    'howto.3': '对方收到并进入后，就连上了',
    'howto.4': '连上后给对方起个称呼。以后就能从通讯录直接呼叫（不再需要room key）',
    'howto.key': '这一步交出去的是「room key」，不是上面的公钥。',
    'contacts.me.copy': '复制公钥',
    'contacts.me.copied': '已复制',
    'contacts.me.name': '你在这里填的名字，对方也会看到。但如果对方给你起了称呼，以对方的为准——申报的名字谁都能冒充。',
    'profile.edit': '编辑资料',
    'profile.name': '名字',
    'profile.bio': '简介',
    'profile.save': '保存',
    'profile.cancel': '取消',
    'profile.none': '还没有填写。',
    'profile.ai.who': '我的电脑代理',
    'profile.ai.hint': '由这台电脑的主人，或坐在该位置的代理自己填写。别的位置的代理改不了。在哪里运行（位置）由人在启动时决定。',
    'profile.face': '头像',
    'profile.face.drop': '把图片拖放到头像上，可以先裁剪再替换（PNG、JPG、WebP）。',
    'crop.title': '选择用作头像的部分',
    'crop.hint': '可以拖动移动、调整大小。圆圈内的部分会成为头像。',
    'crop.zoom': '大小',
    'crop.apply': '就用这个',
    'profile.face.clear': '恢复为初始头像',
    'face.big.open': '放大查看头像',
    'face.clear.confirm': '要恢复为初始头像吗？',
    'face.clear.confirm.hint': '你换上的图片会被删除。再拖放一次就可以放回去。',
    'face.clear.do': '恢复',
    'contacts.note': '备注',
    'contacts.note.hint': '用你自己的话写下这是哪台机器、做什么的人（代理）。不会发送给对方。',
    'room.name': '房间名',
    'room.nth': 'Room {n}（{m} 人）',
    'room.leave': '退出这个 room',
    'room.leave.confirm': '要退出这个 room 吗？',
    'room.leave.hint': 'room 里的所有人都会知道你退出了。这里的会话不会保留。要再进来需要重新拿到room key。',
    'room.leave.do': '退出',
    'room.name.hint': '名字只存在于这个窗口里（关闭后消失）。不会发送给别人。',
    'contacts.desk.how': '要连接智能体，请在这台电脑的智能体设置里写入 warifu 的入口，然后重启智能体。步骤见 docs/mcp.md。',
    'connect.open': '显示连接方法',
    'connect.title': '连接这个智能体',
    'connect.lead': '这些要在那个智能体的机器上用终端执行（不是在运行中的 Claude 里面）。写好设置后请重启智能体 —— 割符的入口在启动时读取。',
    'connect.step1': '1. 把割符的入口写进那个智能体的设置',
    'connect.step2': '2. 重启智能体（设置在启动时读取）',
    'connect.step3': '3. 圆点会变绿，你打的内容就会送到',
    'connect.wake': '不等待就反应（由送到的内容唤起）',
    'connect.paste': '粘贴给已经启动的智能体',
    'connect.paste.hint': '对已经启动的智能体，可以把下面这段话直接粘贴过去请它处理（针对这台电脑的智能体）。写好设置也要重启之后入口才会打开。',
    'connect.paste.body': '请连接到 warifu。两步。1) 在终端执行：{cmd} 2) 写好之后，请告诉这台电脑的主人重启你（MCP 入口在启动时读取）。重启后可以用 warifu 的 chat_read 读取送到的发言。',
    'connect.copy': '复制',
    'connect.copied': '已复制',
    'connect.docs': '允许哪些动作由人来写（默认拒绝）。详见 docs/mcp.md。',
    'chat.shared': '这里不是只和所选对方的会话。',
    'chat.reach': '送达 {who}',
    'chat.reach.none': '目前送不到任何人。有人进入房间，或这台电脑的代理到位后，就会显示在这里。',
    'contacts.presence.none': '无法知道对方现在是否已启动。只有呼叫之后才知道。',
    'reunion.title': '下次见面时',
    'reunion.call': '可以从联系人里呼叫（不需要 room key）。但对方也要记住你，否则呼叫不会通。',
    'reunion.wait': '你无法呼叫对方（不知道对方在哪）。如果对方来呼叫你，就能通。',
    'reunion.key': '和第一次一样，互相交换room key。想让下次不需要 room key，请用名字旁边的铅笔起个称呼。',
    'reunion.room': '给 room 起个名字，下次见面时双方都能知道是哪个 room。',
    'reunion.name': '起个称呼',
    'presence.on': '已连上',
    'presence.off': '已断开',
    'help.open': '显示说明',
    'help.close': '关闭',
    'help.this.title': '「这台电脑」里的行',
    'help.this.me': '你 —— 这台电脑的主人。总是显示（头像右下角一直是绿色）。',
    'help.this.on': '绿色圆点（头像右下角）—— 现在正连在这台电脑上的智能体。你打的内容会送到。',
    'help.this.off': '灰色圆点 —— 以前连过、资料还留着的智能体。你打的内容不会送到。在那边重新启动后会变回绿色。',
    'help.this.others': '下面的「记住的对象」不显示圆点。因为从这里无法知道对方是否在运行。',
    'contacts.forget': '恢复需要会议密钥',
    'contacts.forget.hint': '现在这位不用密钥就能进来。取消后，下次就需要密钥了。',
    'act.chat': '聊天',
    'act.chat.live': '只在双方连着的时候才会送到。对方没启动时，你打的内容不会留在任何地方。设置寄存处后，对方不在也可以先寄存。',
    'act.chat.postbox': '只和这位对象的会话。对方不在时，会封着寄存在寄存处（7 天）。',
    'act.chat.desk': '只和这台电脑的智能体的会话。在会话框里输入，只有那个智能体会收到。',
    'act.chat.desk.none': '现在没有连上，所以你打的内容不会送到。',
    'act.group': '群聊',
    'act.group.what': '建一个 room，把这位对象请进来。按下后会转到发room key的地方（按邀请人数，一次出一把）。',
    'act.group.action': '建一个 room',
    'act.group.desk.none': '没有连上的智能体，也无法请进 room。',
    'act.call': '视频会议',
    'act.call.net': '同一个网络里可以直接连上。不同网络需要 --relay，但我们还没有见过它真的连上过。',
    'act.call.action': '请进视频会议',
    'act.call.desk.none': '没有连上。连上之后，从一开始就在同一个会话里。',
    'act.call.working': '正在呼叫…',
    'act.calendar': '日程',
    'act.calendar.none': '这个界面还没有任何读取日程表的入口，也无法显示空闲时段。',
    'act.state.ok': '可以',
    'act.state.wait': '有条件',
    'act.state.no': '还不行',
    'act.address.none': '还没有记住对方的位置。用密钥连接一次后就会记住。在那之前无法从这边呼叫。',
    'act.already': '已经在同一个房间里了。',
    'act.desk.local': '这台电脑的智能体从一开始就在同一个 room 里，不需要请进来。',
    'act.desk.empty': '这台电脑还没有连上智能体。',
    'act.desk.stop': '让这个代理停下',
    'act.desk.stop.hint': '会传达「停下」。不是杀掉对方，而是收到的一方自己退出。要再次运行，请在那边重新启动。',
    'schedule.title': '日程',
    'schedule.none': '日程还不能用。画面还没有读取日程表的口。',
    'schedule.add': '写一个日程',
    'schedule.date': '日期',
    'schedule.time': '开始',
    'schedule.minutes': '时长（分钟）',
    'schedule.what': '是什么日程',
    'schedule.note': '备注（只有你能看）',
    'schedule.save': '保存',
    'schedule.empty': '还没有日程。在下面写好后按保存，就会显示在这里。',
    'schedule.past': '已结束的日程',
    'schedule.now': '正在进行',
    'schedule.remove': '删除',
    'schedule.remove.confirm': '要删除这个日程吗？',
    'schedule.remove.hint': '删除后无法恢复。备注也会一起删除。',
    'schedule.mine': '放在这里的日程只留在这台机器上，不会交给对方（交给对方的只有空闲时段）。',
    'schedule.bad': '请按这个格式写日期和时间（日期 2026-09-11，开始 9:30）。',
    'chat.placeholder.nobody': '有人进入后即可发送',
    'call.mic': '麦克风',
    'call.camera': '摄像头',
    'call.controls': '通话的开关。与设备确认不同，这会停止你现在正在发送的内容。',
    'video.start': '开始视频会议',
    'video.stop': '结束视频会议',
    'video.title': '视频会议',
    'video.hint': '在当前 room 里加上影像和声音。文字交流照旧继续。',
    'video.off.hint': '这个 room 现在只有文字。没有使用摄像头，也没有使用麦克风。',
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
    'update.check': '업데이트 확인',
    'update.checking': '확인 중…',
    'update.none': '최신입니다({version}).',
    'update.failed': '확인할 수 없었습니다({why}).',
    'update.notes': '무엇이 달라졌는지',
    'update.notes.none': '쓰여 있지 않습니다.',
    'update.downloading': '내려받는 중… {percent}%',
    'update.installed': '설치했습니다. 다시 시작하면 새 것으로 바뀝니다.',
    'update.later': '나중에',
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
    'meeting.key.count': '{n} 개 냈습니다',
    'meeting.key.each': '열쇠 하나에 한 사람입니다. 부를 사람마다 다른 열쇠를 건네세요.',
    'meeting.key.nth': '{n} 번째',
    'key.hand': '이 사람에게 룸 키를 건네기',
    'key.hand.title': '{name}에게 줄 룸 키 한 개',
    'key.hand.hint': '{name}에게 줄 룸 키 한 개입니다. 키 하나로는 한 사람만 들어올 수 있습니다. 링크나 QR, 또는 룸 키 문자를 다른 방법(직접·전화·다른 채팅)으로 건네주세요.',
    'key.hand.for': '{name}에게 건넴',
    'key.hand.again': '이 사람에게는 이미 룸 키를 건넸습니다. 잘 전달되지 않았다면 한 개 더 발급할 수 있습니다(앞의 것도 유효합니다).',
    'meeting.key.howmany': '몇 사람 분',
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
    'meeting.key.copy': '룸 키 복사',
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
    'chat.agent': '내 PC 에이전트',
    'chat.desk': '이 PC의 에이전트가 연결되어 있습니다. 룸에 사람이 없어도 말을 걸 수 있습니다.',
    'chat.placeholder.desk': '이 PC 의 에이전트에게 말을 걸기',
    'chat.late': '부재 중',
    'send.absent': '지금 없습니다. 보관소를 두면 부재 중에도 전달됩니다.',
    'postbox.title': '부재 중 수신(선택)',
    'postbox.hint': '당신이 켜져 있지 않은 동안, 도착한 말을 봉한 채로 맡아 두는 곳입니다. 맡는 쪽도 내용을 읽을 수 없습니다. 설정하지 않으면 켜져 있는 동안에만 도착합니다.',
    'postbox.placeholder': '맡아 줄 상대의 주소를 붙여넣기',
    'postbox.save': '설정하기',
    'postbox.clear': '해제',
    'postbox.saved': '보관소를 두었습니다.',
    'postbox.cleared': '보관소를 내렸습니다.',
    'postbox.kept': '지금 없어서 보관소에 맡겼습니다. 상대가 실행하면 전달됩니다.',
    'mark.sent': '보냄',
    'mark.sent.hint': '상대의 기기로 전달했습니다. 읽었는지는 알 수 없습니다(읽음을 모으는 기계가 없습니다).',
    'mark.kept': '수신 대기',
    'mark.kept.hint': '상대가 받을 수 있게 되면 도착합니다. 그때까지는 보관소가 봉한 채로 맡아 둡니다.',
    'mark.none': '보내지 못함',
    'mark.none.hint': '통로도 보관소도 없어서 어디에도 남지 않았습니다.',
    'postbox.received': '부재 중에 {n} 통이 와 있었습니다.',
    'pane.contacts': '연락처',
    'pane.meeting': '방',
    'pane.schedule': '일정',
    'contacts.this': '이 PC',
    'contacts.me': '당신',
    'contacts.desk': '내 PC 에이전트',
    'contacts.desk.none': '이 PC에 연결된 에이전트가 없습니다. `warifu mcp` 로 연결하면 여기에 나옵니다.',
    'contacts.inmeeting': '지금 같은 방에 있는 사람',
    'contacts.saved': '연락처',
    'contacts.late': '부재 중에 온 상대',
    'contacts.claimed': '본인은 “{name}” 라고 밝히고 있습니다.',
    'contacts.where': '말을 걸 때는 대화 칸에 입력합니다.',
    'contacts.rooms': '지금 있는 방',
    'room.members.some': '사람이 있습니다.',
    'room.alone': '아직 자신뿐입니다. 열쇠를 건네면 들어올 수 있습니다.',
    'room.invite': '이 룸에 사람을 부르기',
    'room.invite.title': '이 룸에 줄 룸 키 한 개',
    'room.invite.hint': '이 룸 키 한 개로 한 사람이 들어올 수 있습니다. 링크나 QR, 또는 룸 키 문자를 부르고 싶은 상대에게 건네주세요. 한 사람 더 부르려면 한 개 더 발급합니다.',
    'room.host': '당신이 만든 방입니다',
    'contacts.empty': '아직 아무도 기억하지 않았습니다. 룸에서 만난 상대에게 이름을 붙이면 여기에 남습니다.',
    'contacts.pick': '상대를 고르면 할 수 있는 일이 나옵니다.',
    'home.title': '할 수 있는 일',
    'home.lead': '상대를 고르면 그 상대에게 할 수 있는 일이 나옵니다. 왼쪽 목록에서 골라 주세요.',
    'home.now': '지금 상황',
    'home.seats': '이 PC의 에이전트',
    'home.rooms': '들어가 있는 룸',
    'home.people': '{n}명',
    'home.rooms.n': '{n}개({m}명)',
    'home.contacts': '연락처',
    'home.postbox': '부재 중 수신',
    'home.postbox.on': '설정됨',
    'home.postbox.off': '설정 안 됨',
    'home.chat': '상대를 고르면 그 사람과만 하는 대화가 됩니다.',
    'home.group': '룸을 만들어 인원 수만큼 룸 키를 나눕니다. 키 하나에 한 사람입니다.',
    'home.call': '같은 망이면 연결됩니다. 다른 망은 --relay가 필요하지만, 연결된 것을 아직 한 번도 보지 못했습니다.',
    'home.calendar': '내 일정을 써서 둘 수 있습니다. 상대의 일정은 볼 수 없습니다(넘어가는 것은 빈 시간뿐입니다).',
    'home.next': '첫걸음',
    'home.next.hint': '아직 아무도 없습니다. 왼쪽 “이 PC”의 내 행을 누르면 룸 키를 건네는 방법이 나옵니다.',
    'contacts.key.label': '공개키(내 ID)',
    'contacts.me.what': '이것이 이 PC 의 당신입니다. 닫아도 같은 사람으로 있습니다.',
    'contacts.me.share': '이 공개키(내 ID)는 상대에게 보여도 됩니다. 이것만으로는 아무도 들어올 수 없습니다 (들어오려면 열쇠가 필요합니다).',
    'meeting.link.label': '들어오게 할 링크',
    'meeting.link.copy': '링크 복사',
    'meeting.copy.which': '보통은 「링크 복사」로 건넵니다. 상대가 누르기만 하면 들어옵니다. 링크를 쓸 수 없는 상대(CLI·종이·링크를 없애는 채팅)에는 「룸 키 복사」로 문자를 건네주세요.',
    'meeting.link.hint': '이것을 상대에게 보냅니다. 상대가 열면 와리후가 들어갈지 묻습니다(멋대로 들어가지 않습니다). 상대에게도 와리후가 설치되어 있어야 합니다.',
    'meeting.qr.reveal': 'QR로 보여주기',
    'meeting.qr.hint': '상대가 눈앞에 있을 때 씁니다. 읽으면 같은 링크가 됩니다.',
    'link.invited': '링크로 방에 초대받았습니다. 들어가시겠습니까?',
    'link.invited.hint': '이런 링크는 누구나 만들 수 있습니다. 짐작 가지 않는 초대에는 들어가지 마세요.',
    'link.invited.enter': '들어간다',
    'link.invited.no': '들어가지 않는다',
    'contacts.me.key.what': '공개키(내 ID)는 「당신이 누구인지」를 나타내는 이름입니다. 이것으로 상대와 연결되는 것이 아닙니다. 상대가 주소록에 등록할 때나, 보관소 관리자가 명단에 적을 때 건넵니다.',
    'howto.title': '아는 사람과 연결하는 순서',
    'howto.1': '「방」을 열고 ［방 만들기］를 누릅니다',
    'howto.2': '나온 열쇠(또는 링크)를 상대에게 건넵니다',
    'howto.3': '상대가 받아서 들어오면 연결됩니다',
    'howto.4': '연결되면 부를 이름을 붙입니다. 다음부터는 주소록에서 부를 수 있습니다(열쇠가 필요 없습니다)',
    'howto.key': '여기서 건네는 것은 「방의 열쇠」이며, 위의 공개키(내 ID)가 아닙니다.',
    'contacts.me.copy': '공개키(내 ID) 복사',
    'contacts.me.copied': '복사했습니다',
    'contacts.me.name': '여기서 댄 이름은 상대 화면에도 나옵니다. 다만 상대가 호칭을 붙였다면 그쪽이 우선입니다 (스스로 댄 이름은 누구나 흉내 낼 수 있습니다).',
    'profile.edit': '프로필 쓰기',
    'profile.name': '이름',
    'profile.bio': '짧은 소개',
    'profile.save': '정하기',
    'profile.cancel': '그만두기',
    'profile.none': '아직 쓰지 않았습니다.',
    'profile.ai.who': '내 PC 에이전트',
    'profile.ai.hint': '이 PC 의 주인과, 그 자리의 에이전트 자신이 쓸 수 있습니다. 다른 자리의 에이전트는 바꿀 수 없습니다. 어디서 도는지(자리)는 띄울 때 사람이 정합니다.',
    'profile.face': '얼굴',
    'profile.face.drop': '아바타 이미지에 이미지를 끌어다 놓으면, 잘라낸 뒤 교체할 수 있습니다(PNG·JPG·WebP).',
    'crop.title': '얼굴로 쓸 부분을 고릅니다',
    'crop.hint': '끌어서 움직이고 크기를 바꿀 수 있습니다. 원 안이 얼굴이 됩니다.',
    'crop.zoom': '크기',
    'crop.apply': '이걸로 한다',
    'profile.face.clear': '초기 아바타로 되돌리기',
    'face.big.open': '얼굴 크게 보기',
    'face.clear.confirm': '초기 아바타로 되돌릴까요?',
    'face.clear.confirm.hint': '바꿔 넣은 이미지는 지워집니다. 다시 끌어다 놓으면 되돌릴 수 있습니다.',
    'face.clear.do': '되돌리기',
    'contacts.note': '메모',
    'contacts.note.hint': '이 상대가 어느 기계의, 무엇을 하는 사람(에이전트)인지 자기 말로 적을 수 있습니다. 상대에게는 보내지 않습니다.',
    'room.name': '방 이름',
    'room.nth': '룸 {n}({m}명)',
    'room.leave': '이 룸에서 나가기',
    'room.leave.confirm': '이 룸에서 나갈까요?',
    'room.leave.hint': '나간 것은 이 룸의 모두에게 전해집니다. 여기의 대화는 남지 않습니다. 다시 들어오려면 룸 키를 다시 받아야 합니다.',
    'room.leave.do': '나가기',
    'room.name.hint': '이름은 이 화면 안에만 있습니다 (닫으면 사라집니다). 상대에게는 보내지 않습니다.',
    'contacts.desk.how': '연결하려면 이 PC의 에이전트 설정에 warifu 입구를 쓰고 에이전트를 다시 시작합니다. 순서는 docs/mcp.md 에 있습니다.',
    'connect.open': '연결 방법 보기',
    'connect.title': '이 에이전트를 연결하기',
    'connect.lead': '이것은 그 에이전트의 기기에서 터미널로 실행하는 것입니다(실행 중인 Claude 안이 아닙니다). 설정을 쓴 뒤 에이전트를 다시 시작해 주세요 —— 와리후의 창구는 시작할 때 읽습니다.',
    'connect.step1': '1. 와리후의 창구를 그 에이전트 설정에 추가',
    'connect.step2': '2. 에이전트를 다시 시작(설정은 시작할 때 읽습니다)',
    'connect.step3': '3. 동그라미가 초록이 되고, 입력한 말이 도착합니다',
    'connect.wake': '기다리지 않고 반응하기(도착한 말로 깨우기)',
    'connect.paste': '이미 실행 중인 에이전트에 붙여넣기',
    'connect.paste.hint': '이미 켜져 있는 에이전트에는 아래 문장을 그대로 붙여 넣어 부탁할 수 있습니다(이 PC의 에이전트용). 설정을 써도 다시 시작하기 전에는 창구가 열리지 않습니다.',
    'connect.paste.body': 'warifu에 연결해 주세요. 두 단계입니다. 1) 터미널에서 실행: {cmd} 2) 다 쓰면, 이 PC의 주인에게 나를 다시 시작해 달라고 전해 주세요(MCP 창구는 시작할 때 읽습니다). 다시 시작한 뒤 warifu의 chat_read로 도착한 발언을 읽을 수 있습니다.',
    'connect.copy': '복사',
    'connect.copied': '복사했습니다',
    'connect.docs': '허용할 동작은 사람이 씁니다(기본은 거부). 자세히는 docs/mcp.md.',
    'chat.shared': '여기는 고른 상대만의 대화가 아닙니다.',
    'chat.reach': '가는 곳 {who}',
    'chat.reach.none': '아직 갈 곳이 없습니다. 방에 사람이 들어오거나 이 PC 의 에이전트가 자리에 앉으면 여기에 나옵니다.',
    'contacts.presence.none': '상대가 지금 켜져 있는지는 알 수 없습니다. 불러 봐야 알 수 있습니다.',
    'reunion.title': '다음에 만날 때',
    'reunion.call': '연락처에서 부를 수 있습니다(룸 키는 필요 없습니다). 다만 상대도 기억하고 있어야 연결됩니다.',
    'reunion.wait': '이쪽에서는 부를 수 없습니다(상대의 위치를 모릅니다). 상대가 부르면 연결됩니다.',
    'reunion.key': '처음과 같이 서로 룸 키를 건넵니다. 다음부터 키 없이 하려면 이름 옆의 연필로 이름을 붙여 주세요.',
    'reunion.room': '룸에 이름을 붙여 두면 다음에 만날 때 어느 룸이었는지 서로 알 수 있습니다.',
    'reunion.name': '이름 붙이기',
    'presence.on': '연결됨',
    'presence.off': '끊김',
    'help.open': '설명 보기',
    'help.close': '닫기',
    'help.this.title': '“이 PC”의 행',
    'help.this.me': '나 —— 이 PC의 주인입니다. 항상 나옵니다(얼굴 오른쪽 아래는 늘 초록).',
    'help.this.on': '초록 동그라미(얼굴 오른쪽 아래) —— 지금 이 PC에 붙어 있는 에이전트입니다. 입력하면 도착합니다.',
    'help.this.off': '회색 동그라미 —— 전에 연결되어 프로필이 남아 있는 에이전트입니다. 입력해도 도착하지 않습니다. 그쪽에서 다시 시작하면 초록으로 돌아옵니다.',
    'help.this.others': '아래 “기억하고 있는 상대”에는 동그라미를 표시하지 않습니다. 상대가 켜져 있는지는 여기서 알 수 없기 때문입니다.',
    'contacts.forget': '다시 회의 룸 키를 받게 하기',
    'contacts.forget.hint': '지금 이 상대는 열쇠 없이 들어올 수 있습니다. 끄면 다음부터는 열쇠가 필요합니다.',
    'act.chat': '채팅',
    'act.chat.live': '연결되어 있는 동안에만 도착합니다. 상대가 켜져 있지 않으면 입력한 내용은 어디에도 남지 않습니다. 보관소를 두면 상대가 없을 때도 맡길 수 있습니다.',
    'act.chat.postbox': '이 상대와만 하는 대화입니다. 상대가 없을 때는 봉한 채로 보관소에 맡깁니다(7일).',
    'act.chat.desk': '이 PC의 에이전트와만 하는 대화입니다. 대화 칸에 입력하면 그 에이전트에게만 갑니다.',
    'act.chat.desk.none': '지금 연결되어 있지 않아서, 입력해도 도착하지 않습니다.',
    'act.group': '그룹 채팅',
    'act.group.what': '룸을 만들어 이 상대를 초대합니다. 누르면 인원 수만큼 룸 키를 발급하는 곳으로 이동합니다.',
    'act.group.action': '룸 만들기',
    'act.group.desk.none': '연결되지 않은 에이전트는 룸에도 부를 수 없습니다.',
    'act.call': '영상 회의',
    'act.call.net': '같은 망이면 그대로 연결됩니다. 다른 망은 --relay가 필요하지만, 연결된 것을 아직 한 번도 보지 못했습니다.',
    'act.call.action': '영상 회의로 부르기',
    'act.call.desk.none': '연결되어 있지 않습니다. 연결되면 처음부터 같은 대화에 있습니다.',
    'act.call.working': '부르는 중…',
    'act.calendar': '일정',
    'act.calendar.none': '이 화면에서 일정표를 읽는 창구가 아직 하나도 없습니다. 빈 시간도 보여줄 수 없습니다.',
    'act.state.ok': '가능',
    'act.state.wait': '조건부',
    'act.state.no': '아직 안 됨',
    'act.address.none': '상대의 위치를 아직 기억하지 못했습니다. 열쇠로 한 번 연결하면 기억합니다. 그전에는 이쪽에서 부를 수 없습니다.',
    'act.already': '이미 같은 방에 있습니다.',
    'act.desk.local': '이 PC의 에이전트는 처음부터 같은 룸에 있습니다. 들일 필요가 없습니다.',
    'act.desk.empty': '이 PC에 연결된 에이전트가 없습니다.',
    'act.desk.stop': '이 에이전트를 멈추기',
    'act.desk.stop.hint': '「멈추라」고 전합니다. 상대를 죽이는 것이 아니라, 받은 쪽이 스스로 내려옵니다. 다시 움직이려면 그쪽에서 다시 시작해 주세요.',
    'schedule.title': '일정',
    'schedule.none': '일정 화면은 아직 움직이지 않습니다. 화면에서 일정표를 읽는 입구가 아직 하나도 없습니다.',
    'schedule.add': '일정 쓰기',
    'schedule.date': '날짜',
    'schedule.time': '시작',
    'schedule.minutes': '길이(분)',
    'schedule.what': '무슨 일정인지',
    'schedule.note': '메모(나만 봄)',
    'schedule.save': '저장',
    'schedule.empty': '아직 일정이 없습니다. 아래에 쓰고 저장을 누르면 여기에 나옵니다.',
    'schedule.past': '끝난 일정',
    'schedule.now': '진행 중',
    'schedule.remove': '삭제',
    'schedule.remove.confirm': '이 일정을 삭제할까요?',
    'schedule.remove.hint': '삭제하면 되돌릴 수 없습니다. 메모도 함께 지워집니다.',
    'schedule.mine': '여기에 둔 일정은 이 기기 안에만 있습니다. 상대에게 넘어가지 않습니다(넘어가는 것은 빈 시간뿐입니다).',
    'schedule.bad': '날짜와 시각을 이 형식으로 써 주세요(날짜 2026-09-11, 시작 9:30).',
    'chat.placeholder.nobody': '누군가 들어오면 보낼 수 있습니다',
    'call.mic': '마이크',
    'call.camera': '카메라',
    'call.controls': '통화 켜고 끄기. 기기 확인과는 다르며, 지금 보내고 있는 것을 멈춥니다.',
    'video.start': '영상 회의 시작',
    'video.stop': '영상 회의 끝내기',
    'video.title': '영상 회의',
    'video.hint': '지금 있는 룸에 영상과 소리를 더합니다. 문자 대화는 그대로 이어집니다.',
    'video.off.hint': '이 룸은 지금 문자만 씁니다. 카메라도 마이크도 쓰지 않습니다.',
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
  'act.chat.live',
  'act.call.net',
  'act.calendar.none',
  'act.address.none',
  'chat.shared',
  'send.absent',
  'postbox.kept',
  'mark.sent',
  'mark.kept',
  'reunion.call',
  'link.invited.hint',
  'meeting.link.hint',
] as const;

/** 翻訳者への注記。**訳文と一緒に渡す。** */
export const TRANSLATOR_NOTES: Partial<Record<MessageKey, string>> = {
  'link.invited.hint':
    '**警告である。**「安全です」「割符が確認済みです」と読める訳にしないこと。' +
    'このリンクは誰でも作れる。割符は差出人を確かめていない。' +
    '確かめられるのは、心当たりがあるかどうかだけで、それは人にしか分からない。',
  'meeting.link.hint':
    '**「押すと入る」と読める訳にしないこと。**押すと割符が開いて、'
    + '**入るかどうかを尋ねる**（そこで人が決める）。'
    + 'また、相手に割符が入っていなければ、リンクは何も起こさない。',
  'send.absent':
    '**まだ送れていない。**「送信しました」「あとで届きます」と読める訳にしないこと。' +
    '預かり所を置いていない状態なので、打った言葉はどこにも残っていない。' +
    '「預かり所を置くと」は条件であって、約束ではない。',
  'mark.sent':
    '**渡したのは相手の機械までである。**「既読」「読みました」と読める訳にしないこと。' +
    '割符には既読を集める機械が無い。**読んだかどうかは、こちらには分からない。**',
  'reunion.call':
    '**呼べば必ず繋がる、と読める訳にしないこと。**通るのは相手の戸口が' +
    'こちらを通すときだけで、それは相手が決めることである。' +
    '「いつでも繋がります」「自動で繋がります」と書かない。',
  'mark.kept':
    '**まだ相手に届いていない。**「送信済み」「配達済み」と読める訳にしないこと。' +
    '預かり所に封のまま置いただけで、届くのは相手が次に起動したときである。',
  'postbox.kept':
    '**相手にはまだ届いていない。**「送信しました」「配達済み」と読める訳にしないこと。' +
    '届くのは相手が次に起動したときであり、いつになるかは分からない。' +
    'また「保存しました」（＝こちらの手元に残した）とも読ませないこと —— 預けた先は別の機械である。',
  'act.chat.live':
    '**まだ届いていない。**「送信しました」「あとで届きます」と読める訳にしないこと。' +
    '預かり所を置いていない状態では、繋がっていない間に打った言葉はどこにも残らない。' +
    '「預かり所を置くと」は条件であって、約束ではない。',
  'act.call.net':
    '**別の網を越えて繋がったことを、一度も見ていない**（実測 0 件）。' +
    '「どこからでも繋がります」「中継が自動で繋ぎます」と読める訳にしないこと。' +
    '`--relay` は付けられるが、繋がる保証はまだ無い。',
  'act.calendar.none':
    '**作っていない**という事実である。「準備中です」「近日対応」のような、' +
    '待てば来ると読める訳にしないこと。読み込みに失敗した（＝あるのに読めない）とも読ませないこと。',
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
    '「安全です」と読める訳にしないこと — **ルームの鍵を他人に見られたら、その人が入れる。**',
  'link.lost':
    '**切れたが、戻ってこられる**という意味（D44）。戻れるのは**さっきまで入っていた同じ相手だけ**で、' +
    'ルームの鍵が別人に渡っても意味は無い（一回性＝D12 は崩していない）。' +
    '「誰でも入れるようになりました」と読める訳にしないこと。' +
    'また「自動で再接続します」とも読ませないこと — **戻るのは相手の操作**であり、こちらは待っているだけである。',
};

/** 差し込み口を埋める。無い鍵はそのまま残す（黙って空にしない）。 */
export function format(template: string, values: Record<string, string | number>): string {
  return template.replace(/\{(\w+)\}/g, (whole, name: string) =>
    name in values ? String(values[name]) : whole,
  );
}
