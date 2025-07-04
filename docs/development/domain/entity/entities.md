---
title: "D-TPRES Entity詳細設計"
description: "全Entityクラスとデータ構造の包括的定義"
tags: ["entity-design", "data-structures", "prd-compliant", "ao-native"]
status: "specification"
created: "2025-06-25"
author: "D-TPRES Development Team"
---

# D-TPRES Entity詳細設計

## 1. 概要

本ドキュメントは、D-TPRESシステムにおける全てのEntityクラスの詳細設計を定義します。Entityはデータ保持に特化し、メソッドを持たない純粋なデータ構造として設計されています。

## 2. 設計原則

### 2.1 基本原則
- **データ保持専用**: メソッドを一切持たない
- **シリアライズ可能**: JSON形式での永続化に対応
- **不変性重視**: 可能な限りイミュータブルな設計
- **型安全性**: 厳密な型定義による安全性確保

### 2.2 命名規約
- Entity名: `<Domain>Entity` 形式（例: ProcessEntity）
- フィールド名: snake_case
- 型名: PascalCase

<<<<<<< HEAD
### 2.3 AOステートレス実行環境の考慮事項

AOプロセスは各メッセージ処理で異なるCompute Unit（CU）で実行される可能性があり、以下の特性を考慮した設計が必要です：

#### 実行環境の特性
1. **インスタンスの非永続性**: Entityインスタンスはメッセージ処理ごとに生成され、処理終了時に破棄される
2. **状態データの永続性**: すべての状態はArweaveに保存され、Repository経由で読み書きされる
3. **コードとデータの分離**: WASMコード（tx_idで参照）とインスタンス、状態データは明確に区別される

#### Entity設計への影響
1. **軽量化の必要性**: 
   - 頻繁にインスタンス化されるEntityは最小限のデータ構造にする
   - 大きなデータ構造は別Entityに分離し、必要時のみロードする

2. **インデックスパターン**:
   - 完全なデータではなく、参照情報（ID、状態、更新日時）を保持
   - 詳細データは別途SecretDetailsEntityなどで管理

3. **メッセージコンテキスト対応**:
   - メッセージから必要なEntity IDを効率的に抽出可能な設計
   - 選択的なEntityロードをサポートする構造

```rust
// 重要な区別:
// - コード（WASM）: Arweaveに保存され、tx_idで参照される実行可能なバイナリ
// - インスタンス: CU上でWASMコードから毎回生成されるオブジェクト  
// - 状態データ: Arweaveに永続化され、Repository経由で読み書きされる実際のデータ
```

=======
>>>>>>> origin/development
## 3. ProcessEntity - マルチロール対応プロセス

### 3.1 概要
各AOプロセスの基本情報と、Owner/Holder/Requesterの全ロールデータを保持する普遍的なEntity。

### 3.2 詳細定義

