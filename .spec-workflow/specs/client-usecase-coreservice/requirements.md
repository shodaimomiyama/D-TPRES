# Requirements Document: client-usecase-coreservice

## Introduction

D-TPRESクライアントライブラリのUseCase層におけるCoreService群の仕様を定義する。CoreServiceは複数のUseCaseService（SecretSharingService、SecretRecoveryService等）から呼び出される共通業務ロジックを提供し、tech.mdで定義された以下の2つのサービスで構成される：

1. **CryptoService**: TPRE・Shamir操作（純粋な暗号ロジック）
   - Umbral TPRE（umbral-pre 0.11）
   - Shamir Secret Sharing（shamirsecretsharing 0.1）
   - AES-GCM対称暗号（aes-gcm）- シェア暗号化用
   - メモリ安全性（zeroize 1.8）
   - 定数時間操作（subtle 2.6）

2. **StorageService**: Arweave操作
   - データ永続化（Capsule、暗号化シェア、kFrag/cFrag）
   - タグベースクエリ
   - トランザクションステータス管理

**ディレクトリ構造**（structure.md準拠）:
```
client/src/usecase/
├── mod.rs              # UseCase層エクスポート
├── error.rs            # ServiceError階層定義
└── core/               # CoreService群
    ├── mod.rs          # CoreServiceエクスポート
    ├── crypto.rs       # CryptoService実装
    └── storage.rs      # StorageService実装
```

**本仕様の方針**: steering documentsを正として要件を定義し、現状の実装で利用できる部分は再利用する。

## Alignment with Product Vision

この機能はproduct.mdで定義された以下の目標をサポートする：

1. **k-of-n閾値プロキシ再暗号化**: CryptoServiceがUmbral TPREとShamir Secret Sharingを実装し、閾値暗号化の中核機能を提供
2. **Arweave永続ストレージ**: StorageServiceがCapsule、暗号化シェア、メタデータの永続化をサポート
3. **Clean Architecture**: CoreServiceがUseCase層の共通ロジックとして分離され、責務の境界で分離された再利用可能な単位として構成

**処理フロー**（tech.md準拠）:
- Phase 1: 秘密分散（Shamir）→ 暗号化（AES-GCM）→ Capsule作成（Umbral）→ kFrag分割 → Arweave保存
- Phase 3: cFrag収集 → Capsule結合 → 復号（Umbral）→ シャミア補間

## Requirements

### Requirement 1: CryptoService - 暗号操作サービス

**User Story:** As a UseCaseService開発者, I want 暗号操作を抽象化したCryptoService, so that Shamir秘密分散、Umbral TPRE、AES-GCM暗号化の複雑な暗号処理を簡潔に利用できる

#### Design Note

CryptoServiceはD-TPRESの中核となる暗号機能を提供する。tech.mdで定義された以下のライブラリを使用：

- **umbral-pre 0.11**: 閾値プロキシ再暗号化（Umbral）
- **shamirsecretsharing 0.1**: シャミア秘密分散（k-of-n）
- **aes-gcm**: シェア暗号化用対称暗号
- **zeroize 1.8**: 秘密情報のメモリクリア
- **subtle 2.6**: 定数時間暗号操作

すべての暗号操作は同期的に実行される（ブラウザ/WASM環境対応）。

**機能カテゴリ**:

1. **Shamir Secret Sharing**: 秘密分散・復元
2. **Umbral TPRE**: プロキシ再暗号化操作
3. **Symmetric Encryption**: シェア暗号化（AES-GCM）
4. **Key Management**: 鍵ペア生成・kFrag生成

#### Acceptance Criteria

**Shamir Secret Sharing**:
1. WHEN `split_secret_shamir`が有効なパラメータ（secret, k, n）で呼ばれる THEN system SHALL n個のShamirシェアを生成し、任意のk個で復元可能にする
2. WHEN `reconstruct_secret_shamir`が閾値k以上のシェアで呼ばれる THEN system SHALL 元の秘密を正確に復元する
3. IF k > n または k < 1 THEN system SHALL ValidationErrorを返す

