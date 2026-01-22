# Requirements Document: client-actions

## Introduction

D-TPRESクライアントライブラリのActions層は、開発者向けエンドポイント関数（Facadeパターン）を提供する最上位層です。structure.mdに基づき、以下の3つの主要API関数で構成されます：

- **`share()`** - Phase 1: 秘密分割と配布
- **`recover()`** - Phase 3: 秘密復元
- **`generateKeyPair()`** - PRE鍵ペア生成

Actions層は内部実装の複雑さを隠蔽し、開発者が最小限のパラメータでD-TPRES機能を利用できるようにします。

## Alignment with Product Vision

### tech.md / structure.md との整合性

Actions層は6層Clean Architectureの最上位層として、以下の役割を担います：

```
Actions → Controller → UseCase → Domain
               ↓
          Repository ← Adapter (implements)
```

- **Actions層**: 開発者向けエンドポイント（`share()`, `recover()`, `generateKeyPair()`）
- **Controller層**: 入力検証（ShareValidator, RecoverValidator）、DTO変換（ShareExtractor, RecoverExtractor）
- **UseCase層**: Phase 1/3オーケストレーション（SecretSharingWorkflowService, SecretRecoveryWorkflowService）

### 既存実装との関係

- **Controller層**: `ControllerContainer`経由でValidator/Extractorにアクセス
- **UseCase層**: `WorkflowServiceContainer`経由でWorkflowServiceにアクセス
- **DTOs**: `SecretSharingRequest`, `SecretSharingResult`, `SecretRecoveryRequest`, `SecretRecoveryResult`を使用
- **CryptoService**: 鍵ペア生成に使用

## Requirements

### Requirement 1: share() - 秘密分割API

**User Story:** As a library developer, I want a simple `share()` function, so that I can split and distribute secrets with minimal parameters.

#### Design Note

`share()` は Actions層のメイン関数で、以下の処理フローを実行します：

1. Controller層で入力パラメータを検証（ShareValidator）
2. Controller層で SecretSharingRequest DTO を生成（ShareExtractor）
3. UseCase層でPhase 1ワークフローを実行（SecretSharingWorkflowService）
4. 結果を SecretSharingResult として返却

開発者が直接触れる唯一のエントリーポイントとして、エラーメッセージは明確で、パラメータは最小限に設計します。

#### Acceptance Criteria

1. WHEN share() is called with valid parameters THEN it SHALL return SecretSharingResult
2. WHEN share() is called with invalid secret (empty) THEN it SHALL return ActionError::ValidationFailed
3. WHEN share() is called with invalid threshold (k=0) THEN it SHALL return ActionError::ValidationFailed
4. WHEN share() is called with threshold > total_shares THEN it SHALL return ActionError::ValidationFailed
5. WHEN share() is called with empty owner_secret_key THEN it SHALL return ActionError::ValidationFailed
6. WHEN share() is called with empty requester_public_key THEN it SHALL return ActionError::ValidationFailed
7. WHEN share() succeeds THEN result.secret_id SHALL be a valid non-empty SecretId
8. WHEN share() succeeds THEN result.kfrag_count SHALL equal total_shares parameter

#### Test Coverage

| AC# | Test Function(s) | Purpose | Pattern |
|-----|------------------|---------|---------|
| 1 | `test_share_valid_params` | Valid share returns result | Success |
| 2 | `test_share_empty_secret` | Empty secret rejected | Validation |
| 3 | `test_share_zero_threshold` | Zero threshold rejected | Validation |
| 4 | `test_share_threshold_exceeds_total` | k > n rejected | Validation |
| 5 | `test_share_empty_owner_key` | Empty owner key rejected | Validation |
| 6 | `test_share_empty_requester_key` | Empty requester key rejected | Validation |
| 7 | `test_share_result_has_valid_secret_id` | Result has valid secret_id | Success |
| 8 | `test_share_result_kfrag_count_matches` | kfrag_count equals total_shares | Success |

**Additional Tests (Beyond AC):**
- `test_share_with_metadata` - Optional metadata handling
- `test_share_various_threshold_combinations` - Various k-of-n combinations

---

### Requirement 2: recover() - 秘密復元API

**User Story:** As a library developer, I want a simple `recover()` function, so that I can recover secrets with minimal parameters.

#### Design Note

`recover()` は以下の処理フローを実行します：

1. Controller層で入力パラメータを検証（RecoverValidator）
2. Controller層で SecretRecoveryRequest DTO を生成（RecoverExtractor）
3. UseCase層でPhase 3ワークフローを実行（SecretRecoveryWorkflowService）
4. 結果を SecretRecoveryResult として返却

