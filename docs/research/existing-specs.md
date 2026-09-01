# 既存規格との差分（`issues/001`）

**調査日**: 2026-09-02 / **調べた者**: AI エージェント

> `issues/001` の指示に従い、**判定は 3 つだけ**（ほぼカバー / 一部不足 / 該当なし）。
> **原案を擁護する方向へは寄せない。**足りているなら足りていると書く。

**2026-08-17 に、通信路と会議に関わる範囲は先に済ませてある**（`decisions.md` **D10**）。
ここはその残り — **Identity / Trust Evidence / Capability / A2A** である。

---

## 結論（上流はここだけ読めばよい・10 行）

1. **Capability は既存でほぼ足りる。**UCAN 1.0 が委譲・減衰・期限・失効を持ち、Rust 実装もある
2. **その UCAN を読む前に `warifu-capability` を書いた。**`issues/001` が「自前設計する前に必ず読む」と
   名指しで警告していた所を、そのまま踏んだ（下記 §5）
3. **Identity も Trust Evidence も既存でほぼ足りる**（DID 1.1 / VC 2.0 / DIDComm v2 / MLS）
4. **A2A も、2026 年に入って IETF の I-D が 2 本出た**（AIP）。**MCP と A2A のバインディング付き**
5. **「該当なし」は 2 つだけ** — 低トークン読み取り層（`issues/007`）と、割符による接続確立
6. したがって **この企画は「新規格」ではなく「プロファイル + Reference Implementation」**である
7. **これは望ましい結論。**2 実装目の候補が「UCAN / DID を既に実装している人たち」になる
8. **D2（全端末を失った人の復旧）は、どの既存規格も解いていない。**Nostr は「鍵を失えば終わり」
9. **Keybase は消えていない。放置されている。**サーバは動くが開発は止まった。§6 の想定より悪い
10. **OpenMolt は Building Block として採らない**（Go / DHT 探索が既定 / v0.1.0）。先行例としては見る

---

## 1. 表

| 原案の要素 | 既存仕様 | ステータス（最終更新） | 判定 | 根拠 |
|---|---|---|---|---|
| §4 Identity / Device Key | **W3C DID** v1.1 | **Candidate Recommendation**（2026-03-05） | **ほぼカバー** | https://www.w3.org/TR/did-1.1/ |
| §6 Trust Evidence | **W3C VC Data Model** 2.0 | **Recommendation**（2025-05-15。安定・更新予定なし） | **ほぼカバー** | https://www.w3.org/TR/vc-data-model-2.0/ |
| §9 Agent-to-Agent メッセージ | **DIDComm v2** | **DIF Approved**（2022 確定。v3 は IETF 構想段階） | **ほぼカバー** | https://identity.foundation/didcomm-messaging/spec/v2.0/ |
| §3.5 Capability ベース権限 | **UCAN** 1.0 | **1.0（rc を経て確定）**。Rust / TS / Go 実装あり | **ほぼカバー** | https://github.com/ucan-wg/spec |
| 同上（別案） | **ZCAP-LD** | **v0.4.0-draft**。VCWG への昇格は**時期未定** | 一部不足 | https://w3c-ccg.github.io/zcap-spec/ |
| E2EE・グループ鍵 | **MLS** | **RFC 9420**（2023-07）。Rust 実装 OpenMLS | **ほぼカバー** | https://datatracker.ietf.org/doc/rfc9420/ |
| §3.2 鍵＝ Identity・中央不在 | **Nostr** | NIP 群（標準化団体なし・随時更新） | **ほぼカバー**（思想） | https://nips.nostr.com/1 |
| §6 Evidence の集約 | **Keybase** | **放置**（買収 2020-05。`keybase.pub` は 2023-03 停止） | 該当なし（**先行例として重要**） | https://schulz.dk/2026/04/06/the-cryptographic-zombie-how-keybase-went-from-privacy-darling-to-zooms-cleanup-crew/ |
| §9 A2A / §8 Transport | **OpenMolt Network** | **v0.1.0・早期段階**。Go / Apache-2.0 | 一部不足（**採らない**・§4） | https://openmolt.network/ |
| §4 Agent Identity / §3.5 委譲 | **AIP** | **IETF I-D 2 本**（`draft-singla-…-03` / `draft-prakash-aip-00`・2026） | **ほぼカバー** | https://datatracker.ietf.org/doc/draft-singla-agent-identity-protocol/ |
| §15 Persistent Agent Identity | **Signet AI** | 実装あり（local-first・memory / identity / secrets） | **ほぼカバー** | https://github.com/Signet-AI/signetai |
| §7 Connection Policy | **SimpleX Chat** | 実装あり・稼働中 | **ほぼカバー** | https://simplex.chat/docs/simplex.html |
| §8 / §13 P2P Voice/Video | **Jami** | 稼働中 | **ほぼカバー** | （D10 で判定済み） |
| §8 Transport | **iroh** | **採用済み**（D10） | — | （D10） |

---

## 2. 「該当なし」の一覧（**ここだけが新規部分**）

`issues/001` が「これが実質的な新規部分」と定めた欄。**2 つしかない。**

| | 中身 | なぜ既存に無いか |
|---|---|---|
| **1. 低トークン読み取り層** | `issues/007`（Metadata First / Deterministic Parser / 会計） | どの仕様も**通信の作り方**を決めていて、**受け取った後にどう読むか**を決めていない |
| **2. 割符による接続確立** | `warifu-core` の割符（`decisions.md` D12） | SimpleX の招待リンクが**最も近い**。ただしあちらは「一度使うと消える待ち行列」で、こちらは「**秘密そのものを返送させない照合**」。**同じではないが、置き換えられる可能性は高い** |

