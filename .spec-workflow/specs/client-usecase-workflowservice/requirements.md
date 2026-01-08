# Requirements Document: client-usecase-workflowservice

## Introduction

本機能は、D-TPRESクライアントライブラリのUseCase層におけるWorkflow Serviceを実装します。Workflow Serviceは、PHASE 1（秘密の分割と初期配布）とPHASE 3（秘密の復元）のビジネスロジックをオーケストレーションし、既存のCore Services（CryptoService、ArweaveStorageService）を組み合わせてエンドツーエンドのワークフローを実現します。

**対象コンポーネント:**
- `SecretSharingWorkflowService` - PHASE 1: O-Browser側の秘密分割・暗号化・kFrag生成・保存・送信
- `SecretRecoveryWorkflowService` - PHASE 3: R-Browser側のCapsule結合・復号・秘密復元

**SDKエンドポイントとの対応:**

| SDK Function | Workflow Service | 責任範囲 |
|--------------|------------------|----------|
| `generateKeyPair()` | **なし**（WorkflowService外） | PRE鍵ペア生成 |
| `dtpres.share()` | `SecretSharingWorkflowService` | PHASE 1 (1-2 〜 1-9) |
| `dtpres.recover()` | `SecretRecoveryWorkflowService` | PHASE 3 (3-4 〜 3-8) |

**PRDベースの処理フロー:**

```
PHASE 1 (O-Browser - SecretSharingWorkflowService):
※ 1-1は generateKeyPair() で処理済み、skₒ, pkₒ, pkᴬ は入力として受け取る

1-2. Shamir分割: f(0) → f(1)...f(n) (k-of-n閾値)
1-3. AES-GCM暗号化: Cᵢ = AES_GCM(kₒ, f(i)) (i=1...n)
1-4. Capsule生成: Capsuleₒ = PRE_Enc(pkₒ, kₒ)
1-5. Requester公開鍵 pkᴬ を受け取り (入力パラメータ)
1-6. 再暗号化キー生成: rekey = PRE_ReKey(skₒ → pkᴬ)
1-7. kFrag分割: kFragⱼ = Shamir_Split(rekey, k, n)
1-8. kFragをOwner-Processに送信 (StorageService経由、AO Network)
1-9. Capsuleₒ, Cᵢ をArweaveに保存

PHASE 3 (R-Browser - SecretRecoveryWorkflowService):
※ skᴬ は generateKeyPair() で生成済み、入力として受け取る
※ cFrags, Capsuleₒ は WorkflowService内でAO/Arweaveから取得

3-4a. Requester-ProcessからcFragを取得 (StorageService経由、AO Network)
3-4b. ArweaveからCapsuleₒを取得 (StorageService経由)
3-4c. Capsule結合: Capsule′ = PRE_Combine(Capsuleₒ, cFrag₁…k)
3-5. 共通鍵復号: kₒ = PRE_Dec(skᴬ, Capsule′)
3-6. Arweaveから Cᵢ を取得
3-7. シェア復号: f(i) = AES_DEC(kₒ, Cᵢ) (k個)
3-8. Shamir補間: f(0) = secret を復元
```

**依存関係（Core Services経由）:**
```
WorkflowService
    ├── CryptoService (暗号操作)
    │   - split_secret_shamir()
    │   - reconstruct_secret_shamir()
    │   - create_pre_capsule() (共通鍵のPRE暗号化)
    │   - generate_reencryption_key()
    │   - create_kfrags()
    │   - combine_and_decrypt() (Capsule結合・復号)
    │   - [NEW] aes_gcm_encrypt() / aes_gcm_decrypt()
    │   - [NEW] generate_symmetric_key()
    │
    └── ArweaveStorageService (ストレージ操作)
        - store_data()         → Arweave保存
        - retrieve_data()      → Arweave取得
        - query_by_tags()      → Arweaveクエリ
        - batch_store()        → 一括保存
        - [NEW] send_kfrag_to_owner_process() → AO通信（kFrag送信）
        - [NEW] retrieve_cfrags_from_requester_process() → AO通信（cFrag取得）
        - [NEW] retrieve_capsule() → Arweave取得（Capsule）
```

