# Requirements Document: Client Domain Entities

## Introduction

FORMIXクライアントライブラリのDomain層におけるエンティティ群とValue Objectsを実装する。これらは、閾値プロキシ再暗号化（Umbral）とシャミア秘密分散を組み合わせた分散型鍵管理システムのコアドメインモデルを形成する。

DDDの原則に従い：
- **Entity**: 一意のIDを持ち、ライフサイクルを通じて同一性を維持。永続化対象。
- **Value Object**: IDを持たず、値で等価性を判断。不変。

**重要な設計方針：**
- Domain Entityは「永続化されるビジネス概念」のみを表現
- 処理の中間状態（平文シェア `f(i)` 等）はService層で扱い、Domain層には含めない
- 暗号化されArweaveに保存されるデータのみがEntityとして定義される
- 一時的な値（鍵、秘密データ）はValue Objectとして定義し、使用後Zeroize

## Alignment with Product Vision

このfeatureは以下のプロダクト目標をサポートする：

1. **分散性優先**: Secret、ShareCollection、KFrag、CFragエンティティがk-of-n閾値暗号の基盤を提供
2. **設計によるセキュリティ**: Zeroizeトレイトによるメモリ安全性を保証
3. **検証可能性**: Capsuleエンティティが暗号カプセルのArweave永続化を抽象化
4. **開発者フレンドリー**: 明確なドメインモデルがSDK統合の基盤を提供

## Requirements: Entities

### Requirement 1: Secret Entity（集約ルート）

**User Story:** As a 開発者, I want Secretエンティティで秘密のメタデータを管理したい, so that 秘密の分割・復元状態を追跡できる

#### Design Note

Secret Entityはメタデータのみを保持。秘密の実データ `f(0)=secret` はValue Object `SecretData` として別途定義され、Service層で一時的に処理される。

#### Acceptance Criteria

1. WHEN Secret::new()が呼ばれた THEN システム SHALL SecretIdを生成し、InitializedステートのSecretを返す
2. WHEN 無効なthreshold（k > n または k = 0）が指定された THEN システム SHALL DomainErrorを返す
3. WHEN Secret.split()が呼ばれた THEN システム SHALL ステートをSplitに遷移し、関連するShareCollectionIdを記録する
4. WHEN Secret.capsule_id()が呼ばれた THEN システム SHALL 関連するCapsuleのID（Option）を返す

#### Test Coverage

| AC# | Test Function(s) | Status |
|-----|------------------|--------|
| 1 | `test_secret_new_valid` | ✅ |
| 2 | `test_secret_new_invalid_threshold_zero`, `test_secret_new_invalid_threshold_k_greater_than_n` | ✅ |
| 3 | `test_secret_split_transition`, `test_secret_invalid_split_from_wrong_state` | ✅ |
| 4 | `test_secret_new_valid`, `test_secret_split_transition` | ✅ |

**追加テスト（要件外）:** `test_secret_new_empty_public_key`, `test_secret_distribute_transition`, `test_secret_distribute_wrong_kfrag_count`

### Requirement 2: ShareCollection Entity（暗号化シェアコレクション）

**User Story:** As a 開発者, I want ShareCollectionエンティティでn個の暗号化シャミアシェアを一括管理したい, so that Arweaveに効率的に永続化し、アトミックな整合性を保証できる

#### Design Note

PRDのライフサイクルに基づく責務分離：
- **ShareCollection Entity (Domain層)**: n個の暗号化シェア `C₁...Cₙ = AES_GCM(kₒ, f(1)...f(n))` を1つのEntityとして管理。1つのArweave txで永続化。
- **PlaintextShare (Service層)**: 平文シェア `f(i)` を一時的に処理。Zeroize必須、永続化しない。

```
Phase 1: f(1)...f(n) → [AES暗号化] → C₁...Cₙ → [Arweave保存: 1tx] → ShareCollection Entity
Phase 3: ShareCollection Entity → [Arweave取得: 1tx] → C₁...Cₙ → [AES復号] → f(i) → [シャミア補間]
```

**設計選択（Option B）の理由:**
- **アトミック性**: n個のシェアは論理的に不可分、部分的な保存失敗のリスク排除
- **効率性**: Arweave txコスト削減、フェッチ回数削減（n回→1回）
- **整合性**: 1 tx内でもArweaveノードに分散保存される

#### Acceptance Criteria

