# Running a mailbox (`warifu relay`)

> 日本語版: [relay.ja.md](relay.ja.md)

> **This is not a central service warifu provides.** Whoever deploys warifu runs it (`.claude/decisions.md` **D68** / **D71**).
> warifu works without one — messages then only arrive **while the other side is running**.

---

## What it is for

warifu runs no servers, so **if the other side is not running, what you typed is gone.**

```
   A ──sealed──▶ mailbox (cannot read it)
                     │
   B starts ─────────┘ takes it; the mailbox no longer has it
```

The mailbox holds the message **sealed**. It has no key to open it, so **it cannot read it** (**D69**).

| | |
|---|---|
| Held for | **7 days**, then dropped |
| Per person | **200 messages**. **One person filling up does not lock anyone else out** |
| Per message | **64 KB** |
| Once handed over | **It lets go.** This is not a place to accumulate things |

---

## 1. Write down who may use it (**a human writes this**)

**One public key per line.** Lines starting with `#` are notes.

```text
# the three of us on the dev team
KKD7IOAKQBSUE6LOTX3HYKFZI7OEEVIWZYI3RK5ZVNOP4MT4ZTXA
SSYWJBFMA2ASFD4TJOHZWM2OQ6GRSU2E5NQSDU35YPACENJLKBOA
```

Each person gets their own public key from `warifu id` on their machine. **It is not a secret** — handing it out is fine.

A line that cannot be read is dropped **on its own**. This avoids the two failure modes where **one typo lets everyone in, or locks everyone out**.

---

## 2. Start it

**You need the CLI (`warifu`).** It is not inside the app (`.dmg` / `.msi`) — it is the separate `warifu` / `warifu.exe` in the same release ([`install.md`](install.md)). On macOS the CLI has shipped since **v0.1.0-alpha.10**.

```bash
warifu relay --allow-file ~/warifu-allow.txt
```

```text
WARIFU1-K5JEMQIDGMPXZIZCTK4GJRUJXTWOWSTU…      ← stdout. Hand this out
warifu relay: up (2 people allowed). Hand out the address above.
Held for 7 days. The contents cannot be read.
```

**It refuses to start with an empty list.**

```text
warifu: no list of permitted people. Write one public key per line in --allow-file <path>
```

This is not there to be inconvenient. **A mailbox that starts empty carries traffic for people its operator does not know.**

---

## 3. Prove it works, on one machine (`warifu post`)

**You do not need two machines to test the round trip.**

```bash
# pretend the other side (B) is not running yet
echo "Leaving this while you are away." | warifu post put --at <mailbox address> --to <B's public key>
→ handed over (74 bytes)

# B starts up
warifu post take --at <mailbox address>
→ 1 message
  2026-09-08 12:04Z   someone we do not know (4ODAS23E…)   Leaving this while you are away.

# again
warifu post take --at <mailbox address>
→ 0 messages            ← **once handed over, it lets go**
```

| | |
|---|---|
| **The body comes from stdin** | As an argument it would stay in your shell history and in `ps` |
| **`--to` takes a name or a key** | A name works if the person is in `warifu contacts` |
| **Times are `Z` (UTC)** | **The sender's clock.** Shown in local time, a message from another timezone reads as if it arrived from the future |
| **The sender is who the signature says** | Not a claimed name (**D72**) |

**Testing on one machine means separating `WARIFU_HOME`** — with one identity you would be messaging yourself.

```bash
WARIFU_HOME=/tmp/w-a warifu id     # A's public key
WARIFU_HOME=/tmp/w-b warifu id     # B's public key — put both in --allow-file
```

---

## 4. It is fine for it to go down

- **Sealed messages live only in memory** and are gone when it closes. This is not the place for anything you cannot lose — same idea as dropping them after 7 days
- To keep one up permanently, hand it to `launchd` / `systemd` / Task Scheduler

---

## Taking messages from another network (`--relay`, **D78**)

**By default it only talks to peers on the same network.** For someone on another network to leave a message, **both the mailbox and the sender pass `--relay`.**

```bash
warifu relay --allow-file ~/warifu-allow.txt --relay
echo "..." | warifu post put --at <mailbox> --to <peer> --relay
warifu post take --at <mailbox> --relay
```

**With it, who connected to whom and when is visible to whoever runs the relay.** The contents are not (they stay sealed).

**Measured 2026-09-09** — the outside address did become visible, but **the relay itself never entered the path.** `warifu doctor --relay` reports what is actually true at that moment. **Whether two different networks actually connect has never been verified by anyone.**

---

## Things to know

- **A human writes the list.** warifu never adds to it (same stance as **D56**)
- **Someone not on the list is dropped silently.** No reason is returned (**D31**). It is recorded locally
- **The mailbox does not know who sent what to whom.** Each message is sealed with a single-use key and carries no sender
- **Running one on a machine exposed to the internet** moves it closer to "carrying traffic for the general public". Some countries require registration for that. **Using it among yourselves does not** (**D68**)