```rust
use std::collections::HashMap;
use serde::{Serialize, Deserialize};

/// プロセスエンティティ - AOプロセスの完全な状態表現
/// 
/// # 特徴
/// - マルチロール対応（Owner/Holder/Requester同時保持可能）
/// - AO環境ネイティブプロパティ対応
/// - パフォーマンスメトリクス統合
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProcessEntity {
    /// プロセス識別子（AO process ID）
    pub process_id: String,
    
    /// プロセス名（人間可読な識別子）
    pub process_name: String,
    
    /// 現在のアクティブロール
    /// 値: ["owner"], ["holder"], ["requester"], またはその組み合わせ
    pub active_roles: Vec<String>,
    
    /// Owner機能データ（Phase 0, 1, 3で使用）
    pub owner_data: Option<OwnerData>,
    
    /// Holder機能データ（Phase 3, 4で使用）
    pub holder_data: Option<HolderData>,
    
    /// Requester機能データ（Phase 2, 4, 5で使用）
    pub requester_data: Option<RequesterData>,
    
    /// プロセス設定（Key-Value形式）
    /// 例: {"max_concurrent_requests": "10", "timeout_seconds": "300"}
    pub configuration: HashMap<String, String>,
    
    /// 対応暗号操作リスト
    /// 値: ["shamir_split", "pre_encrypt", "re_encrypt", "verify_proof"]
    pub supported_crypto_operations: Vec<String>,
    
    /// パフォーマンスメトリクス
    pub performance_metrics: PerformanceMetrics,
    
    /// 作成日時（Unix timestamp秒）
    pub created_at: u64,
    
    /// 最終更新日時（Unix timestamp秒）
    pub updated_at: u64,
    
    /// バージョン番号（楽観ロック用）
    pub version: u64,
}

/// Owner機能データ - skO管理と秘密分割を担当
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OwnerData {
    /// オーナー公開鍵（skOに対応するpkO）
    pub owner_public_key: Vec<u8>,
    
<<<<<<< HEAD
    /// 管理する秘密のインデックス情報
    /// Key: 秘密ID, Value: 軽量な秘密インデックス
    /// 詳細情報はSecretDetailsEntityで別管理
    pub secret_indices: HashMap<String, SecretIndex>,
=======
    /// 管理する秘密群
    /// Key: 秘密ID, Value: 秘密管理データ
    pub managed_secrets: HashMap<String, SecretManagementData>,
>>>>>>> origin/development
    
    /// Owner固有設定
    /// 例: {"default_threshold": "3", "default_shares": "5"}
    pub owner_config: HashMap<String, String>,
}

<<<<<<< HEAD
/// 秘密インデックス - 軽量な秘密管理情報
/// ProcessEntityに保持される最小限の情報
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SecretIndex {
    /// 秘密識別子
    pub secret_id: String,
    
    /// 秘密の状態
    /// 値: "active", "archived", "expired"
    pub status: String,
    
    /// Entity参照情報
    pub entity_references: EntityReferences,
    
    /// 最終更新日時
    pub last_updated: u64,
    
    /// Shamir閾値（k）- 頻繁に参照されるため保持
    pub shamir_threshold: u8,
    
    /// Shamir総シェア数（n）- 頻繁に参照されるため保持
    pub shamir_total_shares: u8,
}

/// Entity参照情報 - 関連EntityのIDのみを保持
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntityReferences {
    /// ShareEntity IDリスト
    pub share_ids: Vec<String>,
    
    /// CapsuleEntity IDリスト
    pub capsule_ids: Vec<String>,
    
    /// アクティブなAccessRequestEntity IDリスト
    pub active_requests: Vec<String>,
    
    /// SecretDetailsEntity ID（詳細情報への参照）
    pub details_entity_id: String,
=======
/// 秘密管理データ - 1つの秘密に関する全管理情報
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SecretManagementData {
    /// 秘密識別子
    pub secret_id: String,
    
    /// 生成されたShareのID群（Phase 1で作成）
    pub generated_shares: Vec<String>,
    
    /// 生成されたCapsuleのID群（Phase 1で作成）
    pub generated_capsules: Vec<String>,
    
    /// アクセス制御条件群
    /// 例: ["erc20_balance_check", "nft_ownership_check"]
    pub access_control_conditions: Vec<String>,
    
    /// 条件別の生成済みkFrag群（Phase 3で作成）
    /// Key: アクセス制御条件, Value: RekeyFragmentEntity IDリスト
    pub generated_kfrags_by_condition: HashMap<String, Vec<String>>,
    
    /// Shamir閾値（k）
    pub shamir_threshold: u8,
    
    /// Shamir総シェア数（n）
    pub shamir_total_shares: u8,
    
    /// 生成日時
    pub created_at: u64,
>>>>>>> origin/development
}

/// Holder機能データ - kFrag保持と再暗号化実行
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HolderData {
    /// 保持中のkFrag群
    /// Key: RekeyFragment ID, Value: フラグメント情報
    pub held_fragments: HashMap<String, HolderFragmentInfo>,
    
    /// アクセス制御条件別のkFrag管理
    /// Key: アクセス制御条件, Value: RekeyFragment IDリスト
    pub fragments_by_condition: HashMap<String, Vec<String>>,
    
    /// 信頼性スコア（0.0-1.0）
    pub reliability_score: f64,
    
    /// 完了済み再暗号化数
    pub completed_reencryptions: u64,
    
    /// 最大保持可能フラグメント数
    pub max_fragment_capacity: u64,
    
    /// 現在の負荷状況（0-100）
    pub current_load: u64,
}

/// Holderフラグメント情報
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HolderFragmentInfo {
    /// フラグメント識別子
    pub fragment_id: String,
    
    /// 関連する秘密ID
    pub secret_id: String,
    
    /// アクセス制御条件
    pub access_control_condition: String,
    
    /// 受信日時
    pub received_at: u64,
    
    /// 使用回数
    pub usage_count: u64,
    
    /// 状態（"active", "expired", "revoked"）
    pub status: String,
}

/// Requester機能データ - アクセス要求とcFrag収集
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RequesterData {
    /// アクティブなアクセス要求ID群
    pub active_requests: Vec<String>,
    
    /// 処理中の再暗号化要求ID群
    pub active_reencryptions: Vec<String>,
    
    /// 完了済み要求数
    pub completed_requests: u64,
    
    /// 成功率（0.0-1.0）
    pub success_rate: f64,
    
    /// 平均処理時間（ミリ秒）
    pub average_processing_time_ms: u64,
    
    /// Requester固有設定
    /// 例: {"retry_attempts": "3", "timeout_ms": "30000"}
    pub requester_config: HashMap<String, String>,
}

/// パフォーマンスメトリクス
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// 成功操作数
    pub successful_operations: u64,
    
    /// 失敗操作数
    pub failed_operations: u64,
    
    /// 平均応答時間（ミリ秒）
    pub average_response_time_ms: u64,
    
    /// 最終メトリクス更新時刻
    pub last_updated_at: u64,
}
```

### 3.3 使用例

