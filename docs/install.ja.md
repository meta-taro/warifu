# 受け取って入れる（**アルファ**）

> **English:** [install.md](install.md)

配布物は [ダウンロードのページ](https://meta-taro.github.io/warifu/) と
[GitHub Releases](https://github.com/meta-taro/warifu/releases) に置いてあります。いまの版は **v0.1.12**。

**何かを預ける前に、[`SECURITY.ja.md`](../SECURITY.ja.md) を読んでください。**アルファ版です。

---

## 動く機械

| | |
|---|---|
| **macOS / Apple Silicon（M1 以降）** | **動きます。**配布物は `aarch64`。**画面も CLI も署名・公証済み** |
| **macOS / Intel** | **配布物はありません。**その機械で建ててください（下記） |
| **Windows（x64）** | **動きます**（映像も音声も）。**署名していません**。**初回はファイアウォールの規則が要ります**（下記） |
| **Linux** | **確かめていません。**建ちはしますが、通した人が居ません |

---

## macOS

### 画面

`.dmg` を開いて `warifu.app` を「アプリケーション」へ入れ、ダブルクリック。
**回避の操作は要りません** —— 署名して公証してあります（`v0.1.0-alpha.15` 以降）。

自分で確かめるなら、**受け取る人と同じ目で見る道具**があります。

```bash
spctl -a -vv -t exec /Applications/warifu.app
#   → accepted / source=Notarized Developer ID
```

### CLI（`warifu`）

`.dmg` は**画面だけ**です。**預かり所（`warifu relay`）・エージェントの口（`warifu mcp`）・
常駐（`warifu agent`）は CLI 側**にあり、同じリリースに別ファイルで入っています。

```bash
chmod +x ./warifu
./warifu version          # → warifu 0.1.12
mkdir -p ~/bin && mv ./warifu ~/bin/     # ~/bin が PATH に入っていれば
```

**v0.1.5 から CLI も署名・公証済みなので、`xattr` は要りません。**
落としたばかりのものに隔離属性を付けて動かして確かめました。

> **v0.1.4 までは、そうではありませんでした。**しかも壊れ方が悪く、
> **出力なしで固まったうえ、ファイルが消えました**（macOS が処分します）。
> 古い版を使うなら、**初めて実行する前に** `xattr -d com.apple.quarantine ./warifu`。

**入れ替えるときは、消してから置いてください。**上書きすると `Killed: 9` になります ——
macOS が**そのパスの署名を覚えている**ためです。

```bash
rm -f ~/bin/warifu && cp ./warifu ~/bin/warifu && chmod +x ~/bin/warifu
```

### Intel の Mac

その機械の上で建ててください。

```bash
git clone https://github.com/meta-taro/warifu.git
cd warifu/apps/desktop
pnpm install --frozen-lockfile
pnpm tauri build --bundles app
```

---

## Windows

### `.msi` ではなく `-setup.exe` を使う

| ファイル | |
|---|---|
| **`warifu_0.1.12_x64-setup.exe`** | **これを使ってください。****利用者ごとに入る**ので管理者権限が要りません |
| `warifu_0.1.12_x64_en-US.msi` | 中身は同じですが**機械ぜんぶに入れる形**で、管理者が居ないと `Error 1406` / `Error 1925` で止まります。**配布の仕組み（Intune・GPO）で配る人向け** |
| `warifu.exe` | CLI。置いた場所から、そのまま打てます |

### SmartScreen が出ます

**Windows は署名していません。**こう出ます。

```
Windows によって PC が保護されました
```

**壊れてはいません。****［詳細情報］→［実行］**を 1 回。
**出どころが分かっているときだけ**にしてください。

**落とし方で変わります**（2026-09-16 実測）—— ブラウザで落とすと `Zone.Identifier` が付き、
**それが SmartScreen の引き金**です。`gh release download` では付かないので、**警告そのものが出ません**
（**署名が無いのは、どちらでも同じ**です）。

```powershell
gh release download v0.1.12 -p 'warifu_*_x64-setup.exe'
Get-Item .\warifu_0.1.12_x64-setup.exe -Stream Zone.Identifier   # → 見つかりません
```

### **初回はファイアウォールの規則が要ります**（ここで詰まります）

入れても `warifu-desktop.exe` の規則は作られず、**環境によっては確認の窓も出ません。**
症状がいちばん厄介で、**文字は届くのに映像だけ乗りません** ——
言い換えると、**コマンドは繋がるのに画面だけ繋がりません。**

**v0.1.7 から、画面が名指しで言い、打つものを渡します。**自分でやるなら、
**管理者の PowerShell** で ——

```powershell
New-NetFirewallRule -DisplayName "warifu (in)"  -Direction Inbound  -Program "$env:LOCALAPPDATA\warifu\warifu-desktop.exe" -Action Allow
New-NetFirewallRule -DisplayName "warifu (out)" -Direction Outbound -Program "$env:LOCALAPPDATA\warifu\warifu-desktop.exe" -Action Allow
```

**いま在るかを見る**（管理者でなくても打てます）——

```powershell
Get-NetFirewallApplicationFilter | Where-Object Program -like "*warifu*" | ForEach-Object { $_.Program }
```

**規則はパスに紐づくので、版を上げても残ります。**足し直す必要はありません。
**そもそも窓が出る環境かどうか**は ——

```powershell
Get-NetFirewallProfile | Select-Object Name, NotifyOnListen
```

`NotifyOnListen: False` なら**窓は出ません** —— 管理者が足すしかありません。

`warifu.exe doctor` は **CLI と画面を別々に**報告します。
「コマンドには規則がある」が「画面には無い」を隠せなくなりました。

### CLI だけ使う

```
warifu.exe host --keys 2      ルームキーが 2 本出る（1 本につき 1 人）
warifu.exe join <ルームキー>  もらった鍵で入る
```

打った行が相手へ飛び、届いた行がそのまま出ます。`/key` でもう 1 本出ます
（`/key` そのものを送りたいときは、頭に空白を 1 つ）。

---

## 版を上げる

画面は起動時に確かめて、自分で案内を出します。

**CLI は別のバイナリで、画面と一緒には上がりません。**画面を上げたら CLI も入れ替えてください ——
**v0.1.3 より前は、画面と CLI が別の身元を名乗っていました。**
いまは `warifu id` が自分で突き合わせて言います。

```
warifu: **この機械には身元が 2 つあります。**
warifu:   いま名乗る身元  S246YYBLEHTP…  /tmp/…
warifu:   画面が使う身元  I3ILQUUJQHVS…  /Users/…/Library/Application Support/warifu
```

---

## タグを待たずに `develop` を試す

`Actions` → `bundle` → 最新の実行 → Artifacts。

| Artifact | 中身 |
|---|---|
| `warifu-macos-latest-develop` | `warifu`（CLI）＋ `warifu-desktop`（画面の実行ファイル・**インストーラではない**） |
| `warifu-windows-latest-develop` | 同じもの（Windows 版） |

**7 日で消えます。**配るためではなく、**試すため**のものです。
インストーラが要るなら、タグの成果物を使ってください。

---

## `warifu://` のリンクについて 1 つ

ルームキーはリンクでも渡せます（`warifu://join/…`）。
**押すと「入りますか？」と尋ねます。勝手には入りません。**

**受け取る側が、一度アプリを開いておく必要があります** ——
OS が `warifu://` を誰の持ち物か覚えるためです。一度も開いていないと、
押しても何も起きません（Windows はインストーラが登録します）。
