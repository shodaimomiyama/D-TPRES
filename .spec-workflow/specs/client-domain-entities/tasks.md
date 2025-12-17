# Tasks: Client Domain Entities

## Overview

承認済みのRequirements/Designに基づき、`client/src/domain/entities/` および `client/src/domain/value_objects/` の実装タスクを定義する。

## Task Groups

### Group 1: Foundation (Value Objects & Errors)

#### Task 1.1: ID Value Objects
- **File**: `client/src/domain/value_objects/ids.rs`
- **Status**: [ ]
- **Description**: 型安全なID Value Objectsを実装
- **Implementation**:
  - `SecretId`, `ShareCollectionId`, `CapsuleId`, `KFragId`, `CFragId`
  - newtype pattern: `pub struct XxxId(String)`
  - Derive: `Debug, Clone, PartialEq, Eq, Hash`
  - Methods: `::new(id: String)`, `::generate()`, `as_str() -> &str`
- **Acceptance Criteria**:
  - 各ID型が型安全に区別される
  - `generate()`でUUID v4形式のIDを生成

#### Task 1.2: SecretData Value Object
- **File**: `client/src/domain/value_objects/secret_data.rs`
- **Status**: [ ]
- **Description**: 秘密データ f(0)=secret を表現するValue Object
- **Implementation**:
  - `#[derive(Zeroize, ZeroizeOnDrop)]`
  - フィールド: `data: Vec<u8>`
  - Methods: `::new(data: Vec<u8>) -> Result<Self, DomainError>`, `as_bytes() -> &[u8]`
  - Clone禁止
- **Acceptance Criteria**:
  - 空データでDomainError返却
  - Drop時にZeroize実行

#### Task 1.3: KeyPair Value Object
- **File**: `client/src/domain/value_objects/key_pair.rs`
- **Status**: [ ]
- **Description**: PRE鍵ペア（skₒ, pkₒ）を表現するValue Object
- **Implementation**:
  - `#[derive(Zeroize, ZeroizeOnDrop)]` (secret_keyのみ)
  - フィールド: `secret_key: Vec<u8>`, `public_key: Vec<u8>`
  - Methods: `::generate() -> Self`, `public_key() -> &[u8]`, `secret_key() -> &[u8]`
  - Clone禁止
- **Acceptance Criteria**:
  - umbral-preで鍵ペア生成
  - Drop時にsecret_keyのみZeroize

#### Task 1.4: SymmetricKey Value Object
- **File**: `client/src/domain/value_objects/symmetric_key.rs`
- **Status**: [ ]
- **Description**: AES-256共通鍵 kₒ を表現するValue Object
- **Implementation**:
  - `#[derive(Zeroize, ZeroizeOnDrop)]`
  - フィールド: `key: [u8; 32]`
  - Methods: `::generate() -> Self`, `as_bytes() -> &[u8; 32]`
  - Clone禁止
- **Acceptance Criteria**:
  - 256-bit鍵生成
  - Drop時にZeroize実行

#### Task 1.5: DomainError拡張
- **File**: `client/src/domain/errors.rs`
- **Status**: [ ]
- **Description**: Domain層エラー型を拡張
- **Implementation**:
  ```rust
  #[derive(Debug, thiserror::Error)]
  pub enum DomainError {
      #[error("Invalid threshold: k={k} must be <= n={n} and k > 0")]
      InvalidThreshold { k: u8, n: u8 },
      #[error("Invalid index: {index} must be in range 1..={max}")]
      InvalidIndex { index: u8, max: u8 },
      #[error("Empty data not allowed for {field}")]
      EmptyData { field: String },
      #[error("Invalid state transition from {from:?} to {to:?}")]
      InvalidStateTransition { from: String, to: String },
      #[error("Entity not found: {entity_type} with id {id}")]
      NotFound { entity_type: String, id: String },
      #[error("Share count mismatch: expected {expected}, got {actual}")]
      ShareCountMismatch { expected: u8, actual: usize },
  }
  ```