```rust
<<<<<<< HEAD
// Phase 0: Owner-Processの生成（軽量化されたバージョン）
=======
// Phase 0: Owner-Processの生成
>>>>>>> origin/development
let owner_process = ProcessEntity {
    process_id: "ao_process_001".to_string(),
    process_name: "AliceOwnerProcess".to_string(),
    active_roles: vec!["owner".to_string()],
    owner_data: Some(OwnerData {
        owner_public_key: vec![/* pkO bytes */],
<<<<<<< HEAD
        secret_indices: HashMap::new(), // 秘密が追加されるまでは空
=======
        managed_secrets: HashMap::new(),
>>>>>>> origin/development
        owner_config: HashMap::from([
            ("default_threshold".to_string(), "3".to_string()),
            ("default_shares".to_string(), "5".to_string()),
        ]),
    }),
    holder_data: None,
    requester_data: None,
    configuration: HashMap::from([
        ("max_secrets".to_string(), "100".to_string()),
    ]),
    supported_crypto_operations: vec![
        "shamir_split".to_string(),
        "pre_encrypt".to_string(),
    ],
    performance_metrics: PerformanceMetrics {
        successful_operations: 0,
        failed_operations: 0,
        average_response_time_ms: 0,
        last_updated_at: 1703001600,
    },
    created_at: 1703001600,
    updated_at: 1703001600,
    version: 1,
};
<<<<<<< HEAD

// Phase 1後: 秘密追加時のインデックス更新
let secret_index = SecretIndex {
    secret_id: "secret_001".to_string(),
    status: "active".to_string(),
    entity_references: EntityReferences {
        share_ids: vec!["share_001_01".to_string(), /* ... */],
        capsule_ids: vec!["capsule_001_01".to_string(), /* ... */],
        active_requests: vec![],
        details_entity_id: "secret_details_001".to_string(),
    },
    last_updated: 1703001700,
    shamir_threshold: 3,
    shamir_total_shares: 5,
};

// OwnerDataに秘密インデックスを追加
owner_process.owner_data.as_mut().unwrap()
    .secret_indices.insert("secret_001".to_string(), secret_index);
=======
>>>>>>> origin/development
```

## 4. ShareEntity - Shamirデータシェア

### 4.1 概要
Shamir Secret Sharingによって分割されたデータ断片を表現するEntity（PRD Phase 1）。

### 4.2 詳細定義

```rust
/// データシェアエンティティ - Shamir分割されたデータ断片
/// 
/// PRD Phase 1で生成され、秘密復元時に必要
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShareEntity {
    /// シェア識別子
    pub share_id: String,
    
    /// データグループ識別子（同一秘密から生成されたシェア群の識別子）
    pub data_id: String,
    
    /// 関連する秘密識別子
    pub secret_id: String,
    
    /// 閾値インデックス（1からnまで）
    pub threshold_index: u8,
    
    /// Shamir閾値（k: 復元に必要な最小シェア数）
    pub shamir_threshold: u8,
    
    /// Shamir総シェア数（n: 生成された総シェア数）
    pub shamir_total_shares: u8,
    
    /// 暗号化されたフラグメントデータ
    /// Ci = AES_GCM(Ki, f(i)) where f(i) is Shamir share
    pub encrypted_fragment: Vec<u8>,
    
    /// フラグメントサイズ（バイト）
    pub fragment_size: usize,
    
    /// データオーナーの公開鍵（pkO）
    pub owner_public_key: Vec<u8>,
    
    /// 整合性検証用ハッシュ（SHA-256）
    pub integrity_hash: Vec<u8>,
    
    /// 作成日時
    pub created_at: u64,
    
    /// 最終アクセス日時
    pub last_accessed_at: Option<u64>,
    
    /// バージョン
    pub version: u64,
}
```

### 4.3 使用例

```rust
// Phase 1: 秘密をShamir分割してShareを生成
let share = ShareEntity {
    share_id: "share_001_01".to_string(),
    data_id: "data_001".to_string(),
    secret_id: "secret_001".to_string(),
    threshold_index: 1,
    shamir_threshold: 3,
    shamir_total_shares: 5,
    encrypted_fragment: vec![/* encrypted f(1) */],
    fragment_size: 256,
    owner_public_key: vec![/* pkO */],
    integrity_hash: vec![/* SHA-256 hash */],
    created_at: 1703001700,
    last_accessed_at: None,
    version: 1,
};
<<<<<<< HEAD

// 軽量化されたProcessEntityではインデックスのみ更新
// ShareEntity自体はRepository経由で別途永続化される
=======
>>>>>>> origin/development
```

## 5. CapsuleEntity - PREカプセル

### 5.1 概要
Proxy Re-Encryptionのカプセル情報を保持するEntity（PRD Phase 1）。

### 5.2 詳細定義

```rust
/// カプセルエンティティ - PRE暗号化のカプセル
/// 
/// Phase 1で生成され、Phase 4の再暗号化で使用
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CapsuleEntity {
    /// カプセル識別子
    pub capsule_id: String,
    
    /// 関連データ識別子
    pub data_id: String,
    
    /// 関連する秘密識別子
    pub secret_id: String,
    
    /// カプセルインデックス（i: 対応するシェアのインデックス）
    pub capsule_index: u8,
    
    /// PRE Capsuleデータ
    /// Capsulei = PRE_Enc(pkO, Ki)
    pub capsule_data: Vec<u8>,
    
    /// 対応する暗号文識別子（Ci = AES_GCM(Ki, f(i))）
    pub corresponding_ciphertext_id: String,
    
    /// オーナー公開鍵（pkO）
    pub owner_public_key: Vec<u8>,
    
    /// カプセル生成に使用されたランダム鍵（暗号化して保存）
    pub encrypted_random_key: Vec<u8>,
    
    /// 作成日時
    pub created_at: u64,
    
    /// バージョン
    pub version: u64,
}
```