**既存実装との関係:**
- Core Services（`usecase/core/crypto.rs`, `usecase/core/storage.rs`）は実装済み
- WorkflowServiceはCore Services経由でのみ操作を行い、Repositoryを直接使用しない
- CryptoServiceにAES-GCM暗号化/復号化メソッド、対称鍵生成メソッドの追加が必要
- StorageServiceにAO通信メソッドの追加が必要（Issue [#47](https://github.com/shodaimomiyama/D-TPRES/issues/47)）
  - `send_kfrag_to_owner_process()` - kFrag送信
  - `retrieve_cfrags_from_requester_process()` - cFrag取得
  - `retrieve_capsule()` - Capsule取得

## Alignment with Product Vision

本機能は以下のプロダクトビジョン要素を実現します：

1. **k-of-n閾値プロキシ再暗号化**: Shamir秘密分散で秘密をn個のシェアに分割し、共通鍵をPREで暗号化してCapsule生成
2. **Arweave永続ストレージ**: ArweaveStorageService経由で暗号化シェア(Cᵢ)とCapsuleₒを不変保存
3. **AO Network連携**: StorageService経由でkFragをOwner-Processに送信
4. **暗号学的な漏洩耐性**: 共通鍵kₒはPREで保護され、シェアはAES-GCMで暗号化、skₒを露出せずに委譲可能
5. **分散性優先**: ローカル(O-Browser/R-Browser)とオンチェーン(AO/Arweave)のみで動作

## Requirements

### Requirement 1: SecretSharingWorkflowService - PHASE 1 秘密分割ワークフロー

**User Story:** As a データ所有者(O-Browser), I want 秘密データをk-of-n閾値で安全に分割・暗号化し、kFragを生成してOwner-Processに送信したい, so that PHASE 2でHolder-Processに分散委譲できる

#### Design Note

SecretSharingWorkflowServiceは、PRD PHASE 1のステップ1-2〜1-9をオーケストレーションします。

**スコープ外（入力として受け取る）:**
- ステップ1-1: 鍵生成（`generateKeyPair()`で処理済み）
- skₒ（Owner秘密鍵）、pkₒ（Owner公開鍵）、pkᴬ（Requester公開鍵）は引数として提供

**暗号化の2段階構造:**
- **AES-GCM**: 各シェアf(i)を共通鍵kₒで暗号化 → Cᵢ
- **PRE**: 共通鍵kₒを所有者公開鍵pkₒで暗号化 → Capsuleₒ

この構造により、PHASE 3でRequesterはPREでkₒを復元し、kₒでシェアを復号できます。

#### Acceptance Criteria

**入力パラメータ:**
- `secret`: 分割する秘密データ f(0)
- `owner_secret_key`: Owner秘密鍵 skₒ（generateKeyPairで生成済み）
- `owner_public_key`: Owner公開鍵 pkₒ（generateKeyPairで生成済み）
- `requester_public_key`: Requester公開鍵 pkᴬ（generateKeyPairで生成済み）
- `threshold`: 閾値 k
- `total_shares`: 総シェア数 n

**共通鍵生成と秘密分割 (1-2):**
1. WHEN `execute_secret_sharing`が呼び出された THEN system SHALL `CryptoService.generate_symmetric_key()`で共通鍵kₒ(AES-256用)を生成する
2. WHEN 共通鍵生成が成功した THEN system SHALL `CryptoService.split_secret_shamir()`で秘密f(0)をn個のシェアf(1)...f(n)に分割する

**シェア暗号化とCapsule生成 (1-3, 1-4):**
3. WHEN シェア分割が成功した THEN system SHALL `CryptoService.aes_gcm_encrypt(kₒ, f(i))`で各シェアを暗号化しCᵢを生成する
4. WHEN シェア暗号化が成功した THEN system SHALL `CryptoService.create_pre_capsule(pkₒ, kₒ)`で共通鍵を暗号化しCapsuleₒを生成する

**再暗号化キーとkFrag生成 (1-5, 1-6, 1-7):**
5. WHEN Capsule生成が成功した THEN system SHALL `CryptoService.generate_reencryption_key(skₒ, pkᴬ)`で再暗号化キーrekeyを生成する
6. WHEN rekey生成が成功した THEN system SHALL `CryptoService.create_kfrags(rekey, k, n)`でn個のkFragⱼを生成する

**kFrag送信 (1-8):**
7. WHEN kFrag生成が成功した THEN system SHALL `ArweaveStorageService.send_kfrag_to_owner_process()`で各kFragをOwner-Processに送信する

**Arweave保存 (1-9):**
8. WHEN kFrag送信が成功した THEN system SHALL `ArweaveStorageService.batch_store()`でCapsuleₒとn個のCᵢを保存する
9. WHEN 保存が成功した THEN system SHALL SecretSharingResultを返却する（secret_id, capsule_tx_id, share_tx_ids含む）

**バリデーション:**
10. IF threshold > total_shares THEN system SHALL ValidationErrorを返却する
11. IF threshold < MIN_THRESHOLD (2) THEN system SHALL ValidationErrorを返却する
12. IF total_shares > MAX_SHARES THEN system SHALL ValidationErrorを返却する
13. IF secret_data.is_empty() THEN system SHALL ValidationErrorを返却する
14. IF owner_secret_key/owner_public_key/requester_public_keyが無効 THEN system SHALL ValidationErrorを返却する

**エラーハンドリング:**
15. WHEN CryptoServiceからエラーが発生した THEN system SHALL WorkflowError::CryptoErrorを返却する
16. WHEN ArweaveStorageServiceからエラーが発生した THEN system SHALL WorkflowError::StorageErrorを返却する
17. WHEN kFrag送信が失敗した THEN system SHALL WorkflowError::AOCommunicationErrorを返却する

#### Test Coverage

| AC# | Test Function(s) | Purpose | Pattern |
|-----|------------------|---------|---------|
| 1 | `test_phase1_generates_symmetric_key` | 共通鍵生成を検証 | Success |
| 2 | `test_phase1_splits_secret_shamir` | Shamir分割を検証 | Success |
| 3 | `test_phase1_encrypts_shares_aes_gcm` | AES-GCMシェア暗号化を検証 | Success |
| 4 | `test_phase1_creates_capsule_for_symmetric_key` | 共通鍵のPRE暗号化を検証 | Success |
| 5 | `test_phase1_generates_reencryption_key` | rekey生成を検証 | Success |
| 6 | `test_phase1_creates_kfrags` | kFrag分割を検証 | Success |
| 7 | `test_phase1_sends_kfrags_to_owner_process` | kFrag送信を検証 | Success |
| 8 | `test_phase1_stores_capsule_and_shares` | Arweave保存を検証 | Success |
| 9 | `test_phase1_returns_complete_result` | 結果の完全性を検証 | Success |
| 10 | `test_phase1_invalid_threshold_exceeds` | threshold > total_sharesを検証 | Validation |
| 11 | `test_phase1_invalid_threshold_below_min` | threshold < 2を検証 | Validation |
| 12 | `test_phase1_invalid_total_shares_exceeds_max` | total_shares > MAXを検証 | Validation |
| 13 | `test_phase1_empty_secret` | 空データエラーを検証 | Validation |
| 14 | `test_phase1_invalid_keys` | 無効な鍵エラーを検証 | Validation |
| 15 | `test_phase1_crypto_service_error` | CryptoServiceエラー伝播を検証 | Error |
| 16 | `test_phase1_storage_service_error` | StorageServiceエラー伝播を検証 | Error |
| 17 | `test_phase1_ao_communication_error` | AO通信エラー伝播を検証 | Error |

**Additional Tests (Beyond AC):**
- `test_phase1_complete_flow` - エンドツーエンドのフロー検証
- `test_phase1_kfrags_count_matches_n` - 生成されたkFrag数がnと一致することを検証

### Requirement 2: SecretSharingWorkflowService - 秘密ステータス管理

**User Story:** As a データ所有者, I want 保存した秘密のステータスを確認したい, so that ライフサイクルを把握できる

#### Design Note

ArweaveStorageService.query_by_tags()を使用してArweave上の保存データを検索します。

#### Acceptance Criteria

1. WHEN `get_secret_status`がsecret_idで呼び出された THEN system SHALL `ArweaveStorageService.query_by_tags()`でステータスを取得する
2. IF secret_idに対応するデータが存在しない THEN system SHALL ResourceNotFoundエラーを返却する
3. WHEN ArweaveStorageServiceからエラーが発生した THEN system SHALL WorkflowError::StorageErrorを返却する

#### Test Coverage

| AC# | Test Function(s) | Purpose | Pattern |
|-----|------------------|---------|---------|
| 1 | `test_get_secret_status_valid` | ステータス取得を検証 | Query |
| 2 | `test_get_secret_status_not_found` | 存在しないsecret_idエラーを検証 | Validation |
| 3 | `test_get_secret_status_storage_error` | StorageServiceエラー伝播を検証 | Error |

### Requirement 3: SecretRecoveryWorkflowService - PHASE 3 秘密復元ワークフロー

**User Story:** As a データアクセス者(R-Browser), I want 収集したcFragとCapsuleから共通鍵を復号し、秘密を復元したい, so that アクセス許可された秘密データを取得できる

#### Design Note

SecretRecoveryWorkflowServiceは、PRD PHASE 3のステップ3-4〜3-8をオーケストレーションします。

**SDK呼び出し:**
```javascript
// === Phase 3: 秘密を復元（Requester側）===
const { secret } = await dtpres.recover(secretId, {
  requesterSecretKey: requesterKeyPair.secretKey  // Requester の PRE 秘密鍵 skᴬ（必須）
});
```

**スコープ外（入力として受け取る）:**
- skᴬ（Requester秘密鍵）は`generateKeyPair()`で生成済み、引数として提供

**WorkflowService内で取得:**
- cFragsは`StorageService.retrieve_cfrags_from_requester_process()`でRequester-ProcessからAO通信で取得
- CapsuleₒはArweaveから`StorageService.retrieve_capsule()`で取得

**復元の2段階構造:**
1. **PRE復号**: cFragsとCapsuleₒを結合してCapsule′を作成し、skᴬで共通鍵kₒを復号
2. **AES-GCM復号**: kₒでk個のCᵢを復号してシェアf(i)を取得し、Shamir補間で秘密f(0)を復元

#### Acceptance Criteria

**入力パラメータ:**
- `secret_id`: 復元対象の秘密ID
- `requester_secret_key`: Requester秘密鍵 skᴬ（generateKeyPairで生成済み）
- `requester_process_id`: Requester-ProcessのID（cFrag取得先）

**cFrag/Capsule取得 (3-4a, 3-4b):**
1. WHEN `recover_secret`がSecretRecoveryRequestで呼び出された THEN system SHALL `StorageService.retrieve_cfrags_from_requester_process()`でcFragsを取得する
2. WHEN cFrag取得が成功した THEN system SHALL `StorageService.retrieve_capsule()`でCapsuleₒを取得する
3. WHEN Capsule取得が成功した THEN system SHALL cFrags数が閾値k以上であることを検証する

**Capsule結合と共通鍵復号 (3-4c, 3-5):**
4. WHEN 検証が成功した THEN system SHALL `CryptoService.combine_and_decrypt(cfrags, skᴬ, Capsuleₒ)`でCapsule′を結合し共通鍵kₒを復号する

**暗号化シェア取得と復号 (3-6, 3-7):**
5. WHEN 共通鍵復号が成功した THEN system SHALL `ArweaveStorageService.query_by_tags()`でk個のCᵢを取得する
6. WHEN Cᵢ取得が成功した THEN system SHALL `CryptoService.aes_gcm_decrypt(kₒ, Cᵢ)`で各シェアf(i)を復号する

**Shamir補間と秘密復元 (3-8):**
7. IF 復号されたシェア数 >= threshold THEN system SHALL `CryptoService.reconstruct_secret_shamir()`でf(0)を復元する
8. WHEN 復元が成功した THEN system SHALL SecretRecoveryResultを返却する（recovered_secret含む）

**監査証跡:**
9. WHEN 復元が成功した THEN system SHALL `ArweaveStorageService.store_data()`で監査証跡を記録する

**バリデーション:**
10. IF cFrags数 < threshold THEN system SHALL InsufficientCFragsErrorを返却する
11. IF Capsuleₒが無効 THEN system SHALL ValidationErrorを返却する
12. IF Cᵢが見つからない THEN system SHALL ResourceNotFoundエラーを返却する
13. IF requester_secret_key(skᴬ)が無効 THEN system SHALL ValidationErrorを返却する

**エラーハンドリング:**
14. WHEN CryptoServiceから復号エラーが発生した THEN system SHALL WorkflowError::DecryptionErrorを返却し、監査ログに失敗を記録する
15. WHEN ArweaveStorageServiceからエラーが発生した THEN system SHALL WorkflowError::StorageErrorを返却する
16. WHEN AO通信エラー（cFrag取得失敗）が発生した THEN system SHALL WorkflowError::AOCommunicationErrorを返却する

#### Test Coverage

| AC# | Test Function(s) | Purpose | Pattern |
|-----|------------------|---------|---------|
| 1 | `test_phase3_retrieves_cfrags_from_requester_process` | Requester-ProcessからcFrag取得を検証 | Query |
| 2 | `test_phase3_retrieves_capsule_from_arweave` | ArweaveからCapsule取得を検証 | Query |
| 3 | `test_phase3_validates_cfrag_count` | cFrag数検証を確認 | Validation |
| 4 | `test_phase3_combines_and_decrypts_symmetric_key` | Capsule結合と共通鍵復号を検証 | Crypto |
| 5 | `test_phase3_fetches_encrypted_shares` | Arweaveから暗号化シェア取得を検証 | Query |
| 6 | `test_phase3_decrypts_shares_aes` | AESでシェア復号を検証 | Crypto |
| 7 | `test_phase3_reconstructs_secret_shamir` | Shamir補間を検証 | Crypto |
| 8 | `test_phase3_returns_complete_result` | 結果の完全性を検証 | Success |
| 9 | `test_phase3_records_audit_trail` | 監査証跡記録を検証 | Mutation |
| 10 | `test_phase3_insufficient_cfrags` | cFrag不足エラーを検証 | Validation |
| 11 | `test_phase3_invalid_capsule` | 無効なCapsuleエラーを検証 | Validation |
| 12 | `test_phase3_encrypted_share_not_found` | Cᵢ未発見エラーを検証 | Validation |
| 13 | `test_phase3_invalid_requester_key` | 無効なskᴬエラーを検証 | Validation |
| 14 | `test_phase3_decryption_error_with_audit` | 復号エラーと監査記録を検証 | Error |
| 15 | `test_phase3_storage_service_error` | StorageServiceエラー伝播を検証 | Error |
| 16 | `test_phase3_ao_retrieval_error` | AO通信エラー伝播を検証 | Error |

**Additional Tests (Beyond AC):**
- `test_phase3_complete_flow` - エンドツーエンドのフロー検証
- `test_phase3_exactly_k_cfrags` - ちょうどk個のcFragでの復元
- `test_phase3_more_than_k_cfrags` - k個以上のcFragでの復元

### Requirement 4: SecretRecoveryWorkflowService - 復元データ検証

**User Story:** As a データアクセス者, I want 復元されたデータの整合性を検証したい, so that 正しいデータを取得したことを確認できる

#### Acceptance Criteria

1. WHEN `verify_recovered_data`が呼び出された THEN system SHALL データが空でないことを検証する
2. WHEN `verify_recovered_data`が呼び出された THEN system SHALL データサイズが妥当であることを検証する
3. IF 検証が成功した THEN system SHALL trueを返却する
4. IF 検証が失敗した THEN system SHALL falseを返却する

#### Test Coverage

| AC# | Test Function(s) | Purpose | Pattern |
|-----|------------------|---------|---------|
| 1 | `test_verify_recovered_data_empty` | 空データの検証失敗を確認 | Validation |
| 2 | `test_verify_recovered_data_size` | サイズ検証を確認 | Validation |
| 3 | `test_verify_recovered_data_valid` | 有効データの検証成功を確認 | Success |
| 4 | `test_verify_recovered_data_returns_false` | 検証失敗時のfalse返却を確認 | Validation |

### Requirement 5: WorkflowError - エラー階層定義

**User Story:** As a 開発者, I want ワークフロー層の明確なエラー階層を使用したい, so that 適切なエラーハンドリングができる

#### Design Note

WorkflowErrorは、既存のServiceError設計と整合性を保ちつつ、PHASE 1/PHASE 3固有のエラーバリアントを提供します。

#### Acceptance Criteria

1. WHEN ValidationErrorが発生した THEN system SHALL ユーザーフレンドリーなメッセージを含むValidationErrorを返却する
2. WHEN CryptoServiceからエラーが伝播した THEN system SHALL WorkflowError::CryptoErrorに変換する
3. WHEN ArweaveStorageServiceからエラーが伝播した THEN system SHALL WorkflowError::StorageErrorに変換する
4. WHEN AO通信エラーが発生した THEN system SHALL WorkflowError::AOCommunicationErrorを返却する
5. WHEN cFrag不足が発生した THEN system SHALL InsufficientCFragsErrorを返却する（必要数と現在数を含む）
6. WHEN 復号失敗が発生した THEN system SHALL DecryptionErrorを返却する（失敗フェーズを含む）

#### Test Coverage

| AC# | Test Function(s) | Purpose | Pattern |
|-----|------------------|---------|---------|
| 1 | `test_workflow_error_validation` | ValidationErrorの生成を検証 | Success |
| 2 | `test_workflow_error_from_crypto_service` | CryptoServiceエラー変換を検証 | Success |
| 3 | `test_workflow_error_from_storage_service` | StorageServiceエラー変換を検証 | Success |
| 4 | `test_workflow_error_ao_communication` | AOCommunicationErrorの生成を検証 | Success |
| 5 | `test_workflow_error_insufficient_cfrags` | InsufficientCFragsの詳細を検証 | Success |
| 6 | `test_workflow_error_decryption` | DecryptionErrorの詳細を検証 | Success |

## Non-Functional Requirements

### Code Architecture and Modularity

- **Single Responsibility Principle**: 各Workflow ServiceはPHASE 1またはPHASE 3の処理のみを担当
- **Core Service依存**: WorkflowServiceはCryptoServiceとArweaveStorageService経由でのみ操作を行い、Repositoryを直接使用しない
- **Dependency Injection**: Core Servicesはtraitを通じて注入され、テスト容易性を確保
- **PRDとの整合性**: 処理フローはPRDの各ステップ番号と対応
- **SDKエンドポイント対応**: `dtpres.share()` → SecretSharingService、`dtpres.recover()` → SecretRecoveryService

### Performance

- **処理時間**: 1MBの秘密データに対するPHASE 1完了が5秒以内
- **メモリ使用**: 秘密データの3倍以下のメモリ使用量
- **WASMサイズ影響**: 追加コードによるバイナリサイズ増加が100KB以下

### Security

- **秘密データのZeroize**: skₒ, kₒ, f(i)などの秘密データは使用後に確実にクリア
- **エラー情報の制限**: エラーメッセージに秘密データや鍵情報を含めない
- **定数時間操作**: 暗号関連の比較はCryptoServiceに委譲（subtle crate使用）
- **AES-GCM認証**: シェア暗号化は認証付き暗号化を使用し、改ざん検出可能
- **入力鍵の検証**: 提供されたskₒ, pkₒ, pkᴬ, skᴬの妥当性を検証

### Reliability

- **アトミック操作**: ArweaveStorageService.batch_store()でCapsuleとCᵢを一括保存
- **エラー回復**: 回復可能なエラーには適切なリトライ情報を提供
- **監査証跡**: PHASE 3の復元操作の成功・失敗をArweaveに記録
- **AO通信信頼性**: kFrag送信失敗時の適切なエラーハンドリング

### Usability

- **エラーメッセージ**: 開発者が問題を特定できる明確なメッセージ
- **型安全性**: Rust型システムを活用した誤用防止
- **ドキュメント**: 各public関数にdoc comment

## Test Coverage Summary

### Overall Statistics

| Requirement | Total AC | Test Functions | Coverage |
|-------------|----------|----------------|----------|
| 1. SecretSharingWorkflowService - PHASE 1 | 17 | 19 | 100% |
| 2. SecretSharingWorkflowService - 管理 | 3 | 3 | 100% |
| 3. SecretRecoveryWorkflowService - PHASE 3 | 16 | 19 | 100% |
| 4. SecretRecoveryWorkflowService - 検証 | 4 | 4 | 100% |
| 5. WorkflowError - エラー階層 | 6 | 6 | 100% |
| **Total** | **46** | **51** | **100%** |

### Test Patterns Used

1. **Success Pattern**: 正常系のCore Service呼び出し検証
2. **Validation Pattern**: 入力検証エラー確認
3. **Query Pattern**: ArweaveStorageService経由のデータ取得
4. **Mutation Pattern**: ArweaveStorageService経由の保存・送信
5. **Crypto Pattern**: CryptoService経由の暗号操作（AES-GCM, PRE, Shamir）
6. **Error Pattern**: Core Serviceエラーの伝播と変換

## Appendix: PRD Phase Reference

### PHASE 1 秘密の分割と初期配布

| # | アクター | ステップ | Workflow Service対応 |
|---|---------|---------|---------------------|
| 1-1 | O-Browser | skₒ, pkₒ, kₒ, f(0)=secret生成 | **スコープ外**: generateKeyPair()で処理 |
| 1-2 | O-Browser | Shamir分割: f(0) → f(1)...f(n) | AC 2 |
| 1-3 | O-Browser | Cᵢ = AES_GCM(kₒ, f(i)) | AC 3 |
| 1-4 | O-Browser | Capsuleₒ = PRE_Enc(pkₒ, kₒ) | AC 4 |
| 1-5 | O-Browser | pkᴬ受け取り | 入力パラメータ |
| 1-6 | O-Browser | rekey = PRE_ReKey(skₒ → pkᴬ) | AC 5 |
| 1-7 | O-Browser | kFragⱼ = Shamir_Split(rekey, k, n) | AC 6 |
| 1-8 | O-Browser | kFragをOwner-Processに送信 | AC 7 |
| 1-9 | O-Browser | Capsuleₒ, Cᵢ をArweave保存 | AC 8 |

### PHASE 3 秘密の復元

| # | アクター | ステップ | Workflow Service対応 |
|---|---------|---------|---------------------|
| 3-4a | R-Browser | Requester-ProcessからcFrag取得 | AC 1 |
| 3-4b | R-Browser | ArweaveからCapsuleₒ取得 | AC 2 |
| 3-4c | R-Browser | Capsule′ = PRE_Combine(Capsuleₒ, cFrag₁…k) | AC 4 |
| 3-5 | R-Browser | kₒ = PRE_Dec(skᴬ, Capsule′) | AC 4 |
| 3-6 | R-Browser | ArweaveからCᵢ取得 | AC 5 |
| 3-7 | R-Browser | f(i) = AES_DEC(kₒ, Cᵢ) | AC 6 |
| 3-8 | R-Browser | Shamir補間でf(0)復元 | AC 7 |
