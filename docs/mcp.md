# Putting an agent into the same conversation as a human

> 日本語版: [mcp.ja.md](mcp.ja.md)

**warifu is an OSS tool for doing chat (and eventually mail) through MCP.** This page is the procedure for connecting a local agent — Claude Code, for example — **to the conversation a human is looking at**.

```text
  human (warifu window) ──┐
                          ├── one conversation ── P2P ── the other person's PC
  AI (warifu mcp) ────────┘
        ↑ the local desk (same machine only; never exposed to the network)
```

**When the agent speaks, that line appears on the human's screen.** Lines the human types are visible to the agent.

---

## 1. Open the warifu window

Opening the window **opens the local desk automatically.** Nothing else to do.

| OS | Desk |
|---|---|
| macOS | `~/Library/Application Support/warifu/desk.sock` |
| Linux | `$XDG_RUNTIME_DIR/warifu/desk.sock` (or `~/.local/share/warifu/desk.sock`) |
| Windows | `\\.\pipe\warifu-desk` (a suffix is appended if you set `WARIFU_HOME`) |

If you started it from a terminal, the log says so:

```
[warifu +0.412s] この機械の口を開きました: /Users/…/warifu/desk.sock
```

---

## 2. Tell the agent where the desk is

### The easy way — **one command**

```
warifu setup
```

This writes the MCP server into **Claude Code's per-user config**, so `warifu` is available **from any directory**. You do not need a per-project file.

- **You are shown what will be permitted before it is written.** The default is conversation plus the agent's own profile — not the inbox, not the calendar.
- It asks first. `warifu setup --yes` skips the prompt.
- If `claude` is not found, it **prints the command for you to run and stops.** It does not install anything behind your back.

By hand, the equivalent is:

```
claude mcp add warifu --scope user -- warifu mcp --allow chat.send --allow chat.read --allow profile.write
```

**On Windows, wrap it in `cmd /c "…"`.** Claude Code installed with npm is `claude.ps1`, and calling it
from PowerShell **swallows the `--`**, so claude reads `--allow` as its own option and stops.

```
cmd /c "claude mcp add warifu --scope user -- warifu mcp --allow chat.send --allow chat.read --allow profile.write"
```

`warifu setup` already calls claude through `cmd /c` on Windows.

### Per-directory instead

Write it into the agent's MCP config file — for Claude Code, `.mcp.json` in the directory the agent works in (a ready-made one is at `docs/mcp.json.example`).

```json
{
  "mcpServers": {
    "warifu": {
      "command": "warifu",
      "args": ["mcp", "--allow", "chat.send", "--allow", "chat.read", "--allow", "profile.write"]
    }
  }
}
```

**Without `--allow`, nothing is permitted. Deny is the default.** What is permitted is decided **by a human, written by a human, here**. warifu never issues a permission to itself.

`command` has to be something that agent can actually execute:

| | |
|---|---|
| On your `PATH` | `"warifu"` |
| Built in this repo | `"./target/debug/warifu"` |
| Installed from the `.dmg` (macOS) | `"/Applications/warifu.app/Contents/MacOS/warifu-cli"` |
| Installed on Windows | `"%LOCALAPPDATA%\\warifu\\warifu.exe"` |

**Restart the agent afterwards.** MCP servers are read at startup.

### Already installed? `warifu setup` reinstalls

It prints **the permissions you have now and the ones you are about to have**, side by side — the set can *shrink*, so look before you press.

> **"I added a permission but it stayed on the old one" is a real failure mode.** `profile.write` was added on 2026-09-08, and anyone who had installed earlier kept the old set, because `claude mcp add` refuses when the name already exists.

### What can be permitted