### 5.3 使用例

```rust
// Phase 1: ShareとペアでCapsuleを生成
let capsule = CapsuleEntity {
    capsule_id: "capsule_001_01".to_string(),
    data_id: "data_001".to_string(),
    secret_id: "secret_001".to_string(),
    capsule_index: 1,
    capsule_data: vec![/* PRE_Enc(pkO, K1) */],
    corresponding_ciphertext_id: "share_001_01".to_string(),
    owner_public_key: vec![/* pkO */],
    encrypted_random_key: vec![/* encrypted K1 */],
    created_at: 1703001700,
    version: 1,
};
<<<<<<< HEAD

// ProcessEntityのインデックスにCapsule IDを追加
// CapsuleEntity自体は必要時までArweaveに保存
=======
>>>>>>> origin/development
```

## 6. AccessRequestEntity - アクセス要求

### 6.1 概要
データアクセス要求とEVM検証結果を管理するEntity（PRD Phase 2）。

### 6.2 詳細定義

```rust
/// アクセス要求エンティティ - データアクセスの要求と検証
/// 
/// Phase 2で作成され、EVM検証を経てPhase 3へ進む
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AccessRequestEntity {
    /// 要求識別子
    pub request_id: String,
    
    /// 要求対象データID
    pub target_data_id: String,
    
    /// 要求対象秘密ID
    pub target_secret_id: String,
    
    /// 要求者プロセスID（R-Proc）
    pub requester_process_id: String,
    
    /// アクセス者の公開鍵（pkA）
    pub accessor_public_key: Vec<u8>,
    
    /// データオーナー公開鍵（pkO）
    pub owner_public_key: Vec<u8>,
    
    /// EVM検証結果
    pub evm_verification: EvmVerificationData,
    
    /// ProofPkg情報（elciaoによる生成）
    pub proof_pkg: Option<ProofPkgData>,
    
    /// 要求状態
    /// 値: "pending", "evm_verified", "approved", "rejected", "completed"
    pub status: String,
    
    /// 要求作成日時
    pub created_at: u64,
    
    /// EVM検証完了日時
    pub evm_verified_at: Option<u64>,
    
    /// 要求完了日時
    pub completed_at: Option<u64>,
    
    /// タイムアウト時刻
    pub timeout_at: u64,
    
    /// バージョン
    pub version: u64,
}

/// EVM検証データ
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvmVerificationData {
    /// EVMトランザクションハッシュ
    pub tx_hash: String,
    
    /// スマートコントラクトアドレス
    pub contract_address: String,
    
    /// VerificationOKイベントデータ
    pub verification_event: Vec<u8>,
    
    /// ブロック高
    pub block_height: u64,
    
    /// 検証日時
    pub verified_at: u64,
}

/// ProofPkgデータ - elciaoによる検証パッケージ
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProofPkgData {
    /// ProofPkg識別子
    pub proof_pkg_id: String,
    
    /// 証明パッケージ（BlockHeader + Receipt + pkA）
    pub proof_package: Vec<u8>,
    
    /// elciaoによる作成日時
    pub created_at: u64,
    
    /// 検証済みフラグ
    pub verified: bool,
}
```

### 6.3 使用例

```rust
// Phase 2: アクセス要求とEVM検証
let access_request = AccessRequestEntity {
    request_id: "req_001".to_string(),
    target_data_id: "data_001".to_string(),
    target_secret_id: "secret_001".to_string(),
    requester_process_id: "ao_process_002".to_string(),
    accessor_public_key: vec![/* pkA */],
    owner_public_key: vec![/* pkO */],
    evm_verification: EvmVerificationData {
        tx_hash: "0x1234...".to_string(),
        contract_address: "0xabcd...".to_string(),
        verification_event: vec![/* event data */],
        block_height: 1000000,
        verified_at: 1703001800,
    },
    proof_pkg: Some(ProofPkgData {
        proof_pkg_id: "proof_001".to_string(),
        proof_package: vec![/* BlockHeader + Receipt + pkA */],
        created_at: 1703001850,
        verified: true,
    }),
    status: "evm_verified".to_string(),
    created_at: 1703001750,
    evm_verified_at: Some(1703001800),
    completed_at: None,
    timeout_at: 1703005350,
    version: 1,
};
<<<<<<< HEAD

// AOステートレス環境では、AccessRequestEntityは別途永続化
// ProcessEntityのインデックスにアクティブな要求IDを追加
=======
>>>>>>> origin/development
```

## 7. RekeyFragmentEntity - 再暗号化キーフラグメント

### 7.1 概要
Shamirで分割された再暗号化キーフラグメント（kFrag）を表現するEntity（PRD Phase 3）。

