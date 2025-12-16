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

```
D-TPRES/
├── src/                           # AO WebAssembly (Rust) - レイヤードアーキテクチャ
│   ├── main.rs                    # AOエントリーポイント & ハンドラー登録
│   ├── di.rs                      # 依存性注入コンテナ
│   ├── lib.rs                     # WASMライブラリエクスポート
│   │
│   ├── usecase/                   # UseCase Layer - AOメッセージハンドラー
│   │   ├── mod.rs                 # 公開エクスポート
│   │   ├── handlers/              # ロールベースメッセージハンドラー
│   │   │   ├── mod.rs
│   │   │   ├── owner_handlers.rs  # Ownerロールハンドラー
│   │   │   ├── holder_handlers.rs # Holderロールハンドラー
│   │   │   ├── requester_handlers.rs # Requesterロールハンドラー
│   │   │   └── common_handlers.rs # 共通ハンドラーユーティリティ
│   │   ├── context.rs             # ハンドラーコンテキスト管理
│   │   └── errors.rs              # UseCase層エラー定義
│   │
│   ├── controller/                # Controller Layer - メッセージ処理
│   │   ├── mod.rs
│   │   ├── message_handler.rs     # 中央MessageHandler
│   │   ├── router.rs              # MessageRouter実装
│   │   ├── validator.rs           # MessageValidator & バリデーションロジック
│   │   ├── extractor.rs           # MessageContextExtractor & DTOs
│   │   ├── response.rs            # レスポンス生成ユーティリティ
│   │   └── errors.rs              # Controller層エラー定義
│   │
│   ├── service/                   # Service Layer - ビジネスロジック
│   │   ├── mod.rs
│   │   ├── workflow/              # Workflow Services (Phaseオーケストレーション)
│   │   │   ├── mod.rs
│   │   │   ├── secret_sharing.rs  # Phase 1: SecretSharingWorkflowService
│   │   │   ├── access_request.rs  # Phase 2: AccessRequestWorkflowService
│   │   │   ├── reencryption.rs    # Phase 3-4: ReencryptionWorkflowService
│   │   │   └── secret_recovery.rs # Phase 5: SecretRecoveryWorkflowService
│   │   ├── core/                  # Core Services (基本操作)
│   │   │   ├── mod.rs
│   │   │   ├── crypto.rs          # CryptoService (TPRE, Shamir)
│   │   │   ├── process.rs         # ProcessManagementService
│   │   │   ├── messaging.rs       # MessageRoutingService
│   │   │   └── storage.rs         # ArweaveStorageService
│   │   ├── container.rs           # ServiceContainer for DI
│   │   └── errors.rs              # Service層エラー定義
│   │
│   ├── domain/                    # Domain Layer - エンティティ & Repository Interface
│   │   ├── mod.rs
│   │   ├── entities/              # 純粋データ構造
│   │   │   ├── mod.rs
│   │   │   ├── process.rs         # ProcessEntity
│   │   │   ├── share.rs           # ShareEntity
│   │   │   ├── capsule.rs         # CapsuleEntity
│   │   │   ├── access_request.rs  # AccessRequestEntity
│   │   │   ├── rekey_fragment.rs  # RekeyFragmentEntity
│   │   │   ├── reencryption.rs    # ReencryptionEntity
│   │   │   ├── secret_details.rs  # SecretDetailsEntity
│   │   │   └── value_objects.rs           # Union型定義（ProcessRole, SecretStatus等）
│   │   ├── repositories/          # Repository Interface (DIP)
│   │   │   ├── mod.rs
│   │   │   ├── process.rs         # ProcessEntityRepository trait
│   │   │   ├── share.rs           # ShareEntityRepository trait
│   │   │   ├── capsule.rs         # CapsuleEntityRepository trait
│   │   │   ├── access_request.rs  # AccessRequestEntityRepository trait
│   │   │   ├── rekey_fragment.rs  # RekeyFragmentEntityRepository trait
│   │   │   └── reencryption.rs    # ReencryptionEntityRepository trait
│   │   └── errors.rs              # Domain層エラー定義
│   │
│   ├── infrastructure/            # Infrastructure Layer - 技術実装
│   │   ├── mod.rs
│   │   ├── repositories/          # Repository実装
│   │   │   ├── mod.rs
│   │   │   ├── arweave_base.rs    # 基盤ArweaveRepository実装
│   │   │   ├── process_impl.rs    # ProcessEntityRepositoryImpl
│   │   │   ├── share_impl.rs      # ShareEntityRepositoryImpl
│   │   │   ├── capsule_impl.rs    # CapsuleEntityRepositoryImpl
│   │   │   ├── access_request_impl.rs # AccessRequestEntityRepositoryImpl
│   │   │   ├── rekey_fragment_impl.rs # RekeyFragmentEntityRepositoryImpl
│   │   │   └── reencryption_impl.rs # ReencryptionEntityRepositoryImpl
│   │   ├── external/              # 外部システムアダプター
│   │   │   ├── mod.rs
│   │   │   ├── arweave_client.rs  # ArweaveClient
│   │   │   └── evm_bridge.rs      # elciao EVM bridge
│   │   ├── cache.rs               # メッセージスコープキャッシュ
│   │   └── errors.rs              # Infrastructure層エラー定義
│   │
│   ├── crypto/                    # 暗号化ユーティリティ
│   │   ├── mod.rs
│   │   ├── umbral.rs              # Umbral TPRE操作
│   │   ├── shamir.rs              # Shamir Secret Sharing
│   │   └── utils.rs               # 暗号化ユーティリティ関数
│   │
│   └── utils/                     # 共有ユーティリティ
│       ├── mod.rs
│       ├── serialization.rs       # Serdeヘルパー
│       ├── time.rs                # タイムスタンプユーティリティ
│       └── constants.rs           # システム定数
│
├── browser/                       # ブラウザフロントエンド
│   ├── packages/
│   │   ├── core/                  # 共通ライブラリ
│   │   │   ├── crypto/            # WebCrypto + WASM統合
│   │   │   ├── ao/                # AO通信ライブラリ
│   │   │   └── types/             # 共通型定義
│   │   ├── o-browser/             # データ所有者UI
│   │   └── a-browser/             # アクセス者UI
│   ├── shared/                    # 共通コンポーネント
│   └── package.json
├── contracts/                     # EVM Smart Contracts
│   ├── src/
│   │   └── VerifyAccess.sol
│   └── package.json
├── wasm/                          # WebAssembly ビルド成果物
│   ├── umbral_wasm.js
│   └── umbral_wasm.wasm
├── scripts/                       # ビルド・デプロイスクリプト
└── docs/
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
