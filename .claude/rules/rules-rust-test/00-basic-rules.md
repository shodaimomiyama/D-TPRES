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
├── client/src/                    # Client Library (Rust) - レイヤードアーキテクチャ
│   ├── lib.rs                     # ライブラリエクスポート
│   ├── di.rs                      # 依存性注入コンテナ
│   │
│   ├── actions/                   # Actions Layer - クライアントAPIエントリーポイント
│   │   ├── client.rs              # share, recover, generateKeyPair アクション
│   │   ├── builder.rs             # ActionsBuilder (DI設定)
│   │   ├── di.rs                  # 型エイリアスとDIコンテナ
│   │   └── options.rs             # アクションオプション/パラメータ
│   │
│   ├── controller/                # Controller Layer - バリデーション & DTO抽出
│   │   ├── validator.rs           # ShareValidator, RecoverValidator
│   │   └── extractor.rs           # ShareExtractor, RecoverExtractor
│   │
│   ├── usecase/                   # UseCase Layer - ビジネスロジック
│   │   ├── core/                  # Core Services (基本操作)
│   │   │   ├── crypto.rs          # CoreCryptoService (TPRE, Shamir)
│   │   │   ├── storage.rs         # ArweaveStorageService
│   │   │   └── contract_storage.rs # ContractStorage (AO状態)
│   │   ├── service/               # Service Layer (コアのラッピング)
│   │   │   ├── crypto_service.rs  # ServiceCryptoService
│   │   │   └── storage_service.rs # ServiceStorageService
│   │   └── workflow/              # Workflow Services (フェーズ調整)
│   │       ├── secret_sharing_service.rs  # Phase 1: 秘密分割・配布
│   │       └── secret_recovery_service.rs # Phase 3: 秘密復元
│   │
│   ├── domain/                    # Domain Layer - エンティティ
│   │   ├── entities/              # Secret, Capsule, KFrag, CFrag, ShareCollection
│   │   └── value_objects/         # IDs, KeyPair, SecretData, SymmetricKey
│   │
│   ├── repositories/              # Repository Interfaces (DIP)
│   │
│   └── adapter/                   # Infrastructure Layer
│       ├── repository_impl/       # Arweaveバックエンドのリポジトリ実装
│       └── external/              # ArweaveClient, AOClient, MockAOClient
│
├── browser/                       # ブラウザフロントエンド
├── wasm/                          # WebAssembly ビルド成果物
├── scripts/                       # ビルド・デプロイスクリプト
└── docs/                          # ドキュメント
```