### 7.2 詳細定義

```rust
/// 再暗号化キーフラグメントエンティティ - 分割された再暗号化キー
/// 
/// Phase 3で生成され、Holderに配布される
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RekeyFragmentEntity {
    /// フラグメント識別子
    pub fragment_id: String,
    
    /// 関連する秘密識別子
    pub secret_id: String,
    
    /// 関連するアクセス要求ID
    pub access_request_id: String,
    
    /// アクセス制御条件
    pub access_control_condition: String,
    
    /// オーナー公開鍵（pkO）
    pub owner_public_key: Vec<u8>,
    
    /// アクセス者公開鍵（pkA）
    pub accessor_public_key: Vec<u8>,
    
    /// Shamirフラグメントインデックス（j: 1からnまで）
    pub shamir_index: u8,
    
    /// Shamir閾値（k: 再暗号化に必要な最小フラグメント数）
    pub shamir_threshold: u8,
    
    /// Shamir総フラグメント数（n）
    pub shamir_total_fragments: u8,
    
    /// kFragデータ（Shamirで分割された再暗号化キー断片）
    /// kFragj = ShamirSplit(ReKey(skO→pkA), j)
    pub kfrag_data: Vec<u8>,
    
    /// 割り当てられたHolderプロセスID
    pub assigned_holder_id: String,
    
    /// フラグメント状態
    /// 値: "created", "distributed", "active", "consumed", "expired"
    pub status: String,
    
    /// 有効期限
    pub expires_at: Option<u64>,
    
    /// 作成日時
    pub created_at: u64,
    
    /// 配布日時
    pub distributed_at: Option<u64>,
    
    /// バージョン
    pub version: u64,
}
```

### 7.3 使用例

```rust
// Phase 3: ReKeyをShamir分割してkFragを生成
let kfrag = RekeyFragmentEntity {
    fragment_id: "kfrag_001_01".to_string(),
    secret_id: "secret_001".to_string(),
    access_request_id: "req_001".to_string(),
    access_control_condition: "erc20_balance_check".to_string(),
    owner_public_key: vec![/* pkO */],
    accessor_public_key: vec![/* pkA */],
    shamir_index: 1,
    shamir_threshold: 3,
    shamir_total_fragments: 5,
    kfrag_data: vec![/* ShamirSplit(ReKey, 1) */],
    assigned_holder_id: "ao_process_h01".to_string(),
    status: "distributed".to_string(),
    expires_at: Some(1703088000),
    created_at: 1703001900,
    distributed_at: Some(1703001950),
    version: 1,
};
```

## 8. ReencryptionEntity - 再暗号化処理

### 8.1 概要
k-of-nプロキシ再暗号化処理の状態を管理するEntity（PRD Phase 4）。

### 8.2 詳細定義

```rust
/// 再暗号化エンティティ - プロキシ再暗号化処理の管理
/// 
/// Phase 4で作成され、cFrag収集と再暗号化を追跡
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReencryptionEntity {
    /// 再暗号化識別子
    pub reencryption_id: String,
    
    /// 関連するアクセス要求ID
    pub access_request_id: String,
    
    /// 対象カプセルID
    pub target_capsule_id: String,
    
    /// 要求者プロセスID（R-Proc）
    pub requester_process_id: String,
    
    /// 対象Holder群のプロセスID
    pub target_holders: Vec<String>,
    
    /// 必要な閾値数（k）
    pub required_threshold: u8,
    
    /// 収集済みcFrag群
    pub collected_cfrags: Vec<CFragData>,
    
    /// 再暗号化状態
    /// 値: "initiated", "collecting", "threshold_met", "completed", "failed"
    pub status: String,
    
    /// 開始日時
    pub started_at: u64,
    
    /// 完了日時
    pub completed_at: Option<u64>,
    
    /// タイムアウト時刻
    pub timeout_at: u64,
    
    /// バージョン
    pub version: u64,
}

/// cFragデータ - 再暗号化された断片
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CFragData {
    /// cFrag識別子
    pub cfrag_id: String,
    
    /// 生成元HolderプロセスID
    pub holder_id: String,
    
    /// cFragデータ
    /// cFragj = PRE_ReEnc(kFragj, Capsulei)
    pub cfrag_data: Vec<u8>,
    
    /// 対応するkFrag ID
    pub corresponding_kfrag_id: String,
    
    /// 生成日時
    pub generated_at: u64,
    
    /// Holder署名（完全性保証）
    pub holder_signature: Vec<u8>,
}
```

### 8.3 使用例

```rust
// Phase 4: 再暗号化プロセスの管理
let reencryption = ReencryptionEntity {
    reencryption_id: "reenc_001".to_string(),
    access_request_id: "req_001".to_string(),
    target_capsule_id: "capsule_001_01".to_string(),
    requester_process_id: "ao_process_002".to_string(),
    target_holders: vec![
        "ao_process_h01".to_string(),
        "ao_process_h02".to_string(),
        "ao_process_h03".to_string(),
    ],
    required_threshold: 3,
    collected_cfrags: vec![
        CFragData {
            cfrag_id: "cfrag_001_01".to_string(),
            holder_id: "ao_process_h01".to_string(),
            cfrag_data: vec![/* PRE_ReEnc result */],
            corresponding_kfrag_id: "kfrag_001_01".to_string(),
            generated_at: 1703002000,
            holder_signature: vec![/* signature */],
        },
        // ... more cFrags
    ],
    status: "collecting".to_string(),
    started_at: 1703001950,
    completed_at: None,
    timeout_at: 1703005550,
    version: 1,
};
```

