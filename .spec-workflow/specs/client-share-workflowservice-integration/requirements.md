# Requirements Document: client-share-workflowservice-integration

## Introduction

本機能は、`SecretSharingWorkflowService` のプレースホルダ実装を `ArweaveStorageService` 統合により完成させます。Issue [#47](https://github.com/shodaimomiyama/FORMIX/issues/47) で実装されるAO Network通信基盤を前提に、PHASE 1ワークフローの以下の未実装箇所を解決します。

**対象ファイル:** `client/src/usecase/workflow/secret_sharing_service.rs`

**関連Issue:**
- **依存先**: [#47](https://github.com/shodaimomiyama/FORMIX/issues/47) (AO Network通信基盤)
- **関連**: [#51](https://github.com/shodaimomiyama/FORMIX/issues/51) (SecretRecoveryWorkflowService - StorageService統合)

**現在のプレースホルダ箇所:**

| 箇所 | 行番号 | 内容 | 状態 |
|------|--------|------|------|
| StorageService DI | L74-77 | `storage_service` フィールド未追加 | TODO |
| kFrag送信 | L223-225 | AO Network経由のkFrag送信 | TODO |
| Arweaveストレージ | L227-236 | Capsule/暗号化シェアの保存 | TODO (placeholder tx_ids) |
| get_secret_status() | L251-258 | ステータス取得 | TODO (常にNotFound) |

**依存するStorageServiceメソッド:**

| メソッド | 用途 | 提供元 |
|----------|------|--------|
| `send_kfrag_to_owner_process()` | kFragをOwner-ProcessにAO送信 | 要新規追加 |
| `store_data()` / `batch_store()` | Capsule/シェアのArweave保存 | 既存 |
| `query_by_tags()` | ステータス照会 | 既存 |

## Alignment with Product Vision

本機能は以下のプロダクトビジョンを完成させます:

1. **Arweave永続ストレージ**: プレースホルダIDから実際のArweaveトランザクションIDへの移行
2. **AO Network連携**: kFragのOwner-Process送信を実現
3. **ライフサイクル管理**: `get_secret_status()` による秘密の状態追跡

## Requirements

### Requirement 1: StorageService依存性注入の追加

**User Story:** As a 開発者, I want SecretSharingWorkflowServiceImplにStorageServiceを注入したい, so that Arweave/AO操作を統合できる

#### Acceptance Criteria

1. WHEN `SecretSharingWorkflowServiceImpl` が定義される THEN struct SHALL `ArweaveStorageService` をジェネリクスパラメータ `S` として追加する
2. WHEN `SecretSharingWorkflowServiceImpl::new()` が呼び出される THEN constructor SHALL `crypto_service: Arc<C>` と `storage_service: Arc<S>` の両方を受け取る
3. WHEN 型パラメータが指定される THEN `S` SHALL `ArweaveStorageService + 'static` バウンドを持つ

#### Test Coverage

| AC# | Test Function(s) | Purpose | Pattern |
|-----|------------------|---------|---------|
| 1-3 | `test_service_creation_with_storage` | StorageService付きインスタンス生成を検証 | Success |

### Requirement 2: kFrag送信の実装 (Step 7)

**User Story:** As a データ所有者, I want 生成したkFragsをOwner-ProcessにAO Network経由で送信したい, so that Holder-Processへの分散委譲が可能になる

#### Design Note

`ArweaveStorageService` トレイトに `send_kfrag_to_owner_process()` メソッドを追加する必要がある。このメソッドはAO Network通信を抽象化し、kFragsのシリアライズと送信を担当する。

#### Acceptance Criteria

1. WHEN kFrag生成が成功した THEN system SHALL `ArweaveStorageService.send_kfrag_to_owner_process()` を呼び出してkFragsをOwner-Processに送信する
2. WHEN kFrag送信が成功した THEN system SHALL 次のArweave保存ステップに進む
3. WHEN kFrag送信が失敗した THEN system SHALL `WorkflowError::AOCommunicationError` を返却する
4. WHEN `send_kfrag_to_owner_process()` が呼び出される THEN system SHALL `owner_process_id` と全kFragsを引数として渡す

#### Test Coverage

| AC# | Test Function(s) | Purpose | Pattern |
|-----|------------------|---------|---------|
| 1 | `test_kfrags_sent_to_owner_process` | kFrag送信呼び出しを検証 | Mutation |
| 2 | `test_kfrag_send_proceeds_to_storage` | 送信後にストレージ保存が続くことを検証 | Success |
| 3 | `test_kfrag_send_failure_returns_ao_error` | AO通信エラー伝播を検証 | Error |
| 4 | `test_kfrag_send_passes_correct_params` | 引数の正確性を検証 | Success |

### Requirement 3: Arweaveストレージの実装 (Step 8)

**User Story:** As a データ所有者, I want CapsuleとencryptedシェアをArweaveに永続化したい, so that PHASE 3での復元が可能になる

#### Design Note

Capsuleと暗号化シェアはArweaveStorageServiceの既存メソッド (`store_data()` or `batch_store()`) を使用して保存する。各トランザクションには `secret_id`, `type` (capsule/encrypted_share), `index` 等のタグを付与する。

#### Acceptance Criteria

1. WHEN Step 7が成功した THEN system SHALL CapsuleをArweaveに保存し実際のトランザクションIDを取得する
2. WHEN Capsule保存が成功した THEN system SHALL 各暗号化シェアをArweaveに保存し実際のトランザクションIDリストを取得する
3. WHEN 保存時 THEN system SHALL Capsuleに `type:capsule`, `secret_id:{id}` タグを付与する
4. WHEN 保存時 THEN system SHALL 各暗号化シェアに `type:encrypted_share`, `secret_id:{id}`, `index:{i}` タグを付与する
5. WHEN Arweave保存が失敗した THEN system SHALL `WorkflowError::StorageError` を返却する
6. WHEN 全保存が成功した THEN system SHALL `SecretSharingResult` に実際のトランザクションIDを設定して返却する

#### Test Coverage

| AC# | Test Function(s) | Purpose | Pattern |
|-----|------------------|---------|---------|
| 1 | `test_capsule_stored_on_arweave` | Capsule保存を検証 | Mutation |
| 2 | `test_encrypted_shares_stored_on_arweave` | シェア保存を検証 | Mutation |
| 3 | `test_capsule_tags_correct` | Capsuleタグを検証 | Success |
| 4 | `test_share_tags_correct` | シェアタグを検証 | Success |
| 5 | `test_arweave_storage_failure` | ストレージエラー伝播を検証 | Error |
| 6 | `test_result_contains_real_tx_ids` | 結果にArweave tx_idが含まれることを検証 | Success |

### Requirement 4: get_secret_status()の実装

**User Story:** As a データ所有者, I want 秘密のステータスを照会したい, so that ライフサイクルを管理できる

#### Design Note

`ArweaveStorageService.query_by_tags()` を使用して `secret_id` に対応するトランザクションを検索し、その存在と種類に基づいて `SecretStatus` を決定する。

#### Acceptance Criteria

1. WHEN `get_secret_status()` が呼び出された THEN system SHALL `query_by_tags()` で `secret_id` タグを検索する
2. WHEN Capsuleトランザクションが存在する THEN system SHALL `SecretStatus::Created` を返却する
3. WHEN 対応するデータが存在しない THEN system SHALL `WorkflowError::ResourceNotFound` を返却する
4. WHEN StorageServiceからエラーが発生した THEN system SHALL `WorkflowError::StorageError` を返却する

#### Test Coverage

| AC# | Test Function(s) | Purpose | Pattern |
|-----|------------------|---------|---------|
| 1 | `test_get_status_queries_by_secret_id` | タグ検索呼び出しを検証 | Query |
| 2 | `test_get_status_returns_created` | Created状態の返却を検証 | Success |
| 3 | `test_get_status_not_found` | 存在しないsecret_idエラーを検証 | Validation |
| 4 | `test_get_status_storage_error` | StorageServiceエラー伝播を検証 | Error |

### Requirement 5: StorageServiceトレイト拡張

**User Story:** As a 開発者, I want ArweaveStorageServiceにAO通信メソッドを追加したい, so that kFrag送信が可能になる

#### Acceptance Criteria

1. WHEN `ArweaveStorageService` traitが定義される THEN trait SHALL `send_kfrag_to_owner_process(&self, kfrags: &[KeyFragment], owner_process_id: &str) -> ServiceResult<()>` メソッドを含む
2. WHEN `ArweaveStorageServiceImpl` が実装される THEN impl SHALL `send_kfrag_to_owner_process` のプレースホルダ実装を提供する (Issue #47完了までの暫定)
3. WHEN kFrag送信メソッドが定義される THEN method SHALL `ServiceError` を返却型に使用する

#### Test Coverage

| AC# | Test Function(s) | Purpose | Pattern |
|-----|------------------|---------|---------|
| 1 | `test_storage_trait_has_kfrag_method` | traitメソッドの存在を検証 | Success |
| 2 | `test_storage_impl_kfrag_placeholder` | プレースホルダ実装を検証 | Success |

### Requirement 6: DI Container更新

**User Story:** As a 開発者, I want WorkflowServiceContainerがStorageServiceを含む構成を受け付けたい, so that 正しく依存性が注入される

#### Acceptance Criteria

1. WHEN `WorkflowServiceContainer` が構築される THEN container SHALL `SecretSharingWorkflowServiceImpl` にCryptoServiceとStorageServiceの両方を注入する
2. WHEN 既存のBuilderパターンがStorageServiceを使用する THEN builder SHALL StorageService経由でworkflow serviceを構築する

#### Test Coverage

| AC# | Test Function(s) | Purpose | Pattern |
|-----|------------------|---------|---------|
| 1 | `test_container_injects_storage` | DIコンテナの構成を検証 | Success |
| 2 | `test_builder_uses_storage_service` | Builder統合を検証 | Success |

## Non-Functional Requirements

### Code Architecture and Modularity

- **後方互換性**: StorageService追加は既存テストのコンパイルを壊さない（MockStorageServiceを提供）
- **Core Service依存**: StorageServiceのtraitのみに依存、具体実装には依存しない
- **段階的移行**: Issue #47未完了でもプレースホルダ実装で動作可能

### Security

- **秘密データのZeroize**: kFragシリアライズ時の中間データは使用後にクリア
- **エラー情報の制限**: AO通信エラーにkFrag内容を含めない
- **タグの安全性**: Arweaveタグに秘密情報を含めない（secret_idは公開情報）

### Reliability

- **アトミック保存**: `batch_store()` 使用でCapsuleとシェアを可能な限り一括保存
- **エラー回復**: Arweave保存失敗はリトライ可能な情報を含む
- **AO通信信頼性**: kFrag送信失敗は適切なエラー情報を提供

## Test Coverage Summary

| Requirement | Total AC | Test Functions | Coverage |
|-------------|----------|----------------|----------|
| 1. StorageService DI | 3 | 1 | 100% |
| 2. kFrag送信 | 4 | 4 | 100% |
| 3. Arweaveストレージ | 6 | 6 | 100% |
| 4. get_secret_status() | 4 | 4 | 100% |
| 5. StorageService拡張 | 3 | 2 | 100% |
| 6. DI Container更新 | 2 | 2 | 100% |
| **Total** | **22** | **19** | **100%** |