1. WHEN ShareCollection::new()が呼ばれた THEN システム SHALL ShareCollectionId、SecretId参照、閾値パラメータ、n個のEncryptedShareDataを持つShareCollectionを返す
2. WHEN シェア数がn（threshold_n）と一致しない THEN システム SHALL DomainErrorを返す
3. WHEN いずれかのシェアインデックスが範囲外（0またはn超過）THEN システム SHALL DomainErrorを返す
4. WHEN collection.get_share(index)が呼ばれた THEN システム SHALL 指定インデックスの暗号化シェアデータへの参照（Option）を返す
5. WHEN collection.get_shares_by_indices(indices)が呼ばれた THEN システム SHALL 指定された複数インデックスの暗号化シェアデータを返す
6. WHEN collection.set_arweave_tx_id()が呼ばれた THEN システム SHALL ArweaveトランザクションIDを記録する

#### Test Coverage

| AC# | Test Function(s) | Status |
|-----|------------------|--------|
| 1 | `test_share_collection_new_valid` | ✅ |
| 2 | `test_share_collection_wrong_share_count` | ✅ |
| 3 | `test_share_collection_invalid_index_zero`, `test_share_collection_invalid_index_too_high` | ✅ |
| 4 | `test_share_collection_get_share` | ✅ |
| 5 | `test_share_collection_get_shares_by_indices` | ✅ |
| 6 | `test_share_collection_set_arweave_tx_id` | ✅ |

**追加テスト（要件外）:** `test_share_collection_duplicate_index`, `test_share_collection_empty_data`

### Requirement 3: Capsule Entity（PREカプセル）

**User Story:** As a 開発者, I want CapsuleエンティティでUmbralカプセルを表現したい, so that プロキシ再暗号化に必要なカプセルを管理できる

#### Design Note

PRDにおけるCapsule:
- `Capsuleₒ = PRE_Enc(pkₒ, kₒ)`: 共通鍵kₒをOwner公開鍵で暗号化したカプセル
- Phase 1-9でArweaveに保存され、Phase 2-4でHolder-Processが取得・再暗号化に使用

#### Acceptance Criteria

1. WHEN Capsule::new()が呼ばれた THEN システム SHALL CapsuleId、SecretId参照、シリアライズされたカプセルデータを持つCapsuleを返す
2. WHEN 無効なカプセルデータ（空または不正フォーマット）が渡された THEN システム SHALL DomainErrorを返す
3. WHEN Capsule.arweave_tx_id()が呼ばれた THEN システム SHALL Arweaveトランザクションへの参照（Option）を返す
4. WHEN Capsule.set_arweave_tx_id()が呼ばれた THEN システム SHALL ArweaveトランザクションIDを記録する

#### Test Coverage

| AC# | Test Function(s) | Status |
|-----|------------------|--------|
| 1 | `test_capsule_new_valid` | ✅ |
| 2 | `test_capsule_empty_data_error` | ✅ |
| 3 | `test_capsule_set_arweave_tx_id` | ✅ |
| 4 | `test_capsule_set_arweave_tx_id` | ✅ |

**追加テスト（要件外）:** `test_capsule_empty_public_key_error`

### Requirement 4: KFrag Entity（鍵フラグメント）

**User Story:** As a 開発者, I want KFragエンティティで再暗号化鍵フラグメントを表現したい, so that Owner-ProcessからHolder-Processへの鍵配布を管理できる

#### Design Note

PRDにおけるKFrag:
- Phase 1-6: `rekey = PRE_ReKey(skₒ → pkᴬ)` で再暗号化鍵を生成
- Phase 1-7: `kFragⱼ = Shamir_Split(rekey, k, n)` でkFragに分割
- Phase 2-2: 各Holder-Processに配布

#### Acceptance Criteria

1. WHEN KFrag::new()が呼ばれた THEN システム SHALL KFragId、SecretId参照、Holderインデックス、シリアライズされたkFragデータを持つKFragを返す
2. WHEN Holderインデックスが総Holder数を超えた THEN システム SHALL DomainErrorを返す
3. IF KFragがDropされた THEN システム SHALL ZeroizeトレイトによりkFragデータをメモリからクリアする
4. WHEN KFrag.holder_process_id()が呼ばれた THEN システム SHALL 割り当てられたHolder-ProcessのID（Option）を返す

#### Test Coverage

