//! ProcessEntity - Multi-role process representation for AO
//!
//! Each AO process can have multiple roles (Owner/Holder/Requester) simultaneously.
//! Designed for AO's stateless execution environment with lightweight secret indexing.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Process entity - Complete state representation of an AO process
///
/// # Features
/// - Multi-role support (Owner/Holder/Requester can coexist)
/// - AO environment native properties
/// - Integrated performance metrics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProcessEntity {
    pub process_id: String,

    pub process_name: String,

    // 単一プロセスが複数の役割を担うことでネットワーク効率を向上
    // 例: ["owner", "holder"] - 自身の秘密を管理しながら他者のkFragも保持
    pub active_roles: Vec<String>,

    // Phase 0, 1, 3で使用されるOwner機能
    // skO（秘密鍵）の管理と秘密分割を担当
    pub owner_data: Option<OwnerData>,

    // Phase 3, 4で使用されるHolder機能
    // kFragの保管と再暗号化の実行を担当
    pub holder_data: Option<HolderData>,

    // Phase 2, 4, 5で使用されるRequester機能
    // アクセス要求の発行とcFrag収集を担当
    pub requester_data: Option<RequesterData>,

    // プロセス固有の設定をKey-Value形式で柔軟に管理
    // 例: {"max_concurrent_requests": "10", "timeout_seconds": "300"}
    pub configuration: HashMap<String, String>,

    // プロセスがサポートする暗号操作を明示
    // 能力ベースのルーティングとロードバランシングに使用
    pub supported_crypto_operations: Vec<String>,

    pub performance_metrics: PerformanceMetrics,

    pub created_at: u64,

    pub updated_at: u64,

    // AOのステートレス環境で同時更新を検出するための楽観的ロック
    pub version: u64,
}

/// Owner functionality data - Manages skO and secret splitting
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OwnerData {
    // skO（秘密鍵）に対応する公開鍵
    // 秘密鍵自体は保存せず、必要時にセキュアストレージから取得
    pub owner_public_key: Vec<u8>,

    // 軽量な秘密インデックス情報のみを保持
    // AOの頻繁なEntity再構築時のメモリ使用量を削減するため
    // 詳細情報はSecretDetailsEntityで別管理
    pub secret_indices: HashMap<String, SecretIndex>,

    // Owner固有の設定
    // デフォルト値を設定することで、秘密ごとの設定の繰り返しを避ける
    pub owner_config: HashMap<String, String>,
}

/// Secret index - Lightweight secret management information
/// Minimal information held in ProcessEntity
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SecretIndex {
    /// Secret identifier
    pub secret_id: String,

    /// Secret status
    /// Values: "active", "archived", "expired"
    pub status: String,

    /// Entity reference information
    pub entity_references: EntityReferences,

    /// Last update timestamp
    pub last_updated: u64,

    /// Shamir threshold (k) - kept for frequent reference
    pub shamir_threshold: u8,

    /// Shamir total shares (n) - kept for frequent reference
    pub shamir_total_shares: u8,
}

/// Entity references - Holds only IDs of related entities
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntityReferences {
    /// ShareEntity ID list
    pub share_ids: Vec<String>,

    /// CapsuleEntity ID list
    pub capsule_ids: Vec<String>,

    /// Active AccessRequestEntity ID list
    pub active_requests: Vec<String>,

    /// SecretDetailsEntity ID (reference to detailed information)
    pub details_entity_id: String,
}

/// Holder functionality data - kFrag storage and re-encryption execution
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HolderData {
    /// Held kFrag collection
    /// Key: RekeyFragment ID, Value: fragment information
    pub held_fragments: HashMap<String, HolderFragmentInfo>,

    /// kFrag management by access control condition
    /// Key: access control condition, Value: RekeyFragment ID list
    pub fragments_by_condition: HashMap<String, Vec<String>>,

    /// Reliability score (0.0-1.0)
    pub reliability_score: f64,

    /// Completed re-encryption count
    pub completed_reencryptions: u64,

    /// Maximum fragment capacity
    pub max_fragment_capacity: u64,

    /// Current load status (0-100)
    pub current_load: u64,
}

/// Holder fragment information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HolderFragmentInfo {
    /// Fragment identifier
    pub fragment_id: String,

    /// Related secret ID
    pub secret_id: String,

    /// Access control condition
    pub access_control_condition: String,

    /// Receipt timestamp
    pub received_at: u64,

    /// Usage count
    pub usage_count: u64,

    /// Status ("active", "expired", "revoked")
    pub status: String,
}

/// Requester functionality data - Access requests and cFrag collection
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RequesterData {
    /// Active access request ID list
    pub active_requests: Vec<String>,

    /// Active re-encryption request ID list
    pub active_reencryptions: Vec<String>,

    /// Completed request count
    pub completed_requests: u64,

    /// Success rate (0.0-1.0)
    pub success_rate: f64,

    /// Average processing time (milliseconds)
    pub average_processing_time_ms: u64,

    /// Requester-specific configuration
    /// Example: {"retry_attempts": "3", "timeout_ms": "30000"}
    pub requester_config: HashMap<String, String>,
}

/// Performance metrics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Successful operation count
    pub successful_operations: u64,

    /// Failed operation count
    pub failed_operations: u64,

    /// Average response time (milliseconds)
    pub average_response_time_ms: u64,

    /// Last metrics update timestamp
    pub last_updated_at: u64,
}
