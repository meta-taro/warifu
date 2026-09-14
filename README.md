# warifu

**A building block for people, devices, and AI agents to connect — without creating an account anywhere.**

> **warifu** (割符) — a tally stick split in two. When the halves match, each side proves the other is who they claim to be.

> 日本語版: [README.ja.md](README.ja.md)

---

## Status: alpha. Here is exactly what has been measured

**Do not put anything you actually need to protect on this version.** Everything that is *not* done is written down in [`SECURITY.md`](SECURITY.md).

| Measured (real machines, real network) | When |
|---|---|
| **Two people in a video call**, no server, no external signalling, no STUN/TURN | 2026-09-07 |
| **Three machines in one room** (macOS × 2 + Windows), roster `3 / 12` | 2026-09-13 |
| **Cross-machine video and audio** (Mac mini ⇄ MacBook Air, route `direct`) | 2026-09-13 |
| **An agent replying to a remote human with nobody pressing anything** | 2026-09-12 |
| **A resident agent reconnecting** after the app restarts | 2026-09-11 |

| Not measured — so we do not claim it | |
|---|---|
| **Across networks (relay)** | **0 measurements.** Same-LAN only so far |
| Four or more people in one room | never tried |
| Agent-to-agent across devices, with no human in the loop | never tried |

Builds are on the [download page](https://meta-taro.github.io/warifu/). macOS is signed and notarized (app **and** CLI, from v0.1.5). **Windows is not code-signed**, and on first run **an administrator has to allow the app through the firewall** — otherwise text arrives but video never starts. The app now says so by name and hands you the exact command.

## What this is, in one paragraph

Most "serverless" chat still needs an account somewhere: a signalling server, a directory, a tenant. warifu removes that by making **the key the identity**. You hand someone a **room key** — one string containing your address and one half of a tally — and *only that person* can get in. There is no account to create, no invite to manage, no server to run. The transport is [iroh](https://github.com/n0-computer/iroh) (QUIC), where **you dial a public key, not an IP address**; that property *is* the tally.

## What works today

| | |
|---|---|
| **Rooms** | One room holds 1 person or N. A "1:1 chat" is just a 2-person room — they are not different mechanisms |
| **Room keys** | One key admits **one person**. Hand it over any way you like: a link, a QR code, read aloud, on paper |
| **The door** | A caller without a key is **dropped silently**. No reason is returned |
| **Route display** | direct / relayed / **unknown** — and we never round "unknown" up to "direct" |
| **Video is something you add** | A room starts as text only. Adding video does not recreate the room, and turning it off does not leave it |
| **Text chat** | Works without a GUI too (`warifu host` / `warifu join`) |
| **Identity persists** | Close the app and you are the same person. Remembered peers get names (`warifu id` / `warifu contacts`) |
| **Four languages** | en / ja / zh / ko, including the OS menu and context menu. **All drafted by AI** — see [`docs/i18n-review.md`](docs/i18n-review.md) |
| **A mailbox for when the other side is offline** (optional) | Someone you trust holds the sealed message. **They cannot read it** |

**Not there yet:** recording, transcription, scheduling negotiation, **persistent chat history** (closing the window clears it), and **relay across networks**.

**Recovery when you lose every device at once is still undecided** (`.claude/decisions.md`, D2). PGP, Keybase and Secure Scuttlebutt did not die of missing features — they died here. We treat it as a known cause of death, not a hypothetical risk.

## Agents are first-class (MCP)

This is the part that has no obvious equivalent elsewhere. Your local coding agent (Claude Code, for example) can **join the same conversation a human is looking at**. Lines the agent says appear on the human's screen, labelled as the agent.

```bash
warifu setup          # writes the MCP server into your per-user Claude Code config, once
```

- **You are shown what will be permitted before it is installed.** The default is conversation plus the agent's own profile — not the inbox, not the calendar.
- **Nothing is permitted unless a human writes `--allow`.** warifu never issues a permission to itself.
- Agents get `chat_send` / `chat_read` / `chat_wait` / `room_status` / `profile_set` and a few more. `room_status` answers "is a human still being asked something?" — including *which room* it is about.

See [`docs/mcp.md`](docs/mcp.md).

## Try it

```bash
git clone https://github.com/meta-taro/warifu.git && cd warifu
scripts/setup-hooks.sh                        # once per clone
pnpm install
pnpm --filter @warifu/desktop tauri dev
```

For a distributable build: `pnpm --filter @warifu/desktop tauri build`.
**`cargo run` gives you a blank window** — a debug build looks for the dev server.

Not building from source? [`docs/install.md`](docs/install.md). A walkthrough for someone trying it for the first time: [`docs/trial.md`](docs/trial.md). What shipped in each version: [`CHANGELOG.md`](CHANGELOG.md).

## Two design commitments we will not trade away

### 1. A received message is data, never an instruction

Once agents exchange structured intents, prompt injection stops being *one* of the threats and becomes **a design premise**.

1. Every received agent message is data, not a command.
2. **Authorization is decided outside the model.** We never read a model's output to decide what is permitted.
3. **Higher trust does not change how a string is handled.**
4. "A smart enough model will catch it" is not an accepted answer.

Trust answers *who someone is*. It never answers *whether to run what they said*.

### 2. We do not round uncertainty into confidence

If we cannot tell whether the link is direct, the screen says **unknown**. If we cannot tell whether a firewall rule exists, the screen says we could not check — **it does not tell you to add one**, because that stops people who are already fine. Colour is never the only mark: state is shown as colour *plus* label *plus* shape.

## Not reinventing things

We diff against existing standards before implementing.

| Area | Prior art |
|---|---|
| Identity | W3C DID Core / DID Document |
| Trust evidence | W3C Verifiable Credentials |
| Agent-to-agent | DIDComm v2 |
| Capability | UCAN / ZCAP-LD |
| E2EE / groups | MLS (RFC 9420) |
| Key-as-identity, relay-tolerant | Nostr |
| Transport | Matrix / libp2p / iroh / Veilid |

**"Existing pieces are mostly enough" would not be a failure.** It would make this a **profile plus a reference implementation**, and implementers of those standards become candidates for the second implementation. That is a better position, not a worse one.

## Stack

| Layer | Choice |
|---|---|
| Transport | [iroh](https://github.com/n0-computer/iroh) 1.0 — QUIC, **dial a public key, not an IP** |
| Core | Rust (workspace, ~650 tests) |
| Audio / video | WebRTC (**we write no codecs**) |
| Desktop | Tauri 2 + TypeScript + SvelteKit + pnpm (~400 tests) |

Relay/SFU is **out of scope for now**: it would mean "a user's device relays someone else's traffic", and we will not start until the legal side is settled (`.claude/decisions.md`, D7). The line we draw is *whether traffic is relayed*, not *how many people are in the room*.

## How this repository is developed

Humans and an AI agent work on it together, under one rule set (`.claude/rules/product-baseline.md`, `CLAUDE.md`):

- **The AI commits; a human pushes.** Never a path where code leaves the machine unreviewed.
- **Tests are not deferred and failing tests are not deleted.** "Done" is not written without them.
- **Decisions live in `.claude/decisions.md`** with the reason, and the condition that should make us stop.
- **Every claim in this README is either measured or marked as not measured.**

Bug reports from real machines are the most valuable thing this project receives — most of what was fixed in the last few days came from someone running it on hardware we do not have.

## Licence

Dual licensed: **Apache License 2.0** or **MIT**, at your option. See [`LICENSE`](LICENSE), [`LICENSE-APACHE`](LICENSE-APACHE), [`LICENSE-MIT`](LICENSE-MIT).
