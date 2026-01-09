# Tasks Document: client-controller

## Overview

D-TPRESクライアントライブラリのController層の実装タスク。Actions層からの入力を受け取り、検証（Validator）とDTO変換（Extractor）を行った後、UseCase層（Workflow Services）に処理を委譲する。

## Implementation Order

Design文書で定義された実装順序に従う：
1. ValidationError定義
2. ShareValidator実装
3. RecoverValidator実装
4. ShareExtractor実装
5. RecoverExtractor実装
6. ControllerContainer実装
7. mod.rsでのエクスポート
8. ユニットテスト

---

- [x] 1. ValidationError定義（error.rs）
  - File: `client/src/controller/error.rs`
  - Controller層固有のバリデーションエラー型を定義
  - エラーコード、メッセージ、フィールド名を保持
  - WorkflowErrorへの変換（From trait）を実装
  - エラーコード定数（error_codes module）を定義
  - Purpose: Controller層の早期検証エラーを構造化して表現
  - _Leverage: `client/src/usecase/error.rs` (WorkflowError)_
  - _Requirements: 5.1, 5.2, 5.3, 5.4_
  - _Prompt: Implement the task for spec client-controller, first run spec-workflow-guide to get the workflow guide then implement the task:

    **Role:** Rust Developer specializing in error handling and type design

    **Task:** Create ValidationError struct and error_codes module in `client/src/controller/error.rs` following the design document. Implement:
    - ValidationError struct with code, message, field (Option) fields (all private with getters)
    - Constructor methods: `new()` and `with_field()`
    - Display and Error trait implementations
    - From<ValidationError> for WorkflowError conversion
    - error_codes module with constants: SECRET_EMPTY, INVALID_THRESHOLD, THRESHOLD_EXCEEDS_TOTAL, TOTAL_SHARES_EXCEEDS_MAX, THRESHOLD_BELOW_MIN, INVALID_OWNER_KEY, INVALID_REQUESTER_KEY, INVALID_SECRET_ID, INVALID_PROCESS_ID
    - Validation constants: MIN_THRESHOLD (2), MAX_SHARES (20)

    **Restrictions:**
    - Do not include secret information in error messages
    - Follow existing WorkflowError patterns from usecase/error.rs
    - Do not add serde derives to the error type
    - Keep error messages user-friendly and consistent

    **_Leverage:**
    - `client/src/usecase/error.rs` for WorkflowError definition

    **_Requirements:** 5.1, 5.2, 5.3, 5.4

    **Success:**
    - ValidationError compiles without errors
    - From<ValidationError> for WorkflowError works correctly
    - All error codes are defined as constants
    - `make check` and `make lint` pass

    **Instructions:**
    1. Before starting, mark this task as in-progress in tasks.md by changing `[ ]` to `[-]`
    2. After completing implementation, use log-implementation tool to record implementation details
    3. Then mark this task as complete in tasks.md by changing `[-]` to `[x]`_

---

- [x] 2. ShareValidator実装（validator.rs）
  - File: `client/src/controller/validator.rs`
  - 秘密分割リクエストの入力パラメータを検証
  - 検証項目：secret空チェック、threshold範囲、total_shares範囲、k<=n、鍵有効性
  - 検証順序は決定論的（同じ入力に対して同じ最初のエラーを返す）
  - Purpose: DTO構築前の早期検証でWorkflowServiceの負荷を軽減
  - _Leverage: `client/src/controller/error.rs`, `client/src/service/core/crypto.rs` (SecretKey, PublicKey)_
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6_
  - _Prompt: Implement the task for spec client-controller, first run spec-workflow-guide to get the workflow guide then implement the task:

    **Role:** Rust Developer with expertise in validation logic and cryptographic types

    **Task:** Create ShareValidator struct in `client/src/controller/validator.rs` following the design document. Implement:
    - ShareValidator struct (unit struct)
    - `new()` constructor and Default trait
    - `validate()` method with parameters: secret (&[u8]), threshold (u8), total_shares (u8), owner_secret_key (&SecretKey), requester_public_key (&PublicKey)
    - Validation order (deterministic):
      1. secret empty check -> SECRET_EMPTY
      2. threshold > 0 check -> INVALID_THRESHOLD
      3. threshold >= MIN_THRESHOLD check -> THRESHOLD_BELOW_MIN
      4. total_shares <= MAX_SHARES check -> TOTAL_SHARES_EXCEEDS_MAX
      5. threshold <= total_shares check -> THRESHOLD_EXCEEDS_TOTAL

    **Restrictions:**
    - Do not validate cryptographic key internals (umbral-pre handles this)
    - Follow validation order exactly as specified
    - Return first error found (fail-fast)
    - Do not add async/await (AO stateless constraint)

    **_Leverage:**
    - `client/src/controller/error.rs` for ValidationError and error_codes
    - `client/src/service/core/crypto.rs` for SecretKey, PublicKey types

    **_Requirements:** 1.1, 1.2, 1.3, 1.4, 1.5, 1.6

    **Success:**
    - All 6 acceptance criteria from requirements are met
    - Validation order is deterministic
    - `make check` and `make lint` pass

    **Instructions:**
    1. Before starting, mark this task as in-progress in tasks.md by changing `[ ]` to `[-]`
    2. After completing implementation, use log-implementation tool to record implementation details
    3. Then mark this task as complete in tasks.md by changing `[-]` to `[x]`_

