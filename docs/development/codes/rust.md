# D‑TPRES Rust Coding Rules（AO Wasm モジュール）

> **目的** ─ 本ドキュメントは *AI 駆動開発*（例：cline / Roo / GitHub Copilot）がソースコードを自動生成・リファクタリングできるよう、D‑TPRES プロジェクトの Rust→Wasm 実装規約とディレクトリ構造を機械が解釈しやすい形で定義します。以後 **“コードガイドライン”** と記載。

---

## 0. メタデータ

```toml
# 🚀 AI‑Friendly metadata (読み取り専用)
[guide]
version     = "0.1.0"
msrv        = "1.72"
wasm-target = "wasm32-wasi"
license     = "Apache-2.0"
```

---

## 1. アーキテクチャ指針

| 項目           | 規約                                                                                                | 理由/ソース                                             |
| ------------ | ------------------------------------------------------------------------------------------------- | -------------------------------------------------- |
| **モジュール数**   | Owner / Holder / Requester の 3 ロールを **単一 Wasm バイナリ**に同居させる                                        | AO `spawn` は `module_tx` を1つしか取らないため（AO Spec §3.2） |
| **デザインパターン** | `enum RoleMsg` + `match dispatch`                                                                 | CosmWasm / Ink! などのスマートコントラクトで実証済みのメッセージ分岐パターン     |
| **依存の最小化**   | 標準ライブラリ + `umbral-pre` `sssa` `ao-sqlite` `zeroize`                                               | WASI 互換・サイズ削減                                      |
| **スレッド禁止**   | `#![cfg_attr(target_arch = "wasm32", feature(panic_immediate_abort))]`<br>`#![deny(unsafe_code)]` | AO Wasm は単一スレッド実行（WASI Spec §1）                    |
| **ヒープ最小**    | `heapless`, `arrayvec`, `SmallVec` を優先                                                            | 決定論的性能と TEE フットプリント確保                              |
| **暗号安全**     | `zeroize` / `secrecy` / `subtle`                                                                  | メモリ残留・タイミング漏洩を防止                                   |
| **CI**       | `cargo fmt --check` + `cargo clippy -- -D warnings`                                               | 一貫したスタイル・品質                                        |
| **バイナリ圧縮**   | `wasm-opt -Oz`, `strip`                                                                           | Arweave Tx 1本 ≤ 2 MiB を保証                          |

---

## 2. ディレクトリ構造

```text
├─ Cargo.toml           # crate‑type = ["cdylib"]
└─ src/
   ├─ lib.rs            # ★エントリポイント (handle)
   ├─ owner.rs          # Owner ロール専用ロジック
   ├─ holder.rs         # Holder ロール専用ロジック
   ├─ requester.rs      # Requester ロール専用ロジック
   ├─ core/
   │   ├─ crypto.rs     # Umbral PRE & SSS ラッパ
   │   └─ storage.rs    # ao‑sqlite API
   └─ util/
       ├─ msg.rs        # RoleMsg enum + 解析
       └─ zero.rs       # Zeroize ヘルパ
```

> **AI Hint:** モジュール階層は固定。生成ツールはファイル位置を変えないこと。

---

## 3. コーディングスタイル

* **Edition** : Rust 2021
* **ファイルヘッダ** : すべてに `//!` モジュールドキュメントを付与。AI モデルはヘッダから機能を推論します。
* **命名規則** : `snake_case` / `CamelCase`（Rust標準）。cFrag/kFrag などドメイン語はそのまま。
* **Panics 禁止** : ビジネスロジックで `panic!` せず `anyhow::Result` を返す。

---

## 4. メッセージフォーマット

```jsonc
{
  "fn":   "owner.init" | "holder.store_kfrag" | "requester.wrap_share" | ... ,
  "role": "owner" | "holder" | "requester",
  "data": { /* ロールごとペイロード */ }
}
```

* `lib.rs` の `handle(raw_msg)` で `serde_json::from_slice` → `RoleMsg`。
* **AI Hint:** 新ハンドラ追加時は `fn` を dotted‑path で命名し、上表に追記。

---

## 5. セキュアコーディング

1. 機密バイト列は `SecretVec<u8>` で保持。
2. `Drop` 実装で `zeroize()` するか、`Secret` ラッパを使う。
3. 比較は `ct_eq()`、決して `==` を使用しない。
4. デバッグ出力 (`Debug`, `Display`) は実装しないか `"***"` を返す。

---

## 6. ビルド & デプロイ

```bash
# 1. Release build
cargo build --release --target wasm32-wasi -Z build-std=std,panic_abort

# 2. サイズ最適化
wasm-opt -Oz -o dtpres_core.wasm target/wasm32-wasi/release/*.wasm
wasm-strip dtpres_core.wasm

# 3. Arweave へアップ
ao deploy dtpres_core.wasm  # 生成された TxID を MODULE_TX として記録
```

> **AI Hint:** 自動化スクリプトは必ず上記 3 ステップを順守すること。

---

## 7. テスト & CI

* `cargo test` (native) + `wasmer run` (WASI) でユニットテストを実行。
* GitHub Actions 例: `ci.yaml` で MSRV (1.72) と latest stable をマトリクスビルド。
* `cargo clippy` は `-D warnings`。

---

## 8. 変更フロー

1. **Issue → PR → Review** の GitHub Flow。
2. 大きな API 変更時は `CHANGELOG.md` を更新。
3. `./scripts/generate_docs.sh` で Rustdoc を HTML 出力、`docs/` へ push。

---

## 9. 参考リンク

* WASI スタートガイド [https://github.com/WebAssembly/WASI](https://github.com/WebAssembly/WASI)
* zeroize クレート [https://crates.io/crates/zeroize](https://crates.io/crates/zeroize)
* subtle クレート [https://crates.io/crates/subtle](https://crates.io/crates/subtle)
* umbral-pre Rust 実装 [https://github.com/nucypher/umbral-pre](https://github.com/nucypher/umbral-pre)
* AO & HyperBEAM リポジトリ [https://github.com/permaweb/HyperBEAM](https://github.com/permaweb/HyperBEAM)

---

> **最終更新: 2025‑05‑10**
