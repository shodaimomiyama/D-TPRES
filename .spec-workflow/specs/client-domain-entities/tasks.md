# Tasks: Client Domain Entities

## Overview

承認済みのRequirements/Designに基づき、`client/src/domain/entities/` および `client/src/domain/value_objects/` の実装タスク。

## Group 1: Foundation (Value Objects & Errors)

- [x] 1. ID Value Objects
  - File: client/src/domain/value_objects/ids.rs
  - Description: 型安全なID Value Objectsを実装 (SecretId, ShareCollectionId, CapsuleId, KFragId, CFragId)
  - Implementation: newtype pattern, Derive Debug/Clone/PartialEq/Eq/Hash, Methods: new(), generate(), as_str()
  - _Leverage: zeroize crate_
  - _Requirements: REQ-1.1_
  - _Prompt: Role: Rust Developer specializing in DDD and type systems | Task: Create type-safe ID value objects using newtype pattern with UUID generation for WASM environment | Restrictions: No external UUID crates for WASM compatibility, implement custom UUID generator with timestamp+counter | Success: All ID types are distinct at compile time, generate() produces unique IDs, tests pass_

- [x] 2. SecretData Value Object
  - File: client/src/domain/value_objects/secret_data.rs
  - Description: 秘密データ f(0)=secret を表現するValue Object with Zeroize
  - Implementation: Derive Zeroize/ZeroizeOnDrop, Methods: new(), as_bytes(), len(), is_empty(), No Clone
  - _Leverage: zeroize crate_
  - _Requirements: REQ-1.2_
  - _Prompt: Role: Rust Security Developer | Task: Create SecretData value object with secure memory handling using Zeroize | Restrictions: Do NOT implement Clone, must clear memory on drop | Success: Empty data returns DomainError, Zeroize executes on drop, debug output is redacted_

- [x] 3. KeyPair Value Object
  - File: client/src/domain/value_objects/key_pair.rs
  - Description: PRE鍵ペア（skₒ, pkₒ）を表現するValue Object with Zeroize on secret_key
  - Implementation: Zeroize only secret_key, Methods: generate(), public_key(), secret_key(), No Clone
  - _Leverage: zeroize crate, umbral-pre crate_
  - _Requirements: REQ-1.3_
  - _Prompt: Role: Cryptography Developer | Task: Create KeyPair value object for PRE keys with selective Zeroize on secret_key only | Restrictions: No Clone trait, only secret_key gets zeroized | Success: Key generation works, secret_key zeroized on drop, public_key accessible_

- [x] 4. SymmetricKey Value Object
  - File: client/src/domain/value_objects/symmetric_key.rs
  - Description: AES-256共通鍵 kₒ を表現するValue Object with Zeroize
  - Implementation: 32-byte key array, Derive Zeroize/ZeroizeOnDrop, Methods: generate(), as_bytes(), No Clone
  - _Leverage: zeroize crate_
  - _Requirements: REQ-1.4_
  - _Prompt: Role: Cryptography Developer | Task: Create SymmetricKey value object for AES-256 with Zeroize | Restrictions: Fixed 32-byte size, no Clone | Success: 256-bit key generation, Zeroize on drop, tests pass_

- [x] 5. DomainError Extension
  - File: client/src/domain/errors.rs
  - Description: Domain層エラー型を拡張
  - Implementation: thiserror::Error, variants: InvalidThreshold, InvalidIndex, EmptyData, InvalidStateTransition, NotFound, ShareCountMismatch, EntityValidation
  - _Leverage: thiserror crate_
  - _Requirements: REQ-1.5_
  - _Prompt: Role: Rust Developer | Task: Extend DomainError enum with all required variants using thiserror | Restrictions: Must use thiserror for error implementation | Success: All error variants defined with proper messages_

## Group 2: Entities

- [x] 6. Secret Entity (Aggregate Root)
  - File: client/src/domain/entities/secret.rs
  - Description: 秘密のメタデータと状態を管理する集約ルート with state machine
  - Implementation: SecretState enum (Initialized→Split→Distributed→Recovered), Methods: new(), split(), distribute(), getters
  - Dependencies: Task 1, Task 5
  - _Leverage: client/src/domain/value_objects/ids.rs, client/src/domain/errors.rs_
  - _Requirements: REQ-2.1_
  - _Prompt: Role: DDD Developer | Task: Create Secret aggregate root with state machine pattern for lifecycle management | Restrictions: Private fields with getter methods, validate threshold k<=n | Success: Invalid threshold returns DomainError, state transitions work correctly_

- [x] 7. ShareCollection Entity
  - File: client/src/domain/entities/share.rs
  - Description: n個の暗号化シェアを一括管理するEntity
  - Implementation: ShareCollection + EncryptedShareData structs, Methods: new(), get_share(), get_shares_by_indices(), shares_count(), set_arweave_tx_id()
  - Dependencies: Task 1, Task 5
  - _Leverage: client/src/domain/value_objects/ids.rs, client/src/domain/errors.rs_
  - _Requirements: REQ-2.2_
  - _Prompt: Role: DDD Developer | Task: Create ShareCollection entity managing n encrypted Shamir shares | Restrictions: Validate share count matches n, private fields | Success: Share count mismatch returns DomainError, index-based retrieval works_

