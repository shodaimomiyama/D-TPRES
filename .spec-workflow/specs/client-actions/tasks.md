# Tasks Document: client-actions

## Overview

FORMIXクライアントライブラリのActions層の実装タスク。開発者向けFacadeとして、内部の複雑さを隠蔽し、`share()`、`recover()`、`generateKeyPair()` の3つのシンプルなAPIを提供する。

## Implementation Order

Design文書で定義された実装順序に従う：
1. ActionError定義
2. ShareOptions, RecoverOptions定義
3. ActionsContainer実装
4. share(), recover(), generateKeyPair() 実装
5. mod.rsでのエクスポート
6. ユニットテスト
7. 統合テスト

---

- [ ] 1. ActionError定義（error.rs）
  - File: `client/src/actions/error.rs`
  - Actions層固有のエラー型を定義
  - ValidationError、WorkflowErrorからの変換（From trait）を実装
  - 4つのバリアント：ValidationFailed, WorkflowFailed, ResourceNotFound, CryptoError
  - Purpose: 内部エラーを開発者フレンドリーな形式でラップ
  - _Leverage: `client/src/controller/error.rs` (ValidationError), `client/src/usecase/error.rs` (WorkflowError)_
  - _Requirements: 4.1, 4.2, 4.3, 4.4_
  - _Prompt: Implement the task for spec client-actions, first run spec-workflow-guide to get the workflow guide then implement the task:

    **Role:** Rust Developer specializing in error handling and type design

    **Task:** Create ActionError enum in `client/src/actions/error.rs` following the design document. Implement:
    - ActionError enum with 4 variants:
      - ValidationFailed { code: String, message: String }
      - WorkflowFailed { message: String }
      - ResourceNotFound { resource: String }
      - CryptoError { message: String }
    - Constructor methods: `validation_failed()`, `workflow_failed()`, `resource_not_found()`, `crypto_error()`
    - Display and Error trait implementations
    - From<ValidationError> for ActionError conversion
    - From<WorkflowError> for ActionError conversion (with pattern matching for ResourceNotFound)

    **Restrictions:**
    - Do not include secret information in error messages
    - Follow existing error patterns from controller/error.rs and usecase/error.rs
    - Keep error messages user-friendly and actionable
    - Do not add serde derives to the error type

    **_Leverage:**
    - `client/src/controller/error.rs` for ValidationError definition
    - `client/src/usecase/error.rs` for WorkflowError definition

    **_Requirements:** 4.1, 4.2, 4.3, 4.4

    **Success:**
    - ActionError compiles without errors
    - From<ValidationError> and From<WorkflowError> work correctly
    - Display formatting shows clear error messages
    - `make check` and `make lint` pass

    **Instructions:**
    1. Before starting, mark this task as in-progress in tasks.md by changing `[ ]` to `[-]`
    2. After completing implementation, use log-implementation tool to record implementation details
    3. Then mark this task as complete in tasks.md by changing `[-]` to `[x]`_

---

- [ ] 2. ShareOptions / RecoverOptions定義（options.rs）
  - File: `client/src/actions/options.rs`
  - オプションパラメータ用の構造体を定義
  - ShareOptions: metadata（Option<SecretMetadata>）
  - RecoverOptions: 将来の拡張用に予約（現在は空）
  - Purpose: 必須パラメータとオプションパラメータを明確に分離
  - _Leverage: `client/src/usecase/dto.rs` (SecretMetadata)_
  - _Requirements: 6.1, 6.2, 6.3_
  - _Prompt: Implement the task for spec client-actions, first run spec-workflow-guide to get the workflow guide then implement the task:

    **Role:** Rust Developer with expertise in API design patterns

    **Task:** Create options structs in `client/src/actions/options.rs` following the design document. Implement:
    - ShareOptions struct with:
      - `metadata: Option<SecretMetadata>` field (pub)
      - `new()` constructor returning default (metadata: None)
      - `with_metadata(metadata: SecretMetadata)` constructor
      - Default trait implementation
      - Debug, Clone derives
    - RecoverOptions struct with:
      - No fields (empty struct for future extension)
      - `new()` constructor
      - Default trait implementation
      - Debug, Clone derives

    **Restrictions:**
    - Keep structs simple and focused
    - Use pub fields for options (builder pattern not required)
    - Do not add validation in options (validation is in Controller layer)

    **_Leverage:**
    - `client/src/usecase/dto.rs` for SecretMetadata type

    **_Requirements:** 6.1, 6.2, 6.3

    **Success:**
    - Both option structs compile without errors
    - ShareOptions::default() returns metadata: None
    - ShareOptions::with_metadata() sets the metadata correctly
    - `make check` and `make lint` pass

    **Instructions:**
    1. Before starting, mark this task as in-progress in tasks.md by changing `[ ]` to `[-]`
    2. After completing implementation, use log-implementation tool to record implementation details
    3. Then mark this task as complete in tasks.md by changing `[-]` to `[x]`_