**Umbral TPRE**:
4. WHEN `generate_keypair`が呼ばれる THEN system SHALL umbral-preの有効な鍵ペア（SecretKey, PublicKey）を生成する
5. WHEN `create_pre_capsule`が有効な公開鍵と平文で呼ばれる THEN system SHALL Umbral暗号化を実行しCapsuleと暗号文を返す
6. WHEN `generate_kfrags`が委任者秘密鍵、受信者公開鍵、閾値パラメータで呼ばれる THEN system SHALL n個のkFragsを生成する
7. WHEN `verify_cfrag`がcFragと検証情報で呼ばれる THEN system SHALL cFragの正当性を検証しboolを返す
8. WHEN `decrypt_original`がOwner秘密鍵とCapsuleで呼ばれる THEN system SHALL 元の平文を復号する
9. WHEN `decrypt_reencrypted`が閾値以上のcFragsとRequester秘密鍵で呼ばれる THEN system SHALL 再暗号化されたデータを復号し元の平文を返す

**Symmetric Encryption（AES-GCM）**:
10. WHEN `encrypt_share`がシェアデータと鍵で呼ばれる THEN system SHALL AES-GCMで暗号化されたデータを返す
11. WHEN `decrypt_share`が暗号化データと鍵で呼ばれる THEN system SHALL 復号されたシェアデータを返す
12. IF 復号に失敗する THEN system SHALL CryptoErrorを返す

**Memory Safety**:
13. IF 秘密鍵を含む構造体がドロップされる THEN system SHALL Zeroize + ZeroizeOnDropにより自動的にメモリをクリアする
14. IF subtleクレートを使用する比較操作 THEN system SHALL 定数時間で実行される

#### Test Coverage

| AC# | Test Function(s) | Purpose | Pattern |
|-----|------------------|---------|---------|
| 1 | `test_shamir_split_valid_params` | 有効なk,nでシェア生成を確認 | Success |
| 2 | `test_shamir_reconstruct_with_threshold` | k個のシェアで復元を確認 | Success |
| 2 | `test_shamir_reconstruct_with_more_than_threshold` | k+1個のシェアで復元を確認 | Success |
| 3 | `test_shamir_split_invalid_threshold_k_greater_than_n` | k > nでValidationError | Validation |
| 3 | `test_shamir_split_invalid_threshold_k_zero` | k = 0でValidationError | Validation |
| 4 | `test_generate_keypair` | 鍵ペア生成を確認 | Success |
| 5 | `test_create_pre_capsule` | Capsule生成を確認 | Success |
| 6 | `test_generate_kfrags` | kFrags生成を確認 | Success |
| 6 | `test_generate_kfrags_threshold_validation` | kFrag閾値検証 | Validation |
| 7 | `test_verify_cfrag_valid` | 有効なcFrag検証 | Success |
| 7 | `test_verify_cfrag_invalid` | 無効なcFrag検証 | Validation |
| 8 | `test_decrypt_original` | オリジナル復号を確認 | Success |
| 9 | `test_decrypt_reencrypted` | 再暗号化復号を確認 | Success |
| 9 | `test_decrypt_reencrypted_insufficient_cfrags` | cFrag不足時エラー | Validation |
| 10 | `test_encrypt_share_aes_gcm` | AES-GCM暗号化を確認 | Success |
| 11 | `test_decrypt_share_aes_gcm` | AES-GCM復号を確認 | Success |
| 12 | `test_decrypt_share_invalid_ciphertext` | 不正な暗号文でCryptoError | Validation |
| 12 | `test_decrypt_share_wrong_key` | 不正な鍵でCryptoError | Validation |
| 13 | N/A (コンパイル時検証) | Zeroize + ZeroizeOnDrop実装確認 | Type Safety |
| 14 | N/A (コンパイル時検証) | subtle crate使用確認 | Type Safety |

### Requirement 2: StorageService - ストレージサービス

**User Story:** As a UseCaseService開発者, I want Arweave操作を抽象化したStorageService, so that 永続ストレージの複雑さを意識せずにデータ永続化を実行できる

#### Design Note

StorageServiceはArweaveへのデータ永続化を抽象化する。tech.mdで定義された以下のデータを保存：

**保存対象**（tech.md準拠）:
- Capsule（暗号カプセル）
- 暗号化シェア（Cᵢ）- AES-GCMで暗号化されたShamirシェア
- kFrag（鍵フラグメント）
- cFrag（再暗号化フラグメント）
- メタデータ

**データフォーマット**:
- bincode: バイナリデータ（Capsule、暗号化シェア、kFrag、cFrag）
- JSON: メタデータ

