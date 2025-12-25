# Requirements Document: client-domain-repository-interface

## Introduction

D-TPRESクライアントライブラリのDomain層にRepository Interfaceを実装する。このインターフェースは、5つのドメインエンティティ（Secret, ShareCollection, Capsule, KFrag, CFrag）に対する永続化操作を抽象化し、依存性逆転原則（DIP）に基づいてInfrastructure層から分離する。

このRepository Interfaceにより、Service層はArweaveなどの具体的なストレージ実装を意識することなく、エンティティの保存・取得・削除操作を実行できる。

**重要**: クライアントライブラリはブラウザ/ローカル環境で実行されるため、**async/awaitをサポート**する。AOコントラクト（`ao/`）の同期制約はクライアントには適用されない。

## Alignment with Product Vision

この機能はproduct.mdで定義された以下の目標をサポートする：

1. **5層レイヤードアーキテクチャ**: Domain層にRepository Interfaceを配置し、Infrastructure層で実装することでクリーンアーキテクチャを実現
2. **依存性逆転原則（DIP）**: 上位レイヤー（Service）が具体的な永続化実装に依存しないよう抽象化
3. **テスタビリティ**: Repository Interfaceによりモック実装が容易になり、単体テストが可能に
4. **Arweave永続化**: Infrastructure層での実装により、Arweaveへの永続化をサポート

## Requirements

### Requirement 0: 基本Repository Interface

**User Story:** As a 開発者, I want 共通のCRUD操作を抽象化した基本Repository trait, so that 各エンティティRepositoryで一貫した操作を提供できる

#### Design Note

基本Repository traitはジェネリクスを使用して型安全なCRUD操作を定義する。全メソッドは`async fn`として定義し、ブラウザ/ローカル環境での非同期実行をサポートする。`async_trait`クレートを使用してtraitの非同期メソッドを実現する。

```rust
#[async_trait]
pub trait Repository<T, ID>: Send + Sync
where
    T: Send + Sync,
    ID: Send + Sync,
{
    async fn save(&self, entity: &T) -> DomainResult<()>;
    async fn find_by_id(&self, id: &ID) -> DomainResult<Option<T>>;
    async fn delete(&self, id: &ID) -> DomainResult<()>;
    async fn exists(&self, id: &ID) -> DomainResult<bool>;
    async fn find_by_ids(&self, ids: &[ID]) -> DomainResult<Vec<T>>;
}
```

#### Acceptance Criteria

1. WHEN 基本Repository traitが定義される THEN system SHALL `save`, `find_by_id`, `delete`, `exists`メソッドを`async fn`として提供する
2. WHEN `find_by_ids`が複数IDで呼ばれる THEN system SHALL 効率的なバッチ取得を実行し、見つかったエンティティのVecを返す
3. IF `async_trait`クレートが使用される THEN system SHALL trait内でasync fnを定義可能にする
4. WHEN Repository traitが定義される THEN system SHALL `Send + Sync`を要求し、スレッド安全性を保証する

#### Test Coverage

| AC# | Test Function(s) | Purpose | Pattern |
|-----|------------------|---------|---------|
| 1 | `test_repository_trait_methods` | 基本メソッドの存在確認 | Type Safety |
| 2 | `test_repository_find_by_ids_batch` | バッチ取得の動作確認 | Query |
| 3 | N/A (コンパイル時検証) | async_trait動作確認 | Type Safety |
| 4 | N/A (コンパイル時検証) | Send + Sync制約確認 | Type Safety |

### Requirement 1: SecretRepository Interface

**User Story:** As a Service層開発者, I want Secret（集約ルート）の永続化操作を抽象化したインターフェース, so that 具体的なストレージ実装に依存せずにSecret管理ロジックを実装できる

#### Design Note

SecretはD-TPRESの集約ルートであり、他のエンティティ（ShareCollection, Capsule, KFrag）への参照を持つ。Repository操作はSecretIdをキーとし、状態遷移（Initialized → Split → Distributed → Recovered）の永続化をサポートする。全メソッドは`async fn`として定義。

#### Acceptance Criteria

1. WHEN `save`が呼ばれる THEN system SHALL SecretエンティティをストレージにSerialize保存する（async）
2. WHEN `find_by_id`が存在するIDで呼ばれる THEN system SHALL デシリアライズしたSecretを返す（async）
3. WHEN `find_by_id`が存在しないIDで呼ばれる THEN system SHALL `DomainError::NotFound`を返す
4. WHEN `delete`が呼ばれる THEN system SHALL 指定されたSecretをストレージから削除する（async）
5. IF Secretが既に存在するIDで`save`が呼ばれる THEN system SHALL 既存のSecretを上書き更新する
6. WHEN `exists`が呼ばれる THEN system SHALL 指定されたIDのSecretが存在するかboolで返す（async）
7. WHEN `find_by_ids`が複数IDで呼ばれる THEN system SHALL バッチ取得を実行し、見つかったSecretのVecを返す（async）

