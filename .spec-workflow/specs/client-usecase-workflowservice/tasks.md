# Tasks Document: client-usecase-workflowservice

## Prerequisites

**AO Network通信基盤**: Issue [#47](https://github.com/shodaimomiyama/D-TPRES/issues/47) で実装
- StorageService: `send_kfrag_to_owner_process()`, `retrieve_cfrags_from_requester_process()`, `retrieve_capsule()`
- KFrag/CFragのRepository層
- AOClient (adapter/external)

---

## Phase 0: CryptoService拡張

- [x] 0.1. CryptoService AES-GCM暗号化メソッド追加
  - File: `client/src/usecase/core/crypto.rs`
  - `aes_gcm_encrypt(&self, key: &[u8], plaintext: &[u8]) -> ServiceResult<Vec<u8>>` を追加
  - `aes_gcm_decrypt(&self, key: &[u8], ciphertext: &[u8]) -> ServiceResult<Vec<u8>>` を追加
  - AES-256-GCM、Nonce 12バイト（ランダム生成、暗号文に付加）、認証タグ16バイト
  - Purpose: シェア暗号化/復号のためのAES-GCM操作を提供
  - _Leverage: `aes-gcm` crate_
  - _Requirements: 1.3, 3.4_
  - _Prompt: Role: Rust Developer with cryptography expertise | Task: Add AES-256-GCM encrypt/decrypt methods to CryptoService | Restrictions: Use aes-gcm crate, prepend random nonce to ciphertext, ensure Zeroize on sensitive data | Success: Encrypt/decrypt roundtrip works, proper error handling for invalid key/ciphertext_

- [x] 0.2. CryptoService 対称鍵生成メソッド追加
  - File: `client/src/usecase/core/crypto.rs`
  - `generate_symmetric_key(&self) -> ServiceResult<Vec<u8>>` を追加
  - 32バイト（AES-256用）の暗号学的に安全な乱数を生成
  - Purpose: PHASE 1での共通鍵kₒ生成
  - _Leverage: `rand` crate with OsRng_
  - _Requirements: 1.1_
  - _Prompt: Role: Rust Developer with cryptography expertise | Task: Add symmetric key generation method to CryptoService | Restrictions: Use OsRng for cryptographically secure randomness, return exactly 32 bytes | Success: Generated keys are 32 bytes, cryptographically random_

- [x] 0.3. CryptoService拡張のユニットテスト
  - File: `client/src/usecase/core/crypto.rs` (test module)
  - test_aes_gcm_encrypt_decrypt_roundtrip
  - test_aes_gcm_decrypt_invalid_key
  - test_aes_gcm_decrypt_corrupted_ciphertext
  - test_generate_symmetric_key_length
  - test_generate_symmetric_key_randomness
  - Purpose: AES-GCM操作の正確性検証
  - _Requirements: 1.1, 1.3, 3.4_
  - _Prompt: Role: QA Engineer | Task: Write unit tests for CryptoService AES-GCM methods | Restrictions: Test roundtrip, error cases, key randomness | Success: All tests pass, edge cases covered_

---

## Phase 1: 基盤コンポーネント

- [x] 1. WorkflowError定義
  - File: `client/src/usecase/error.rs`
  - WorkflowError enumを定義（ValidationError, CryptoError, StorageError, AOCommunicationError, InsufficientCFrags, DecryptionError, ResourceNotFound）
  - thiserror crateを使用してエラーメッセージを生成
  - CryptoServiceError, StorageServiceErrorからのFrom実装
  - Purpose: Workflow層固有のエラー型を提供
  - _Leverage: `client/src/service/error.rs`（既存ServiceError設計）_
  - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5, 5.6_
  - _Prompt: Role: Rust Developer specializing in error handling | Task: Create WorkflowError enum following requirement 5, implementing From traits for CryptoServiceError and StorageServiceError, using thiserror for derive | Restrictions: Do not include secret data in error messages, maintain consistency with existing ServiceError patterns | Success: All error variants defined, From implementations work correctly, error messages are user-friendly without exposing sensitive information_

- [x] 2. Request/Result DTOモデル定義
  - File: `client/src/usecase/dto.rs`
  - SecretSharingRequest, SecretSharingResult構造体を定義
  - SecretRecoveryRequest, SecretRecoveryResult構造体を定義
  - SecretStatus enumを定義
  - Zeroize/ZeroizeOnDropを秘密データを含む構造体に実装
  - Purpose: Workflow Service入出力の型安全性を確保
  - _Leverage: `client/src/domain/entities/`（既存エンティティ設計）_
  - _Requirements: 1 (入力パラメータ), 3 (入力パラメータ)_
  - _Prompt: Role: Rust Developer with expertise in type design and memory safety | Task: Create DTO structs for SecretSharingRequest/Result and SecretRecoveryRequest/Result following requirements 1 and 3, implementing Zeroize for sensitive data | Restrictions: SecretRecoveryRequest must only contain secret_id, requester_secret_key, and requester_process_id (cFrags/Capsule are fetched internally), ensure proper memory cleanup | Success: All DTOs compile correctly, Zeroize implemented for sensitive fields, clear documentation on each field's purpose_

---

## Phase 2: SecretSharingWorkflowService

- [-] 3. SecretSharingWorkflowService trait定義
  - File: `client/src/usecase/secret_sharing_service.rs`
  - SecretSharingWorkflowService traitを定義
  - `execute_secret_sharing(&self, request: SecretSharingRequest) -> ServiceResult<SecretSharingResult>` メソッド
  - `get_secret_status(&self, secret_id: &SecretId) -> ServiceResult<SecretStatus>` メソッド
  - Purpose: PHASE 1ワークフローのインターフェース定義
  - _Leverage: `client/src/usecase/core/crypto.rs`, `client/src/usecase/core/storage.rs`_
  - _Requirements: 1, 2_
  - _Prompt: Role: Rust Developer specializing in trait design | Task: Define SecretSharingWorkflowService trait with execute_secret_sharing and get_secret_status methods following requirements 1 and 2 | Restrictions: Trait must be Send + Sync, use ServiceResult for error handling, do not expose implementation details | Success: Trait is well-defined with clear method signatures, compatible with dependency injection_

- [ ] 4. SecretSharingWorkflowServiceImpl - バリデーション実装
  - File: `client/src/usecase/secret_sharing_service.rs`
  - SecretSharingWorkflowServiceImpl構造体を作成
  - CryptoService, ArweaveStorageServiceを依存注入で受け取る
  - 入力パラメータのバリデーションを実装（threshold, total_shares, secret, keys）
  - Purpose: PHASE 1の入力検証
  - _Leverage: `client/src/usecase/core/crypto.rs`, `client/src/usecase/core/storage.rs`_
  - _Requirements: 1.10, 1.11, 1.12, 1.13, 1.14_
  - _Prompt: Role: Rust Developer with expertise in validation | Task: Implement SecretSharingWorkflowServiceImpl with input validation following requirements 1.10-1.14 | Restrictions: Return WorkflowError::ValidationError for all validation failures, validate all parameters before any crypto operations | Success: All validation rules implemented, clear error messages for each validation failure_

- [ ] 5. SecretSharingWorkflowServiceImpl - PHASE 1ワークフロー実装
  - File: `client/src/usecase/secret_sharing_service.rs` (continue from task 4)
  - Step 1-2: generate_symmetric_key() + split_secret_shamir()
  - Step 1-3: aes_gcm_encrypt() for each share
  - Step 1-4: create_pre_capsule()
  - Step 1-5, 1-6: generate_reencryption_key()
  - Step 1-7: create_kfrags()
  - Step 1-8: send_kfrag_to_owner_process()
  - Step 1-9: batch_store() for Capsule and encrypted shares
  - Purpose: PHASE 1完全ワークフローの実装
  - _Leverage: CryptoService, ArweaveStorageService_
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 1.8, 1.9_
  - _Prompt: Role: Rust Developer with expertise in cryptographic workflows | Task: Implement execute_secret_sharing following PRD PHASE 1 steps 1-2 through 1-9, orchestrating CryptoService and StorageService calls | Restrictions: Must call services in correct order, handle errors at each step with appropriate WorkflowError, ensure Zeroize on intermediate secrets | Success: Complete PHASE 1 flow works end-to-end, all intermediate secrets are zeroized, proper error handling at each step_

- [ ] 6. SecretSharingWorkflowServiceImpl - ステータス管理実装
  - File: `client/src/usecase/secret_sharing_service.rs` (continue from task 5)
  - get_secret_status()実装
  - ArweaveStorageService.query_by_tags()を使用してステータス取得
  - ResourceNotFoundエラーハンドリング
  - Purpose: 秘密のライフサイクル管理
  - _Leverage: ArweaveStorageService_
  - _Requirements: 2.1, 2.2, 2.3_
  - _Prompt: Role: Rust Developer | Task: Implement get_secret_status using ArweaveStorageService.query_by_tags following requirement 2 | Restrictions: Return ResourceNotFound for missing secrets, propagate storage errors correctly | Success: Status retrieval works correctly, proper error handling for all scenarios_

---

## Phase 3: SecretRecoveryWorkflowService

- [ ] 7. SecretRecoveryWorkflowService trait定義
  - File: `client/src/usecase/secret_recovery_service.rs`
  - SecretRecoveryWorkflowService traitを定義
  - `recover_secret(&self, request: SecretRecoveryRequest) -> ServiceResult<SecretRecoveryResult>` メソッド
  - `verify_recovered_data(&self, data: &[u8]) -> bool` メソッド
  - Purpose: PHASE 3ワークフローのインターフェース定義
  - _Leverage: `client/src/usecase/core/crypto.rs`, `client/src/usecase/core/storage.rs`_
  - _Requirements: 3, 4_
  - _Prompt: Role: Rust Developer specializing in trait design | Task: Define SecretRecoveryWorkflowService trait with recover_secret and verify_recovered_data methods following requirements 3 and 4 | Restrictions: Trait must be Send + Sync, recover_secret takes only secret_id, requester_secret_key, and requester_process_id | Success: Trait is well-defined, compatible with dependency injection_

- [ ] 8. SecretRecoveryWorkflowServiceImpl - バリデーション実装
  - File: `client/src/usecase/secret_recovery_service.rs`
  - SecretRecoveryWorkflowServiceImpl構造体を作成
  - CryptoService, ArweaveStorageServiceを依存注入で受け取る
  - 入力パラメータのバリデーションを実装（secret_id, accessor_secret_key）
  - Purpose: PHASE 3の入力検証
  - _Leverage: CryptoService, ArweaveStorageService_
  - _Requirements: 3.9, 3.11_
  - _Prompt: Role: Rust Developer with expertise in validation | Task: Implement SecretRecoveryWorkflowServiceImpl with input validation following requirements 3.9 and 3.11 | Restrictions: Return WorkflowError::ValidationError for invalid inputs | Success: All validation rules implemented, clear error messages_

- [ ] 9. SecretRecoveryWorkflowServiceImpl - PHASE 3ワークフロー実装
  - File: `client/src/usecase/secret_recovery_service.rs` (continue from task 8)
  - Step 1: retrieve_cfrags_from_requester_process() - AO通信でcFrag取得
  - Step 2: retrieve_capsule() - ArweaveからCapsule取得
  - Step 3: combine_and_decrypt() - Capsule結合と共通鍵復号
  - Step 4: query_by_tags() - 暗号化シェア取得
  - Step 5: aes_gcm_decrypt() for each share
  - Step 6: reconstruct_secret_shamir()
  - Step 7: store_data() for audit trail
  - cFrag数の検証（閾値チェック）
  - Purpose: PHASE 3完全ワークフローの実装
  - _Leverage: CryptoService, ArweaveStorageService_
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 3.7, 3.8, 3.10, 3.12, 3.13_
  - _Prompt: Role: Rust Developer with expertise in cryptographic workflows | Task: Implement recover_secret following PRD PHASE 3 steps, first fetching cFrags from Requester-Process via AO and Capsule from Arweave, then orchestrating decryption | Restrictions: Must fetch cFrags via retrieve_cfrags_from_requester_process() and Capsule via retrieve_capsule() before crypto operations, validate cFrag count >= threshold, record audit trail on success/failure | Success: Complete PHASE 3 flow works end-to-end, cFrags and Capsule fetched correctly, audit recorded_

- [ ] 10. SecretRecoveryWorkflowServiceImpl - 復元データ検証実装
  - File: `client/src/usecase/secret_recovery_service.rs` (continue from task 9)
  - verify_recovered_data()実装
  - 空データチェック
  - サイズ妥当性チェック
  - Purpose: 復元データの整合性検証
  - _Requirements: 4.1, 4.2, 4.3, 4.4_
  - _Prompt: Role: Rust Developer | Task: Implement verify_recovered_data following requirement 4 | Restrictions: Return bool, do not throw errors, check for empty and size validity | Success: Verification logic works correctly for all cases_

---

## Phase 4: モジュールエクスポートとDI

- [ ] 11. モジュールエクスポート設定
  - File: `client/src/usecase/mod.rs`
  - error, dto, secret_sharing_service, secret_recovery_serviceをpub mod
  - 必要な型をre-export
  - Purpose: UseCase層の公開インターフェース整理
  - _Leverage: 既存のmod.rsパターン_
  - _Requirements: All_
  - _Prompt: Role: Rust Developer | Task: Configure module exports in mod.rs to expose WorkflowServices, DTOs, and errors | Restrictions: Only expose public API, keep internal implementation private | Success: All public types are accessible from client::usecase_

- [ ] 12. DI設定更新
  - File: `client/src/usecase/container.rs` or relevant DI file
  - SecretSharingWorkflowServiceの登録
  - SecretRecoveryWorkflowServiceの登録
  - CryptoService, StorageServiceの依存関係設定
  - Purpose: 依存性注入コンテナでのサービス登録
  - _Leverage: 既存のDI設定パターン_
  - _Requirements: All_
  - _Prompt: Role: Rust Developer with expertise in DI | Task: Register WorkflowServices in DI container with proper dependency configuration | Restrictions: Follow existing DI patterns, ensure services are correctly wired | Success: Services can be resolved from DI container with all dependencies_

---

## Phase 5: ユニットテスト

- [ ] 13. WorkflowErrorユニットテスト
  - File: `client/src/usecase/error.rs` (test module)
  - test_workflow_error_validation
  - test_workflow_error_from_crypto_service
  - test_workflow_error_from_storage_service
  - test_workflow_error_ao_communication
  - test_workflow_error_insufficient_cfrags
  - test_workflow_error_decryption
  - Purpose: エラー型の正確性検証
  - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5, 5.6_
  - _Prompt: Role: QA Engineer | Task: Write unit tests for WorkflowError covering all variants and From implementations | Restrictions: Test error message content, verify From trait conversions | Success: All error variants tested, From implementations verified_

- [ ] 14. SecretSharingWorkflowService ユニットテスト - バリデーション
  - File: `client/src/usecase/secret_sharing_service.rs` (test module)
  - test_phase1_invalid_threshold_exceeds
  - test_phase1_invalid_threshold_below_min
  - test_phase1_invalid_total_shares_exceeds_max
  - test_phase1_empty_secret
  - test_phase1_invalid_keys
  - Purpose: PHASE 1入力バリデーションの検証
  - _Leverage: mockall for mocking Core Services_
  - _Requirements: 1.10, 1.11, 1.12, 1.13, 1.14_
  - _Prompt: Role: QA Engineer | Task: Write unit tests for SecretSharingWorkflowService validation using mocked Core Services | Restrictions: Use mockall for mocking, test each validation rule independently | Success: All validation scenarios tested, correct errors returned_

- [ ] 15. SecretSharingWorkflowService ユニットテスト - ワークフロー
  - File: `client/src/usecase/secret_sharing_service.rs` (test module)
  - test_phase1_generates_symmetric_key
  - test_phase1_splits_secret_shamir
  - test_phase1_encrypts_shares_aes_gcm
  - test_phase1_creates_capsule_for_symmetric_key
  - test_phase1_generates_reencryption_key
  - test_phase1_creates_kfrags
  - test_phase1_sends_kfrags_to_owner_process
  - test_phase1_stores_capsule_and_shares
  - test_phase1_returns_complete_result
  - test_phase1_complete_flow
  - test_phase1_kfrags_count_matches_n
  - Purpose: PHASE 1ワークフローの正常系検証
  - _Leverage: mockall for mocking Core Services_
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 1.8, 1.9_
  - _Prompt: Role: QA Engineer | Task: Write unit tests for SecretSharingWorkflowService workflow steps using mocked Core Services | Restrictions: Verify each step calls correct service method with correct parameters | Success: All workflow steps tested, service calls verified_

- [ ] 16. SecretSharingWorkflowService ユニットテスト - エラーハンドリング
  - File: `client/src/usecase/secret_sharing_service.rs` (test module)
  - test_phase1_crypto_service_error
  - test_phase1_storage_service_error
  - test_phase1_ao_communication_error
  - test_get_secret_status_valid
  - test_get_secret_status_not_found
  - test_get_secret_status_storage_error
  - Purpose: PHASE 1エラー伝播の検証
  - _Leverage: mockall for mocking Core Services_
  - _Requirements: 1.15, 1.16, 1.17, 2.1, 2.2, 2.3_
  - _Prompt: Role: QA Engineer | Task: Write unit tests for error propagation in SecretSharingWorkflowService | Restrictions: Mock service errors and verify correct WorkflowError is returned | Success: All error scenarios tested, correct error types returned_

- [ ] 17. SecretRecoveryWorkflowService ユニットテスト - バリデーション
  - File: `client/src/usecase/secret_recovery_service.rs` (test module)
  - test_phase3_validates_cfrag_count
  - test_phase3_insufficient_cfrags
  - test_phase3_invalid_capsule
  - test_phase3_encrypted_share_not_found
  - test_phase3_invalid_accessor_key
  - Purpose: PHASE 3入力バリデーションの検証
  - _Leverage: mockall for mocking Core Services_
  - _Requirements: 3.1, 3.8, 3.9, 3.10, 3.11_
  - _Prompt: Role: QA Engineer | Task: Write unit tests for SecretRecoveryWorkflowService validation using mocked Core Services | Restrictions: Test cFrag count validation after fetching from AO | Success: All validation scenarios tested_

- [ ] 18. SecretRecoveryWorkflowService ユニットテスト - ワークフロー
  - File: `client/src/usecase/secret_recovery_service.rs` (test module)
  - test_phase3_retrieves_cfrags_from_requester_process
  - test_phase3_retrieves_capsule_from_arweave
  - test_phase3_combines_and_decrypts_symmetric_key
  - test_phase3_fetches_encrypted_shares
  - test_phase3_decrypts_shares_aes
  - test_phase3_reconstructs_secret_shamir
  - test_phase3_returns_complete_result
  - test_phase3_records_audit_trail
  - test_phase3_complete_flow
  - test_phase3_exactly_k_cfrags
  - test_phase3_more_than_k_cfrags
  - Purpose: PHASE 3ワークフローの正常系検証
  - _Leverage: mockall for mocking Core Services_
  - _Requirements: 3.2, 3.3, 3.4, 3.5, 3.6, 3.7_
  - _Prompt: Role: QA Engineer | Task: Write unit tests for SecretRecoveryWorkflowService workflow steps, including cFrag/Capsule retrieval from AO/Arweave | Restrictions: Verify retrieve_cfrags_from_requester_process and retrieve_capsule are called before crypto operations | Success: All workflow steps tested including AO/Arweave fetch_

- [ ] 19. SecretRecoveryWorkflowService ユニットテスト - エラーハンドリング
  - File: `client/src/usecase/secret_recovery_service.rs` (test module)
  - test_phase3_decryption_error_with_audit
  - test_phase3_storage_service_error
  - test_phase3_ao_retrieval_error
  - test_verify_recovered_data_empty
  - test_verify_recovered_data_size
  - test_verify_recovered_data_valid
  - test_verify_recovered_data_returns_false
  - Purpose: PHASE 3エラー伝播と検証の検証
  - _Leverage: mockall for mocking Core Services_
  - _Requirements: 3.12, 3.13, 4.1, 4.2, 4.3, 4.4_
  - _Prompt: Role: QA Engineer | Task: Write unit tests for error propagation and data verification in SecretRecoveryWorkflowService | Restrictions: Verify audit is recorded even on failure, test AO retrieval errors | Success: All error scenarios and verification tested_

---

## Phase 6: 統合テスト

- [ ] 20. PHASE 1統合テスト
  - File: `client/tests/integration/secret_sharing_workflow_test.rs`
  - test_phase1_integration_complete_flow
  - 実CryptoService、MockStorageServiceを使用
  - 暗号操作の正確性検証
  - Purpose: PHASE 1の暗号フロー統合検証
  - _Leverage: 実CryptoService実装_
  - _Requirements: 1 (all)_
  - _Prompt: Role: Integration Test Engineer | Task: Write integration test for PHASE 1 using real CryptoService and mocked StorageService | Restrictions: Verify crypto operations produce valid outputs, test full workflow | Success: Integration test passes, crypto outputs are valid_

- [ ] 21. PHASE 3統合テスト
  - File: `client/tests/integration/secret_recovery_workflow_test.rs`
  - test_phase3_integration_complete_flow
  - 実CryptoService、MockStorageServiceを使用
  - cFrag/Capsule取得のモック
  - 復号・復元の正確性検証
  - Purpose: PHASE 3の暗号フロー統合検証
  - _Leverage: 実CryptoService実装_
  - _Requirements: 3 (all)_
  - _Prompt: Role: Integration Test Engineer | Task: Write integration test for PHASE 3 using real CryptoService and mocked StorageService | Restrictions: Mock AO/Arweave responses for cFrags/Capsule, verify decryption produces original secret | Success: Integration test passes, recovered secret matches original_

- [ ] 22. PHASE 1 → PHASE 3 ラウンドトリップテスト
  - File: `client/tests/integration/workflow_roundtrip_test.rs`
  - test_share_and_recover_roundtrip
  - PHASE 1で秘密を分割、PHASE 3で復元
  - 復元された秘密が元の秘密と一致することを検証
  - Purpose: エンドツーエンドの暗号フロー検証
  - _Leverage: 実CryptoService実装_
  - _Requirements: 1, 3_
  - _Prompt: Role: Integration Test Engineer | Task: Write roundtrip integration test that shares a secret in PHASE 1 and recovers it in PHASE 3 | Restrictions: Use real CryptoService, mock storage responses appropriately, verify exact match of original and recovered secret | Success: Roundtrip test passes, secrets match exactly_

---

## Summary

| Phase | Tasks | Description |
|-------|-------|-------------|
| 0 | 0.1-0.3 | CryptoService拡張（AES-GCM） |
| 1 | 1-2 | 基盤コンポーネント（Error, DTO） |
| 2 | 3-6 | SecretSharingWorkflowService |
| 3 | 7-10 | SecretRecoveryWorkflowService |
| 4 | 11-12 | モジュールエクスポートとDI |
| 5 | 13-19 | ユニットテスト |
| 6 | 20-22 | 統合テスト |

**Total Tasks:** 25
**Estimated Test Functions:** 56 (51 + 5 for CryptoService AES-GCM)

**Prerequisites (別Issue):** [#47](https://github.com/shodaimomiyama/D-TPRES/issues/47) - AO Network通信基盤