<<<<<<< HEAD
## 9. SecretDetailsEntity - 秘密管理詳細情報

### 9.1 概要
秘密に関する詳細な管理情報を保持するEntity。ProcessEntityの軽量化のため、頻繁にアクセスされない詳細情報を分離して管理する。

### 9.2 詳細定義

```rust
/// 秘密管理詳細エンティティ - 1つの秘密に関する詳細情報
/// 
/// ProcessEntityから分離され、必要時のみロードされる
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SecretDetailsEntity {
    /// 詳細エンティティ識別子
    pub details_id: String,
    
    /// 関連する秘密識別子
    pub secret_id: String,
    
    /// アクセス制御条件群
    /// 例: ["erc20_balance_check", "nft_ownership_check"]
    pub access_control_conditions: Vec<String>,
    
    /// 条件別の生成済みkFrag群（Phase 3で作成）
    /// Key: アクセス制御条件, Value: RekeyFragmentEntity IDリスト
    pub generated_kfrags_by_condition: HashMap<String, Vec<String>>,
    
    /// アクセス履歴
    pub access_history: Vec<AccessRecord>,
    
    /// 秘密のメタデータ
    pub metadata: HashMap<String, String>,
    
    /// 秘密の説明
    pub description: Option<String>,
    
    /// 有効期限
    pub expires_at: Option<u64>,
    
    /// 生成日時
    pub created_at: u64,
    
    /// 最終更新日時
    pub updated_at: u64,
    
    /// バージョン
    pub version: u64,
}

/// アクセス記録
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AccessRecord {
    /// アクセス要求ID
    pub request_id: String,
    
    /// アクセス者プロセスID
    pub accessor_process_id: String,
    
    /// アクセス日時
    pub accessed_at: u64,
    
    /// アクセス結果
    /// 値: "granted", "denied", "expired"
    pub result: String,
    
    /// 使用されたアクセス制御条件
    pub condition_used: String,
}
```

### 9.3 使用例

```rust
// Phase 1: 秘密分割時にSecretDetailsEntityを作成
let secret_details = SecretDetailsEntity {
    details_id: "secret_details_001".to_string(),
    secret_id: "secret_001".to_string(),
    access_control_conditions: vec![
        "erc20_balance_check".to_string(),
        "nft_ownership_check".to_string(),
    ],
    generated_kfrags_by_condition: HashMap::new(),
    access_history: vec![],
    metadata: HashMap::from([
        ("encryption_algorithm".to_string(), "AES-256-GCM".to_string()),
        ("data_type".to_string(), "document".to_string()),
    ]),
    description: Some("Confidential financial report Q4 2023".to_string()),
    expires_at: Some(1735689600), // 2025-01-01
    created_at: 1703001700,
    updated_at: 1703001700,
    version: 1,
};

// メッセージハンドラーでの選択的ロード
async fn handle_access_request(ctx: &HandlerContext, msg: Message) -> Result<()> {
    let secret_id = msg.tags.get("Secret-Id")?;
    
    // まずProcessEntityのインデックスを確認
    let index = ctx.process.owner_data.as_ref()
        .and_then(|od| od.secret_indices.get(secret_id))
        .ok_or(Error::SecretNotFound)?;
    
    // アクセス制御条件の確認が必要な場合のみ詳細をロード
    if msg.tags.get("Action")? == "Verify-Conditions" {
        let details = ctx.repositories.secret_details_repo
            .find_by_id(&index.details_entity_id)
            .await?;
        
        // アクセス制御条件の検証処理
        // ...
    }
    
    Ok(())
}
```

## 10. Entity関連図
=======
## 9. Entity関連図
>>>>>>> origin/development

```mermaid
graph TB
    subgraph "Phase 0: Process Spawn"
<<<<<<< HEAD
        PE[ProcessEntity<br/>with SecretIndex]
=======
        PE[ProcessEntity]
>>>>>>> origin/development
    end
    
    subgraph "Phase 1: Secret Sharing"
        SE[ShareEntity]
        CE[CapsuleEntity]
<<<<<<< HEAD
        SDE[SecretDetailsEntity]
=======
>>>>>>> origin/development
    end
    
    subgraph "Phase 2: Access Request"
        ARE[AccessRequestEntity]
    end
    
    subgraph "Phase 3: Key Fragmentation"
        RFE[RekeyFragmentEntity]
    end
    
    subgraph "Phase 4: Re-encryption"
        RE[ReencryptionEntity]
    end
    
<<<<<<< HEAD
    PE -->|references| SDE
=======
>>>>>>> origin/development
    PE -->|creates| SE
    PE -->|creates| CE
    PE -->|initiates| ARE
    ARE -->|triggers| RFE
    RFE -->|enables| RE
    
<<<<<<< HEAD
    SDE -.->|tracks| SE
    SDE -.->|tracks| CE
=======
>>>>>>> origin/development
    SE -.->|referenced by| RE
    CE -.->|used in| RE
```