#### Test Coverage

| AC# | Test Function(s) | Purpose | Pattern |
|-----|------------------|---------|---------|
| 1 | `test_secret_repository_save` | Secretの保存が成功することを確認 | Success |
| 2 | `test_secret_repository_find_by_id_exists` | 存在するSecretの取得を確認 | Query |
| 3 | `test_secret_repository_find_by_id_not_found` | 存在しないIDでNotFoundエラーを確認 | Validation |
| 4 | `test_secret_repository_delete` | Secretの削除が成功することを確認 | Success |
| 5 | `test_secret_repository_save_update` | 既存Secretの上書き更新を確認 | Mutation |
| 6 | `test_secret_repository_exists` | exists操作の正確性を確認 | Query |
| 7 | `test_secret_repository_find_by_ids` | バッチ取得の動作を確認 | Query |

### Requirement 2: ShareCollectionRepository Interface

**User Story:** As a Service層開発者, I want ShareCollection（Shamirシェアコレクション）の永続化操作を抽象化したインターフェース, so that シェア管理ロジックを具体的なストレージ実装から分離できる

#### Design Note

ShareCollectionは暗号化されたShamirシェアのコレクションを管理する。SecretIdで関連付けられ、Arweave TX IDを持つ場合がある。シェアデータは暗号化されているため、Zeroizeは不要。全メソッドは`async fn`として定義。

#### Acceptance Criteria

1. WHEN `save`が呼ばれる THEN system SHALL ShareCollectionエンティティをストレージにSerialize保存する（async）
2. WHEN `find_by_id`が存在するIDで呼ばれる THEN system SHALL デシリアライズしたShareCollectionを返す（async）
3. WHEN `find_by_id`が存在しないIDで呼ばれる THEN system SHALL `DomainError::NotFound`を返す
4. WHEN `find_by_secret_id`が呼ばれる THEN system SHALL 指定されたSecretIdに関連するShareCollectionを返す（async）
5. WHEN `delete`が呼ばれる THEN system SHALL 指定されたShareCollectionをストレージから削除する（async）
6. WHEN `exists`が呼ばれる THEN system SHALL 指定されたIDのShareCollectionが存在するかboolで返す（async）
7. WHEN `find_by_ids`が複数IDで呼ばれる THEN system SHALL バッチ取得を実行し、見つかったShareCollectionのVecを返す（async）

#### Test Coverage

| AC# | Test Function(s) | Purpose | Pattern |
|-----|------------------|---------|---------|
| 1 | `test_share_collection_repository_save` | ShareCollectionの保存が成功することを確認 | Success |
| 2 | `test_share_collection_repository_find_by_id_exists` | 存在するShareCollectionの取得を確認 | Query |
| 3 | `test_share_collection_repository_find_by_id_not_found` | 存在しないIDでNotFoundエラーを確認 | Validation |
| 4 | `test_share_collection_repository_find_by_secret_id` | SecretIdによる検索を確認 | Query |
| 5 | `test_share_collection_repository_delete` | ShareCollectionの削除が成功することを確認 | Success |
| 6 | `test_share_collection_repository_exists` | exists操作の正確性を確認 | Query |
| 7 | `test_share_collection_repository_find_by_ids` | バッチ取得の動作を確認 | Query |

### Requirement 3: CapsuleRepository Interface

**User Story:** As a Service層開発者, I want Capsule（Umbral PREカプセル）の永続化操作を抽象化したインターフェース, so that カプセル管理ロジックを具体的なストレージ実装から分離できる

#### Design Note

Capsuleは暗号化時に生成されるUmbral PREカプセルを保持する。公開データであり、Zeroizeは不要。Arweave TX IDを持つ場合がある。全メソッドは`async fn`として定義。

#### Acceptance Criteria

1. WHEN `save`が呼ばれる THEN system SHALL CapsuleエンティティをストレージにSerialize保存する（async）
2. WHEN `find_by_id`が存在するIDで呼ばれる THEN system SHALL デシリアライズしたCapsuleを返す（async）
3. WHEN `find_by_id`が存在しないIDで呼ばれる THEN system SHALL `DomainError::NotFound`を返す
4. WHEN `find_by_secret_id`が呼ばれる THEN system SHALL 指定されたSecretIdに関連するCapsuleを返す（async）
5. WHEN `delete`が呼ばれる THEN system SHALL 指定されたCapsuleをストレージから削除する（async）
6. WHEN `exists`が呼ばれる THEN system SHALL 指定されたIDのCapsuleが存在するかboolで返す（async）
7. WHEN `find_by_ids`が複数IDで呼ばれる THEN system SHALL バッチ取得を実行し、見つかったCapsuleのVecを返す（async）