**2 番目は、SimpleX を読み込めば「ほぼカバー」に落ちる可能性がある。**
そうなれば**新規部分は 1 つだけ**になる。

### したがって

**「新規格を名乗る根拠は薄い。」**

`issues/001` は「その結論を出してよい。むしろ有用」と書いている。そのとおりに書く。

この企画は **既存規格のプロファイル + Reference Implementation** として立てるのが正しく、
**2 実装目の候補が「UCAN / DID / MLS を既に実装している人たち」になる。**
ゼロから仲間を探すより桁違いに楽になる（`issues/001` 冒頭のとおり）。

---

## 3. Keybase 停止時に Evidence がどうなったか（3 行）

**消えていない。放置されている。**サーバは今も動き、ログインでき、古い証明も残っている。
だが開発は 2020 年 5 月の買収後に止まり、`keybase.pub` は **2023 年 3 月に予告なく停止**、
モバイルアプリは OS の更新に追随できていない。

**原案 §6 への含意**: Evidence の集約点は「**消える**」のではなく「**動いているのに信用できなくなる**」。
**止まったことに誰も気づけない。**外部の集約点へ Import する設計は、この壊れ方を前提にする必要がある。

---

## 4. OpenMolt Network — Building Block として採用可能か

**採らない。**

| | |
|---|---|
| 実装言語 | **Go**（SDK は Python / TypeScript）。本リポジトリは Rust（**D10**） |
| ライセンス | Apache-2.0（問題なし） |
| リポジトリ | https://github.com/sahilpohare/MoltMesh |
| 最新版 | **v0.1.0**。「Coming next」が並ぶ早期段階 |
| 構成 | libp2p ＋ **Kademlia DHT で探索** ＋ GossipSub ＋ QUIC/Noise ＋ Ed25519 の Agent Card |

**採らない理由は 1 つで足りる — 探索が既定で付いている。**
DHT による探索は、**原案 §21 が「持たない」と決めた機能**である（D10 で libp2p を採らなかったのと同じ理由）。
探せる名簿があると、なりすましに意味が生まれる（PRD §12-3）。

**それとは別に、先行例としては価値がある。**「daemon を隣に置いて gRPC で話す」形は、
`warifu-mcp` が採った「MCP の口を隣に出す」形と同じ発想である。

---

## 5. **自分で踏んだ穴**（この調査で一番重要なこと）

`issues/001` は表にこう書いていた。

> §3.5 Capability ベース権限 → **UCAN / ZCAP-LD ← 自前設計する前に必ず読む**

**読む前に `warifu-capability` を書いた**（2026-09-02・`decisions.md` **D24**）。

| `warifu-capability` | UCAN 1.0 |
|---|---|
| `Grant`（誰に・何を・いつまで） | ✅ 委譲トークン（`exp` / `nbf`） |
| 完全一致の照合 | ✅ **減衰**（attenuation。より強い） |
| `revoke` | ✅ 失効（RECOMMENDED） |
| — | ✅ **委譲の連鎖**（A→B→C）。**こちらには無い** |
| — | ✅ DID を主体にする |
| 判定を関所で行う / 本文を入れない | ❌ 仕様の範囲外（**運用の形**であって、データ形式ではない） |

**判定: UCAN が上位互換である。**`warifu-capability` の器は UCAN で置き換えられる。
残るのは最後の 1 行 — 「**判定の入力に本文を入れない**」という**置き場所の決め方**だけで、
これはトークン形式の話ではないので、UCAN を使っても保てる。

**書いたものが無駄だったとは言わない**（D5 を実装で確かめる役には立った）。
だが**順序を間違えた。**`issues/001` は、まさにこれを止めるために書かれていた。

---

## 6. どの既存規格も解いていないもの

**D2 — 全端末を同時に失った人間の Identity。**

- **Nostr**: 鍵を失えば終わり。**復旧の仕組みが無い**（鍵がそのまま Identity なので、構造上作れない）
- **DID**: 復旧は DID Method 任せ。**Method ごとにばらばら**で、共通の答えは無い
- **UCAN / AIP**: 委譲は解くが、**根の鍵を失った場合は範囲外**
- **Keybase**: 集約点があったが、**その集約点自体が放置された**（§3）

**ここは既存に寄せられない。**`issues/002` は、この調査の後でも生きている。

---

## 7. 分からなかったこと（**埋めない**）

- **AIP の実装の有無。**I-D は 2 本あるが、**動いている実装を確認できていない。**
  仕様があっても実装が 1 つも無いなら「既に解決されている」ではない（`issues/001` の制約）
- **UCAN 1.0 の確定日。**リポジトリからは版が 1.0 であることまでしか読めなかった
- **DIDComm v2 の実装の成熟度。**DIF 自身が「実装を一覧しているだけで、
  成熟度や適合性は保証しない」と書いている
- **Veilid / OpenAgents** は見ていない（`issues/001` は OpenAgents を「優先度は低くてよい」としている）

---

## 8. 次に決めること（**AI は決めない**）

1. **`warifu-capability` を UCAN へ寄せるか。**
   寄せる場合、依存ゼロ（**D18** / **D24**）を崩すことになる。**これは設計の根幹に触るので上流の判断**
2. **「プロファイル + Reference Implementation」を名乗るか。**
   名乗るなら PRD §0-c の位置づけを書き換えることになる
3. **AIP の I-D 2 本を、Phase 3 の下敷きにするか**（MCP と A2A のバインディングが既にある）