- **Acceptance Criteria**:
  - 全エラーバリアントが定義されている
  - thiserror::Errorを使用

### Group 2: Entities

#### Task 2.1: Secret Entity (Aggregate Root)
- **File**: `client/src/domain/entities/secret.rs`
- **Status**: [ ]
- **Dependencies**: Task 1.1, Task 1.5
- **Description**: 秘密のメタデータと状態を管理する集約ルート
- **Implementation**:
  ```rust
  pub struct Secret {
      id: SecretId,
      threshold_k: u8,
      threshold_n: u8,
      state: SecretState,
      capsule_id: Option<CapsuleId>,
      share_collection_id: Option<ShareCollectionId>,
      kfrag_ids: Vec<KFragId>,
      owner_public_key: Vec<u8>,
      requester_public_key: Option<Vec<u8>>,
      created_at: u64,
  }

  pub enum SecretState {
      Initialized,
      Split,
      Distributed,
      Recovered,
  }
  ```
  - Methods: `::new(k, n, owner_pk)`, `split()`, `distribute()`, getters
- **Acceptance Criteria**:
  - 無効なthresholdでDomainError
  - 状態遷移が正しく機能

#### Task 2.2: ShareCollection Entity
- **File**: `client/src/domain/entities/share_collection.rs`
- **Status**: [ ]
- **Dependencies**: Task 1.1, Task 1.5
- **Description**: n個の暗号化シェアを一括管理するEntity
- **Implementation**:
  ```rust
  pub struct ShareCollection {
      id: ShareCollectionId,
      secret_id: SecretId,
      threshold_k: u8,
      threshold_n: u8,
      shares: Vec<EncryptedShareData>,
      arweave_tx_id: Option<String>,
      created_at: u64,
  }

  pub struct EncryptedShareData {
      index: u8,
      encrypted_data: Vec<u8>,
  }
  ```
  - Methods: `::new()`, `get_share()`, `get_shares_by_indices()`, `shares_count()`, `set_arweave_tx_id()`
- **Acceptance Criteria**:
  - シェア数がnと一致しない場合DomainError
  - インデックス指定でシェア取得可能

#### Task 2.3: Capsule Entity
- **File**: `client/src/domain/entities/capsule.rs`
- **Status**: [ ]
- **Dependencies**: Task 1.1, Task 1.5
- **Description**: Umbral PREカプセルを表現するEntity
- **Implementation**:
  ```rust
  pub struct Capsule {
      id: CapsuleId,
      secret_id: SecretId,
      capsule_data: Vec<u8>,
      arweave_tx_id: Option<String>,
      created_at: u64,
  }
  ```
  - Methods: `::new()`, getters, `set_arweave_tx_id()`
- **Acceptance Criteria**:
  - 空のcapsule_dataでDomainError

#### Task 2.4: KFrag Entity
- **File**: `client/src/domain/entities/kfrag.rs`
- **Status**: [ ]
- **Dependencies**: Task 1.1, Task 1.5
- **Description**: 再暗号化鍵フラグメントを表現するEntity
- **Implementation**:
  ```rust
  #[derive(Zeroize, ZeroizeOnDrop)]
  pub struct KFrag {
      id: KFragId,
      secret_id: SecretId,
      holder_index: u8,
      #[zeroize(skip)]
      holder_process_id: Option<String>,
      kfrag_data: Vec<u8>,
      created_at: u64,
  }
  ```
  - Methods: `::new()`, getters, `set_holder_process_id()`
- **Acceptance Criteria**:
  - Drop時にkfrag_dataがZeroize
  - 無効なholder_indexでDomainError

#### Task 2.5: CFrag Entity
- **File**: `client/src/domain/entities/cfrag.rs`
- **Status**: [ ]
- **Dependencies**: Task 1.1, Task 1.5
- **Description**: 再暗号化フラグメントを表現するEntity
- **Implementation**:
  ```rust
  #[derive(Zeroize, ZeroizeOnDrop)]
  pub struct CFrag {
      id: CFragId,
      secret_id: SecretId,
      kfrag_id: KFragId,
      holder_index: u8,
      cfrag_data: Vec<u8>,
      created_at: u64,
  }
  ```
  - Methods: `::new()`, getters, `verify()`