| AC# | Test Function(s) | Status |
|-----|------------------|--------|
| 1 | `test_kfrag_new_valid` | ✅ |
| 2 | `test_kfrag_invalid_index_zero`, `test_kfrag_invalid_index_too_high` | ✅ |
| 3 | `test_kfrag_debug_redacted` | ⚠️ 間接的 |
| 4 | `test_kfrag_set_holder_process_id` | ✅ |

**Note:** AC3のZeroizeテストはDebug出力のredactionで間接的に確認。直接的なメモリクリアテストは実装困難。

**追加テスト（要件外）:** `test_kfrag_empty_data_error`

### Requirement 5: CFrag Entity（再暗号化フラグメント）

**User Story:** As a 開発者, I want CFragエンティティで再暗号化されたフラグメントを表現したい, so that Holder-ProcessからRequesterへの再暗号化結果を管理できる

#### Design Note

PRDにおけるCFrag:
- Phase 2-5: `cFragⱼ = PRE_ReEnc(kFragⱼ, Capsuleₒ)` でHolder-Processが生成
- Phase 2-6: ArweaveまたはAOに保存
- Phase 3-2: Requester-Processがk個以上収集

#### Acceptance Criteria

1. WHEN CFrag::new()が呼ばれた THEN システム SHALL CFragId、SecretId参照、Holderインデックス、シリアライズされたcFragデータを持つCFragを返す
2. WHEN 対応するKFragIdが存在しない THEN システム SHALL DomainErrorを返す
3. IF CFragがDropされた THEN システム SHALL ZeroizeトレイトによりcFragデータをメモリからクリアする
4. WHEN CFrag.verify()が呼ばれた THEN システム SHALL カプセルとの整合性を検証しResult<bool>を返す

#### Test Coverage

| AC# | Test Function(s) | Status |
|-----|------------------|--------|
| 1 | `test_cfrag_new_valid` | ✅ |
| 2 | (なし - 設計上、呼び出し側で検証) | ⚠️ N/A |
| 3 | `test_cfrag_debug_redacted` | ⚠️ 間接的 |
| 4 | `test_cfrag_verify_valid`, `test_cfrag_verify_empty_capsule_error` | ✅ |

**Note:** AC2はCFrag::newがKFragIdを受け取る設計のため、存在確認は呼び出し側（Service層）で行う。AC3はDebug出力のredactionで間接的に確認。

**追加テスト（要件外）:** `test_cfrag_empty_data_error`

## Requirements: Value Objects

### Requirement 6: ID Value Objects

**User Story:** As a 開発者, I want 型安全なID Value Objectsを使いたい, so that 異なるエンティティのIDを混同しない

#### Design Note

各EntityのIDはnewtypeパターンでラップし、型安全性を確保する。

#### Acceptance Criteria

1. WHEN SecretId::new()が呼ばれた THEN システム SHALL 一意のIDを生成して返す
2. WHEN 同じ文字列値を持つSecretIdを比較した THEN システム SHALL 等価と判断する
3. WHEN SecretIdをShareCollectionIdが期待される場所で使用した THEN システム SHALL コンパイルエラーを発生させる
4. 以下のID Value Objectsを定義する:
   - `SecretId` - Secret Entity用
   - `ShareCollectionId` - ShareCollection Entity用
   - `CapsuleId` - Capsule Entity用
   - `KFragId` - KFrag Entity用
   - `CFragId` - CFrag Entity用

#### Test Coverage

| AC# | Test Function(s) | Status |
|-----|------------------|--------|
| 1 | `test_secret_id_new_and_as_str`, `test_secret_id_generate` | ✅ |
| 2 | `test_secret_id_equality` | ✅ |
| 3 | `test_type_safety_compile_time` | ✅ |
| 4 | `test_share_collection_id`, `test_capsule_id`, `test_kfrag_id`, `test_cfrag_id` | ✅ |

### Requirement 7: SecretData Value Object

**User Story:** As a 開発者, I want SecretData Value Objectで秘密の実データを扱いたい, so that 秘密を安全に一時処理できる

#### Design Note

PRD Phase 1-1の `f(0)=secret` に対応。SDK利用者から受け取り、シャミア分散後に破棄される。

#### Acceptance Criteria

1. WHEN SecretData::new()が呼ばれた THEN システム SHALL バイト列をラップしたSecretDataを返す
2. WHEN 空のバイト列が渡された THEN システム SHALL DomainErrorを返す
3. IF SecretDataがDropされた THEN システム SHALL Zeroizeによりメモリをクリアする
4. SecretData SHALL Cloneトレイトを実装しない（偶発的コピー防止）

