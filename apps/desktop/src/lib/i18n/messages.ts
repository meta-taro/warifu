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
  | 'edit.cut'
  | 'edit.copy'
  | 'edit.paste'
  | 'edit.selectAll'
  | 'edit.pasteHint'
  | 'link.closed'
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
    'meeting.key.hint': 'これを参加者に渡します。渡した相手だけが入室できます。10 分で切れます。紙に書いても、読み上げても渡せます。',
    'meeting.start.action': '会議をはじめる',
    'meeting.join.title': 'もらった会議キーで入室する',
    'meeting.join.hint': 'もらった側が入室します。渡した側は待つだけです。',
    'meeting.join.action': '入室する',
    'meeting.key.own': 'これは自分の会議キーです。参加者に渡してください。',
    'link.closed': '経路が閉じました。',
    'browser.only': 'ブラウザで開いています。会議は warifu の窓でだけ動きます。',
    'camera.denied': 'カメラとマイクが許可されていません。OS の設定で許可してください。',
    'camera.missing': 'カメラかマイクが見つかりません。',
    'camera.busy': 'ほかのアプリがカメラを使っています。',
    'meeting.key.copy': 'コピーする',
    'meeting.key.copied': 'コピーしました',
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
    'chat.hint': '会議に入っている人へ届きます。**残りません** — 閉じると消えます。\n',
    'chat.placeholder': '書いて Enter',
    'chat.send': '送る',
    'chat.empty': 'まだ何もありません',
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
    'meeting.key.hint': 'Hand this to the people joining. Only they can enter. It expires in 10 minutes. You can write it down or read it aloud.',
    'meeting.start.action': 'Start a meeting',
    'meeting.join.title': 'Enter with a meeting key',
    'meeting.join.hint': 'Whoever received the key enters. Whoever gave it just waits.',
    'meeting.join.action': 'Enter the meeting',
    'meeting.key.own': 'That is your own meeting key. Hand it to the people joining.',
    'link.closed': 'The connection closed.',
    'browser.only': 'Opened in a browser. Meetings only work inside the warifu window.',
    'camera.denied': 'Camera and microphone are not allowed. Allow them in your OS settings.',
    'camera.missing': 'No camera or microphone found.',
    'camera.busy': 'Another app is using the camera.',
    'meeting.key.copy': 'Copy',
    'meeting.key.copied': 'Copied',
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
    'chat.hint': 'Goes to everyone in the meeting. **Not saved** — it disappears when you close.\n',
    'chat.placeholder': 'Type and press Enter',
    'chat.send': 'Send',
    'chat.empty': 'Nothing yet',
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
    'meeting.key.hint': '把它交给参加者。只有拿到的人才能进入。10 分钟后失效。可以写在纸上，也可以念给对方。',
    'meeting.start.action': '开始会议',
    'meeting.join.title': '用收到的会议密钥进入',
    'meeting.join.hint': '收到密钥的一方进入，交出的一方等待即可。',
    'meeting.join.action': '进入会议',
    'meeting.key.own': '这是你自己的会议密钥。请交给参加者。',
    'link.closed': '连接已关闭。',
    'browser.only': '正在浏览器中打开。会议只能在 warifu 窗口里进行。',
    'camera.denied': '未允许使用摄像头和麦克风。请在系统设置中允许。',
    'camera.missing': '找不到摄像头或麦克风。',
    'camera.busy': '其他应用正在使用摄像头。',
    'meeting.key.copy': '复制',
    'meeting.key.copied': '已复制',
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
    'chat.hint': '发送给会议中的所有人。**不会保存** — 关闭后消失。\n',
    'chat.placeholder': '输入后按 Enter',
    'chat.send': '发送',
    'chat.empty': '还没有内容',
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
    'meeting.key.hint': '참가자에게 건네주세요. 건넨 사람만 입장할 수 있습니다. 10분 뒤에 만료됩니다. 종이에 적어도, 말로 전해도 됩니다.',
    'meeting.start.action': '회의 시작하기',
    'meeting.join.title': '받은 회의 키로 입장하기',
    'meeting.join.hint': '받은 쪽이 입장합니다. 건넨 쪽은 기다리기만 하면 됩니다.',
    'meeting.join.action': '입장하기',
    'meeting.key.own': '자신의 회의 키입니다. 참가자에게 건네주세요.',
    'link.closed': '연결이 끊겼습니다.',
    'browser.only': '브라우저에서 열렸습니다. 회의는 warifu 창에서만 동작합니다.',
    'camera.denied': '카메라와 마이크가 허용되지 않았습니다. OS 설정에서 허용해 주세요.',
    'camera.missing': '카메라나 마이크를 찾을 수 없습니다.',
    'camera.busy': '다른 앱이 카메라를 사용 중입니다.',
    'meeting.key.copy': '복사하기',
    'meeting.key.copied': '복사했습니다',
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
    'chat.hint': '회의에 있는 사람에게 전달됩니다. **남지 않습니다** — 닫으면 사라집니다.\n',
    'chat.placeholder': '입력 후 Enter',
    'chat.send': '보내기',
    'chat.empty': '아직 아무것도 없습니다',
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
] as const;

/** 翻訳者への注記。**訳文と一緒に渡す。** */
export const TRANSLATOR_NOTES: Partial<Record<MessageKey, string>> = {
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
};

/** 差し込み口を埋める。無い鍵はそのまま残す（黙って空にしない）。 */
export function format(template: string, values: Record<string, string | number>): string {
  return template.replace(/\{(\w+)\}/g, (whole, name: string) =>
    name in values ? String(values[name]) : whole,
  );
}
