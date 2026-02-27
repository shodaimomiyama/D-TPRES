# Tasks Document: client-share-workflowservice-integration

## Prerequisites

**AO Network通信基盤**: Issue [#47](https://github.com/shodaimomiyama/FORMIX/issues/47) で実装
- AOClient (adapter/external)
- `send_kfrag_to_owner_process()` の実体実装

**注**: Issue #47が未完了の場合、`send_kfrag_to_owner_process()` はプレースホルダ実装（常に `Ok(())` 返却）とする。

---

## Phase 1: StorageService trait拡張

- [ ] 1. ArweaveStorageService traitに `send_kfrag_to_owner_process()` 追加
  - File: `client/src/usecase/core/storage.rs`
  - `fn send_kfrag_to_owner_process(&self, kfrags: &[KeyFragment], owner_process_id: &str) -> ServiceResult<()>` を追加
  - `ArweaveStorageServiceImpl` にプレースホルダ実装を追加（`Ok(())` を返却）
  - `KeyFragment` のインポートを追加
  - Purpose: AO通信メソッドのインターフェース提供
  - _Requirements: 5.1, 5.2, 5.3_
  - _Prompt: Role: Rust Developer | Task: Add send_kfrag_to_owner_process method to ArweaveStorageService trait and provide placeholder implementation | Restrictions: Return Ok(()) for now (Issue #47 dependency), add KeyFragment import from crypto module | Success: Trait compiles, impl compiles, existing tests still pass_

---

## Phase 2: SecretSharingWorkflowServiceImpl変更

- [ ] 2. StorageServiceジェネリクスパラメータ追加
  - File: `client/src/usecase/workflow/secret_sharing_service.rs`
  - `SecretSharingWorkflowServiceImpl<C: CryptoService>` を `SecretSharingWorkflowServiceImpl<C: CryptoService, S: ArweaveStorageService>` に変更
  - `storage_service: Arc<S>` フィールド追加
  - `new()` コンストラクタに `storage_service: Arc<S>` 引数追加
  - `ArweaveStorageService` のインポート追加
  - Purpose: StorageServiceの依存性注入
  - _Requirements: 1.1, 1.2, 1.3_
  - _Prompt: Role: Rust Developer | Task: Add ArweaveStorageService generic parameter S to SecretSharingWorkflowServiceImpl, add storage_service field and update constructor | Restrictions: Keep existing CryptoService parameter, update all impl blocks with new type parameter | Success: Struct compiles with both type parameters, constructor accepts both services_

- [ ] 3. kFrag送信実装 (Step 7)
  - File: `client/src/usecase/workflow/secret_sharing_service.rs`
  - Step 7のTODOコメントを実際の `send_kfrag_to_owner_process()` 呼び出しに置換
  - エラーを `WorkflowError::ao_communication()` にマッピング
  - Purpose: kFragのAO Network送信
  - _Requirements: 2.1, 2.2, 2.3, 2.4_
  - _Prompt: Role: Rust Developer | Task: Replace Step 7 TODO with actual send_kfrag_to_owner_process call, map errors to WorkflowError::AOCommunicationError | Restrictions: Pass kfrags and owner_process_id from request, handle error with map_err | Success: kFrag sending calls StorageService correctly, errors mapped properly_

- [ ] 4. Arweaveストレージ実装 (Step 8)
  - File: `client/src/usecase/workflow/secret_sharing_service.rs`
  - Step 8のプレースホルダtx_id生成を実際のArweave保存に置換
  - Capsule: `store_data()` で保存、タグ: `type:capsule`, `secret_id:{id}`
  - 暗号化シェア: `batch_store()` で保存、タグ: `type:encrypted_share`, `secret_id:{id}`, `index:{i}`
  - `Tag` のインポート追加
  - Purpose: Arweave永続化の実現
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.6_
  - _Prompt: Role: Rust Developer | Task: Replace Step 8 placeholder with actual store_data and batch_store calls, add proper tags for capsule and encrypted shares | Restrictions: Use store_data for capsule, batch_store for shares, include type/secret_id/index tags | Success: Real Arweave tx_ids in result, proper tags on all transactions_

- [ ] 5. get_secret_status()実装
  - File: `client/src/usecase/workflow/secret_sharing_service.rs`
  - プレースホルダの `ResourceNotFound` 返却を `query_by_tags()` ベースの実装に置換
  - `secret_id` と `type:capsule` タグで検索
  - 結果が空なら `ResourceNotFound`、存在すれば `SecretStatus::Created`
  - `QueryParams`, `Tag` のインポート追加
  - Purpose: 秘密ステータスの実照会
  - _Requirements: 4.1, 4.2, 4.3, 4.4_
  - _Prompt: Role: Rust Developer | Task: Implement get_secret_status using query_by_tags to search for capsule transactions by secret_id | Restrictions: Return Created if found, ResourceNotFound if empty, StorageError for service errors | Success: Status queries work correctly for existing and non-existing secrets_

---

## Phase 3: DI Container更新

- [ ] 6. WorkflowServiceContainer更新
  - File: `client/src/usecase/workflow/container.rs`
  - `SecretSharingWorkflowServiceImpl` のインスタンス化にStorageServiceを追加
  - 型パラメータの伝搬を確認
  - Purpose: DIコンテナでのStorageService注入
  - _Requirements: 6.1, 6.2_
  - _Prompt: Role: Rust Developer | Task: Update WorkflowServiceContainer to inject StorageService into SecretSharingWorkflowServiceImpl | Restrictions: Follow existing DI patterns | Success: Container builds correctly with StorageService injected_

- [ ] 7. ShareBuilder更新
  - File: `client/src/actions/builder.rs`
  - `ShareBuilder` のWorkflowService構築にStorageServiceを追加
  - Purpose: Builder層からの統合確認
  - _Requirements: 6.2_
  - _Prompt: Role: Rust Developer | Task: Update ShareBuilder to pass StorageService when constructing workflow service | Restrictions: Follow existing builder patterns | Success: Builder compiles and creates workflow service with StorageService_

---

## Phase 4: テスト更新

- [ ] 8. 既存テストのMock更新
  - File: `client/src/usecase/workflow/secret_sharing_service.rs` (test module)
  - 既存テストで `SecretSharingWorkflowServiceImpl::new()` に MockStorageService を追加
  - MockStorageService定義を追加
  - 既存テストが引き続きパスすることを確認
  - Purpose: 後方互換性の維持
  - _Requirements: All_
  - _Prompt: Role: QA Engineer | Task: Add MockStorageService to existing tests, update service instantiation to include storage_service parameter | Restrictions: Do not change test assertions, only update service construction | Success: All existing tests pass with new type parameter_

- [ ] 9. kFrag送信テスト追加
  - File: `client/src/usecase/workflow/secret_sharing_service.rs` (test module)
  - `test_kfrags_sent_to_owner_process` - kFrag送信呼び出し検証
  - `test_kfrag_send_failure_returns_ao_error` - AO通信エラー検証
  - `test_kfrag_send_passes_correct_params` - 引数検証
  - Purpose: kFrag送信の正確性検証
  - _Requirements: 2.1, 2.3, 2.4_
  - _Prompt: Role: QA Engineer | Task: Write unit tests for kFrag sending using MockStorageService expectations | Restrictions: Verify send_kfrag_to_owner_process is called with correct kfrags and owner_process_id | Success: Tests verify calling pattern and error handling_

- [ ] 10. Arweaveストレージテスト追加
  - File: `client/src/usecase/workflow/secret_sharing_service.rs` (test module)
  - `test_capsule_stored_on_arweave` - Capsule保存検証
  - `test_encrypted_shares_stored_on_arweave` - シェア保存検証
  - `test_capsule_tags_correct` - タグ検証
  - `test_result_contains_real_tx_ids` - 結果tx_id検証
  - `test_arweave_storage_failure` - ストレージエラー検証
  - Purpose: Arweave保存の正確性検証
  - _Requirements: 3.1, 3.2, 3.3, 3.5, 3.6_
  - _Prompt: Role: QA Engineer | Task: Write unit tests for Arweave storage of capsule and encrypted shares | Restrictions: Verify store_data and batch_store calls, check tags, verify tx_ids in result | Success: Tests verify storage calls, tags, and result construction_

- [ ] 11. get_secret_status()テスト追加
  - File: `client/src/usecase/workflow/secret_sharing_service.rs` (test module)
  - `test_get_status_returns_created` - Created状態検証
  - `test_get_status_not_found` - NotFound検証
  - `test_get_status_storage_error` - StorageServiceエラー検証
  - Purpose: ステータス照会の正確性検証
  - _Requirements: 4.1, 4.2, 4.3, 4.4_
  - _Prompt: Role: QA Engineer | Task: Write unit tests for get_secret_status using MockStorageService | Restrictions: Test Created, NotFound, and StorageError scenarios | Success: All status scenarios correctly tested_

- [ ] 12. 既存テストの更新（プレースホルダ動作 → 実動作）
  - File: `client/src/usecase/workflow/secret_sharing_service.rs` (test module)
  - `test_get_secret_status_not_implemented` → 正常系テストに変更または削除
  - `test_phase1_workflow_complete` → 実際のトランザクションIDを検証するよう更新
  - Purpose: プレースホルダテストの実装テストへの移行
  - _Requirements: All_
  - _Prompt: Role: QA Engineer | Task: Update existing placeholder tests to verify actual StorageService integration | Restrictions: Remove placeholder assertions, add real tx_id and storage call verification | Success: Tests verify actual behavior instead of placeholder behavior_

---

## Phase 5: ビルド検証

- [ ] 13. コンパイルとリント確認
  - Command: `make check && make lint`
  - 全ファイルがコンパイルを通ること
  - clippy警告が解消されていること
  - Purpose: ビルド品質の確認
  - _Prompt: Role: Rust Developer | Task: Run make check and make lint, fix any compilation errors or clippy warnings | Success: Both commands pass without errors_

- [ ] 14. テスト実行
  - Command: `make test`
  - 全テスト（既存 + 新規）がパスすること
  - Purpose: 機能の正確性確認
  - _Prompt: Role: QA Engineer | Task: Run make test and verify all tests pass | Success: All tests pass including new storage integration tests_

---

## Summary

| Phase | Tasks | Description |
|-------|-------|-------------|
| 1 | 1 | StorageService trait拡張 |
| 2 | 2-5 | SecretSharingWorkflowServiceImpl変更 |
| 3 | 6-7 | DI Container更新 |
| 4 | 8-12 | テスト更新 |
| 5 | 13-14 | ビルド検証 |

**Total Tasks:** 14
**Prerequisites:** Issue [#47](https://github.com/shodaimomiyama/FORMIX/issues/47) (AO Network通信基盤) - プレースホルダ実装で先行可能