#### Test Coverage

| AC# | Test Function(s) | Status |
|-----|------------------|--------|
| 1 | `test_secret_data_new_valid` | ✅ |
| 2 | `test_secret_data_new_empty_error` | ✅ |
| 3 | `test_secret_data_debug_redacted` | ⚠️ 間接的 |
| 4 | (コンパイル時検証) | ✅ |

### Requirement 8: KeyPair Value Object

**User Story:** As a 開発者, I want KeyPair Value ObjectでPRE鍵ペアを扱いたい, so that Owner/Requesterの鍵を型安全に管理できる

#### Design Note

PRD Phase 1-1の `skₒ(PRE)`, `pkₒ(PRE)` に対応。SDK利用者が管理し、FORMIXは生成・使用のみ。

#### Acceptance Criteria

1. WHEN KeyPair::generate()が呼ばれた THEN システム SHALL umbral-preで鍵ペアを生成しKeyPairを返す
2. WHEN KeyPair.public_key()が呼ばれた THEN システム SHALL 公開鍵への参照を返す
3. WHEN KeyPair.secret_key()が呼ばれた THEN システム SHALL 秘密鍵への参照を返す
4. IF KeyPairがDropされた THEN システム SHALL Zeroizeにより秘密鍵をメモリからクリアする
5. KeyPair SHALL Cloneトレイトを実装しない（秘密鍵の偶発的コピー防止）

#### Test Coverage

| AC# | Test Function(s) | Status |
|-----|------------------|--------|
| 1 | (なし - generate()メソッド未実装) | ❌ メソッド未実装 |
| 2 | `test_key_pair_new` | ✅ |
| 3 | `test_key_pair_new` | ✅ |
| 4 | `test_key_pair_debug_redacted` | ⚠️ 間接的 |
| 5 | (コンパイル時検証) | ✅ |

**Note:** AC1の`generate()`メソッドは未実装。現在は`new()`コンストラクタのみ。将来的にumbral-preを使用したキー生成を実装予定。

**追加テスト:** `test_key_pair_from_slice_valid`, `test_key_pair_from_slice_invalid_length` など将来追加可能

### Requirement 9: SymmetricKey Value Object

**User Story:** As a 開発者, I want SymmetricKey Value ObjectでAES共通鍵を扱いたい, so that シェア暗号化用の鍵を安全に管理できる

#### Design Note

PRD Phase 1-1の `kₒ` に対応。Capsule生成時に使用され、暗号化後は破棄される。

#### Acceptance Criteria

1. WHEN SymmetricKey::generate()が呼ばれた THEN システム SHALL 256-bit AES鍵を生成しSymmetricKeyを返す
2. WHEN SymmetricKey.as_bytes()が呼ばれた THEN システム SHALL 鍵バイト列への参照を返す
3. IF SymmetricKeyがDropされた THEN システム SHALL Zeroizeによりメモリをクリアする
4. SymmetricKey SHALL Cloneトレイトを実装しない（偶発的コピー防止）

#### Test Coverage

| AC# | Test Function(s) | Status |
|-----|------------------|--------|
| 1 | (なし - generate()メソッド未実装) | ❌ メソッド未実装 |
| 2 | `test_symmetric_key_new`, `test_symmetric_key_from_slice_valid` | ✅ |
| 3 | `test_symmetric_key_debug_redacted` | ⚠️ 間接的 |
| 4 | (コンパイル時検証) | ✅ |

**Note:** AC1の`generate()`メソッドは未実装。現在は`new()`/`from_slice()`コンストラクタのみ。将来的に乱数を使用した鍵生成を実装予定。

**追加テスト（実装済み）:**
- `test_symmetric_key_from_slice_invalid_length` - 不正な長さの拒否

## Non-Functional Requirements

### Code Architecture and Modularity

- **Single Responsibility Principle**: 各エンティティ/Value Objectファイルは1つの型のみを定義
- **Modular Design**: Domain層内で独立し、外部依存を持たない
- **Dependency Management**: serde等のシリアライズ注釈はInfrastructure層で追加（Domain層では純粋）
- **Clear Interfaces**: new()/generate()コンストラクタとgetter/setterで明確なインターフェースを提供
- **Layer Separation**:
  - Domain層: Entity（Secret, ShareCollection, Capsule, KFrag, CFrag）+ Value Objects
  - Service層: 処理中間状態（PlaintextShare等）はZeroize付きの内部構造体として定義

