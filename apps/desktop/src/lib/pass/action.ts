// 札の動作（`chat.send` など）を、人に見せる言葉にする。
import type { MessageKey } from '$lib/i18n/messages';

/** 言い方を持っている動作。**口が聞く動作を足したら、ここと辞書に足す**（`action.test.ts` が落ちる）。 */
const 言い方: Record<string, MessageKey> = {
  'chat.read': 'pass.action.chat.read',
  'chat.send': 'pass.action.chat.send',
  'chat.export': 'pass.action.chat.export',
  'contacts.read': 'pass.action.contacts.read',
  'contacts.add': 'pass.action.contacts.add',
  'contacts.call': 'pass.action.contacts.call',
  'room.invite': 'pass.action.room.invite',
  'room.join': 'pass.action.room.join',
  'room.leave': 'pass.action.room.leave',
  'room.rename': 'pass.action.room.rename',
  'room.handoff': 'pass.action.room.handoff',
  'screen.read': 'pass.action.screen.read',
  'voice.say': 'pass.action.voice.say',
  'profile.write': 'pass.action.profile.write',
  'rules.list': 'pass.action.rules.list',
  'calendar.freebusy': 'pass.action.calendar.freebusy',
  'inbox.list': 'pass.action.inbox.list',
  'mail.account': 'pass.action.mail.account',
  'inbox.open.summary': 'pass.action.inbox.open.summary',
  'inbox.open.structured': 'pass.action.inbox.open.structured',
  'inbox.open.raw': 'pass.action.inbox.open.raw',
  'inbox.open.attachments': 'pass.action.inbox.open.attachments',
};

/**
 * 帯に出す動作の言葉。**内部名は出さない**（spec 08 S08-1）。
 *
 * 言い方を持たない動作（新しい口）は、隠さずに内部名のまま出す ——
 * 何を許すのか分からない帯より、読みにくくても正しい名前のほうがよい。
 */
export function 動作の言葉(動作: string, t: (k: MessageKey) => string): string {
  const 鍵 = 言い方[動作];
  return 鍵 ? t(鍵) : 動作;
}