復元された秘密データは Zeroize トレイトにより使用後自動クリアされます。

#### Acceptance Criteria

1. WHEN recover() is called with valid parameters THEN it SHALL return SecretRecoveryResult
2. WHEN recover() is called with empty secret_id THEN it SHALL return ActionError::ValidationFailed
3. WHEN recover() is called with empty requester_secret_key THEN it SHALL return ActionError::ValidationFailed
4. WHEN recover() is called with empty requester_process_id THEN it SHALL return ActionError::ValidationFailed
5. WHEN recover() is called with non-existent secret_id THEN it SHALL return ActionError::ResourceNotFound
6. WHEN recover() succeeds THEN result.recovered_secret SHALL contain the original secret data

#### Test Coverage

| AC# | Test Function(s) | Purpose | Pattern |
|-----|------------------|---------|---------|
| 1 | `test_recover_valid_params` | Valid recover returns result | Success |
| 2 | `test_recover_empty_secret_id` | Empty secret_id rejected | Validation |
| 3 | `test_recover_empty_requester_key` | Empty requester key rejected | Validation |
| 4 | `test_recover_empty_process_id` | Empty process_id rejected | Validation |
| 5 | `test_recover_nonexistent_secret` | Non-existent secret returns NotFound | Validation |
| 6 | `test_recover_returns_original_secret` | Recovered secret matches original | Success |

**Additional Tests (Beyond AC):**
- `test_recover_secret_zeroized_on_drop` - Zeroize trait verification

---

### Requirement 3: generateKeyPair() - 鍵ペア生成API

**User Story:** As a library developer, I want a simple `generateKeyPair()` function, so that I can generate PRE key pairs for share/recover operations.

#### Design Note

`generateKeyPair()` は CryptoService を使用してUmbral PRE用の鍵ペアを生成します。生成された鍵ペアは以下の用途で使用されます：

- **Owner用**: share()で owner_secret_key, owner_public_key として使用
- **Requester用**: recover()で requester_secret_key として使用、share()で requester_public_key として使用

Owner/Requester 両方とも同じ鍵ペア生成関数を使用します。

#### Acceptance Criteria

1. WHEN generateKeyPair() is called THEN it SHALL return a valid (SecretKey, PublicKey) tuple
2. WHEN generateKeyPair() is called THEN secret_key SHALL be 32 bytes
3. WHEN generateKeyPair() is called THEN public_key SHALL not be empty
4. WHEN generateKeyPair() is called twice THEN it SHALL return different key pairs
5. WHEN generateKeyPair() succeeds THEN keys SHALL be usable with share() and recover()

#### Test Coverage

| AC# | Test Function(s) | Purpose | Pattern |
|-----|------------------|---------|---------|
| 1 | `test_generate_keypair_valid` | Returns valid key pair | Success |
| 2 | `test_generate_keypair_secret_key_size` | Secret key is 32 bytes | Success |
| 3 | `test_generate_keypair_public_key_not_empty` | Public key not empty | Success |
| 4 | `test_generate_keypair_unique` | Each call returns unique keys | Success |
| 5 | `test_generate_keypair_usable_with_share` | Keys work with share/recover | Integration |

**Additional Tests (Beyond AC):**
- `test_generate_keypair_secret_key_zeroized` - SecretKey Zeroize verification

---

### Requirement 4: ActionError - Actions層エラー型

**User Story:** As a library developer, I want structured action errors, so that error handling is consistent and informative.

#### Design Note

Actions層固有の `ActionError` を定義します。Controller層の `ValidationError` および UseCase層の `WorkflowError` をラップし、開発者に分かりやすいエラーメッセージを提供します。

```rust
pub enum ActionError {
    ValidationFailed { code: String, message: String },
    WorkflowFailed { message: String },
    ResourceNotFound { resource: String },
    CryptoError { message: String },
}
```

#### Acceptance Criteria

1. WHEN ValidationError occurs in Controller THEN it SHALL be converted to ActionError::ValidationFailed
2. WHEN WorkflowError occurs in UseCase THEN it SHALL be converted to ActionError::WorkflowFailed
3. WHEN ActionError is displayed THEN it SHALL show a clear error message
4. WHEN ActionError::ValidationFailed is created THEN it SHALL contain error code

#### Test Coverage

| AC# | Test Function(s) | Purpose | Pattern |
|-----|------------------|---------|---------|
| 1 | `test_action_error_from_validation_error` | Conversion from ValidationError | Type Safety |
| 2 | `test_action_error_from_workflow_error` | Conversion from WorkflowError | Type Safety |
| 3 | `test_action_error_display` | Display formatting | Success |
| 4 | `test_action_error_validation_has_code` | ValidationFailed has code | Success |