---

- [x] 3. RecoverValidator実装（validator.rs）
  - File: `client/src/controller/validator.rs` (continue from task 2)
  - 秘密復元リクエストの入力パラメータを検証
  - 検証項目：secret_id空チェック、requester_secret_key、requester_process_id空チェック
  - Purpose: 復元リクエストの早期検証
  - _Leverage: `client/src/controller/error.rs`, `client/src/service/core/crypto.rs`_
  - _Requirements: 2.1, 2.2, 2.3, 2.4_
  - _Prompt: Implement the task for spec client-controller, first run spec-workflow-guide to get the workflow guide then implement the task:

    **Role:** Rust Developer with expertise in validation logic

    **Task:** Add RecoverValidator struct to `client/src/controller/validator.rs` following the design document. Implement:
    - RecoverValidator struct (unit struct)
    - `new()` constructor and Default trait
    - `validate()` method with parameters: secret_id (&str), requester_secret_key (&SecretKey), requester_process_id (&str)
    - Validation order:
      1. secret_id empty check -> INVALID_SECRET_ID
      2. requester_process_id empty check -> INVALID_PROCESS_ID

    **Restrictions:**
    - Do not validate cryptographic key internals
    - Keep consistent validation style with ShareValidator
    - Do not add async/await

    **_Leverage:**
    - `client/src/controller/error.rs` for ValidationError and error_codes
    - `client/src/service/core/crypto.rs` for SecretKey type

    **_Requirements:** 2.1, 2.2, 2.3, 2.4

    **Success:**
    - All 4 acceptance criteria from requirements are met
    - Validation is consistent with ShareValidator style
    - `make check` and `make lint` pass

    **Instructions:**
    1. Before starting, mark this task as in-progress in tasks.md by changing `[ ]` to `[-]`
    2. After completing implementation, use log-implementation tool to record implementation details
    3. Then mark this task as complete in tasks.md by changing `[-]` to `[x]`_

---

- [x] 4. ShareExtractor実装（extractor.rs）
  - File: `client/src/controller/extractor.rs`
  - 検証済みパラメータからSecretSharingRequest DTOを生成
  - 既存のSecretSharingRequest構造体（usecase/dto.rs）をそのまま使用
  - Purpose: Actions層からの生パラメータをWorkflowService向けDTOに変換
  - _Leverage: `client/src/usecase/dto.rs` (SecretSharingRequest, SecretMetadata)_
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5_
  - _Prompt: Implement the task for spec client-controller, first run spec-workflow-guide to get the workflow guide then implement the task:

    **Role:** Rust Developer with expertise in DTO patterns and data transformation

    **Task:** Create ShareExtractor struct in `client/src/controller/extractor.rs` following the design document. Implement:
    - ShareExtractor struct (unit struct)
    - `new()` constructor and Default trait
    - `extract()` method that creates SecretSharingRequest from parameters:
      - secret: Vec<u8>
      - owner_secret_key: SecretKey
      - owner_public_key: PublicKey
      - requester_public_key: PublicKey
      - threshold: u8
      - total_shares: u8
      - owner_process_id: String
      - metadata: Option<SecretMetadata>

    **Restrictions:**
    - Do not create new DTO types - use existing SecretSharingRequest from usecase/dto.rs
    - Do not perform validation in extractor (that's validator's job)
    - Avoid unnecessary data copies

    **_Leverage:**
    - `client/src/usecase/dto.rs` for SecretSharingRequest and SecretMetadata

    **_Requirements:** 3.1, 3.2, 3.3, 3.4, 3.5

    **Success:**
    - All 5 acceptance criteria from requirements are met
    - Uses existing SecretSharingRequest DTO
    - `make check` and `make lint` pass

    **Instructions:**
    1. Before starting, mark this task as in-progress in tasks.md by changing `[ ]` to `[-]`
    2. After completing implementation, use log-implementation tool to record implementation details
    3. Then mark this task as complete in tasks.md by changing `[-]` to `[x]`_

---