---

- [ ] 3. ActionsContainer実装（di.rs）
  - File: `client/src/actions/di.rs`
  - Actions層の依存性注入コンテナ
  - ControllerContainer, WorkflowServiceContainer, CryptoServiceをまとめて管理
  - share(), recover(), generateKeyPair() メソッドを提供
  - Purpose: Actions層コンポーネントへの簡易アクセスとAPI実行
  - _Leverage: `client/src/controller/di.rs` (ControllerContainer), `client/src/usecase/workflow/di.rs` (WorkflowServiceContainer), `client/src/usecase/core/crypto.rs` (CryptoService)_
  - _Requirements: 5.1, 5.2, 5.3, 5.4_
  - _Prompt: Implement the task for spec client-actions, first run spec-workflow-guide to get the workflow guide then implement the task:

    **Role:** Rust Developer with expertise in dependency injection and facade patterns

    **Task:** Create ActionsContainer struct in `client/src/actions/di.rs` following the design document. Implement:
    - ActionsContainer struct with fields:
      - controller: ControllerContainer
      - workflow_services: WorkflowServiceContainer
      - crypto_service: Arc<CryptoService>
    - `new()` constructor that creates all components with defaults
    - `with_dependencies()` constructor for testing (accepts custom dependencies)
    - Default trait implementation
    - Getter methods:
      - `controller(&self) -> &ControllerContainer`
      - `workflow_services(&self) -> &WorkflowServiceContainer`
      - `crypto_service(&self) -> &CryptoService`

    **Restrictions:**
    - Use Arc<CryptoService> for shared ownership
    - Keep the container simple (no complex lifecycle management)
    - Do not add async/await (AO stateless constraint)

    **_Leverage:**
    - `client/src/controller/di.rs` for ControllerContainer
    - `client/src/usecase/workflow/di.rs` for WorkflowServiceContainer
    - `client/src/usecase/core/crypto.rs` for CryptoService

    **_Requirements:** 5.1, 5.2, 5.3, 5.4

    **Success:**
    - Container provides access to all required components
    - All 4 acceptance criteria from requirements are met
    - `make check` and `make lint` pass

    **Instructions:**
    1. Before starting, mark this task as in-progress in tasks.md by changing `[ ]` to `[-]`
    2. After completing implementation, use log-implementation tool to record implementation details
    3. Then mark this task as complete in tasks.md by changing `[-]` to `[x]`_

---

- [ ] 4. share() 関数実装（di.rs）
  - File: `client/src/actions/di.rs` (continue from task 3)
  - 秘密分割APIの実装
  - フロー: Validate → Extract → Execute Workflow
  - Purpose: 開発者向けの秘密分割エントリーポイント
  - _Leverage: Controller層（ShareValidator, ShareExtractor）、UseCase層（SecretSharingWorkflowService）_
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 1.8_
  - _Prompt: Implement the task for spec client-actions, first run spec-workflow-guide to get the workflow guide then implement the task:

    **Role:** Rust Developer with expertise in facade pattern and orchestration

    **Task:** Add share() method to ActionsContainer in `client/src/actions/di.rs` following the design document. Implement:
    - `share()` method with parameters:
      - secret: Vec<u8>
      - threshold: u8
      - total_shares: u8
      - owner_secret_key: SecretKey
      - requester_public_key: PublicKey
      - owner_process_id: String
      - options: Option<ShareOptions>
    - Implementation flow:
      1. Derive owner_public_key from owner_secret_key
      2. Call controller.share_validator().validate()
      3. Call controller.share_extractor().extract()
      4. Call workflow_services.secret_sharing_service().execute()
      5. Return SecretSharingResult or ActionError

    **Restrictions:**
    - Use ? operator for error propagation (From traits handle conversion)
    - Do not add additional validation (use Controller layer)
    - Do not modify the DTOs (use existing types)
    - Do not add async/await

    **_Leverage:**
    - `client/src/controller/validator.rs` for ShareValidator
    - `client/src/controller/extractor.rs` for ShareExtractor
    - `client/src/usecase/workflow/secret_sharing_service.rs` for SecretSharingWorkflowService

    **_Requirements:** 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 1.8

    **Success:**
    - All 8 acceptance criteria from requirements are met
    - Validation errors are properly converted to ActionError::ValidationFailed
    - Workflow errors are properly converted to ActionError
    - `make check` and `make lint` pass

    **Instructions:**
    1. Before starting, mark this task as in-progress in tasks.md by changing `[ ]` to `[-]`
    2. After completing implementation, use log-implementation tool to record implementation details
    3. Then mark this task as complete in tasks.md by changing `[-]` to `[x]`_

