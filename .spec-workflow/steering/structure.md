# プロジェクト構造

## 概要

FORMIXは3つの主要コンポーネントで構成される：
- **client/**: ローカル暗号処理ライブラリ（Rust → WASM）
- **ao/**: AO Network上のコントラクト（Rust → WASM）
- **dtpres-sdk/**: 統合SDK（TypeScript、設計中）

## ディレクトリ構造

```
FORMIX/
├── client/                         # クライアントライブラリ（Rust）
│   ├── src/
│   │   ├── lib.rs                  # ライブラリエントリーポイント
│   │   ├── di.rs                   # 依存性注入コンテナ
│   │   │
│   │   ├── actions/                # Actions層（Facade） ※旧: usecase/
│   │   │   ├── mod.rs
│   │   │   ├── share.rs            # share() API
│   │   │   ├── recover.rs          # recover() API
│   │   │   └── keygen.rs           # generateKeyPair() API
│   │   │
│   │   ├── controller/             # Controller層
│   │   │   ├── mod.rs
│   │   │   ├── validator.rs        # 入力の妥当性検証
│   │   │   └── extractor.rs        # UseCase層向けDTO変換
│   │   │
│   │   ├── usecase/                # UseCase層 ※旧: service/
│   │   │   ├── mod.rs
│   │   │   ├── error.rs            # UseCase層エラー定義
│   │   │   ├── secret_sharing_service.rs  # Phase 1処理
│   │   │   ├── secret_recovery_service.rs # Phase 3処理
│   │   │   └── core/               # CoreService
│   │   │       ├── mod.rs
│   │   │       ├── crypto.rs       # CryptoService（TPRE・Shamir）
│   │   │       └── storage.rs      # StorageService
│   │   │
│   │   ├── domain/                 # Domain層
│   │   │   ├── mod.rs
│   │   │   ├── errors.rs           # Domain層エラー定義
│   │   │   ├── entities/           # エンティティ定義（5つ）
│   │   │   │   ├── mod.rs
│   │   │   │   ├── secret.rs       # Secret（集約ルート）
│   │   │   │   ├── share.rs        # ShareCollection（Shamirシェアコレクション）
│   │   │   │   ├── capsule.rs      # Capsule（PREカプセル）
│   │   │   │   ├── kfrag.rs        # KFrag（鍵フラグメント）
│   │   │   │   └── cfrag.rs        # CFrag（再暗号化フラグメント）
│   │   │   └── value_objects/      # Value Objects
│   │   │       ├── mod.rs
│   │   │       └── ids.rs          # SecretId, CapsuleId, etc.
│   │   │
│   │   ├── repositories/           # Repository層（Repository Interface）
│   │   │   ├── mod.rs              # 基本Repository trait + モジュールエクスポート
│   │   │   ├── secret_interface.rs # SecretRepository trait
│   │   │   ├── share_interface.rs  # ShareCollectionRepository trait
│   │   │   ├── capsule_interface.rs # CapsuleRepository trait
│   │   │   ├── kfrag_interface.rs  # KFragRepository trait
│   │   │   └── cfrag_interface.rs  # CFragRepository trait
│   │   │
│   │   └── adapter/                # Adapter層（Repository Interfaceを実装）
│   │       ├── mod.rs
│   │       ├── errors.rs           # Adapter層エラー定義
│   │       ├── repository_impl/    # Repository実装（Arweave永続化）
│   │       │   ├── mod.rs
│   │       │   ├── secret_impl.rs  # ArweaveSecretRepository
│   │       │   ├── share_impl.rs   # ArweaveShareCollectionRepository
│   │       │   ├── capsule_impl.rs # ArweaveCapsuleRepository
│   │       │   ├── kfrag_impl.rs   # ArweaveKFragRepository
│   │       │   └── cfrag_impl.rs   # ArweaveCFragRepository
│   │       └── external/           # 外部システムアダプター
│   │           ├── mod.rs
│   │           ├── arweave_client.rs # ArweaveClient（Arweave通信）
│   │           └── ao_client.rs    # AOClient（AO通信）
│   │
│   ├── docs/                       # クライアントライブラリ設計ドキュメント
│   ├── Cargo.toml                  # Rust依存関係
│   ├── rust-toolchain.toml         # Rustバージョン固定（1.86.0）
│   ├── Makefile                    # ビルドコマンド
│   ├── clippy.toml                 # Clippy設定
│   └── rustfmt.toml                # フォーマット設定
│
├── ao/                             # AOコントラクト
│   ├── contracts/                  # コントラクト実装
│   │   ├── src/
│   │   │   ├── lib.rs              # WASMエントリーポイント
│   │   │   ├── contract.rs         # コントラクトロジック
│   │   │   ├── handlers.rs         # メッセージハンドラー
│   │   │   ├── state.rs            # プロセス状態管理
│   │   │   └── msg.rs              # メッセージ型定義
│   │   ├── tests/                  # コントラクトテスト
│   │   ├── docs/                   # コントラクト設計ドキュメント
│   │   ├── Cargo.toml
│   │   └── package.json
│   ├── scripts/                    # デプロイ・管理スクリプト
│   └── test/                       # 統合テスト
│
├── dtpres-sdk/                     # 統合SDK（設計中）
│   ├── src/
│   │   ├── index.ts                # SDKエントリーポイント
│   │   ├── dtpres.ts               # DTPRES クラス
│   │   ├── types.ts                # 型定義
│   │   ├── errors.ts               # エラークラス
│   │   ├── client/                 # client/ WASMバインディング
│   │   └── ao/                     # AO通信モジュール
│   ├── package.json
│   └── tsconfig.json
│
├── docs/                           # プロジェクト全体ドキュメント
│   ├── PRD.md                      # Product Requirements Document
│   └── development/                # 開発者向けドキュメント
│
├── .spec-workflow/                 # Spec Workflow設定
│   ├── steering/                   # ステアリングドキュメント
│   │   ├── product.md              # プロダクト定義
│   │   ├── tech.md                 # 技術スタック
│   │   └── structure.md            # 本ファイル
│   └── specs/                      # 機能仕様書
│
├── .claude/                        # Claude Code設定
│   ├── rules/                      # コーディングルール
│   └── plans/                      # 実装計画
│
├── .github/                        # GitHub設定
│   └── workflows/                  # CI/CD
│
├── CLAUDE.md                       # Claude Code向けプロジェクト説明
├── README.md                       # プロジェクト概要（英語）
└── README_ja.md                    # プロジェクト概要（日本語）
```

## コンポーネント詳細

### client/ - クライアントライブラリ

**責務**: Phase 1（秘密分割）とPhase 3（秘密復元）のローカル暗号処理

**Clean Architecture（6層構成）**:

| レイヤー | ディレクトリ | 役割 |
|---------|------------|------|
| Actions | `actions/` | 開発者向けエンドポイント（Facade） |
| Controller | `controller/` | 入力検証、DTO変換 |
| UseCase | `usecase/` | Phase 1/3オーケストレーション |
| UseCase | `usecase/core/` | CoreService（暗号・ストレージ操作） |
| Domain | `domain/entities/` | エンティティ定義（Secret, ShareCollection, Capsule, KFrag, CFrag） |
| Domain | `domain/value_objects/` | Value Objects（ID型など） |
| Repository | `repositories/` | Repository Interface（DIP） |
| Adapter | `adapter/repository_impl/` | Arweave永続化実装（Repository Interfaceを実装） |
| Adapter | `adapter/external/` | 外部システムアダプター |

**エンティティ（5つ）**:
| エンティティ | 説明 |
|------------|------|
| `Secret` | 秘密メタデータ（集約ルート） |
| `ShareCollection` | Shamirシェアコレクション |
| `Capsule` | PREカプセル |
| `KFrag` | 鍵フラグメント（Owner→Holder） |
| `CFrag` | 再暗号化フラグメント（Holder→Requester） |

**依存関係フロー（Clean Architecture）**:
```
Actions → Controller → UseCase → Domain
                           ↓
                      Repository ← Adapter (implements)
```

**Note**: Adapter層はRepository層のRepository Interfaceを実装する。Domain層とRepository層はAdapter層に依存しない（依存性逆転原則）。

### ao/ - AOコントラクト

**責務**: AO Network上でのkFrag配布・再暗号化

**主要ファイル**:
| ファイル | 役割 |
|---------|------|
| `lib.rs` | WASMエントリーポイント |
| `contract.rs` | コントラクトメインロジック |
| `handlers.rs` | メッセージハンドラー（Owner/Holder/Requester） |
| `state.rs` | プロセス状態（kFrag、cFrag保存） |
| `msg.rs` | AOメッセージ型定義 |

**プロセスロール**:
- **Owner-Process**: `DelegateKFrag`, `DelegateCapsule`
- **Holder-Process**: `SubmitKFrag`, `SubmitCapsule`, `perform_reencryption`
- **Requester-Process**: `GetCFrag`

### dtpres-sdk/ - 統合SDK

**責務**: 開発者向け統一エンドポイント

**API関数**:
| 関数 | 用途 |
|------|------|
| `DTPRES.init()` | SDK初期化 |
| `generateKeyPair()` | PRE鍵ペア生成 |
| `share()` | Phase 1: 秘密共有 |
| `recover()` | Phase 3: 秘密復元 |

**内部モジュール**:
- `client/`: client/のWASMバインディング
- `ao/`: AO Network通信（aoconnect使用）

## ファイル命名規則

### Rust（client/, ao/）

| 種別 | 命名規則 | 例 |
|------|---------|-----|
| モジュール | snake_case | `crypto.rs`, `secret_sharing.rs` |
| 構造体 | PascalCase | `Secret`, `CryptoService` |
| トレイト | PascalCase | `SecretRepository` |
| 関数 | snake_case | `generate_keypair()`, `split_secret()` |
| 定数 | SCREAMING_SNAKE_CASE | `MAX_THRESHOLD`, `DEFAULT_N` |

### TypeScript（dtpres-sdk/）

| 種別 | 命名規則 | 例 |
|------|---------|-----|
| ファイル | camelCase | `dtpres.ts`, `cryptoService.ts` |
| クラス | PascalCase | `DTPRES`, `ShareError` |
| 関数 | camelCase | `generateKeyPair()`, `share()` |
| 定数 | SCREAMING_SNAKE_CASE | `DEFAULT_THRESHOLD` |
| 型 | PascalCase | `ShareOptions`, `KeyPair` |

## 依存関係

```
dtpres-sdk (TypeScript)
    ├── client/ (WASM)
    │   └── Arweave (Storage)
    └── ao/ (AO Network)
        ├── Arweave (Storage)
        └── RandAO (Holder選出)
```

## ビルド成果物

| コンポーネント | 成果物 | 出力先 |
|--------------|--------|-------|
| client/ | WASM + JS bindings | `client/pkg/` |
| ao/contracts/ | WASM | `ao/contracts/target/wasm32-unknown-unknown/` |
| dtpres-sdk/ | npm package | `dtpres-sdk/dist/` |
