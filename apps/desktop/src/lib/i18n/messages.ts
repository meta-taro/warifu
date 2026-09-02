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
  | 'roster.capacity';

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
};

/** 差し込み口を埋める。無い鍵はそのまま残す（黙って空にしない）。 */
export function format(template: string, values: Record<string, string | number>): string {
  return template.replace(/\{(\w+)\}/g, (whole, name: string) =>
    name in values ? String(values[name]) : whole,
  );
}
