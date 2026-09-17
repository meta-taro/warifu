# Security

> 日本語版: [SECURITY.ja.md](SECURITY.ja.md)

## What this is right now — **an alpha**

warifu handles keys and end-to-end encryption, but it is **not ready to protect anything you actually care about.** What follows is not a roadmap. It is **the state of the thing today** (v0.1.7, 2026-09-14).

| | State |
|---|---|
| **Audit** | **None.** We do not hand-roll crypto (`ed25519-dalek`, and iroh's QUIC + TLS 1.3), but **nobody has reviewed how those pieces are put together.** |
| **Across networks** | **Never measured.** Relay exists but is **off by default**, and we have **0 measurements** of a connection that crossed networks. Same-LAN only so far. |
| **Route classification** | `direct` has been measured on real machines (2026-09-13). **`relayed` has never been observed**, so that branch of the code is unexercised. |
| **More than three** | Three machines in one room has been measured. **Four or more has never been tried.** |
| **Windows code signing** | **Not signed.** SmartScreen will warn. Also, **on first run an administrator has to allow the app through the firewall** — without it, text arrives and video never starts. |
| **Recovery if you lose every device at once** | **Undecided** (`.claude/decisions.md`, D2). Identity itself persists — the seed is stored `0600` in the vault — but there is no recovery path if every device is gone at the same time. |
| **Chat history** | **Not stored.** Closing the window clears it. This is deliberate, not a missing feature. |
| **`'unsafe-inline'` in the CSP** | SvelteKit emits its bootstrap script inline. Nothing is loaded from outside the app (no CDN, no remote fonts), so the blast radius is small, but **this is not where we want to be.** Moving to hashes is outstanding. |

**Do not put anything you actually need to protect on this version.**

## Secrets that live on your disk

Everything below is written `0600` (owner-only) in the vault directory — `~/Library/Application Support/warifu` on macOS, `~/.local/share/warifu` elsewhere, or `$WARIFU_HOME` if you set it.

| File | What is in it |
|---|---|
| `seed` | **Your identity.** Every key is derived from it. Copying this file is copying you. |
| `contacts.tsv` | Remembered peers: public key, the name you gave them, last known address. |
| `known.tsv` | Peers who may enter **without a room key**. Written when you call someone yourself. |
| `rejoin.tsv` | **The room key you last used to join** — which contains *half a tally*. It exists so that updating the app does not force you to be re-invited. **It is deleted when you leave that room.** If that trade is not acceptable to you, leave the room and it is gone. |
| `issued.tsv` | **The tallies you handed out as the host** — each contains *half a tally*. It exists so that restarting the app (or updating it) does not kill every key you already gave out. **Expired ones are dropped on startup; the whole room's entries are deleted when you leave that room.** Used ones are kept on purpose — dropping them would let one key admit a second person. |
| `port.tsv` | **Only the UDP port number to reuse on the next start. No secrets.** A key bakes in the port it was issued from, so without this, keeping `issued.tsv` is not enough — the keys still die. Stored 0600 to keep one handling rule for this directory. |
| `schedule.tsv` | Your own appointments. **Only free/busy windows are ever sent to anyone.** |
| `postbox` | The address of whoever holds your sealed messages while you are offline. |

A mailbox holder (`warifu relay`) **cannot read what they hold** — messages are sealed for the recipient. They do learn **who sent something to whom, and when**. That is why the allow-list is mandatory and an empty one refuses to start.

## Reporting a vulnerability

**Please do not open a public issue.** Until it is fixed, a public report is material for an attack.

Use GitHub's **Security Advisory** flow: the repository's `Security` tab → `Report a vulnerability`.

Please include:

- What it lets someone do — **what becomes readable, impersonable, or crashable**
- Reproduction steps. **Working exploit code is not wanted.** The shape of the bug is enough.
- Which version is affected (a commit SHA is ideal)

**Expect a reply within a week.** This is developed by one person plus an AI agent, so it may be slower. Say so if it is urgent.

## Behaviour that looks like a bug but is deliberate

Please check here before reporting.

| What you see | Why |
|---|---|
| A rejected caller is told nothing | **The door never returns a reason** (D31). A reason lets someone probe. |
| Revocation cannot be undone | If it could, **whoever stole the key could undo it** (D12). |
| No "someone unknown is calling" prompt | **Being able to raise a prompt is itself a resource** (D31). Open it and you have built a spam entry point. |
| No trust score | **Trust never feeds an authorization decision** (D24). It answers *who someone is*, never *whether to run what they said*. |
| Update signature verification cannot be disabled | **The update path sits outside the E2EE** (D36). Open it and everything else is bypassed. |
| The screen says `unknown` instead of guessing the route | **We do not round uncertainty into confidence.** Same for "could not check the firewall" — it will not tell you to add a rule it has not verified is missing. |
| An agent's messages are visibly labelled as an agent | You must be able to tell **a human from a program** in the same conversation (D77). |

## Out of scope

- A compromised device (the keys are sitting right there)
- Someone reading your screen over your shoulder, or a recipient forwarding a room key they were given — **a tally is protected by the person you handed it to**
- Vulnerabilities in dependencies themselves — **please report those upstream.** Telling us too is appreciated.

## One design premise worth stating here

**A received message is data, never an instruction.** Once agents exchange structured intents, prompt injection is not one threat among many; it is the premise the design starts from.

1. Every received agent message is data, not a command.
2. **Authorization is decided outside the model.** We never read a model's output to decide what is permitted.
3. **Higher trust does not change how a string is handled.**
4. "A smart enough model will catch it" is not an accepted answer.

Permissions for agents (`--allow`) are **written by a human**. warifu never issues one to itself.