**設計原則**:
- Repository Interfaceを通じてAdapter層と連携（DIP）
- 実際のArweave通信はAdapter層のArweaveClientが担当
- CoreServiceとしてはストレージ操作のビジネスロジックを提供

#### Acceptance Criteria

**基本操作**:
1. WHEN `store_capsule`が有効なCapsuleで呼ばれる THEN system SHALL Arweaveに保存しトランザクションIDを返す
2. WHEN `store_encrypted_share`が暗号化シェアで呼ばれる THEN system SHALL Arweaveに保存しトランザクションIDを返す
3. WHEN `store_kfrag`がkFragで呼ばれる THEN system SHALL Arweaveに保存しトランザクションIDを返す
4. WHEN `store_cfrag`がcFragで呼ばれる THEN system SHALL Arweaveに保存しトランザクションIDを返す

**取得操作**:
5. WHEN `retrieve_capsule`が存在するIDで呼ばれる THEN system SHALL 保存されたCapsuleを返す
6. WHEN `retrieve_encrypted_shares`がSecretIDで呼ばれる THEN system SHALL 関連する暗号化シェアをVecで返す
7. WHEN `retrieve_kfrags`がSecretIDで呼ばれる THEN system SHALL 関連するkFragsをVecで返す
8. WHEN `retrieve_cfrags`がSecretIDで呼ばれる THEN system SHALL 関連するcFragsをVecで返す

**クエリ操作**:
9. WHEN `query_by_tags`が有効なタグで呼ばれる THEN system SHALL マッチするトランザクションをVecで返す
10. WHEN `exists`がトランザクションIDで呼ばれる THEN system SHALL 存在確認結果をboolで返す

**エラー処理**:
11. IF 存在しないIDで取得が試みられる THEN system SHALL NotFoundエラーを返す
12. IF ストレージ操作が失敗する THEN system SHALL StorageErrorを返す

#### Test Coverage

| AC# | Test Function(s) | Purpose | Pattern |
|-----|------------------|---------|---------|
| 1 | `test_store_capsule` | Capsule保存を確認 | Success |
| 2 | `test_store_encrypted_share` | 暗号化シェア保存を確認 | Success |
| 3 | `test_store_kfrag` | kFrag保存を確認 | Success |
| 4 | `test_store_cfrag` | cFrag保存を確認 | Success |
| 5 | `test_retrieve_capsule_exists` | 存在するCapsule取得を確認 | Query |
| 6 | `test_retrieve_encrypted_shares_by_secret_id` | SecretIDで暗号化シェア取得 | Query |
| 7 | `test_retrieve_kfrags_by_secret_id` | SecretIDでkFrag取得 | Query |
| 8 | `test_retrieve_cfrags_by_secret_id` | SecretIDでcFrag取得 | Query |
| 9 | `test_query_by_tags` | タグクエリを確認 | Query |
| 9 | `test_query_by_tags_no_match` | マッチなしで空Vec | Query |
| 10 | `test_exists_true` | 存在する場合true | Query |
| 10 | `test_exists_false` | 存在しない場合false | Query |
| 11 | `test_retrieve_capsule_not_found` | 存在しないIDでNotFound | Validation |
| 11 | `test_retrieve_kfrags_empty` | 関連なしで空Vec | Query |
| 12 | `test_store_capsule_storage_error` | ストレージエラーを確認 | Validation |

### Requirement 3: ServiceError階層

**User Story:** As a Service層開発者, I want 構造化されたエラー型, so that ビジネスエラーとシステムエラーを明確に区別し適切なエラーハンドリングを実装できる

#### Design Note

tech.mdで定義されたthiserror crateを使用し、TERASOLUNAガイドラインに従ってBusinessException（回復可能なビジネスエラー）とSystemException（回復不能なシステムエラー）を区別する。

**エラー階層**:
```
ServiceError
├── Business(BusinessException)
│   ├── ValidationError      # 入力検証エラー（閾値不正等）
│   ├── ResourceNotFound     # リソース未検出
│   ├── ThresholdNotMet      # 閾値未達（k個のシェア/cFrag不足）
│   └── InvalidOperation     # 不正な操作
└── System(SystemException)
    ├── CryptoError          # 暗号操作エラー
    ├── StorageError         # ストレージ操作エラー
    ├── SerializationError   # シリアライズ/デシリアライズエラー
    └── InternalError        # 内部エラー
```

**設計原則**:
- thiserror crateによる構造化エラー定義
- DomainErrorからの自動変換（From trait実装）
- エラーメッセージに機密情報を含めない

