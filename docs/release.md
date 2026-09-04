# 配る手順（macOS・`.dmg`）

> **ここに秘密情報を書かない。**証明書・パスワード・API キーは**人が入れる**（product-baseline §14）。
> この文書に載っているのは**変数の名前と手順だけ**である。

---

## 何が要るか

ダウンロードページに `.dmg` を置く場合、**署名と公証の両方が要る。**
片方でも欠けると、**落とした人の Mac は「壊れている」と言って開かない。**

| | 何 | 誰が |
|---|---|---|
| 1 | **Developer ID Application** の証明書 | **人**（Apple の開発者ポータルで発行） |
| 2 | 公証（notarize）のための Apple ID と **app 用パスワード**、Team ID | **人** |
| 3 | `.dmg` を作る・署名する・公証する・ステープルする | 自動（`release.yml`） |

**「Apple Distribution」では配れない。**あれは App Store / TestFlight 用である。
**ダウンロード配布には Developer ID Application が要る。**名前が似ているので取り違えやすい。

---

## 1. 証明書を作る（人）

Apple の開発者ポータル → Certificates → **Developer ID Application** を作る。
できたものをキーチェーンへ入れ、**`.p12` として書き出す**（パスワードを付ける）。

```bash
security find-identity -v -p codesigning   # 出てくることを確かめる
```

---

## 2. 公証の材料を用意する（人）

- **Apple ID**（開発者アカウントのもの）
- **app 用パスワード** — Apple ID の管理画面で作る。**通常のパスワードではない**
- **Team ID** — 開発者ポータルで確認できる

---

## 3. GitHub の Secrets へ入れる（人）

リポジトリの Settings → Secrets and variables → Actions。**名前は下のとおり。**

| 名前 | 中身 |
|---|---|
| `APPLE_SIGNING_IDENTITY` | `Developer ID Application: …` の文字列 |
| `APPLE_CERTIFICATE` | `.p12` を base64 にしたもの |
| `APPLE_CERTIFICATE_PASSWORD` | その `.p12` のパスワード |
| `APPLE_ID` | Apple ID |
| `APPLE_PASSWORD` | **app 用パスワード** |
| `APPLE_TEAM_ID` | Team ID |

**AI はここに触らない。**値を作らず、置かず、ログにも書き写さない。

---

## 4. 作る

タグを打つか、Actions から `release` を手で起動する。

```bash
git tag v0.1.0-alpha && git push origin v0.1.0-alpha
```

**材料が揃っていなければ、署名せずに作る。**ただし**黙って作らない** —
「配れない `.dmg` ができた」と警告が出る。**それを配らないこと。**

出来たものは Actions の成果物（`warifu-macos-dmg`）から落とせる。

---

## 5. 確かめる（**落とす側と同じ経路で**）

**作った本人の Mac では、署名が壊れていても開けてしまうことがある。**
必ず**別の Mac へダウンロードして**開く。

```bash
spctl -a -vv -t install warifu.dmg    # accepted と出ること
xcrun stapler validate warifu.dmg     # 公証が貼られていること
```

---

## まだ無いもの

| | |
|---|---|
| **Windows** | **建たない**（`decisions.md` **D40**）。上流の依存衝突 |
| **自動更新の署名鍵** | minisign の鍵が未生成（**D36**）。鍵ができるまで更新は配れない |
| **Developer ID の証明書** | **持っていない**（2026-09-04 に実測。あるのは Distribution と Development だけ） |