---

- [ ] 5. recover() 関数実装（di.rs）
  - File: `client/src/actions/di.rs` (continue from task 4)
  - 秘密復元APIの実装
  - フロー: Validate → Extract → Execute Workflow
  - Purpose: 開発者向けの秘密復元エントリーポイント
  - _Leverage: Controller層（RecoverValidator, RecoverExtractor）、UseCase層（SecretRecoveryWorkflowService）_
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 2.6_
  - _Prompt: Implement the task for spec client-actions, first run spec-workflow-guide to get the workflow guide then implement the task:

    **Role:** Rust Developer with expertise in facade pattern and orchestration

    **Task:** Add recover() method to ActionsContainer in `client/src/actions/di.rs` following the design document. Implement:
    - `recover()` method with parameters:
      - secret_id: &str
      - requester_secret_key: SecretKey
      - requester_process_id: String
      - options: Option<RecoverOptions>
    - Implementation flow:
      1. Call controller.recover_validator().validate()
      2. Convert secret_id string to SecretId value object
      3. Call controller.recover_extractor().extract()
      4. Call workflow_services.secret_recovery_service().execute()
      5. Return SecretRecoveryResult or ActionError

    **Restrictions:**
    - Use ? operator for error propagation
    - Handle ResourceNotFound case from WorkflowError
    - Do not add additional validation
    - Do not add async/await

    **_Leverage:**
    - `client/src/controller/validator.rs` for RecoverValidator
    - `client/src/controller/extractor.rs` for RecoverExtractor
    - `client/src/usecase/workflow/secret_recovery_service.rs` for SecretRecoveryWorkflowService
    - `client/src/domain/value_objects/ids.rs` for SecretId

    **_Requirements:** 2.1, 2.2, 2.3, 2.4, 2.5, 2.6

    **Success:**
    - All 6 acceptance criteria from requirements are met
    - Non-existent secret returns ActionError::ResourceNotFound
    - Recovered secret matches original (verified in integration test)
    - `make check` and `make lint` pass

    **Instructions:**
    1. Before starting, mark this task as in-progress in tasks.md by changing `[ ]` to `[-]`
    2. After completing implementation, use log-implementation tool to record implementation details
    3. Then mark this task as complete in tasks.md by changing `[-]` to `[x]`_

---

- [ ] 6. generateKeyPair() 関数実装（di.rs）
  - File: `client/src/actions/di.rs` (continue from task 5)
  - PRE鍵ペア生成APIの実装
  - CryptoServiceのgenerate_keypair()をラップ
  - Purpose: 開発者向けの鍵ペア生成ユーティリティ
  - _Leverage: `client/src/usecase/core/crypto.rs` (CryptoService)_
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5_
  - _Prompt: Implement the task for spec client-actions, first run spec-workflow-guide to get the workflow guide then implement the task:

    **Role:** Rust Developer with expertise in cryptographic APIs

    **Task:** Add generate_keypair() method to ActionsContainer in `client/src/actions/di.rs` following the design document. Implement:
    - `generate_keypair()` method that returns Result<(SecretKey, PublicKey), ActionError>
    - Implementation:
      1. Call crypto_service.generate_keypair()
      2. Convert any error to ActionError::CryptoError
      3. Return the key pair

    **Restrictions:**
    - Use CryptoService for actual key generation (do not implement crypto directly)
    - Ensure SecretKey is Zeroize (handled by umbral-pre)
    - Do not log or expose secret key material in error messages

    **_Leverage:**
    - `client/src/usecase/core/crypto.rs` for CryptoService.generate_keypair()

    **_Requirements:** 3.1, 3.2, 3.3, 3.4, 3.5

    **Success:**
    - All 5 acceptance criteria from requirements are met
    - Secret key is 32 bytes
    - Public key is not empty
    - Each call returns different key pairs
    - `make check` and `make lint` pass

    **Instructions:**
    1. Before starting, mark this task as in-progress in tasks.md by changing `[ ]` to `[-]`
    2. After completing implementation, use log-implementation tool to record implementation details
    3. Then mark this task as complete in tasks.md by changing `[-]` to `[x]`_

