# Installing (**alpha**)

> 日本語版: [install.ja.md](install.ja.md)

Builds live on the [download page](https://meta-taro.github.io/warifu/) and in [GitHub Releases](https://github.com/meta-taro/warifu/releases). Current version: **v0.1.19**.

**Read [`SECURITY.md`](../SECURITY.md) first if you are planning to rely on this for anything.** It is an alpha.

---

## What runs

| | |
|---|---|
| **macOS / Apple Silicon (M1 or later)** | **Works.** Builds are `aarch64`. App and CLI are **signed and notarized** |
| **macOS / Intel** | **No build shipped.** Build it locally (below) |
| **Windows (x64)** | **Works**, video and audio included. **Not code-signed**, and **first run needs a firewall rule** (below) |
| **Linux** | **Not verified.** It compiles; nobody has run it end to end |

---

## macOS

### The app

Open the `.dmg`, drag `warifu.app` into Applications, double-click. **No workaround needed** — it is signed and notarized (since v0.1.0-alpha.15).

Check it yourself, with the same tool your OS uses:

```bash
spctl -a -vv -t exec /Applications/warifu.app
#   → accepted / source=Notarized Developer ID
```

### The CLI (`warifu`)

The `.dmg` contains **the app only**. The mailbox (`warifu relay`), agent endpoint (`warifu mcp`) and resident agent (`warifu agent`) are in the **CLI**, which is a separate download in the same release.

```bash
chmod +x ./warifu
./warifu version          # → warifu 0.1.19
mkdir -p ~/bin && mv ./warifu ~/bin/     # if ~/bin is on your PATH
```

**From v0.1.5 the CLI is signed *and notarized*, so no `xattr` dance is required.** We verified that by putting the quarantine flag on a fresh download and running it.

> **Up to v0.1.4 this was not true**, and it failed in the worst way: the binary hung with no output **and then disappeared** — macOS removed it. If you are on an older build, `xattr -d com.apple.quarantine ./warifu` **before** the first run.

**Replacing an existing CLI: delete it, then copy.** Overwriting in place gives you `Killed: 9`, because macOS has the old signature cached for that path.

```bash
rm -f ~/bin/warifu && cp ./warifu ~/bin/warifu && chmod +x ~/bin/warifu
```

### Intel Macs

Build on the machine:

```bash
git clone https://github.com/meta-taro/warifu.git
cd warifu/apps/desktop
pnpm install --frozen-lockfile
pnpm tauri build --bundles app
```

---

## Windows

### Use `-setup.exe`, not `.msi`

| File | |
|---|---|
| **`warifu_0.1.19_x64-setup.exe`** | **Use this.** Installs **per user**, so no administrator is needed |
| `warifu_0.1.19_x64_en-US.msi` | Same contents, but **machine-wide** — it needs an administrator and fails with `Error 1406` / `Error 1925` without one. For deployment tooling (Intune, GPO) |
| `warifu.exe` | The CLI. Runs from wherever you put it |

### SmartScreen will warn

**Windows is not code-signed yet.** You will see:

```
Windows によって PC が保護されました
```

**It is not broken.** Press **More info** → **Run anyway**, once. Do this only if you know where the file came from.

**How you download it changes this** (measured 2026-09-16): a browser marks the file with `Zone.Identifier`, which is what triggers SmartScreen. `gh release download` does not mark it, so **the warning never appears** — the file is still unsigned either way.

```powershell
gh release download v0.1.19 -p 'warifu_*_x64-setup.exe'
Get-Item .\warifu_0.1.19_x64-setup.exe -Stream Zone.Identifier   # → not found
```

### **First run needs a firewall rule** (this one bites)

Installing does **not** create a rule for `warifu-desktop.exe`, and in many environments **no prompt appears either**. The symptom is the hardest kind to diagnose: **text arrives, video never starts** — or, put differently, **the CLI connects and the GUI does not.**

Since v0.1.7 **the app tells you this by name and hands you the command**. To do it yourself, in an **administrator** PowerShell:

```powershell
New-NetFirewallRule -DisplayName "warifu (in)"  -Direction Inbound  -Program "$env:LOCALAPPDATA\warifu\warifu-desktop.exe" -Action Allow
New-NetFirewallRule -DisplayName "warifu (out)" -Direction Outbound -Program "$env:LOCALAPPDATA\warifu\warifu-desktop.exe" -Action Allow
```

Check what exists (no administrator needed):

```powershell
Get-NetFirewallApplicationFilter | Where-Object Program -like "*warifu*" | ForEach-Object { $_.Program }
```

**Rules are bound to the path, so they survive an upgrade.** You do not need to add them again after updating. Whether a prompt would appear at all:

```powershell
Get-NetFirewallProfile | Select-Object Name, NotifyOnListen
```

`NotifyOnListen: False` means **you will never see a prompt** — an administrator has to add the rule.

`warifu.exe doctor` reports the CLI and the GUI **separately**, so "the command has rules" can no longer hide "the window does not".

### CLI only

```
warifu.exe host --keys 2      prints 2 room keys (one key = one person)
warifu.exe join <room key>    join with a key you were given
```

Lines you type go to the other side; lines that arrive are printed. Type `/key` to issue another key (prefix a space if you want to *send* the text `/key`).

---

## Updating

The app checks on startup and offers the update itself.

**The CLI is a separate binary and does not update with the app.** After updating the app, replace the CLI too — otherwise you are running two different versions, and before v0.1.3 that meant **the app and the CLI claimed different identities**. `warifu id` now checks this for you and says so:

```
warifu: **この機械には身元が 2 つあります。**
warifu:   いま名乗る身元  S246YYBLEHTP…  /tmp/…
warifu:   画面が使う身元  I3ILQUUJQHVS…  /Users/…/Library/Application Support/warifu
```

---

## Trying `develop` without waiting for a tag

`Actions` → `bundle` → latest run → Artifacts.

| Artifact | Contents |
|---|---|
| `warifu-macos-latest-develop` | `warifu` (CLI) + `warifu-desktop` (app binary, **not an installer**) |
| `warifu-windows-latest-develop` | the same, for Windows |

**These expire after 7 days** and are for trying, not for distributing. If you need an installer, use a tagged release.

---

## One thing about `warifu://` links

A room key can be handed over as a link (`warifu://join/…`). **Pressing it asks whether you want to join — it never joins on its own.**

**The receiving side has to have opened the app at least once**, so the OS knows who owns `warifu://`. If it has never been opened, pressing the link does nothing. (On Windows, the installer registers it.)

---

## First three steps, once it runs

Two machines, **on the same Wi-Fi** (across networks has never been measured).

### 1. On machine A: create a room and issue a key

Contacts → **Room** → **［このルームに人を呼ぶ］** (invite someone to this room).
You get **one key per person** — a link, a QR code, or the raw text. Hand it over by any means you like: another chat app, a phone call, a printed page.

**One key admits one person.** For a second person, issue a second key. **Earlier keys stay valid.** Keys expire after 24 hours.

### 2. On machine B: paste the key

Contacts → **Room** → the panel at the bottom, **「もらったルームキーでルームに入る」** (join with a room key you were given) → paste → **［ルームに入る］**.

If you were given a `warifu://join/…` link, pressing it works too — **it asks before joining.** Note that machine B has to have opened the app at least once for the link to be recognised.

### 3. Talk, then add video if you want it

The room is **text only** to start with. Nothing touches your camera until you press **［ビデオ会議を始める］** (start a video meeting), and pressing it **adds video to the room you are already in** — it does not create a new one. Turning it off **does not leave the room**.

### What should be true

| | |
|---|---|
| The roster says `2 / 12` | both of you are in |
| The route says `direct` | you are connected without a relay |
| A line typed on A appears on B | and is marked **送信済み** (sent) |

If the route stays `unknown` for more than about 15 seconds, the app will say what it suspects — including, on Windows, whether the firewall rule is actually missing.

### Then: put an agent in the room

```bash
warifu setup
```

See [`mcp.md`](mcp.md). **An agent in the room is the part of this that has no obvious equivalent** — it can read and write the same conversation a human is looking at, and it is labelled so nobody mistakes it for a person.