---

### Requirement 5: ActionsContainer - 依存性注入コンテナ

**User Story:** As a library developer, I want a DI container for actions, so that dependencies can be easily managed.

#### Design Note

ActionsContainer は ControllerContainer と WorkflowServiceContainer をまとめて管理し、Actions層関数からの利用を容易にします。依存性注入により、テスト時のモック差し替えも可能になります。

#### Acceptance Criteria

1. WHEN ActionsContainer is created THEN it SHALL provide access to share() function
2. WHEN ActionsContainer is created THEN it SHALL provide access to recover() function
3. WHEN ActionsContainer is created THEN it SHALL provide access to generateKeyPair() function
4. WHEN ActionsContainer::new() is called THEN it SHALL initialize all dependencies

#### Test Coverage

| AC# | Test Function(s) | Purpose | Pattern |
|-----|------------------|---------|---------|
| 1 | `test_container_provides_share` | share() accessible | Success |
| 2 | `test_container_provides_recover` | recover() accessible | Success |
| 3 | `test_container_provides_generate_keypair` | generateKeyPair() accessible | Success |
| 4 | `test_container_initializes_dependencies` | All dependencies initialized | Success |

**Additional Tests (Beyond AC):**
- `test_container_default` - Default trait implementation

---

### Requirement 6: ShareOptions / RecoverOptions - オプション構造体

**User Story:** As a library developer, I want option structs for share/recover, so that optional parameters are clearly separated from required ones.

#### Design Note

API の使いやすさを向上させるため、オプションパラメータを専用の構造体にまとめます。

```rust
pub struct ShareOptions {
    pub metadata: Option<SecretMetadata>,
}

pub struct RecoverOptions {
    // 将来の拡張用（現在は空）
}
```

これにより、必須パラメータとオプションパラメータが明確に分離されます。

#### Acceptance Criteria

1. WHEN share() is called with ShareOptions THEN metadata SHALL be passed to WorkflowService
2. WHEN share() is called without metadata THEN default (None) SHALL be used
3. WHEN ShareOptions::default() is called THEN metadata SHALL be None

#### Test Coverage

| AC# | Test Function(s) | Purpose | Pattern |
|-----|------------------|---------|---------|
| 1 | `test_share_with_options_metadata` | Metadata passed correctly | Success |
| 2 | `test_share_without_options` | Default options used | Success |
| 3 | `test_share_options_default` | Default values correct | Success |

---

## Non-Functional Requirements

### Code Architecture and Modularity
- **Facade Pattern**: Actions層は内部の複雑さを隠蔽し、シンプルなAPIを提供
- **依存関係の方向**: Actions → Controller → UseCase（単方向依存）
- **エラー変換**: 内部エラーを開発者フレンドリーなActionErrorに変換

### Performance
- Actions層自体はオーバーヘッドを最小限に抑える
- バリデーションは O(1) で完了

### Security
- 秘密データを含む結果は Zeroize トレイトを実装
- エラーメッセージに秘密情報を含めない
- SecretKeyは生成後すぐに使用され、不要になった時点でゼロ化される

### Reliability
- すべてのエラーは明確なエラーコードを持つ
- Controller層とUseCase層の両方でバリデーションを実施（多層防御）

### Usability
- 関数シグネチャは最小限のパラメータで設計
- オプションパラメータは専用の構造体にまとめる
- ドキュメントコメントで使用例を提供

## Test Coverage Summary

### Overall Statistics

| Requirement | Total AC | Test Functions | Coverage |
|-------------|----------|----------------|----------|
| 1. share() | 8 | 10 | 100% |
| 2. recover() | 6 | 7 | 100% |
| 3. generateKeyPair() | 5 | 6 | 100% |
| 4. ActionError | 4 | 4 | 100% |
| 5. ActionsContainer | 4 | 5 | 100% |
| 6. ShareOptions/RecoverOptions | 3 | 3 | 100% |
| **Total** | **30** | **35** | **100%** |

### Test Patterns Used

1. **Success Pattern**: 有効な入力に対する正常動作確認
2. **Validation Pattern**: 無効な入力に対するエラー確認
3. **Type Safety Pattern**: 型変換の正確性確認
4. **Integration Pattern**: 複数コンポーネント間の連携確認

### Legend

- ✅ **Covered**: Test implemented and fully covers AC
- ⚠️ **Partial**: Indirect test or N/A
- ❌ **Missing**: Test not implemented
