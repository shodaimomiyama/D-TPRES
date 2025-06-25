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
    
    /// 管理する秘密群
    /// Key: 秘密ID, Value: 秘密管理データ
    pub managed_secrets: HashMap<String, SecretManagementData>,
    
    /// Owner固有設定
    /// 例: {"default_threshold": "3", "default_shares": "5"}
    pub owner_config: HashMap<String, String>,
}

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
// Phase 0: Owner-Processの生成
let owner_process = ProcessEntity {
    process_id: "ao_process_001".to_string(),
    process_name: "AliceOwnerProcess".to_string(),
    active_roles: vec!["owner".to_string()],
    owner_data: Some(OwnerData {
        owner_public_key: vec![/* pkO bytes */],
        managed_secrets: HashMap::new(),
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

## 9. Entity関連図

```mermaid
graph TB
    subgraph "Phase 0: Process Spawn"
        PE[ProcessEntity]
    end
    
    subgraph "Phase 1: Secret Sharing"
        SE[ShareEntity]
        CE[CapsuleEntity]
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
    
    PE -->|creates| SE
    PE -->|creates| CE
    PE -->|initiates| ARE
    ARE -->|triggers| RFE
    RFE -->|enables| RE
    
    SE -.->|referenced by| RE
    CE -.->|used in| RE
```

## 10. 実装ガイドライン

### 10.1 Entityの実装規約

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

### 10.2 シリアライゼーション

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

### 10.3 フィールド命名規約

| フィールド型 | 命名パターン | 例 |
|------------|------------|---|
| 識別子 | `{entity}_id` | `process_id`, `share_id` |
| 日時 | `{action}_at` | `created_at`, `updated_at` |
| 状態 | `status` | `status: "active"` |
| バージョン | `version` | `version: 1` |
| 公開鍵 | `{owner}_public_key` | `owner_public_key` |
| データ | `{type}_data` | `kfrag_data`, `cfrag_data` |

## 11. まとめ

D-TPRESのEntity設計は以下の特徴を持ちます：

1. **純粋なデータ構造**: メソッドを持たない
2. **PRD準拠**: 各PhaseのワークフローをEntityで正確に表現
3. **型安全性**: 厳密な型定義による安全性確保
4. **シリアライズ対応**: JSON/バイナリ形式での永続化
5. **トレーサビリティ**: 全Entityにタイムスタンプとバージョン

これらのEntityは、Repository層を通じてArweaveに永続化され、D-TPRESシステムの状態を完全に表現します。

---

**Document Status**: Entity Design Specification  
**Version**: 1.0  
**Next Steps**: Repository Interface設計の実装（repositories.md参照）