# Rust Test Basic Rules

## Purpose

This document defines the behavior for Roo or Cline to operate autonomously in Rust development.
You are expected to write consistent, maintainable, and intentional code that adheres to state machine behavior, coding rules, and best practices.

**Before planning or implementing any task, always refer to and adhere to the requirements defined in `docs/PRD.md` and the coding rules specified in `.claude/rules/rules-rust-code/01-global-coding-rules.md`.**

## Editable Scope

**You do not have permission to edit tests. Please switch to `rust-test` mode to edit tests.**
You should only modify code relevant to the feature you are creating or modifying in this branch. If you are unsure about the task or which files you are allowed to edit, confirm with the user regarding edit permissions.

## State machine

**Skipping states or simultaneous processing is prohibited. Be sure to output which step you are in.**

```mermaid
stateDiagram-v2
    [*] --> InformationGathering
 InformationGathering --> InformationGathering
 InformationGathering --> TargetFileSelecting
 InformationGathering --> SwitchMode
    TargetFileSelecting --> InformationGathering
    TargetFileSelecting --> SourceCodeEditing
    SourceCodeEditing --> SelfReview
 SelfReview --> ReviewFix
 ReviewFix --> TargetFileSelecting
 ReviewFix --> InformationGathering

 state "Target File Selecting" as TargetFileSelecting
    state "Information Gathering" as InformationGathering
    state "Source Code Editing" as SourceCodeEditing
    state "Self-Review" as SelfReview
 state "Review Fix" as ReviewFix
```

### 3.1. Information Gathering

Observe source code, compile results, and lint results as needed. Print debugging is also required if information is missing.

### 3.2. Target File Selecting

Select a single file to be edited. Do not select files that are not relevant to the current task. If you are unsure, be sure to ask the user.

### 3.3. Source Code Editing

Change the source code. **Editing anything other than the target file is absolutely prohibited.**

### 3.4 Self-Review, Review Fix

**Steps:**

1. Clarify which architectural layer the component being created/modified belongs to.
2. Create a checklist of applicable rules (refer to Global rules + rules for the specific architectural layer in this document).
3. Complete the checklist to verify if the changes comply with the rules defined in `.claude/rules/rules-rust-code/01-global-coding-rules.md` and align with the requirements in `docs/PRD.md`. If violations exist, state them clearly and rethink the solution.
4. make sure `make fmt` is done and `make lint` passes.

```markdown
<!-- Checklist format -->
- Current Task: (Summarize the current task in about 30 chars)
- Focus: (e.g., Entity, Usecase)
- Rule Checklist
  <!-- List Global rules -->
  - [] Global.CommentConvention
  <!-- List relevant architectural layer rules -->
  - [] Domain.EntityConstraints
  - [] Adherence to .claude/rules/rules-rust-code/01-global-coding-rules.md
  - [] Alignment with docs/PRD.md
```

**Note:**

- **Generated code is likely to contain rule violations. Please review the code thoroughly for any mistakes and ensure compliance with all rules.**
- Address any comments violating the rules found in the target file, even if they are outside the modified section.

## Mode Switching

- Use `rust-test` mode for implementing or modifying tests.
- Use `pr` mode for creating Pull Requests.

## Directory Structure

```
FORMIX/
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
│   │   │   └── reencryption.rs    # ReencryptionEntity
│   │   ├── repositories/          # Repository Interface (DIP)
│   │   │   ├── mod.rs
│   │   │   ├── process.rs         # ProcessEntityRepository trait
│   │   │   ├── share.rs           # ShareEntityRepository trait
│   │   │   ├── capsule.rs         # CapsuleEntityRepository trait
│   │   │   ├── access_request.rs  # AccessRequestEntityRepository trait
│   │   │   ├── rekey_fragment.rs  # RekeyFragmentEntityRepository trait
│   │   │   └── reencryption.rs    # ReencryptionEntityRepository trait
│   │   ├── value_objects/         # ドメイン値オブジェクト
│   │   │   ├── mod.rs
│   │   │   ├── process_role.rs    # ProcessRole enum
│   │   │   ├── secret_id.rs       # SecretId値オブジェクト
│   │   │   └── phase.rs           # Phase enum
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