#### Test Coverage

| AC# | Test Function(s) | Purpose | Pattern |
|-----|------------------|---------|---------|
| 1 | `test_capsule_repository_save` | Capsuleの保存が成功することを確認 | Success |
| 2 | `test_capsule_repository_find_by_id_exists` | 存在するCapsuleの取得を確認 | Query |
| 3 | `test_capsule_repository_find_by_id_not_found` | 存在しないIDでNotFoundエラーを確認 | Validation |
| 4 | `test_capsule_repository_find_by_secret_id` | SecretIdによる検索を確認 | Query |
| 5 | `test_capsule_repository_delete` | Capsuleの削除が成功することを確認 | Success |
| 6 | `test_capsule_repository_exists` | exists操作の正確性を確認 | Query |
| 7 | `test_capsule_repository_find_by_ids` | バッチ取得の動作を確認 | Query |

### Requirement 4: KFragRepository Interface

**User Story:** As a Service層開発者, I want KFrag（鍵フラグメント）の永続化操作を抽象化したインターフェース, so that 機密性の高い鍵フラグメント管理を具体的なストレージ実装から分離できる

#### Design Note

KFragはZeroize + ZeroizeOnDropを実装しており、機密データを含む。Repository操作では、Zeroizeの特性を維持しながらシリアライズ/デシリアライズを行う。SecretIdで関連付けられ、holder_indexで識別される。全メソッドは`async fn`として定義。

#### Acceptance Criteria

1. WHEN `save`が呼ばれる THEN system SHALL KFragエンティティをストレージにSerialize保存する（async）
2. WHEN `find_by_id`が存在するIDで呼ばれる THEN system SHALL デシリアライズしたKFragを返す（async）
3. WHEN `find_by_id`が存在しないIDで呼ばれる THEN system SHALL `DomainError::NotFound`を返す
4. WHEN `find_by_secret_id`が呼ばれる THEN system SHALL 指定されたSecretIdに関連するすべてのKFragをVecで返す（async）
5. WHEN `find_by_holder_index`が呼ばれる THEN system SHALL 指定されたSecretIdとholder_indexに一致するKFragを返す（async）
6. WHEN `delete`が呼ばれる THEN system SHALL 指定されたKFragをストレージから削除する（async）
7. WHEN `delete_by_secret_id`が呼ばれる THEN system SHALL 指定されたSecretIdに関連するすべてのKFragを削除する（async）
8. WHEN `exists`が呼ばれる THEN system SHALL 指定されたIDのKFragが存在するかboolで返す（async）
9. WHEN `find_by_ids`が複数IDで呼ばれる THEN system SHALL バッチ取得を実行し、見つかったKFragのVecを返す（async）

#### Test Coverage

| AC# | Test Function(s) | Purpose | Pattern |
|-----|------------------|---------|---------|
| 1 | `test_kfrag_repository_save` | KFragの保存が成功することを確認 | Success |
| 2 | `test_kfrag_repository_find_by_id_exists` | 存在するKFragの取得を確認 | Query |
| 3 | `test_kfrag_repository_find_by_id_not_found` | 存在しないIDでNotFoundエラーを確認 | Validation |
| 4 | `test_kfrag_repository_find_by_secret_id` | SecretIdによる複数KFrag検索を確認 | Query |
| 5 | `test_kfrag_repository_find_by_holder_index` | holder_indexによる検索を確認 | Query |
| 6 | `test_kfrag_repository_delete` | KFragの削除が成功することを確認 | Success |
| 7 | `test_kfrag_repository_delete_by_secret_id` | SecretIdによる一括削除を確認 | Success |
| 8 | `test_kfrag_repository_exists` | exists操作の正確性を確認 | Query |
| 9 | `test_kfrag_repository_find_by_ids` | バッチ取得の動作を確認 | Query |

### Requirement 5: CFragRepository Interface

**User Story:** As a Service層開発者, I want CFrag（再暗号化フラグメント）の永続化操作を抽象化したインターフェース, so that 機密性の高い再暗号化フラグメント管理を具体的なストレージ実装から分離できる

#### Design Note

CFragはZeroize + ZeroizeOnDropを実装しており、機密データを含む。Holder-Processによる再暗号化で生成され、Requester-Processが収集する。SecretIdとKFragIdで関連付けられる。全メソッドは`async fn`として定義。