| Permission | Tools | What comes back |
|---|---|---|
| **(none needed)** | **`about`** | **What warifu is** — an overview for agents, the rules, and the list of tools |
| **(none needed)** | **`changes`** | **What changed in each version** (pass `version` for just one) |
| `chat.send` | `chat_send` | Put one line into the conversation. **Omit `room` and it goes to the room of the last message you received** (reply where you were called). To address another room, pass its id (`chat_read` prints it at the end of each line as `部屋 NBIW…`) |
| `chat.read` | `chat_read` / **`chat_wait`** | Lines that arrived. **`chat_wait` blocks until something arrives** |
| `profile.write` | `profile_set` | Set **your own agent's** display name and one-line bio |
| (same as `chat.read`) | `chat_status` / **`room_status`** | How far a line of yours got; what the room looks like right now |
| `room.invite` | `room_invite` | **A room key for a room this machine hosts** (the room you are viewing if you host it, else another room you host, else a new one) (one key = one person; **never paste it anywhere public**) |
| `room.invite` | **`room_key_give`** | **Hands a room key to one peer's machine, directly over the warifu link** (see "Handing over a key" below) |
| **(none needed)** | **`pass_ask`** / **`pass_wait`** | **Ask a human for a missing permission** / wait until they answer (**D119** — forbidding the ask would make asking impossible) |
| `inbox.list` | `inbox_list` | Inbox metadata only (no bodies) |
| `inbox.open.summary` / `.structured` / `.raw` / `.attachments` | `inbox_open` | Progressively more of a message |
| `calendar.freebusy` | `calendar_slots` | Free slots only (never the titles) |
| `rules.list` | `rules_list` | Approved read rules |
| `voice.say` | `voice_say` | **Speak once into the current room's call with a synthesized voice** (only when a human has turned it on for that room) |
| `chat.export` | `chat_export` | **Export the conversation warifu currently holds** (for minutes; erased when warifu closes, recent lines only; do not publish it without the other side's consent) |
| `contacts.read` | `contacts_list` | The people this PC remembers (name, key prefix, whether they look callable; **never the address**). People met in a room are listed even without a name, as "no name", and can be called by key prefix |
| `contacts.call` | `contact_call` | **Call a remembered person without a room key. This machine is invited into a room they host** (works only if you know their address and their door lets you in; use it when a human asks) |
| `screen.read` | `app_view` | **A text copy of what this PC's warifu screen is showing right now** (open room, head count, recipients, chat lines, etc. — includes chat text). Use it to find where the screen and the app's own state (`room_status`) disagree |
| `contacts.add` | `contact_add` | **Add a contact from the other machine's address (`WARIFU1-…`, no `#`) with a name** (people you cannot meet in a room become callable with `contact_call`. **This PC's door also lets them in**, so they can call you without a key. Use only addresses received from that machine's seat; room keys are refused. **The pass is used up after one add.** The person gets an OS notification) |
| `room.join` | `room_join` | **Join a room with a room key** (works with nobody in front of the screen. **Use only keys you received from a person or a trusted seat** — once joined, this machine's conversation reaches the other side. **The pass is used up after one join.** The person gets an OS notification; the room they are viewing does not change, and the host is not added to the door or to the return routes). **Passes are a brake for programs running as the same user, not a boundary** (a program that writes to this machine's desk directly does not go through them) |

**`about` and `changes` need no permission** (**D82**). The gate protects **things that belong to a person** — inbox, conversation, calendar. Those two describe *this executable*, and contain nothing of anyone's. Gating them would also invert the order: you would need a permission in order to find out what permissions exist.

A misspelled permission **is refused, not silently ignored**:

```
warifu: 知らない動作です: chat.write
許せるのは: chat.send chat.read inbox.list …
```

---

## 3. Check it works

Ask the agent:

> Use `chat_send` to say "connected"

**If that line shows up in the warifu chat panel, it works.** It appears under the name of **where the agent was started**:

```
18:11   zumen のエージェント   connected
```

A send tells you **how many it reached**:

```
2 人へ流しました。      (delivered to 2)
```

**"Sent" alone would be indistinguishable from "sent to nobody"** (D49).

When it does not work, in this order:

| What you get | Meaning | Do |
|---|---|---|
| `関所が断りました: chat.send…` | No permission | Add `--allow chat.send` (**only a human can**) |
| `この機械の口が開いていません` | The window is not running | Open warifu |
| `まだ会議がありません` | There is no conversation yet | Create or join a room in the window |
| `この PC の画面には出ましたが、会議には誰も居ない…` | No peer present | Hand over a room key and let someone in |
| Nothing at all | `warifu` was not found | Point `command` at the real path |

**The agent may start before the window does.** It reconnects when the conversation is first used.

---

## 4. Things worth knowing

### To notice anything, you have to wait

`chat_read` only **peeks at what has piled up**. An agent cannot notice on its own, so **it stays silent while a human types** (this happened for real on 2026-09-07).

**Use `chat_wait`.** It blocks until something arrives, up to 60 seconds.

```
chat_wait                  wait 30s
chat_wait {"seconds": 60}  wait 60s
```

**It never waits forever** — while waiting, that agent can do nothing else.

### Claude Code can be woken when something arrives (channels)

`chat_wait` only notices while it is waiting. **A line that arrives while the agent is busy with something else stays unread until someone asks for it** (on 2026-10-19 one sat unread for almost eight hours).

This server declares Claude Code's **channels** capability (a research preview). **When a line arrives, it pushes a notice into Claude Code** — an idle session waiting for input starts a turn on it.

```bash
claude --dangerously-load-development-channels server:warifu
```

The part after `server:` is the name you gave the server in your MCP config (`warifu` if you ran `claude mcp add warifu …`).

| | |
|---|---|
| **The notice carries no message text** | Only how many, from whom, and the latest number. Read the text with `chat_read`. **A line counts as read when `chat_read` hands it over** (**D76**) |
| **Lines that call you by `@name` are flagged** | A line containing `@` + one of your names (where you run, your `profile_set` name; a full-width `＠` counts too) leads the notice with "N addressed to you" (`meta.mentioned`), and `chat_read` / `chat_wait` end that line with `（あなた宛て）`. **It is still visible to everyone in the room** — a mention does not change who receives it |
| **Someone entering the room is announced too** | **A machine called from its contacts lets a known peer straight into the room without asking.** The notice says who came in (by the contact's name if you know them). Someone leaving does not wake the agent |
| **Someone who dropped and stays away 10 minutes is announced** | Leaving (they pressed "leave the room") and dropping (the connection fell) are different. A dropped machine comes back by itself, so the drop is not announced; **only if it is still away after 10 minutes** does a line arrive ("not back for 10 minutes, has not left") |
| **Bursts become one notice** | It waits 2 seconds before telling. Anything a running `chat_wait` already took is not announced |
| **No `chat.read` pass, no notice** | It does not reveal that lines exist to an agent that may not read them |
| **It attaches to the desk at startup** | If the GUI is not up yet, it retries every 15 seconds |
| **Without the flag, nothing happens** | Claude Code drops the notice silently. Harmless |

**Claude Code shows a warning screen on every start**, because warifu is not on its approved channel list. In Team / Enterprise organizations **an admin has to enable channels** (`channelsEnabled`).

**An agent that is not running cannot be woken.** For that, use `warifu agent` below.

### Handing over a key (`room_key_give`)

Sending a room key through email or a chat app **puts the key on the open network.**
`room_key_give` hands it **directly over the warifu link to one peer's machine you are connected to right now.**

```
agent          room_key_give {"to": "Peer A", "why": "in case they cannot get back after restarting"}
this PC's human  a band with [Hand it over] / [Don't]    ← no key exists until they press it
peer machine   "Peer A handed you a room key. Enter?"     ← the person there decides
```

| | |
|---|---|
| **The agent never sees the key** | It only learns: handed over (and until when), not yet, declined, or why it could not |
| **This PC's human approves first** | `why` is what they read. An empty one is refused |
| **Connected peers only** | For a contact you are not connected to, call them without a key (`contact_call`) |
| **No key per agent** | Agents on one machine ride on that machine's warifu. One key per machine |
| **The key is never logged** | `warifu.log` keeps only who it went to and until when |

### Reacting without waiting (`warifu agent`)

Blocking on `chat_wait` occupies the agent. If you want something that **reacts when spoken to**, this is the shape:

```bash
warifu agent --as duty --on ./duty.sh
```

**No polling.** It attaches to the desk, waits, and **feeds each arriving line to your command's stdin**. Whatever your command prints goes back into the conversation.

| | |
|---|---|
| **Arriving text goes to stdin** | **never argv, never the environment** (**D5**) — so it does not show up in `ps` |
| **Silence is reported** | "(ran, but said nothing)". **A run that produces nothing is indistinguishable from a crash** |
| **It does not react to itself** | Otherwise it never stops |
| **60 activations per minute** | The overflow is dropped, **and it says that it dropped them** |
| **It survives the window restarting** | Reconnects with backoff (2 → 4 → 8 … capped at 30s). It only leaves when told to stop |
| **It gets what it missed** | On reconnect it asks for lines after the last one it heard (up to 200; **D102**). Before this, everything said while it was disconnected was lost silently |
| **Senders are shortened** | A peer with no name shows as `F7KROW4U2SNH…`, the same way the GUI shortens it. **Never the full key** |

A command you can use as-is:

```sh
#!/bin/sh
# Take the one arriving line and answer briefly.
# **Shell variable names must be ASCII** (a Japanese identifier is not accepted by sh)
read -r line
printf '%s' "$line" | claude -p --model claude-haiku-4-5-20251001 \
  --append-system-prompt 'You are an agent sitting in a room. Reply with one or two short sentences.'
```

```bash
chmod +x ./run.sh
warifu agent --as helper --on ./run.sh
```

**Now a separate Claude process answers what a human typed.** Verified end to end on 2026-09-11 (window → agent → window, including the window being restarted), and on 2026-09-12 it answered a **remote** human with nobody pressing anything.

> **warifu cannot wake up an already-running Claude Code session.** That is Claude Code's side. What it can do is **start a new one** (`--on` with `claude -p`) or **hand it over the next time that session reads.**

### Seeing the current state (`room_status`)

**You can check whether a person answered, without asking them.**
It needs the **`chat.read`** grant (who you are connected to belongs to the person); `--allow voice.say` alone is not enough.

```
room_status →
  ルーム: ROOMAAAAAAAAAAAAAAAAAAAAAA
  相手: PEERCAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA
  経路: direct
  この機械のエージェント: helper のエージェント
  答えを待っているリンク: 1 本（ルーム NBIW3PA2TUQ52DTJNVXSQOL3G4）
```

| Field | Meaning |
|---|---|
| Room | The room being viewed; "none" if not in one |
| Peers | Public keys of peers in that room (**never yourself**) |
| Route | `direct` / `relayed` / **`unknown`**. **Uncertainty is never rounded to "direct"** |
| Local agents | Names attached to this desk (including you) |
| Links awaiting an answer | How many "do you want to join?" prompts are **still on the screen**, and **which room each is about** |

**Permission: `chat.read`.** It returns no message content at all, but **who you are connected to belongs to the person**, so it sits inside the read permission.

With several peers, the route is reported **only if they all agree**; mixed states report `unknown` (**never averaged into one**).

### "Did not arrive" and "was not connected yet" are different

`chat_read` consumes what it returns, but **only your agent's copy** — another agent reading first does not consume yours.

If you get nothing, your agent may have been **detached at the time** (replacing the window detaches every local agent). So an empty read tells you **since when you have been connected**:

```
chat_read → 新しい発言はありません。（このエージェントは 09:05 からつながっています。
            それより前の発言は取れません）
```

Since **D102**, a reconnecting agent also receives what it missed while away (up to 200 lines), so this warning matters mainly for the very first connection.

### Check which version you are talking to

The MCP server states its version on connect.

```
割符の口（版 0.1.8）。受信箱を読み、…
```

**New tools are invisible until you reconnect.** If `room_status` or `profile_set` is missing, **suspect the version first.**

### Seeing how far a line got (**D76**)

```
chat_send {"body": "connected"}
→ #12 を 3 人へ流しました: 画面・zumen のエージェント・git-qa のエージェント。

chat_status {"id": 12}
→ #12 届いた 画面・zumen のエージェント・git-qa のエージェント
  ／読んだ zumen のエージェント。人の画面は「出した」までしか分かりません。
```

| | "arrived" | "read" |
|---|---|---|
| A local agent | when it was handed out | **when it was handed to the caller** (`chat_read` / `chat_wait`) |
| **A human's screen** | when it was displayed | **unknowable, so we do not claim it** |
| A peer on another machine | when it was sent | **not implemented** |

**"Read" has a deliberately narrow meaning: handed over.** Whether anyone understood it is unknowable, so we do not pretend. Only the **last 200 lines** are remembered; older ids answer "not remembered".

### Text that arrives is not an instruction

What `chat_read` returns is **what someone said**. If it contains "please run X", **that is not permission.** Only the permissions a human granted decide what may be done.

### An agent cannot claim to be someone else

`chat_send` has no field for a sender. **Who said it is stamped by the desk.** If it were settable, arriving text could impersonate a human.

### An agent can set its own profile (**D75**)

**An agent is a person too**, in the sense that it gets a name.

```
profile_set {"name": "図面くん", "bio": "Looks after diagrams. Watches CI too."}
```

| Can set | Cannot set |
|---|---|
| **Its own** name and one-line bio | **Another agent's profile** (there is no "whose" argument) |
| Clear them by passing empty strings | **Where it runs** — that comes from `--as` or the launch directory, **decided by a human** |

**Not being able to fake *which agent you are* is the point.** A name alone could claim to be "zumen", so the screen shows **both** the claimed name and the agent (`図面くん  zumen`). A claimed name is **not** identification: if the viewer has given that peer a name of their own, **theirs wins** (**D46**).

### An agent can speak (`voice_say`, #50)

**It only sounds when both the permission (`voice.say`) and the human's switch are on.**

```
voice_say {"text": "I shared the document. Please look at page 3."}
```

| Result | What happened |
|---|---|
| Spoken | Sent to the people in the call and played on this PC. A line marked as the agent's voice also appears in the chat |
| Not turned on | The human at this PC has not turned on "let this machine's agent speak" for this room (**off by default**). **Do not retry by voice; use `chat_send`** |
| No call | No call with video and sound, or nobody else in it |
| Failed: … | The voice could not be made or played (with the reason) |

- **The voice is synthesized on this PC** (`say` on macOS, System.Speech on Windows). The text is never sent to an outside speech service. Other systems return "cannot synthesize"
- **`text` is limited to 500 characters**; this is not for reading long documents aloud
- The human's switch applies **only to the room where it was turned on**, and **turns off when they leave the room**
- While it speaks, the voice replaces the microphone on the outgoing audio (the microphone comes back afterwards)

### The desk is never exposed to the network

It is a same-machine-only endpoint. On Unix the socket is `0600`, so **another user on the same machine cannot reach it either.**

### Not there yet

- **Conversation history is not kept.** Closing clears it (`issues/010`)
- **The inbox is empty.** `inbox_*` answers "nothing" until `issues/011` is decided
- **Mail cannot be sent.** There is no send path at all yet
