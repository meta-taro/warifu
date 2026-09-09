# 配布物に署名する（macOS・Developer ID）

> **秘密の値はこの文書に 1 つも書きません。**何を、どこで作って、どの名前で入れるか、だけです。
> 作成と投入は**人が行います**（`.claude/rules/product-baseline.md` §14）。

---

## なぜ要るか

署名していない配布物は、**macOS から見て毎回「別のアプリ」**になります。

```
alpha.12 の .dmg   Identifier = warifu_desktop-5b83332214043146
alpha.13 の .dmg   Identifier = warifu_desktop-a66d64ca12308d6c   ← 同じ commit でも違う
手元で建てたもの     Identifier = warifu_desktop-bd90e0f5d0b07300
```

そのため、**更新するたびに「ローカルネットワークを許可しますか」を聞き直されます。**
前に許した分は引き継がれません（2026-09-09 に実測）。

**Developer ID で署名すると、識別子が固定され、許可が続きます。**
受け取った人が「壊れているため開けません」を見ることもなくなります。

---

## fork した人・PR を送る人はどうなるか

**何も変わりません。**署名は「プロジェクト」ではなく「配る人」に紐づきます。

| | どうなるか |
|---|---|
| **手元で建てて自分で使う** | ad-hoc のまま建ちます。自分で建てたものなので Gatekeeper も出ません |
| **fork して自分の名前で配る** | **自分の Developer ID が要ります。**`release.yml` は材料が無ければ**署名せずに作り、警告を出します**（落としません）。**識別子（`app.warifu.desktop`）も自分のものに変えてください** —— 変えないと、受け取る側が取り違えます |
| **PR を送る** | 証明書に触れません。`release` はタグと手動でしか走らないので、**fork の PR に Secrets は渡りません**（GitHub 側の既定でもそうです） |

---

## 1 回だけの用意（人が行う）

### 1. Developer ID Application 証明書を作る

1. **キーチェーンアクセス** ▸ 証明書アシスタント ▸ **認証局に証明書を要求…**
   - 通称は**用途が分かる名前**にする（他の証明書と鍵が混ざらないように）
   - **「ディスクに保存」＋「鍵ペア情報を指定」**（2048 / RSA）
   - **iOS 用など、既にある CSR を使い回さない。**片方が漏れたら両方やり直しになります
2. `developer.apple.com` ▸ Certificates ▸ **＋** ▸ **Developer ID Application**
   - **Profile Type は G2 Sub-CA** を選ぶ（`Previous Sub-CA` は 2027-02-01 で切れます）
   - さきほどの CSR を上げる
3. 落ちてきた `.cer` をダブルクリックしてキーチェーンに入れる

**確かめ方**（秘密は出ません）

```bash
security find-identity -v -p codesigning
#   → "Developer ID Application: <組織名> (<TEAM_ID>)"  が valid で出ること
```

> キーチェーンアクセスの一覧が空に見えることがあります。**⌘Q で閉じて開き直す**と出ます
> （開いたまま外から足すと、一覧が読み直されないため）。

### 2. `.p12` に書き出す

**その 1 本だけ**を右クリック ▸ 書き出す ▸ **個人情報交換 (.p12)**。**パスワードを付ける。**

> **`security export` で一括に書き出さないこと。**手元の全部の秘密鍵が 1 つの `.p12` に入り、
> **関係のない鍵まで Secrets に載ります。**

**秘密鍵はこの Mac の中にしかありません。**`.p12` を失うと、同じ証明書では二度と署名できません。
パスワード付きの控えを、安全な所に取っておいてください。

### 3. 公証用の App 用パスワードを作る

`account.apple.com` ▸ サインインとセキュリティ ▸ **App 用パスワード**。

- **Apple Account のログインパスワードではありません**（`xxxx-xxxx-xxxx-xxxx` の形）
- 最初に聞かれるのは**名札**です（何に使うかの名前。パスワードはそのあとに出ます）
- **表示はその一度きり。**閉じたら作り直し（古いほうは消してよい）

### 4. GitHub の Secrets へ入れる

`Settings ▸ Secrets and variables ▸ Actions` に、**この 6 つの名前**で入れます。

| 名前 | 中身 |
|---|---|
| `APPLE_SIGNING_IDENTITY` | `security find-identity` に出た名前そのまま（**秘密ではありません**） |
| `APPLE_CERTIFICATE` | `.p12` を base64 にしたもの |
| `APPLE_CERTIFICATE_PASSWORD` | ②で付けたパスワード |
| `APPLE_ID` | 開発者アカウントのメール |
| `APPLE_PASSWORD` | ③の **App 用パスワード** |
| `APPLE_TEAM_ID` | チーム ID（**秘密ではありません**） |

**base64 は画面に出さずクリップボードへ入れます。**

```bash
base64 -i <.p12 の場所> | tr -d '\n' | pbcopy
```

**貼り終わったら、別のものをコピーしてクリップボードを流してください。**

---

## 効いているかの確かめ方

タグを打つと `release` が走ります。**「署名したつもり」で配らない**ために、
ワークフローの中で実物を検めています。

```bash
codesign -dv <warifu.app>          # Signature=adhoc ではなくなること／TeamIdentifier が出ること
codesign --verify --deep --strict <warifu.app>
xcrun stapler validate <warifu.app>   # 公証が貼られていること
```

**材料が揃っていなければ、署名せずに作ります。**ただし黙って作らず、
**「配れない .dmg ができた」と分かる形で警告を出します**（`release.yml`）。

---

## 期限

- **証明書は 5 年**で切れます。切れる前に作り直して Secrets を入れ替えてください
- **App 用パスワード**は、Apple Account のパスワードを変えると無効になります