<<<<<<< HEAD
## 11. 実装ガイドライン

### 11.1 Entityの実装規約
=======
## 10. 実装ガイドライン

### 10.1 Entityの実装規約
>>>>>>> origin/development

```rust
// ✅ 正しい実装例
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MyEntity {
    pub id: String,
    pub data: Vec<u8>,
    pub created_at: u64,
}

// ❌ 間違った実装例（メソッドを含む）
impl MyEntity {
    pub fn new(id: String) -> Self {  // NG: コンストラクタ
        Self {
            id,
            data: vec![],
            created_at: 0,
        }
    }
    
    pub fn validate(&self) -> bool {  // NG: バリデーションメソッド
        !self.id.is_empty()
    }
}
```

<<<<<<< HEAD
### 11.2 シリアライゼーション
=======
### 10.2 シリアライゼーション
>>>>>>> origin/development

全てのEntityは以下の形式でシリアライズ可能：

```rust
// JSONシリアライズ
let json = serde_json::to_string(&entity)?;

// JSONデシリアライズ
let entity: ProcessEntity = serde_json::from_str(&json)?;

// バイナリシリアライズ（bincode使用時）
let bytes = bincode::serialize(&entity)?;
let entity: ProcessEntity = bincode::deserialize(&bytes)?;
```

<<<<<<< HEAD
### 11.3 フィールド命名規約
=======
### 10.3 フィールド命名規約
>>>>>>> origin/development

| フィールド型 | 命名パターン | 例 |
|------------|------------|---|
| 識別子 | `{entity}_id` | `process_id`, `share_id` |
| 日時 | `{action}_at` | `created_at`, `updated_at` |
| 状態 | `status` | `status: "active"` |
| バージョン | `version` | `version: 1` |
| 公開鍵 | `{owner}_public_key` | `owner_public_key` |
| データ | `{type}_data` | `kfrag_data`, `cfrag_data` |

<<<<<<< HEAD
### 11.4 AOステートレス環境でのEntity管理パターン

#### EntityBundle パターン
関連するEntityを効率的に管理するためのバンドルパターン：

```rust
/// Entity管理用バンドル - メッセージ処理中のEntity群を管理
pub struct EntityBundle {
    /// ShareEntityリスト（選択的ロード）
    pub shares: Option<Vec<ShareEntity>>,
    
    /// CapsuleEntityリスト（選択的ロード）
    pub capsules: Option<Vec<CapsuleEntity>>,
    
    /// AccessRequestEntityリスト
    pub requests: Option<Vec<AccessRequestEntity>>,
    
    /// SecretDetailsEntity（必要時のみ）
    pub secret_details: Option<SecretDetailsEntity>,
    
    /// ロードされた時刻
    pub loaded_at: u64,
}

impl EntityBundle {
    /// 最小限のBundle（メタデータのみ）
    pub fn minimal(index: &SecretIndex) -> Self {
        Self {
            shares: None,
            capsules: None,
            requests: None,
            secret_details: None,
            loaded_at: current_timestamp(),
        }
    }
    
    /// 完全なBundle（全Entityデータ）
    pub fn full(
        shares: Vec<ShareEntity>,
        capsules: Vec<CapsuleEntity>,
        details: SecretDetailsEntity,
    ) -> Self {
        Self {
            shares: Some(shares),
            capsules: Some(capsules),
            requests: None,
            secret_details: Some(details),
            loaded_at: current_timestamp(),
        }
    }
    
    /// アクション別の選択的Bundle生成
    pub async fn for_action(
        repos: &RepositoryContainer,
        index: &SecretIndex,
        action: &str,
    ) -> Result<Self> {
        match action {
            "Split-Secret" => Ok(Self::minimal(index)),
            "Access-Request" => {
                // 秘密の詳細情報のみ必要
                let details = repos.secret_details_repo
                    .find_by_id(&index.details_entity_id)
                    .await?;
                Ok(Self {
                    secret_details: Some(details),
                    ..Self::minimal(index)
                })
            },
            "Re-Encrypt" => {
                // ShareとCapsuleの完全データが必要
                let (shares, capsules) = tokio::join!(
                    repos.share_repo.find_by_ids(&index.entity_references.share_ids),
                    repos.capsule_repo.find_by_ids(&index.entity_references.capsule_ids),
                );
                Ok(Self::full(shares?, capsules?, details))
            },
            _ => Ok(Self::minimal(index))
        }
    }
}
```

#### MessageContext パターン
メッセージから必要なEntityを特定するためのコンテキスト：

