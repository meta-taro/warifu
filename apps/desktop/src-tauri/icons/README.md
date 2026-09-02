# icons — **仮の穴埋め**

ここにあるのは**ロゴではない。**意匠を含まない角丸の四角で、
承認済みのアクセント色（`DESIGN.md` §3.1 の `--accent` = `#5b5bd6`）で塗っただけのもの。

## なぜ置いているか

Tauri の `generate_context!` は `icons/icon.png` が無いと**コンパイルできない**。
一方 **ロゴは人が決める領域**である（product-baseline §11）。
決めずに済ませられないが、AI が意匠を作るとそれがそのまま既成事実になる。

そこで「意匠を含まないもの」を置いた。**印も文字も入れていない。**

## 作り直し方

```bash
python3 scripts/make-placeholder-icon.py
```

## 人が決めたら

`DESIGN.md` にロゴの節を足し（dbboard の `DESIGN.md` の Logo 節が参考になる）、
**このディレクトリと `scripts/make-placeholder-icon.py` ごと捨てる。**

配布に要るのは `icon.png` だけではない。`.ico`（Windows）・`.icns`（macOS）と、
`tauri.conf.json` の `bundle.icon` への登録が要る。**今は `bundle.icon` を空にしてある。**