- [x] 5. RecoverExtractor実装（extractor.rs）
  - File: `client/src/controller/extractor.rs` (continue from task 4)
  - 検証済みパラメータからSecretRecoveryRequest DTOを生成
  - 既存のSecretRecoveryRequest構造体（usecase/dto.rs）をそのまま使用
  - Purpose: 復元リクエストのDTO変換
  - _Leverage: `client/src/usecase/dto.rs` (SecretRecoveryRequest), `client/src/domain/value_objects/ids.rs` (SecretId)_
  - _Requirements: 4.1, 4.2, 4.3, 4.4_
  - _Prompt: Implement the task for spec client-controller, first run spec-workflow-guide to get the workflow guide then implement the task:

    **Role:** Rust Developer with expertise in DTO patterns

    **Task:** Add RecoverExtractor struct to `client/src/controller/extractor.rs` following the design document. Implement:
    - RecoverExtractor struct (unit struct)
    - `new()` constructor and Default trait
    - `extract()` method that creates SecretRecoveryRequest from parameters:
      - secret_id: SecretId
      - requester_secret_key: SecretKey
      - requester_process_id: String

    **Restrictions:**
    - Do not create new DTO types - use existing SecretRecoveryRequest
    - Do not perform validation in extractor
    - Keep consistent style with ShareExtractor

    **_Leverage:**
    - `client/src/usecase/dto.rs` for SecretRecoveryRequest
    - `client/src/domain/value_objects/ids.rs` for SecretId

    **_Requirements:** 4.1, 4.2, 4.3, 4.4

    **Success:**
    - All 4 acceptance criteria from requirements are met
    - Uses existing SecretRecoveryRequest DTO
    - `make check` and `make lint` pass

    **Instructions:**
    1. Before starting, mark this task as in-progress in tasks.md by changing `[ ]` to `[-]`
    2. After completing implementation, use log-implementation tool to record implementation details
    3. Then mark this task as complete in tasks.md by changing `[-]` to `[x]`_

---

- [x] 6. ControllerContainer実装（di.rs）
  - File: `client/src/controller/di.rs`
  - Controller層コンポーネントの依存性注入コンテナ
  - ShareValidator, RecoverValidator, ShareExtractor, RecoverExtractorをまとめて管理
  - Purpose: Actions層からのController層コンポーネントへの簡易アクセス
  - _Leverage: Task 1-5で作成したコンポーネント_
  - _Requirements: 6.1, 6.2, 6.3, 6.4_
  - _Prompt: Implement the task for spec client-controller, first run spec-workflow-guide to get the workflow guide then implement the task:

    **Role:** Rust Developer with expertise in dependency injection patterns

    **Task:** Create ControllerContainer struct in `client/src/controller/di.rs` following the design document. Implement:
    - ControllerContainer struct with fields:
      - share_validator: ShareValidator
      - recover_validator: RecoverValidator
      - share_extractor: ShareExtractor
      - recover_extractor: RecoverExtractor
    - `new()` constructor that creates all components with defaults
    - Default trait implementation
    - Getter methods:
      - `share_validator(&self) -> &ShareValidator`
      - `recover_validator(&self) -> &RecoverValidator`
      - `share_extractor(&self) -> &ShareExtractor`
      - `recover_extractor(&self) -> &RecoverExtractor`

    **Restrictions:**
    - Keep the container simple (no complex lifecycle management)
    - All components should be created with their Default implementations
    - Do not add Arc/Mutex unless needed

    **_Leverage:**
    - Components from tasks 1-5

    **_Requirements:** 6.1, 6.2, 6.3, 6.4

    **Success:**
    - All 4 acceptance criteria from requirements are met
    - Container provides access to all components
    - `make check` and `make lint` pass

    **Instructions:**
    1. Before starting, mark this task as in-progress in tasks.md by changing `[ ]` to `[-]`
    2. After completing implementation, use log-implementation tool to record implementation details
    3. Then mark this task as complete in tasks.md by changing `[-]` to `[x]`_

---

- [x] 7. mod.rsエクスポート設定
  - File: `client/src/controller/mod.rs`
  - 全Controllerコンポーネントの公開エクスポートを設定
  - Purpose: Controller層の公開インターフェースを定義
  - _Leverage: Task 1-6で作成したファイル_
  - _Requirements: All_
  - _Prompt: Implement the task for spec client-controller, first run spec-workflow-guide to get the workflow guide then implement the task:

    **Role:** Rust Developer with expertise in module organization

    **Task:** Create or update `client/src/controller/mod.rs` to export all Controller layer components:
    - Declare submodules: error, validator, extractor, di
    - Re-export public types:
      - From error: ValidationError, error_codes, MIN_THRESHOLD, MAX_SHARES
      - From validator: ShareValidator, RecoverValidator
      - From extractor: ShareExtractor, RecoverExtractor
      - From di: ControllerContainer

    **Restrictions:**
    - Only export types that are meant to be public
    - Follow existing module patterns in the codebase
    - Keep the module structure flat for easy imports

    **_Leverage:**
    - Files created in tasks 1-6

    **_Requirements:** All requirements

    **Success:**
    - All Controller components are accessible via `crate::controller::*`
    - `make check` and `make lint` pass
    - Module organization is clean and intuitive

    **Instructions:**
    1. Before starting, mark this task as in-progress in tasks.md by changing `[ ]` to `[-]`
    2. After completing implementation, use log-implementation tool to record implementation details
    3. Then mark this task as complete in tasks.md by changing `[-]` to `[x]`_