#### Acceptance Criteria

1. WHEN ビジネスロジック違反が発生する THEN system SHALL BusinessExceptionを返す
2. WHEN システム障害が発生する THEN system SHALL SystemExceptionを返す
3. WHEN DomainErrorが発生する THEN system SHALL 適切なServiceErrorに変換される（From trait）
4. WHEN is_recoverable()が呼ばれる THEN system SHALL 回復可能なエラーかどうかをboolで返す
5. WHEN エラーがDisplay traitで表示される THEN system SHALL 機密情報を含まないメッセージを返す
6. WHEN エラーがDebug traitで表示される THEN system SHALL デバッグ情報を返す

#### Test Coverage

| AC# | Test Function(s) | Purpose | Pattern |
|-----|------------------|---------|---------|
| 1 | `test_business_exception_validation_error` | ValidationError生成を確認 | Success |
| 1 | `test_business_exception_resource_not_found` | ResourceNotFound生成を確認 | Success |
| 1 | `test_business_exception_threshold_not_met` | ThresholdNotMet生成を確認 | Success |
| 2 | `test_system_exception_crypto_error` | CryptoError生成を確認 | Success |
| 2 | `test_system_exception_storage_error` | StorageError生成を確認 | Success |
| 3 | `test_from_domain_error_not_found` | DomainError::NotFound変換 | Success |
| 3 | `test_from_domain_error_storage` | DomainError::Storage変換 | Success |
| 4 | `test_is_recoverable_business` | BusinessExceptionはtrue | Query |
| 4 | `test_is_recoverable_system` | SystemExceptionはfalse | Query |
| 5 | `test_display_no_sensitive_data` | Display出力に機密情報なし | Security |
| 6 | `test_debug_output` | Debug出力を確認 | Success |

## Non-Functional Requirements

### Code Architecture and Modularity
- **Single Responsibility Principle**: 各CoreServiceは単一の責務（暗号/ストレージ）に特化
- **Modular Design**: CoreServiceはtrait + 実装として分離され、テスト時にMock差し替え可能
- **Dependency Management**: Repository Interfaceを通じてAdapter層に依存（DIP）
- **Clear Interfaces**: すべてのCoreServiceでResult<T, ServiceError>を返す一貫したインターフェース

### Performance（product.md準拠）
- **再暗号化レイテンシ**: k-of-n閾値再暗号化完了まで1秒未満
- **暗号学的正確性**: UmbralおよびShamir操作のすべてのテストベクターがパス
- **WASMバイナリサイズ**: 最適化のためAOネットワークデプロイ用に2MB未満を目標

### Security（product.md準拠）
- **IND-CPA準拠**: secp256k1、256-bit暗号学的強度
- **k-of-n閾値共謀耐性**: k-1では復元不可
- **Zeroize**: すべての秘密鍵構造体はZeroize + ZeroizeOnDropを実装
- **定数時間操作**: subtleクレートによるタイミング攻撃防止
- **エラーメッセージ**: 機密データをエラーメッセージに含めない

### Reliability
- **エラー階層**: BusinessException/SystemExceptionによる明確なエラー分類
- **閾値検証**: Shamir/Umbral操作で閾値パラメータを厳密に検証（k ≤ n、k ≥ 1）
- **Arweave不変性対応**: 更新操作は新規トランザクションとして実行

### Usability
- **一貫したAPI**: すべてのCoreServiceでResult<T, ServiceError>を返す
- **明確なエラーメッセージ**: 問題特定が容易な詳細エラー
- **ドキュメント**: 各サービスにrustdocコメントを提供

## Test Coverage Summary

### Overall Statistics

| Requirement | Total AC | Test Functions | Coverage |
|-------------|----------|----------------|----------|
| 1. CryptoService | 14 | 20 | 100% |
| 2. StorageService | 12 | 15 | 100% |
| 3. ServiceError | 6 | 11 | 100% |
| **Total** | **32** | **46** | **100%** |

### Implemented Test Functions

#### CryptoService (crypto.rs) - 20 tests

**Shamir Secret Sharing Tests:**
- `test_shamir_split_valid_params` - 有効なパラメータでシェア生成
- `test_shamir_reconstruct_with_threshold` - 閾値シェアで復元
- `test_shamir_reconstruct_with_more_than_threshold` - 閾値超シェアで復元
- `test_shamir_split_invalid_threshold_k_greater_than_n` - k > n検証
- `test_shamir_split_invalid_threshold_k_zero` - k = 0検証