---

- [ ] 7. mod.rsエクスポート設定
  - File: `client/src/actions/mod.rs`
  - 全Actionsコンポーネントの公開エクスポートを設定
  - Purpose: Actions層の公開インターフェースを定義
  - _Leverage: Task 1-6で作成したファイル_
  - _Requirements: All_
  - _Prompt: Implement the task for spec client-actions, first run spec-workflow-guide to get the workflow guide then implement the task:

    **Role:** Rust Developer with expertise in module organization

    **Task:** Create `client/src/actions/mod.rs` to export all Actions layer components:
    - Declare submodules: error, options, di
    - Re-export public types:
      - From error: ActionError
      - From options: ShareOptions, RecoverOptions
      - From di: ActionsContainer
    - Optionally add convenience functions that delegate to ActionsContainer:
      - `share()` function (creates container and calls share)
      - `recover()` function (creates container and calls recover)
      - `generate_keypair()` function (creates container and calls generate_keypair)

    **Restrictions:**
    - Only export types that are meant to be public API
    - Follow existing module patterns in the codebase
    - Keep the module structure flat for easy imports

    **_Leverage:**
    - Files created in tasks 1-6

    **_Requirements:** All requirements

    **Success:**
    - All Actions components are accessible via `crate::actions::*`
    - Convenience functions work correctly
    - `make check` and `make lint` pass
    - Module organization is clean and intuitive

    **Instructions:**
    1. Before starting, mark this task as in-progress in tasks.md by changing `[ ]` to `[-]`
    2. After completing implementation, use log-implementation tool to record implementation details
    3. Then mark this task as complete in tasks.md by changing `[-]` to `[x]`_

---

