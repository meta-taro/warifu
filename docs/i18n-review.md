# Translation review (**a human writes this**)

> 日本語版: [i18n-review.ja.md](i18n-review.ja.md)

> **The AI does not fill this in** (`.claude/rules/product-baseline.md` §19 / §27).
> All four languages are **AI first drafts**. **Eight strings will cause real harm if mistranslated.**

Sheet: **`docs/i18n-review.tsv`** (one row per key × 4 languages)

---

## What to look at

`DESIGN.md §9` (**D35**) fixes the UI languages at **en / ja / zh / ko**. The sheet is one row per key, with the four languages side by side.

| Column | |
|---|---|
| `鍵` (key) | The name the code looks up. **Do not change it** |
| `en` `ja` `zh` `ko` | The current translation (**AI draft**) |
| **`事故になるか`** (harmful if wrong) | **The 8 rows marked ★ change what a person does if mistranslated** |
| `翻訳者への注記` (note to translator) | Attached to the ★ rows. It says **what the string must not be read as** |
| **`判定`** (verdict) | **Empty. You write it** |
| **`直した訳`** (corrected translation) | **Empty.** Put corrections here |

`ok` or `直す` ("fix") is enough for the verdict. **A row left empty means "not looked at yet"** — please do not fill it with `—` or `N/A` (baseline §19).

---

## The eight marked ★

**If you only review these, that is already worth it. The rest can wait.**

| Key | What it protects |
|---|---|
| `revoke.irreversible` | **"Cannot be undone" is a fact.** It must not read as "you can restore it later" |
| `revoke.confirm` | The press has to feel **final** |
| `door.refused` | It means **the refusal already happened**. If it reads as "checking…", someone waits forever for a caller who is not coming |
| `meeting.key.hint` | "Only the person you handed it to can enter" describes **the mechanism, not a safety guarantee** |
| `link.lost` | **The link broke, but they can come back** (D44). Must not read as "reconnecting automatically", nor as "anyone can walk in" |
| `act.mail.none` | **"Cannot send yet" is a statement about there being no route.** It must not read as "sending failed" (tried and failed) nor "coming soon" (wait and it arrives). **Believing you sent something you did not is the worst outcome here** |
| `act.address.none` | **We are the ones who do not remember.** It must not read as "that person was not found" or "they refused" — blaming the other side sends people off to ask them about nothing |
| `chat.scope` | **There is one conversation.** Must not read as "only this person receives it" or "private chat" — someone who believes that writes things they would not want others to see |

---

## Getting corrections in

Fill in the `直した訳` column and hand it back. **We put it into the code.**
Afterwards the sheet is regenerated with `python3 scripts/i18n-review.py`.

**Adding a key means regenerating the sheet.** Forget and the test fails
(`src/lib/i18n/review.test.ts`) — because **an unreviewed gap that nobody can see is the worst state to be in.**