### Performance

- **メモリ効率**: 不必要なClone実装を避け、参照ベースのAPIを提供
- **アロケーション最小化**: WASMターゲットを考慮し、ヒープ割り当てを最小限に
- **Arweaveトランザクション効率**: ShareCollectionによる一括保存でtx数を削減

### Security

- **Zeroize実装**: 秘密データを含むValue Objects（SecretData, KeyPair, SymmetricKey）とEntities（KFrag, CFrag）はZeroizeを実装
- **Clone禁止**: 秘密データを含む型はCloneを実装しない
- **不変条件検証**: コンストラクタで閾値パラメータ等の妥当性を検証
- **平文の非永続化**: 平文シェア `f(i)` はDomain層に含めず、Service層で一時的に処理後即座にZeroize

### Reliability

- **型安全性**: 強い型付け（newtypeパターン）によりコンパイル時にエラーを検出
- **Result型**: 失敗可能な操作はすべてResult<T, DomainError>を返す
- **アトミック整合性**: ShareCollectionによりn個のシェアの部分的な保存失敗を防止

### Usability

- **ドキュメント**: 各公開APIにrustdocコメントを追加
- **Debug実装**: デバッグ用にDebugトレイトを実装（秘密データは隠蔽）

## Test Coverage Summary

### Overall Statistics

| Requirement | Total AC | Covered | Partial | Missing |
|-------------|----------|---------|---------|---------|
| 1. Secret Entity | 4 | 4 | 0 | 0 |
| 2. ShareCollection Entity | 6 | 6 | 0 | 0 |
| 3. Capsule Entity | 4 | 4 | 0 | 0 |
| 4. KFrag Entity | 4 | 3 | 1 | 0 |
| 5. CFrag Entity | 4 | 2 | 1 | 1 |
| 6. ID Value Objects | 4 | 4 | 0 | 0 |
| 7. SecretData | 4 | 3 | 1 | 0 |
| 8. KeyPair | 5 | 3 | 1 | 1 |
| 9. SymmetricKey | 4 | 2 | 1 | 1 |
| **Total** | **39** | **31** | **5** | **3** |

**カバレッジ率: 79.5% (31/39 fully covered)**

**テスト数: 49テスト（全てパス）**

### Legend

- ✅ **Covered**: テストが実装され、ACを完全にカバー
- ⚠️ **Partial**: 間接的なテスト（例: Debug redactionでZeroizeを確認）またはN/A
- ❌ **Missing**: テストが未実装

### Missing Test Coverage

以下のAcceptance Criteriaに対するテストが未実装（メソッド自体が未実装のため）:

#### KeyPair (Requirement 8)
- AC1: `KeyPair::generate()` - メソッド未実装。umbral-preを使用した鍵ペア生成の実装が必要。

#### SymmetricKey (Requirement 9)
- AC1: `SymmetricKey::generate()` - メソッド未実装。乱数を使用した鍵生成の実装が必要。

#### CFrag (Requirement 5)
- AC2: 対応するKFragIdが存在しない場合 - 設計上、呼び出し側（Service層）で検証するため、Domain層では実装不要。

### 実装済みテスト一覧

**KeyPair (2テスト):**
- `test_key_pair_new` - コンストラクタとgetter
- `test_key_pair_debug_redacted` - Debug redaction

**SymmetricKey (4テスト):**
- `test_symmetric_key_new` - コンストラクタ
- `test_symmetric_key_from_slice_valid` - from_slice成功ケース
- `test_symmetric_key_from_slice_invalid_length` - from_slice失敗ケース
- `test_symmetric_key_debug_redacted` - Debug redaction

### Test Patterns Used

1. **Success Pattern**: 有効入力での生成と全getterの検証
2. **Validation Pattern**: 無効入力（空データ、境界外、重複等）でのエラー確認
3. **State Machine Pattern**: 有効/無効な状態遷移の検証
4. **Mutation Pattern**: setter操作と永続性の確認
5. **Query Pattern**: 単一/複数アイテムの取得操作
6. **Security Pattern**: Debug出力での秘密データ redaction
7. **Type Safety Pattern**: コンパイル時型安全性の文書化
8. **Crypto Pattern**: 暗号操作の正当性検証
