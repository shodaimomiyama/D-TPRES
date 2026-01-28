# Requirements Document: client-controller

## Introduction

D-TPRESクライアントライブラリのController層は、Actions層からの入力を受け取り、UseCase層（Workflow Services）に処理を委譲するための中間層です。structure.mdに基づき、以下の2つのコンポーネントで構成されます：

- **Validator** (`validator.rs`): 入力の妥当性検証
- **Extractor** (`extractor.rs`): UseCase層向けDTO変換

## Alignment with Product Vision

### tech.md / structure.md との整合性

Controller層は6層Clean Architectureの一部として、以下の役割を担います：

```
Actions → Controller → UseCase → Domain
               ↓
          Repository ← Adapter (implements)
```

- **Actions層**: 開発者向けエンドポイント（`share()`, `recover()`, `generateKeyPair()`）
- **Controller層**: 入力検証、DTO変換
- **UseCase層**: Phase 1/3オーケストレーション（SecretSharingWorkflowService, SecretRecoveryWorkflowService）

### 既存実装との関係

- **SecretSharingRequest / SecretRecoveryRequest**: 既に `usecase/dto.rs` で定義済み
- **WorkflowError**: 既に `usecase/error.rs` で定義済み
- Controller層は Actions 層からの生の入力パラメータを受け取り、これらの既存DTOに変換する

## Requirements

### Requirement 1: ShareValidator - 秘密分割リクエストの検証

**User Story:** As a library developer, I want input validation for share operations, so that invalid parameters are rejected before creating DTOs.

#### Design Note

ShareValidatorは Actions 層の `share()` から渡される生のパラメータを検証します。検証成功後、Extractorが SecretSharingRequest DTO を生成します。

WorkflowService内の `validate_request()` はDTO構築後の検証であり、Controller層の検証はDTO構築前の早期検証を担います。

#### Acceptance Criteria

1. WHEN secret data is empty THEN validator SHALL return ValidationError with "secret_empty" code
2. WHEN threshold is zero THEN validator SHALL return ValidationError with "invalid_threshold" code
3. WHEN threshold exceeds total_shares THEN validator SHALL return ValidationError with "threshold_exceeds_total" code
4. WHEN owner_secret_key is empty THEN validator SHALL return ValidationError with "invalid_owner_key" code
5. WHEN requester_public_key is empty THEN validator SHALL return ValidationError with "invalid_requester_key" code
6. WHEN all parameters are valid THEN validator SHALL return Ok(())

#### Test Coverage

| AC# | Test Function(s) | Purpose | Pattern |
|-----|------------------|---------|---------|
| 1 | `test_share_validator_empty_secret` | Empty secret rejected | Validation |
| 2 | `test_share_validator_zero_threshold` | Zero threshold rejected | Validation |
| 3 | `test_share_validator_threshold_exceeds_total` | k > n rejected | Validation |
| 4 | `test_share_validator_empty_owner_key` | Empty owner key rejected | Validation |
| 5 | `test_share_validator_empty_requester_key` | Empty requester key rejected | Validation |
| 6 | `test_share_validator_valid_params` | Valid params accepted | Success |

**Additional Tests (Beyond AC):**
- `test_share_validator_total_shares_exceeds_max` - MAX_SHARES超過検証
- `test_share_validator_threshold_below_min` - MIN_THRESHOLD未満検証

---

### Requirement 2: RecoverValidator - 秘密復元リクエストの検証

**User Story:** As a library developer, I want input validation for recover operations, so that invalid parameters are rejected before creating DTOs.

#### Design Note

RecoverValidatorは Actions 層の `recover()` から渡される生のパラメータを検証します。

#### Acceptance Criteria

1. WHEN secret_id is empty THEN validator SHALL return ValidationError with "invalid_secret_id" code
2. WHEN requester_secret_key is empty THEN validator SHALL return ValidationError with "invalid_requester_key" code
3. WHEN requester_process_id is empty THEN validator SHALL return ValidationError with "invalid_process_id" code
4. WHEN all parameters are valid THEN validator SHALL return Ok(())

#### Test Coverage

| AC# | Test Function(s) | Purpose | Pattern |
|-----|------------------|---------|---------|
| 1 | `test_recover_validator_empty_secret_id` | Empty secret_id rejected | Validation |
| 2 | `test_recover_validator_empty_requester_key` | Empty requester key rejected | Validation |
| 3 | `test_recover_validator_empty_process_id` | Empty process_id rejected | Validation |
| 4 | `test_recover_validator_valid_params` | Valid params accepted | Success |

---

### Requirement 3: ShareExtractor - SecretSharingRequest DTO生成

**User Story:** As a library developer, I want automatic DTO construction from validated inputs, so that WorkflowService receives well-structured data.

#### Design Note

ShareExtractorは検証済みの生パラメータから `SecretSharingRequest` DTO を生成します。既存の `SecretSharingRequest` 構造体（`usecase/dto.rs`）をそのまま使用します。

#### Acceptance Criteria

1. WHEN extract() is called with valid parameters THEN it SHALL create SecretSharingRequest with all fields populated
2. WHEN extract() succeeds THEN secret field SHALL contain the original secret data
3. WHEN extract() succeeds THEN threshold and total_shares SHALL match input parameters
4. WHEN extract() succeeds THEN owner_secret_key and owner_public_key SHALL match input keys
5. WHEN extract() succeeds THEN requester_public_key SHALL match input key

#### Test Coverage

| AC# | Test Function(s) | Purpose | Pattern |
|-----|------------------|---------|---------|
| 1 | `test_share_extractor_creates_request` | DTO created with all fields | Success |
| 2 | `test_share_extractor_secret_preserved` | Secret data preserved | Success |
| 3 | `test_share_extractor_threshold_params` | Threshold params correct | Success |
| 4 | `test_share_extractor_owner_keys` | Owner keys preserved | Success |
| 5 | `test_share_extractor_requester_key` | Requester key preserved | Success |

