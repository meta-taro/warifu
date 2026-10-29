# 受け取って入れる（**アルファ**）

> **English:** [install.md](install.md)

配布物は [ダウンロードのページ](https://meta-taro.github.io/warifu/) と
[GitHub Releases](https://github.com/meta-taro/warifu/releases) に置いてあります。いまの版は **v0.1.41**。

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
./warifu version          # → warifu 0.1.41+<commit>
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
| **`warifu_0.1.41_x64-setup.exe`** | **これを使ってください。****利用者ごとに入る**ので管理者権限が要りません |
| `warifu_0.1.41_x64_en-US.msi` | 中身は同じですが**機械ぜんぶに入れる形**で、管理者が居ないと `Error 1406` / `Error 1925` で止まります。**配布の仕組み（Intune・GPO）で配る人向け** |
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
gh release download v0.1.41 -p 'warifu_*_x64-setup.exe'
Get-Item .\warifu_0.1.41_x64-setup.exe -Stream Zone.Identifier   # → 見つかりません
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

### ウイルス対策ソフトに止められたら（Norton：`Access is denied`）

ウイルス対策ソフトによっては（Norton で報告あり・#42）、**ダウンロードした** Windows 版の起動を止めます。
署名が無いためです。

**画面版は、Norton が窓を出して止めることがあります**（2026-09-30・v0.1.20 のインストーラーで実測）——

> ノートンが**詳しく調査する必要があります**

**［開く］を押すと起動します**（窓が出ないまま 30 秒ほど何も起きないこともあります。そのときは待つか、
通知領域の Norton から自動保護を 10 分止めてから起動してください）。

**除外を入れても、起動して 1〜2 分で消える／エクスプローラーから開くと「アクセスが拒否されました」になるとき**は、
Norton の［**ノートン製品を修復する**］を実行してください（2026-10-21・v0.1.28 で実測。修復のあと、同じ exe がそのまま起動した）。
このとき割符の記録（`warifu.log`）には 1 行も残りません —— 割符のコードより手前で止められています。

**新しい版を入れた直後は、約 1 時間止められます**（2026-10-21・Norton 360 で 2 回実測。0.1.28・0.1.29 とも、入れてから約 65 分で人の手なしに通った）。
その間は「アクセスが拒否されました」になるか、Norton の囲い（`C:\Norton sandbox\…`）の中で動かされます。
通知も履歴の行も出ず、フォルダーの除外・ワイルドカード・再起動では早まりませんでした。
アプリの中からの更新も、この間はインストーラーが囲いの中で落ちて通りません。**新しい版を入れたら、約 1 時間おいてから起動してください。**

**同じ機械でソースから建てたものは止められませんでした**（2026-09-28 に確認・CLI のみ）。

```powershell
git clone https://github.com/meta-taro/warifu.git
cd warifu
cargo build --release -p warifu-cli     # → target\release\warifu.exe
```

Rust と Visual Studio Build Tools が要ります。**ARM64 の Windows では、LLVM（clang）も要ります** ——
無いと、依存が `clang` を探したところで止まります（2026-09-28・Snapdragon の機械で確認）。

---

## 自前の中継を使う（別の網の相手と繋ぐとき）

直接つながれない相手（自宅と事務所など）とは、**自分たちで立てた中継**を予備の道にできます。
中継の URL とトークンは、立てた人から受け取ります（**公開の場に書かないこと**）。

```bash
# トークンは引数に書かない（シェルの履歴に残る）。標準入力で渡す
pbpaste | warifu relay-server set https://<中継の名前>/relay   # クリップボードにトークンを入れておく
warifu relay-server show    # URL と「トークンあり／なし」だけが出る
warifu relay-server clear   # 使うのをやめる
```

- 控えは身元と同じ置き場所の `relay-server.tsv`（本人だけが読める）。**画面版は次の起動から使います**
- 中継は、直接つながれないときだけ通ります。**中継の運用者には「誰がいつ誰に繋いだか」が見えます**（中身は見えません）
- CLI は `--relay` を付けたときに使います。環境変数 `WARIFU_RELAY_URL`・`WARIFU_RELAY_TOKEN` があれば、そちらが勝ちます

## 版を上げる

画面は起動時と、動いている間も 1 時間ごとに確かめて、自分で案内を出します（入れ替えは人が押します）。

**Windows では、入れ替える前に Claude Code（`warifu mcp`）を全部止めてください。**
古い `warifu mcp` が机のパイプ（`\\.\pipe\warifu-desk`）を掴んだままだと、新しい画面版が
「この機械を開けませんでした: アクセスが拒否されました (os error 5)」で開けません（0.1.41 で実測）。
止めてから入れ替え、入れ替えたら Claude Code を立ち上げ直してください。
記録は `%USERPROFILE%\.local\share\warifu\warifu.log` にあります（macOS は `~/Library/Application Support/warifu/warifu.log`）。

**CLI を入れ替えたら、動いている `warifu mcp` も立て直してください。**動いているプロセスは古い版のままです。
Claude Code の口は `/mcp` の繋ぎ直しで入れ替わりますが、**自分で立てた常駐**（`warifu agent`・エージェントが
Bash から立てた聞き役のスクリプトなど）はそれでは入れ替わらないので、止めて立て直します。

**`/mcp` の繋ぎ直しでも、古い `warifu mcp` が残ることがあります**（2026-10-28 に macOS で見ました。古い口が同じ親の子として残り、
画面の「この PC」に同じ席が「〜のエージェント 2」と二重に出ます）。残っていたら、`pgrep -fl 'warifu mcp'`（Windows は
`tasklist | findstr warifu`）で数えて、古いほうを止めてください。0.1.41 からは、古い口が抜けると「2」は元の名に戻ります。
どれが古いままかは `ps -axo pid,ppid,lstart,command | grep '[w]arifu mcp'` で起動時刻を見れば分かります
（Windows は `tasklist | findstr warifu`）。

**CLI は別のバイナリで、画面と一緒には上がりません。**画面を上げたら CLI も入れ替えてください ——
**v0.1.3 より前は、画面と CLI が別の身元を名乗っていました。**
いまは `warifu id` が自分で突き合わせて言います。

```
warifu: **この機械には身元が 2 つあります。**
warifu:   いま名乗る身元  S246YYBLEHTP…  /tmp/…
warifu:   画面が使う身元  PEERAAAAAAAA…  /Users/…/Library/Application Support/warifu
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