- **Acceptance Criteria**:
  - Drop時にcfrag_dataがZeroize

### Group 3: Module Integration

#### Task 3.1: Value Objects Module Export
- **File**: `client/src/domain/value_objects/mod.rs`
- **Status**: [ ]
- **Dependencies**: Task 1.1-1.4
- **Description**: Value Objectsのre-export
- **Implementation**:
  ```rust
  mod ids;
  mod secret_data;
  mod key_pair;
  mod symmetric_key;

  pub use ids::*;
  pub use secret_data::SecretData;
  pub use key_pair::KeyPair;
  pub use symmetric_key::SymmetricKey;
  ```

#### Task 3.2: Entities Module Export
- **File**: `client/src/domain/entities/mod.rs`
- **Status**: [ ]
- **Dependencies**: Task 2.1-2.5
- **Description**: Entitiesのre-export
- **Implementation**:
  ```rust
  mod secret;
  mod share_collection;
  mod capsule;
  mod kfrag;
  mod cfrag;

  pub use secret::{Secret, SecretState};
  pub use share_collection::{ShareCollection, EncryptedShareData};
  pub use capsule::Capsule;
  pub use kfrag::KFrag;
  pub use cfrag::CFrag;
  ```

#### Task 3.3: Domain Module Export
- **File**: `client/src/domain/mod.rs`
- **Status**: [ ]
- **Dependencies**: Task 1.5, Task 3.1, Task 3.2
- **Description**: Domain層全体のre-export
- **Implementation**:
  ```rust
  pub mod entities;
  pub mod value_objects;
  pub mod errors;

  pub use errors::DomainError;
  ```

### Group 4: Testing

#### Task 4.1: Value Objects Unit Tests
- **File**: `client/src/domain/value_objects/` (各ファイル内 or tests/)
- **Status**: [ ]
- **Dependencies**: Group 1完了
- **Description**: Value Objectsのユニットテスト
- **Test Cases**:
  - ID生成・比較テスト
  - SecretData: 有効/無効入力テスト
  - KeyPair: 生成・アクセステスト
  - SymmetricKey: 生成・アクセステスト

#### Task 4.2: Entities Unit Tests
- **File**: `client/src/domain/entities/` (各ファイル内 or tests/)
- **Status**: [ ]
- **Dependencies**: Group 2完了
- **Description**: Entitiesのユニットテスト
- **Test Cases**:
  - Secret: 状態遷移テスト、無効threshold拒否
  - ShareCollection: シェア取得、カウント不一致エラー
  - Capsule: 空データ拒否
  - KFrag/CFrag: Zeroize確認

## Task Execution Order

```
Phase 1: Foundation
├── Task 1.1 (IDs)
├── Task 1.2 (SecretData)
├── Task 1.3 (KeyPair)
├── Task 1.4 (SymmetricKey)
└── Task 1.5 (DomainError)

Phase 2: Entities (Task 1.1 + 1.5 完了後)
├── Task 2.1 (Secret)
├── Task 2.2 (ShareCollection)
├── Task 2.3 (Capsule)
├── Task 2.4 (KFrag)
└── Task 2.5 (CFrag)

Phase 3: Integration (Group 1 + 2 完了後)
├── Task 3.1 (value_objects/mod.rs)
├── Task 3.2 (entities/mod.rs)
└── Task 3.3 (domain/mod.rs)

Phase 4: Testing (Group 3 完了後)
├── Task 4.1 (Value Objects Tests)
└── Task 4.2 (Entities Tests)
```

## Completion Checklist

全タスク完了後、以下を確認:
- `make check` 成功
- `make lint` 成功
- `make test` 成功