**Additional Tests (Beyond AC):**
- `test_share_extractor_with_metadata` - Optional metadata handling

---

### Requirement 4: RecoverExtractor - SecretRecoveryRequest DTO生成

**User Story:** As a library developer, I want automatic DTO construction for recovery operations, so that WorkflowService receives well-structured data.

#### Design Note

RecoverExtractorは検証済みの生パラメータから `SecretRecoveryRequest` DTO を生成します。既存の `SecretRecoveryRequest` 構造体（`usecase/dto.rs`）をそのまま使用します。

#### Acceptance Criteria

1. WHEN extract() is called with valid parameters THEN it SHALL create SecretRecoveryRequest
2. WHEN extract() succeeds THEN secret_id SHALL match input
3. WHEN extract() succeeds THEN requester_secret_key SHALL match input
4. WHEN extract() succeeds THEN requester_process_id SHALL match input

#### Test Coverage

| AC# | Test Function(s) | Purpose | Pattern |
|-----|------------------|---------|---------|
| 1 | `test_recover_extractor_creates_request` | DTO created | Success |
| 2 | `test_recover_extractor_secret_id` | Secret ID preserved | Success |
| 3 | `test_recover_extractor_requester_key` | Requester key preserved | Success |
| 4 | `test_recover_extractor_process_id` | Process ID preserved | Success |

---

### Requirement 5: ValidationError - Controller層エラー型

**User Story:** As a library developer, I want structured validation errors, so that error handling is consistent and informative.

#### Design Note

Controller層固有の `ValidationError` を定義します。WorkflowError（UseCase層）とは別に、Controller層の早期検証エラーを表現します。ValidationErrorは WorkflowError::ValidationError への変換が可能である必要があります。

#### Acceptance Criteria

1. WHEN ValidationError is created THEN it SHALL contain error code and message
2. WHEN ValidationError is created THEN it SHALL optionally contain field name
3. WHEN ValidationError is converted to WorkflowError THEN it SHALL become WorkflowError::ValidationError
4. WHEN ValidationError is displayed THEN it SHALL show code and message

#### Test Coverage

| AC# | Test Function(s) | Purpose | Pattern |
|-----|------------------|---------|---------|
| 1 | `test_validation_error_code_message` | Error has code and message | Success |
| 2 | `test_validation_error_with_field` | Error with field name | Success |
| 3 | `test_validation_error_to_workflow_error` | Conversion to WorkflowError | Type Safety |
| 4 | `test_validation_error_display` | Display formatting | Success |

---

### Requirement 6: ControllerContainer - 依存性注入コンテナ

**User Story:** As a library developer, I want a DI container for controller components, so that dependencies can be easily managed.

#### Design Note

ControllerContainerは ShareValidator, RecoverValidator, ShareExtractor, RecoverExtractor をまとめて管理し、Actions層からの利用を容易にします。

#### Acceptance Criteria

1. WHEN ControllerContainer is created THEN it SHALL provide access to ShareValidator
2. WHEN ControllerContainer is created THEN it SHALL provide access to RecoverValidator
3. WHEN ControllerContainer is created THEN it SHALL provide access to ShareExtractor
4. WHEN ControllerContainer is created THEN it SHALL provide access to RecoverExtractor

#### Test Coverage

| AC# | Test Function(s) | Purpose | Pattern |
|-----|------------------|---------|---------|
| 1 | `test_container_share_validator` | ShareValidator accessible | Success |
| 2 | `test_container_recover_validator` | RecoverValidator accessible | Success |
| 3 | `test_container_share_extractor` | ShareExtractor accessible | Success |
| 4 | `test_container_recover_extractor` | RecoverExtractor accessible | Success |

**Additional Tests (Beyond AC):**
- `test_container_new` - Default construction

---

## Non-Functional Requirements

### Code Architecture and Modularity
- **Single Responsibility Principle**: Validator は検証のみ、Extractor は変換のみを担当
- **既存DTO再利用**: 新しいDTO定義は行わず、`usecase/dto.rs` の既存DTOを使用
- **依存関係の方向**: Controller → UseCase（Controller は UseCase 層のDTOに依存）

### Performance
- 検証は O(1) で完了すること（入力サイズに依存する検証は最小限）
- DTO構築時の不要なコピーを避ける

### Security
- 秘密データを含むDTOは Zeroize トレイトを実装（既存DTOの仕様に従う）
- エラーメッセージに秘密情報を含めない

### Reliability
- すべての検証エラーは明確なエラーコードを持つ
- 検証の順序は決定論的（同じ入力に対して同じ最初のエラーを返す）

## Test Coverage Summary

### Overall Statistics

| Requirement | Total AC | Test Functions | Coverage |
|-------------|----------|----------------|----------|
| 1. ShareValidator | 6 | 8 | 100% |
| 2. RecoverValidator | 4 | 4 | 100% |
| 3. ShareExtractor | 5 | 6 | 100% |
| 4. RecoverExtractor | 4 | 4 | 100% |
| 5. ValidationError | 4 | 4 | 100% |
| 6. ControllerContainer | 4 | 5 | 100% |
| **Total** | **27** | **31** | **100%** |

### Test Patterns Used

1. **Success Pattern**: 有効な入力に対する正常動作確認
2. **Validation Pattern**: 無効な入力に対するエラー確認
3. **Type Safety Pattern**: 型変換の正確性確認

### Legend

- ✅ **Covered**: Test implemented and fully covers AC
- ⚠️ **Partial**: Indirect test or N/A
- ❌ **Missing**: Test not implemented
