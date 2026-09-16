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
| `chat.send` | `chat_send` | Put one line into the conversation |
| `chat.read` | `chat_read` / **`chat_wait`** | Lines that arrived. **`chat_wait` blocks until something arrives** |
| `profile.write` | `profile_set` | Set **your own agent's** display name and one-line bio |
| (same as `chat.read`) | `chat_status` / **`room_status`** | How far a line of yours got; what the room looks like right now |
| `inbox.list` | `inbox_list` | Inbox metadata only (no bodies) |
| `inbox.open.summary` / `.structured` / `.raw` / `.attachments` | `inbox_open` | Progressively more of a message |
| `calendar.freebusy` | `calendar_slots` | Free slots only (never the titles) |
| `rules.list` | `rules_list` | Approved read rules |

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
warifu agent --as souta --on ./run.sh
```

**Now a separate Claude process answers what a human typed.** Verified end to end on 2026-09-11 (window → agent → window, including the window being restarted), and on 2026-09-12 it answered a **remote** human with nobody pressing anything.

> **warifu cannot wake up an already-running Claude Code session.** That is Claude Code's side. What it can do is **start a new one** (`--on` with `claude -p`) or **hand it over the next time that session reads.**

### Seeing the current state (`room_status`)

**You can check whether a person answered, without asking them.**

```
room_status →
  ルーム: AFUF2T4ECVKMFP4L4GPO2E56LU
  相手: EKBN2GCQO35WAMMOF7EHBN5SKV6Q3JL7BTEGCTSQ3IA6AAWQHAHA
  経路: direct
  この機械のエージェント: souta のエージェント
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

### The desk is never exposed to the network

It is a same-machine-only endpoint. On Unix the socket is `0600`, so **another user on the same machine cannot reach it either.**

### Not there yet

- **Conversation history is not kept.** Closing clears it (`issues/010`)
- **The inbox is empty.** `inbox_*` answers "nothing" until `issues/011` is decided
- **Mail cannot be sent.** There is no send path at all yet