```rust
/// メッセージコンテキスト - AOメッセージから抽出した情報
pub struct MessageContext {
    /// アクション種別
    pub action: String,
    
    /// 対象秘密ID
    pub secret_id: Option<String>,
    
    /// 関連EntityのID群
    pub entity_ids: Vec<String>,
    
    /// メッセージタグ
    pub tags: HashMap<String, String>,
}

impl MessageContext {
    /// メッセージからコンテキストを抽出
    pub fn from_message(msg: &Message) -> Result<Self> {
        Ok(Self {
            action: msg.tags.get("Action")
                .ok_or(Error::MissingAction)?
                .to_string(),
            secret_id: msg.tags.get("Secret-Id")
                .map(|s| s.to_string()),
            entity_ids: msg.tags.get("Entity-Ids")
                .map(|s| s.split(',').map(|id| id.to_string()).collect())
                .unwrap_or_default(),
            tags: msg.tags.clone(),
        })
    }
    
    /// 必要なEntityタイプを判定
    pub fn required_entities(&self) -> Vec<EntityType> {
        match self.action.as_str() {
            "Split-Secret" => vec![EntityType::ProcessEntity],
            "Access-Request" => vec![
                EntityType::ProcessEntity,
                EntityType::SecretDetailsEntity,
            ],
            "Distribute-KFrag" => vec![
                EntityType::ProcessEntity,
                EntityType::AccessRequestEntity,
                EntityType::SecretDetailsEntity,
            ],
            "Re-Encrypt" => vec![
                EntityType::ProcessEntity,
                EntityType::ShareEntity,
                EntityType::CapsuleEntity,
                EntityType::RekeyFragmentEntity,
            ],
            _ => vec![EntityType::ProcessEntity],
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum EntityType {
    ProcessEntity,
    ShareEntity,
    CapsuleEntity,
    AccessRequestEntity,
    RekeyFragmentEntity,
    ReencryptionEntity,
    SecretDetailsEntity,
}
```

#### 使用例：効率的なメッセージハンドラー

```rust
/// AOメッセージハンドラーの実装例
async fn handle_message(msg: Message) -> Response {
    // 1. コンテキスト抽出
    let context = match MessageContext::from_message(&msg) {
        Ok(ctx) => ctx,
        Err(e) => return error_response(e),
    };
    
    // 2. HandlerContext初期化（ProcessEntityは常にロード）
    let mut handler_ctx = match initialize_handler_context().await {
        Ok(ctx) => ctx,
        Err(e) => return error_response(e),
    };
    
    // 3. 秘密インデックスの取得（秘密関連の操作の場合）
    let secret_index = if let Some(secret_id) = &context.secret_id {
        match get_secret_index(&handler_ctx.process, secret_id) {
            Some(index) => index,
            None => return error_response(Error::SecretNotFound),
        }
    } else {
        return handle_non_secret_action(&handler_ctx, &context).await;
    };
    
    // 4. 必要なEntityのみロード
    let entity_bundle = match EntityBundle::for_action(
        &handler_ctx.repositories,
        &secret_index,
        &context.action,
    ).await {
        Ok(bundle) => bundle,
        Err(e) => return error_response(e),
    };
    
    // 5. アクション別のビジネスロジック実行
    let result = match context.action.as_str() {
        "Split-Secret" => split_secret_service(&handler_ctx, &context).await,
        "Access-Request" => access_request_service(&handler_ctx, &entity_bundle).await,
        "Re-Encrypt" => reencrypt_service(&handler_ctx, &entity_bundle).await,
        _ => Err(Error::UnknownAction),
    };
    
    // 6. 結果に応じたレスポンス生成
    match result {
        Ok(data) => success_response(data),
        Err(e) => error_response(e),
    }
}

fn get_secret_index(process: &ProcessEntity, secret_id: &str) -> Option<&SecretIndex> {
    process.owner_data.as_ref()
        .and_then(|od| od.secret_indices.get(secret_id))
}
```

## 12. まとめ

D-TPRESのEntity設計は以下の特徴を持ちます：

1. **純粋なデータ構造**: メソッドを持たない
2. **PRD準拠**: 各PhaseのワークフローをEntityで正確に表現
3. **型安全性**: 厳密な型定義による安全性確保
4. **シリアライズ対応**: JSON/バイナリ形式での永続化
5. **トレーサビリティ**: 全Entityにタイムスタンプとバージョン
6. **AOステートレス対応**: 
   - ProcessEntityの軽量化（SecretIndexによる参照管理）
   - SecretDetailsEntityによる詳細情報の分離
   - EntityBundleによる効率的なロード戦略
   - MessageContextによる必要Entity判定

これらのEntityは、Repository層を通じてArweaveに永続化され、AOのステートレス実行環境でも効率的に動作します。メッセージ処理ごとにインスタンスが生成される特性を考慮し、必要最小限のデータ構造と選択的なロードパターンにより、スケーラブルな秘密管理を実現します。

---

**Document Status**: Entity Design Specification  
**Version**: 2.0  
**Updates**: 
- AOステートレス実行環境への対応（セクション2.3追加）
- ProcessEntityの軽量化（SecretIndex導入）
- SecretDetailsEntityの追加（セクション9）
- EntityBundle/MessageContextパターンの追加（セクション11.4）
**Next Steps**: Repository Interface設計の実装（repositories.md参照）