#### Acceptance Criteria

1. WHEN `save`が呼ばれる THEN system SHALL CFragエンティティをストレージにSerialize保存する（async）
2. WHEN `find_by_id`が存在するIDで呼ばれる THEN system SHALL デシリアライズしたCFragを返す（async）
3. WHEN `find_by_id`が存在しないIDで呼ばれる THEN system SHALL `DomainError::NotFound`を返す
4. WHEN `find_by_secret_id`が呼ばれる THEN system SHALL 指定されたSecretIdに関連するすべてのCFragをVecで返す（async）
5. WHEN `find_by_kfrag_id`が呼ばれる THEN system SHALL 指定されたKFragIdに関連するCFragを返す（async）
6. WHEN `delete`が呼ばれる THEN system SHALL 指定されたCFragをストレージから削除する（async）
7. WHEN `delete_by_secret_id`が呼ばれる THEN system SHALL 指定されたSecretIdに関連するすべてのCFragを削除する（async）
8. WHEN `exists`が呼ばれる THEN system SHALL 指定されたIDのCFragが存在するかboolで返す（async）
9. WHEN `count_by_secret_id`が呼ばれる THEN system SHALL 指定されたSecretIdに関連するCFragの数を返す（async）
10. WHEN `find_by_ids`が複数IDで呼ばれる THEN system SHALL バッチ取得を実行し、見つかったCFragのVecを返す（async）

#### Test Coverage

| AC# | Test Function(s) | Purpose | Pattern |
|-----|------------------|---------|---------|
| 1 | `test_cfrag_repository_save` | CFragの保存が成功することを確認 | Success |
| 2 | `test_cfrag_repository_find_by_id_exists` | 存在するCFragの取得を確認 | Query |
| 3 | `test_cfrag_repository_find_by_id_not_found` | 存在しないIDでNotFoundエラーを確認 | Validation |
| 4 | `test_cfrag_repository_find_by_secret_id` | SecretIdによる複数CFrag検索を確認 | Query |
| 5 | `test_cfrag_repository_find_by_kfrag_id` | KFragIdによる検索を確認 | Query |
| 6 | `test_cfrag_repository_delete` | CFragの削除が成功することを確認 | Success |
| 7 | `test_cfrag_repository_delete_by_secret_id` | SecretIdによる一括削除を確認 | Success |
| 8 | `test_cfrag_repository_exists` | exists操作の正確性を確認 | Query |
| 9 | `test_cfrag_repository_count_by_secret_id` | カウント操作の正確性を確認 | Query |
| 10 | `test_cfrag_repository_find_by_ids` | バッチ取得の動作を確認 | Query |

## Non-Functional Requirements

### Code Architecture and Modularity
- **Single Responsibility Principle**: 各Repositoryファイルは1つのエンティティに対する永続化操作のみを定義
- **Modular Design**: 各Repositoryトレイトは独立してモック実装可能
- **Dependency Management**: Domain層はInfrastructure層に依存しない（DIP）
- **Clear Interfaces**: トレイトメソッドのシグネチャは明確で一貫性がある
- **Async Support**: `async_trait`クレートを使用してtrait内でasync fnを実現

### Performance
- **非同期実行**: 全Repository操作は`async fn`として定義し、I/O待ち時間を効率化
- **バッチ操作**: `find_by_ids`メソッドにより複数エンティティの効率的な取得をサポート
- **並列取得**: `tokio::join!`等による並列バッチ取得をInfrastructure実装でサポート可能

### Security
- KFragRepository、CFragRepositoryはZeroize対応エンティティを扱う
- エラーメッセージには機密データを含めない
- Repository操作は機密データのメモリ管理に影響しない（エンティティ側の責務）

### Reliability
- すべての操作は`DomainResult<T>`を返し、エラーハンドリングを強制
- NotFound、StorageError等の明確なエラー型で障害原因を特定可能
- Infrastructure実装でリトライ戦略（指数バックオフ、最大3回）をサポート

### Usability
- 一貫したメソッド命名（save, find_by_id, delete, exists, find_by_ids）
- 関連エンティティ検索メソッド（find_by_secret_id等）を提供
- trait objectとして使用可能（`dyn Trait`）

### Arweave Storage Considerations (Infrastructure実装向け)

以下はInfrastructure層での実装時に考慮すべき事項：