**Umbral TPRE Tests:**
- `test_generate_keypair` - 鍵ペア生成
- `test_create_pre_capsule` - Capsule生成
- `test_generate_kfrags` - kFrags生成
- `test_generate_kfrags_threshold_validation` - kFrag閾値検証
- `test_verify_cfrag_valid` - 有効cFrag検証
- `test_verify_cfrag_invalid` - 無効cFrag検証
- `test_decrypt_original` - オリジナル復号
- `test_decrypt_reencrypted` - 再暗号化復号
- `test_decrypt_reencrypted_insufficient_cfrags` - cFrag不足エラー

**AES-GCM Tests:**
- `test_encrypt_share_aes_gcm` - AES-GCM暗号化
- `test_decrypt_share_aes_gcm` - AES-GCM復号
- `test_decrypt_share_invalid_ciphertext` - 不正暗号文エラー
- `test_decrypt_share_wrong_key` - 不正鍵エラー

**End-to-End Tests:**
- `test_full_encryption_flow` - 完全暗号化フロー
- `test_full_reencryption_flow` - 完全再暗号化フロー

#### StorageService (storage.rs) - 15 tests

**Store Operations Tests:**
- `test_store_capsule` - Capsule保存
- `test_store_encrypted_share` - 暗号化シェア保存
- `test_store_kfrag` - kFrag保存
- `test_store_cfrag` - cFrag保存

**Retrieve Operations Tests:**
- `test_retrieve_capsule_exists` - Capsule取得
- `test_retrieve_encrypted_shares_by_secret_id` - シェア取得
- `test_retrieve_kfrags_by_secret_id` - kFrag取得
- `test_retrieve_cfrags_by_secret_id` - cFrag取得

**Query Operations Tests:**
- `test_query_by_tags` - タグクエリ
- `test_query_by_tags_no_match` - 空結果
- `test_exists_true` - 存在確認true
- `test_exists_false` - 存在確認false

**Error Handling Tests:**
- `test_retrieve_capsule_not_found` - NotFoundエラー
- `test_retrieve_kfrags_empty` - 空結果
- `test_store_capsule_storage_error` - StorageError

#### ServiceError (error.rs) - 11 tests

**Business Exception Tests:**
- `test_business_exception_validation_error` - ValidationError
- `test_business_exception_resource_not_found` - ResourceNotFound
- `test_business_exception_threshold_not_met` - ThresholdNotMet

**System Exception Tests:**
- `test_system_exception_crypto_error` - CryptoError
- `test_system_exception_storage_error` - StorageError

**Conversion Tests:**
- `test_from_domain_error_not_found` - DomainError変換
- `test_from_domain_error_storage` - DomainError変換

**Utility Tests:**
- `test_is_recoverable_business` - 回復可能判定true
- `test_is_recoverable_system` - 回復可能判定false
- `test_display_no_sensitive_data` - Display安全性
- `test_debug_output` - Debug出力

### Test Patterns Used

1. **Success Pattern**: 正常系操作の検証（save, generate, encrypt等）
2. **Validation Pattern**: 入力検証エラーの確認（閾値不正、不正鍵等）
3. **Query Pattern**: データ取得操作の検証（find, retrieve, exists等）
4. **Security Pattern**: セキュリティ要件の検証（機密情報非露出等）
5. **Type Safety Pattern**: コンパイル時型検証（Zeroize実装、Send + Sync等）
6. **Crypto Pattern**: 暗号学的正確性の検証（往復テスト、閾値テスト等）

### Dev Dependencies

```toml
[dev-dependencies]
tokio = { version = "1", features = ["rt", "macros"] }
```

## Implementation Priority

1. **Phase 1 - Core Crypto**:
   - Shamir Secret Sharing（AC 1-3）
   - Umbral TPRE基本操作（AC 4-9）

2. **Phase 2 - Share Encryption**:
   - AES-GCM暗号化（AC 10-12）
   - Memory Safety検証（AC 13-14）

3. **Phase 3 - Storage**:
   - 基本ストレージ操作（AC 1-4）
   - 取得・クエリ操作（AC 5-10）
   - エラー処理（AC 11-12）

4. **Phase 4 - Error Handling**:
   - ServiceError階層実装（AC 1-6）