- [x] 8. Capsule Entity
  - File: client/src/domain/entities/capsule.rs
  - Description: Umbral PREカプセルを表現するEntity
  - Implementation: Private fields, Methods: new(), getters, set_arweave_tx_id()
  - Dependencies: Task 1, Task 5
  - _Leverage: client/src/domain/value_objects/ids.rs, client/src/domain/errors.rs_
  - _Requirements: REQ-2.3_
  - _Prompt: Role: DDD Developer | Task: Create Capsule entity for Umbral PRE capsule data | Restrictions: Empty capsule_data returns DomainError, private fields | Success: Empty data validation works, getters return correct values_

- [x] 9. KFrag Entity
  - File: client/src/domain/entities/kfrag.rs
  - Description: 再暗号化鍵フラグメントを表現するEntity with Zeroize
  - Implementation: Derive Zeroize/ZeroizeOnDrop, Methods: new(), getters, set_holder_process_id()
  - Dependencies: Task 1, Task 5
  - _Leverage: client/src/domain/value_objects/ids.rs, client/src/domain/errors.rs, zeroize crate_
  - _Requirements: REQ-2.4_
  - _Prompt: Role: Cryptography DDD Developer | Task: Create KFrag entity with Zeroize for secure memory handling | Restrictions: kfrag_data must be zeroized on drop, invalid holder_index returns error | Success: Zeroize on drop, validation works_

- [x] 10. CFrag Entity
  - File: client/src/domain/entities/cfrag.rs
  - Description: 再暗号化フラグメントを表現するEntity with Zeroize
  - Implementation: Derive Zeroize/ZeroizeOnDrop, Methods: new(), getters, verify()
  - Dependencies: Task 1, Task 5
  - _Leverage: client/src/domain/value_objects/ids.rs, client/src/domain/errors.rs, zeroize crate_
  - _Requirements: REQ-2.5_
  - _Prompt: Role: Cryptography DDD Developer | Task: Create CFrag entity with Zeroize for re-encrypted fragments | Restrictions: cfrag_data must be zeroized on drop | Success: Zeroize on drop, all getters work correctly_

## Group 3: Module Integration

- [x] 11. Value Objects Module Export
  - File: client/src/domain/value_objects/mod.rs
  - Description: Value Objectsのre-export
  - Dependencies: Tasks 1-4
  - _Leverage: Rust module system_
  - _Requirements: REQ-3.1_
  - _Prompt: Role: Rust Developer | Task: Configure module exports for all value objects | Restrictions: Follow Rust re-export conventions | Success: All value objects accessible from domain::value_objects_

- [x] 12. Entities Module Export
  - File: client/src/domain/entities/mod.rs
  - Description: Entitiesのre-export
  - Dependencies: Tasks 6-10
  - _Leverage: Rust module system_
  - _Requirements: REQ-3.2_
  - _Prompt: Role: Rust Developer | Task: Configure module exports for all entities | Restrictions: Follow Rust re-export conventions | Success: All entities accessible from domain::entities_

- [x] 13. Domain Module Export
  - File: client/src/domain/mod.rs
  - Description: Domain層全体のre-export
  - Dependencies: Tasks 5, 11, 12
  - _Leverage: Rust module system_
  - _Requirements: REQ-3.3_
  - _Prompt: Role: Rust Developer | Task: Configure domain module exports for public API | Restrictions: Re-export commonly used types at domain level | Success: Public API is clean and accessible_

## Group 4: Testing

- [x] 14. Value Objects Unit Tests
  - File: client/src/domain/value_objects/ (inline tests)
  - Description: Value Objectsのユニットテスト
  - Test Cases: ID generation/comparison, SecretData validation, KeyPair generation, SymmetricKey generation
  - Dependencies: Group 1 complete
  - _Leverage: Rust #[cfg(test)] inline tests_
  - _Requirements: REQ-4.1_
  - _Prompt: Role: Rust Test Developer | Task: Write comprehensive unit tests for all value objects | Restrictions: Inline tests using #[cfg(test)], test both valid and invalid inputs | Success: All tests pass, edge cases covered_

- [x] 15. Entities Unit Tests
  - File: client/src/domain/entities/ (inline tests)
  - Description: Entitiesのユニットテスト
  - Test Cases: Secret state transitions, ShareCollection retrieval, Capsule validation, KFrag/CFrag Zeroize
  - Dependencies: Group 2 complete
  - _Leverage: Rust #[cfg(test)] inline tests_
  - _Requirements: REQ-4.2_
  - _Prompt: Role: Rust Test Developer | Task: Write comprehensive unit tests for all entities | Restrictions: Test state machines, validation, Zeroize behavior | Success: All 49 tests pass, state transitions verified_

## Task Execution Order

```
Phase 1: Foundation
├── Task 1 (IDs)
├── Task 2 (SecretData)
├── Task 3 (KeyPair)
├── Task 4 (SymmetricKey)
└── Task 5 (DomainError)

Phase 2: Entities (Tasks 1 + 5 完了後)
├── Task 6 (Secret)
├── Task 7 (ShareCollection)
├── Task 8 (Capsule)
├── Task 9 (KFrag)
└── Task 10 (CFrag)

Phase 3: Integration (Group 1 + 2 完了後)
├── Task 11 (value_objects/mod.rs)
├── Task 12 (entities/mod.rs)
└── Task 13 (domain/mod.rs)

Phase 4: Testing (Group 3 完了後)
├── Task 14 (Value Objects Tests)
└── Task 15 (Entities Tests)
```

## Completion Checklist

- [x] `make check` 成功
- [x] `make lint` 成功
- [x] `make test` 成功 (49 tests passed)