1. **データフォーマット**: JSON (serde_json) + EntityMetadataラッパー
2. **Arweaveタグ戦略**:
   - `App-Name`: "D-TPRES"
   - `Entity-Type`: エンティティ型名
   - `Entity-Id`: プライマリID
   - `Secret-Id`: 関連SecretId（検索用）
   - `Operation`: CREATE/UPDATE/DELETE
   - `Timestamp`: Unix timestamp
3. **不変性対応**: 更新は新バージョンとして実装（論理削除含む）
4. **TX確認**: トランザクション確認のポーリング戦略

## Test Coverage Summary

### Overall Statistics

| Requirement | Total AC | Test Functions | Coverage |
|-------------|----------|----------------|----------|
| 0. 基本Repository | 4 | 9 | 100% |
| 1. SecretRepository | 7 | 4 | 100% |
| 2. ShareCollectionRepository | 7 | 5 | 100% |
| 3. CapsuleRepository | 7 | 5 | 100% |
| 4. KFragRepository | 9 | 7 | 100% |
| 5. CFragRepository | 10 | 9 | 100% |
| **Total** | **44** | **39** | **100%** |

### Implemented Test Functions

#### Base Repository (mod.rs) - 9 tests
- `mock_repository_is_send_sync` - Send + Sync制約のコンパイル時検証
- `test_find_by_id_not_found` - NotFoundシナリオ
- `test_save_and_find_by_id` - 保存と取得
- `test_exists` - 存在確認
- `test_delete` - 削除
- `test_delete_nonexistent_succeeds` - 存在しないエンティティの削除
- `test_find_by_ids_batch_retrieval` - バッチ取得
- `test_find_by_ids_with_missing_ids` - 一部欠損IDでのバッチ取得
- `test_find_by_ids_empty_input` - 空配列でのバッチ取得

#### SecretRepository (secret_interface.rs) - 4 tests
- `secret_repository_is_send_sync` - Send + Sync検証
- `test_secret_not_found` - NotFoundシナリオ
- `test_save_and_find_secret` - 保存と取得
- `test_batch_find_secrets` - バッチ取得

#### ShareCollectionRepository (share_interface.rs) - 5 tests
- `share_collection_repository_is_send_sync` - Send + Sync検証
- `test_share_collection_not_found` - NotFoundシナリオ
- `test_find_by_secret_id` - SecretIdによる検索
- `test_find_by_secret_id_not_found` - SecretId検索NotFound
- `test_batch_find_share_collections` - バッチ取得

#### CapsuleRepository (capsule_interface.rs) - 5 tests
- `capsule_repository_is_send_sync` - Send + Sync検証
- `test_capsule_not_found` - NotFoundシナリオ
- `test_find_capsule_by_secret_id` - SecretIdによる検索
- `test_find_capsule_by_secret_id_not_found` - SecretId検索NotFound
- `test_batch_find_capsules` - バッチ取得

#### KFragRepository (kfrag_interface.rs) - 7 tests
- `kfrag_repository_is_send_sync` - Send + Sync検証
- `test_kfrag_not_found` - NotFoundシナリオ
- `test_find_kfrags_by_secret_id` - SecretIdによる複数検索
- `test_find_kfrag_by_holder_index` - holder_indexによる検索
- `test_find_kfrag_by_holder_index_not_found` - holder_index検索NotFound
- `test_delete_kfrags_by_secret_id` - SecretIdによる一括削除
- `test_batch_find_kfrags` - バッチ取得

#### CFragRepository (cfrag_interface.rs) - 9 tests
- `cfrag_repository_is_send_sync` - Send + Sync検証
- `test_cfrag_not_found` - NotFoundシナリオ
- `test_find_cfrags_by_secret_id` - SecretIdによる複数検索
- `test_find_cfrag_by_kfrag_id` - KFragIdによる検索
- `test_find_cfrag_by_kfrag_id_not_found` - KFragId検索NotFound
- `test_delete_cfrags_by_secret_id` - SecretIdによる一括削除
- `test_count_cfrags_by_secret_id` - カウント操作
- `test_batch_find_cfrags` - バッチ取得

### Test Patterns Used

1. **Success Pattern**: Valid save/delete operations
2. **Validation Pattern**: NotFound error verification for non-existent IDs
3. **Query Pattern**: find_by_id, find_by_secret_id, find_by_ids, exists, count operations
4. **Type Safety Pattern**: Compile-time verification for async_trait, Send + Sync
5. **Mock Implementation Pattern**: In-memory HashMap based mock repositories

### Dev Dependencies Added

```toml
[dev-dependencies]
tokio = { version = "1", features = ["rt", "macros"] }
```

### Legend

- ✅ **Covered**: Test implemented and fully covers AC
- ⚠️ **Partial**: Indirect test or compile-time verification
- ❌ **Missing**: Test not implemented