---

- [x] 8. ユニットテスト実装
  - File: `client/src/controller/` (各モジュールの#[cfg(test)]セクション)
  - 全コンポーネントのユニットテストを実装
  - テストパターン：Success, Validation, Type Safety
  - Purpose: Controller層の信頼性確保とリグレッション防止
  - _Leverage: 既存テストパターン、テストヘルパー_
  - _Requirements: All Test Coverage items_
  - _Prompt: Implement the task for spec client-controller, first run spec-workflow-guide to get the workflow guide then implement the task:

    **Role:** QA Engineer with expertise in Rust unit testing

    **Task:** Add unit tests to Controller layer modules following the test coverage defined in requirements.md:

    **error.rs tests:**
    - test_validation_error_code_message
    - test_validation_error_with_field
    - test_validation_error_to_workflow_error
    - test_validation_error_display

    **validator.rs tests (ShareValidator):**
    - test_share_validator_empty_secret
    - test_share_validator_zero_threshold
    - test_share_validator_threshold_exceeds_total
    - test_share_validator_empty_owner_key (if applicable)
    - test_share_validator_empty_requester_key (if applicable)
    - test_share_validator_valid_params
    - test_share_validator_total_shares_exceeds_max
    - test_share_validator_threshold_below_min

    **validator.rs tests (RecoverValidator):**
    - test_recover_validator_empty_secret_id
    - test_recover_validator_empty_requester_key (if applicable)
    - test_recover_validator_empty_process_id
    - test_recover_validator_valid_params

    **extractor.rs tests:**
    - test_share_extractor_creates_request
    - test_share_extractor_secret_preserved
    - test_share_extractor_threshold_params
    - test_share_extractor_owner_keys
    - test_share_extractor_requester_key
    - test_share_extractor_with_metadata
    - test_recover_extractor_creates_request
    - test_recover_extractor_secret_id
    - test_recover_extractor_requester_key
    - test_recover_extractor_process_id

    **di.rs tests:**
    - test_container_new
    - test_container_share_validator
    - test_container_recover_validator
    - test_container_share_extractor
    - test_container_recover_extractor

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
    - All 31 test functions from requirements are implemented
    - `make test` passes all tests
    - Coverage meets 100% for acceptance criteria

    **Instructions:**
    1. Before starting, mark this task as in-progress in tasks.md by changing `[ ]` to `[-]`
    2. After completing implementation, use log-implementation tool to record implementation details
    3. Then mark this task as complete in tasks.md by changing `[-]` to `[x]`_

---

## Task Summary

| Task | Component | File | Requirements |
|------|-----------|------|--------------|
| 1 | ValidationError | controller/error.rs | 5.1-5.4 |
| 2 | ShareValidator | controller/validator.rs | 1.1-1.6 |
| 3 | RecoverValidator | controller/validator.rs | 2.1-2.4 |
| 4 | ShareExtractor | controller/extractor.rs | 3.1-3.5 |
| 5 | RecoverExtractor | controller/extractor.rs | 4.1-4.4 |
| 6 | ControllerContainer | controller/di.rs | 6.1-6.4 |
| 7 | Module exports | controller/mod.rs | All |
| 8 | Unit tests | controller/*.rs | All Test Coverage |

## Dependencies

### Required Before Implementation

1. `client/src/usecase/dto.rs` - SecretSharingRequest, SecretRecoveryRequest
2. `client/src/usecase/error.rs` - WorkflowError
3. `client/src/domain/value_objects/ids.rs` - SecretId
4. `client/src/service/core/crypto.rs` - SecretKey, PublicKey

### Task Dependencies

```
Task 1 (ValidationError) - no dependencies
    ↓
Task 2 (ShareValidator) - depends on Task 1
Task 3 (RecoverValidator) - depends on Task 1
    ↓
Task 4 (ShareExtractor) - no Controller dependencies
Task 5 (RecoverExtractor) - no Controller dependencies
    ↓
Task 6 (ControllerContainer) - depends on Tasks 2-5
    ↓
Task 7 (mod.rs) - depends on Tasks 1-6
    ↓
Task 8 (Tests) - depends on Tasks 1-7
```