- [ ] 8. ユニットテスト実装
  - File: `client/src/actions/` (各モジュールの#[cfg(test)]セクション)
  - 全コンポーネントのユニットテストを実装
  - テストパターン：Success, Validation, Type Safety
  - Purpose: Actions層の信頼性確保とリグレッション防止
  - _Leverage: 既存テストパターン、テストヘルパー_
  - _Requirements: All Test Coverage items_
  - _Prompt: Implement the task for spec client-actions, first run spec-workflow-guide to get the workflow guide then implement the task:

    **Role:** QA Engineer with expertise in Rust unit testing

    **Task:** Add unit tests to Actions layer modules following the test coverage defined in requirements.md:

    **error.rs tests (Requirement 4):**
    - test_action_error_from_validation_error (AC 4.1)
    - test_action_error_from_workflow_error (AC 4.2)
    - test_action_error_display (AC 4.3)
    - test_action_error_validation_has_code (AC 4.4)

    **options.rs tests (Requirement 6):**
    - test_share_options_default (AC 6.3)
    - test_share_options_with_metadata (AC 6.1)
    - test_share_without_options (AC 6.2)

    **di.rs tests (Requirement 5):**
    - test_container_provides_share (AC 5.1)
    - test_container_provides_recover (AC 5.2)
    - test_container_provides_generate_keypair (AC 5.3)
    - test_container_initializes_dependencies (AC 5.4)
    - test_container_default

    **generateKeyPair tests (Requirement 3):**
    - test_generate_keypair_valid (AC 3.1)
    - test_generate_keypair_secret_key_size (AC 3.2)
    - test_generate_keypair_public_key_not_empty (AC 3.3)
    - test_generate_keypair_unique (AC 3.4)

    **Restrictions:**
    - Use #[cfg(test)] mod tests pattern
    - Do not test external dependencies directly
    - Create test helper functions for common setup
    - Test both success and failure scenarios

    **_Leverage:**
    - Existing test patterns in the codebase
    - Test helper utilities if available

    **_Requirements:** All Test Coverage items from requirements.md

    **Success:**
    - All test functions from requirements are implemented
    - `make test` passes all tests
    - Coverage meets requirements

    **Instructions:**
    1. Before starting, mark this task as in-progress in tasks.md by changing `[ ]` to `[-]`
    2. After completing implementation, use log-implementation tool to record implementation details
    3. Then mark this task as complete in tasks.md by changing `[-]` to `[x]`_

---

- [ ] 9. 統合テスト実装
  - File: `client/tests/actions_integration.rs` または `client/src/actions/` 内のintegration tests
  - share()とrecover()のend-to-endテスト
  - 完全な秘密分割→復元フローの検証
  - Purpose: Actions層全体の動作確認
  - _Leverage: 既存テストパターン_
  - _Requirements: 1.1, 2.6, 3.5 (Integration tests)_
  - _Prompt: Implement the task for spec client-actions, first run spec-workflow-guide to get the workflow guide then implement the task:

    **Role:** QA Engineer with expertise in integration testing

    **Task:** Create integration tests for Actions layer:

    **share() integration tests (Requirement 1):**
    - test_share_valid_params (AC 1.1)
    - test_share_empty_secret (AC 1.2)
    - test_share_zero_threshold (AC 1.3)
    - test_share_threshold_exceeds_total (AC 1.4)
    - test_share_empty_owner_key (AC 1.5)
    - test_share_empty_requester_key (AC 1.6)
    - test_share_result_has_valid_secret_id (AC 1.7)
    - test_share_result_kfrag_count_matches (AC 1.8)
    - test_share_with_metadata (Additional)
    - test_share_various_threshold_combinations (Additional)

    **recover() integration tests (Requirement 2):**
    - test_recover_valid_params (AC 2.1)
    - test_recover_empty_secret_id (AC 2.2)
    - test_recover_empty_requester_key (AC 2.3)
    - test_recover_empty_process_id (AC 2.4)
    - test_recover_nonexistent_secret (AC 2.5)
    - test_recover_returns_original_secret (AC 2.6)

    **Roundtrip tests:**
    - test_share_and_recover_roundtrip (AC 3.5 - keys usable with share/recover)
    - test_roundtrip_with_metadata
    - test_roundtrip_various_k_of_n

    **Restrictions:**
    - Use real components (not mocks) for integration tests
    - Clean up test data after each test
    - Test with realistic data sizes
    - Verify Zeroize behavior where applicable

    **_Leverage:**
    - Existing integration test patterns
    - ActionsContainer for test setup

    **_Requirements:** 1.1, 2.6, 3.5 and related ACs

    **Success:**
    - All integration test functions pass
    - Share→Recover roundtrip works correctly
    - Error cases are properly handled
    - `make test` passes all tests

    **Instructions:**
    1. Before starting, mark this task as in-progress in tasks.md by changing `[ ]` to `[-]`
    2. After completing implementation, use log-implementation tool to record implementation details
    3. Then mark this task as complete in tasks.md by changing `[-]` to `[x]`_

---

## Task Summary

| Task | Component | File | Requirements |
|------|-----------|------|--------------|
| 1 | ActionError | actions/error.rs | 4.1-4.4 |
| 2 | ShareOptions, RecoverOptions | actions/options.rs | 6.1-6.3 |
| 3 | ActionsContainer | actions/di.rs | 5.1-5.4 |
| 4 | share() | actions/di.rs | 1.1-1.8 |
| 5 | recover() | actions/di.rs | 2.1-2.6 |
| 6 | generateKeyPair() | actions/di.rs | 3.1-3.5 |
| 7 | Module exports | actions/mod.rs | All |
| 8 | Unit tests | actions/*.rs | All Unit Tests |
| 9 | Integration tests | tests/actions_integration.rs | Integration Tests |

## Dependencies

### Required Before Implementation

1. `client/src/controller/` - ControllerContainer, ShareValidator, RecoverValidator, ShareExtractor, RecoverExtractor, ValidationError
2. `client/src/usecase/workflow/` - WorkflowServiceContainer, SecretSharingWorkflowService, SecretRecoveryWorkflowService
3. `client/src/usecase/core/crypto.rs` - CryptoService, SecretKey, PublicKey
4. `client/src/usecase/dto.rs` - SecretSharingRequest, SecretSharingResult, SecretRecoveryRequest, SecretRecoveryResult, SecretMetadata
5. `client/src/usecase/error.rs` - WorkflowError
6. `client/src/domain/value_objects/ids.rs` - SecretId

### Task Dependencies

```
Task 1 (ActionError) - no dependencies within Actions layer
Task 2 (Options) - no dependencies within Actions layer
    ↓
Task 3 (ActionsContainer) - depends on Tasks 1, 2
    ↓
Task 4 (share) - depends on Task 3
Task 5 (recover) - depends on Task 3
Task 6 (generateKeyPair) - depends on Task 3
    ↓
Task 7 (mod.rs) - depends on Tasks 1-6
    ↓
Task 8 (Unit Tests) - depends on Tasks 1-7
    ↓
Task 9 (Integration Tests) - depends on Tasks 1-8
```
