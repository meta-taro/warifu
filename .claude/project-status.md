# プロジェクトステータス — warifu

- **現在フェーズ**: **M1 完了（テスト 38 件 green）** → 次は M2
- **最終更新**: 2026-08-17

## 2026-08-17 に何が変わったか

オーナー指示により **Voice / Video を前倒し**した（`decisions.md` **D9**）。
併せて、これまで意図的に伏せていた**実装言語と Transport を決定**した（**D10**）。
md-business との関係は **一方向の依存**として固定した（**D11**）。

**Embedded SFU（5 人以上）は動かしていない。**D7（利用者端末が他人の通信を中継する）が未決のため。
D8 が定めた「覆す場合も SFU は最後尾に残す」という但し書きを、そのまま守っている。

## 完了した作業

| | 内容 |
|---|---|
| 正本の更新 | `decisions.md` D9 / D10 / D11 追加・D2 に追記 |
| | `roadmap.md` Step 1 / 2 / 3 更新・Step 3.5 / 3.6 新設・Phase 4 を取り消し |
| | `PRD.md` §8 / §10 / §11-3 を D9 / D10 に追従 |
| | `README.md` 公開面を更新 |
| ローカル Issue | `005-4人までのP2P会議.md` / `006-md-business-との掛け合わせ.md` 起票 |
| 調査 | Transport 候補の比較（iroh / libp2p / Matrix / Trystero / Jami）→ D10 |
| | md-business のスタック確認（Tauri 2 + TypeScript + SvelteKit + pnpm / MIT） |
| 環境確認 | rustc 1.95.0 / cargo 1.95.0 / node v22.22.2 / pnpm 11.1.1 |
| **M1** | `warifu-core` — シード / Profile 鍵 / Device 鍵 / 割符の生成と検証 / 失効 | **テスト 38 件 green** |

## M1 で作ったもの

| ファイル | 中身 |
|---|---|
| `crates/warifu-core/src/key.rs` | シード → Profile → Device の**決定的**な鍵導出（HKDF-SHA512 / Ed25519） |
| `crates/warifu-core/src/tally.rs` | 割符の発行・受諾・照合。**秘密そのものを返送させない**（証だけを返す） |
| `crates/warifu-core/src/revocation.rs` | 失効の名簿。**取り消せない**（降ろせると鍵を盗った側が降ろせる） |
| `crates/warifu-core/src/base32.rs` | RFC 4648 base32。**同じバイト列に複数の表記を許さない** |
| `crates/warifu-core/src/error.rs` | 失敗の種類。秘密は入らない |

**ネットワークのコードは 1 行も入っていない。**割符は QR・紙・口頭でも成立する。

### テストで固定した振る舞い

| | |
|---|---|
| `tests/derivation.rs` (10) | 同じシードから同じ鍵 / Personal と Work が結び付かない / ラベルの区切りを跨いだ衝突がない |
| `tests/tally.rs` (15) | 一度使った割符は二度使えない / 期限切れは両側で止まる / 1 bit でも書き換われば受け取らない / 秘密が `Debug` 出力に出ない |
| `tests/revocation.rs` (6) | 失効は取り消せない / 最初に失効させた時刻が残る / 関係ない相手を巻き込まない |
| `src/base32.rs` (6) + doc test (1) | RFC 4648 の例と一致 / 非正規な表記を受け取らない |

## 未完了の作業

| | 内容 | 状態 |
|---|---|---|
| M2 | `warifu-net` — iroh で 2 台をつなぐ | **次はここ** |
| M3 | Intent の口（`file.*` / `meeting.*`） | 未 |
| M4〜M7 | 4 人までの会議（`issues/005`） | 未 |
| M8〜M9 | md-business との掛け合わせ（`issues/006`） | 未 |
| `issues/001` | 既存規格との差分調査（Identity / VC / Capability 側） | **会議に関わる範囲だけ完了**。残りは並行 |
| `issues/002` | D1 / D2 の決着 | D2 は**着手のブロッカーではなくなった**（決定的鍵導出）が、**人に配る前には決める必要がある** |
| `DESIGN.md` | 会議画面の見た目の方向性 | **人が決める領域**（baseline §11）。AI は埋めない |

## テスト状況

**`cargo test` 38 件 green**（単体 6 / derivation 10 / tally 15 / revocation 6 / doc 1）。
`cargo fmt` 済み・`cargo clippy --all-targets -- -D warnings` 通過。

RED → GREEN の順で書いた（テストを先に置き、落ちることを確認してから実装した）。

**ただし、これは自動テストが通ったという意味でしかない。**人が動かして確かめた工程はまだ無い
（M5 以降・baseline §29）。

## 既知の問題

| | 内容 |
|---|---|
| **「4 人まで」は文献値** | 手元の回線で成立する保証がない。**M6 で実測するまで確定しない**（`issues/005`） |
| **D7 が未決** | 5 人以上に進めない。法的整理が要る |
| **D2 が未決** | 全端末を同時に失った場合の復旧方式。実装は止まらないが、**配布前には決着が要る** |
| **iroh の既定 Relay** | n0 が運用する公開 Relay を既定で使う。**誰が誰に繋いだかのメタデータが n0 側に出る**。自前 Relay に切り替えられることは確認済みだが、未実施（D10） |
